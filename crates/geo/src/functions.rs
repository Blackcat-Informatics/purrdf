// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `geof:` scalar-function family, as registrations against the evaluator's
//! native user-function seam.
//!
//! OGC 22-047r1 defines `geof:` as a set of *scalar* functions: each takes
//! already-evaluated argument terms and returns one term. That is exactly the
//! shape of [`UserFunctionRegistry::register_native`], so this module is a table
//! ([`GeofFunction`]) plus one dispatcher ([`evaluate`]) — the whole family walks
//! [`GeofFunction::ALL`] in a fixed order and no map iteration reaches a result.
//!
//! # This seam needs no parser configuration at all
//!
//! A call-position IRI that sits under **no** configured extension-function
//! namespace is lowered by the SPARQL parser to
//! `purrdf_sparql_algebra::Function::Custom` and resolved at
//! *evaluation* time against the injected registry. So a host wires `geof:` by
//! building a registry and passing it in `QueryOptions::functions`; it does not
//! have to declare the `geof:` namespace in `ParserOptions`, and
//! `geof:sfWithin(?a, ?b)` in a query text parses before anything has been
//! registered. That is why this seam is so much cheaper to wire than the relation
//! (property-function) seam, which is admission-checked at *prepare* time and
//! therefore has to agree with the registry the plan was prepared against.
//!
//! # What is registered, and what is loudly not
//!
//! Every function the standard defines is registered, including the ones this
//! crate does not implement. An unimplemented `geof:` call **fails the query by
//! name** ([`GeoError::Unsupported`] ⇒ [`EvalError::Function`]); it never returns
//! a default, never returns `false`, and never returns a plausible wrong
//! geometry. A `false` from an unimplemented topological predicate is
//! indistinguishable from an honest `false`, and there is nothing downstream that
//! can catch it — so the gap is made loud here rather than left silent.
//!
//! The six GeoSPARQL **aggregates** are the one thing deliberately *not*
//! registered here; see [`GeofFunction::AGGREGATES`] for why.
//!
//! # A domain refusal is a per-solution error, not a failed query
//!
//! The opposite mistake is just as expensive, and it is the easier one to make:
//! refusing too much. A `geof:` call whose *arguments* are unusable — a malformed
//! WKT literal, a measure of an empty geometry, two geometries in different
//! coordinate reference systems — is a SPARQL **expression error** (§17.2:
//! "Functions invoked with an argument of the wrong type will produce a type
//! error"), which the enclosing operator resolves per its own context: a `FILTER`
//! drops that one solution, a `BIND`/`SELECT` expression leaves the variable
//! unbound, and every other row is answered normally. Aborting the query instead
//! would mean a single bad geometry anywhere in a dataset makes every query that
//! scans past it fail — which is the mirror image of the silent-`false` bug above,
//! and no less wrong. [`evaluate`] is where the two distances are separated, by
//! asking [`GeoError::is_expression_error`]; [`compute`] is the same computation
//! with the refusal itself still in hand.
//!
//! # Volatility
//!
//! Every registration is [`Volatility::Stable`]. Each body is pure arithmetic
//! over its own argument terms: it reads no dataset, no clock, no RNG, and no
//! external index, and the arithmetic is the exact integer arithmetic of
//! [`crate::exact`], so two calls with equal arguments produce byte-identical
//! results on every target. `Stable` is the strongest class this seam offers —
//! there is no `Immutable` variant — and it is what lets the fork-join gate run a
//! `geof:` call across workers, which is the whole point of declaring it.
//!
//! # The single float boundary
//!
//! The crate root denies `clippy::float_arithmetic`, and this module honours it:
//! the only `f64` that exists anywhere in here is the one
//! [`crate::exact::Rat::to_f64`] assembles with integer arithmetic and
//! [`f64::from_bits`], handed straight to [`purrdf_xsd::numeric::canonical_double`] to be
//! rendered. No value is added, multiplied or compared as a float.

use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_sparql_eval::{Arity, EvalError, NativeFnBody, UserFunctionRegistry, Volatility};
use purrdf_xsd::numeric::canonical_double;
use purrdf_xsd::{XSD_NS, XsdDatatype};

use crate::de9im::{Dim, IntersectionMatrix, Slot};
use crate::error::GeoError;
use crate::exact::Rat;
use crate::geom::{CoordDim, Geometry, GeometryBody, GeometryLiteral};
use crate::relations::SpatialRelation;
use crate::topology::{relate, topological_dimension};
use crate::vocab::{GeoTerm, GeoVocab};
use crate::{construct, geojson, measure, wkt};

// ---------------------------------------------------------------------------
// The inventory
// ---------------------------------------------------------------------------

/// One `geof:` scalar function.
///
/// The variant order is the order [`Self::ALL`] registers them in, and it is the
/// order the standard's own clauses list them in: the twenty-four topological
/// relations, `geof:relate`, the accessors, the measures, the constructors, the
/// serializations, and finally the twelve that are registered but unimplemented.
///
/// The topological family is [`Self::Relation`] wrapping a
/// [`SpatialRelation`] rather than twenty-four fresh variants, so the `geof:`
/// local names cannot drift from the `geo:` property local names — the standard
/// uses one local name for both (Tables 9, 10 and 11) and this type reuses the
/// one table that spells them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GeofFunction {
    /// One of the twenty-four topological relations, as a two-argument
    /// `(geomLiteral, geomLiteral) -> xsd:boolean` function.
    Relation(SpatialRelation),
    /// `geof:relate(geom1, geom2, pattern) -> xsd:boolean`.
    Relate,

    /// `geof:dimension(geom) -> xsd:integer`.
    Dimension,
    /// `geof:coordinateDimension(geom) -> xsd:integer`.
    CoordinateDimension,
    /// `geof:spatialDimension(geom) -> xsd:integer`.
    SpatialDimension,
    /// `geof:geometryType(geom) -> xsd:anyURI`.
    GeometryType,
    /// `geof:isEmpty(geom) -> xsd:boolean`.
    IsEmpty,
    /// `geof:isSimple(geom) -> xsd:boolean`.
    IsSimple,
    /// `geof:is3D(geom) -> xsd:boolean`.
    Is3D,
    /// `geof:isMeasured(geom) -> xsd:boolean`.
    IsMeasured,
    /// `geof:getSRID(geom) -> xsd:anyURI`.
    GetSrid,
    /// `geof:numGeometries(geom) -> xsd:integer`.
    NumGeometries,
    /// `geof:geometryN(geom, n) -> ogc:geomLiteral`, 1-based.
    GeometryN,

    /// `geof:minX(geom) -> xsd:double`.
    MinX,
    /// `geof:maxX(geom) -> xsd:double`.
    MaxX,
    /// `geof:minY(geom) -> xsd:double`.
    MinY,
    /// `geof:maxY(geom) -> xsd:double`.
    MaxY,
    /// `geof:minZ(geom) -> xsd:double`.
    MinZ,
    /// `geof:maxZ(geom) -> xsd:double`.
    MaxZ,

    /// `geof:envelope(geom) -> ogc:geomLiteral`.
    Envelope,
    /// `geof:boundary(geom) -> ogc:geomLiteral`.
    Boundary,
    /// `geof:convexHull(geom) -> ogc:geomLiteral`.
    ConvexHull,
    /// `geof:centroid(geom) -> ogc:geomLiteral`.
    Centroid,

    /// `geof:area(geom, units) -> xsd:double`.
    Area,
    /// `geof:metricArea(geom) -> xsd:double`.
    MetricArea,
    /// `geof:length(geom, units) -> xsd:double`.
    Length,
    /// `geof:metricLength(geom) -> xsd:double`.
    MetricLength,
    /// `geof:perimeter(geom, unit) -> xsd:double`.
    ///
    /// The standard spells this parameter `unit`, singular, unlike `units` on
    /// every sibling. That is a spelling of the *specification's prose*, not of
    /// anything positional: SPARQL arguments are positional, so the parameter's
    /// name is invisible to a caller and irrelevant to this implementation. It is
    /// recorded here only so a reader diffing this file against Clause 11 does not
    /// read the difference as a transcription slip.
    Perimeter,
    /// `geof:metricPerimeter(geom) -> xsd:double`.
    MetricPerimeter,
    /// `geof:distance(geom1, geom2, units) -> xsd:double`.
    Distance,
    /// `geof:metricDistance(geom1, geom2) -> xsd:double`.
    MetricDistance,

    /// `geof:asWKT(geom) -> geo:wktLiteral`.
    AsWkt,
    /// `geof:asGeoJSON(geom) -> geo:geoJSONLiteral`.
    AsGeoJson,

    /// `geof:transform` — registered, unimplemented, hard-errors by name.
    Transform,
    /// `geof:buffer` — registered, unimplemented, hard-errors by name.
    Buffer,
    /// `geof:metricBuffer` — registered, unimplemented, hard-errors by name.
    MetricBuffer,
    /// `geof:boundingCircle` — registered, unimplemented, hard-errors by name.
    BoundingCircle,
    /// `geof:concaveHull` — registered, unimplemented, hard-errors by name.
    ConcaveHull,
    /// `geof:intersection` — registered, unimplemented, hard-errors by name.
    Intersection,
    /// `geof:union` — registered, unimplemented, hard-errors by name.
    Union,
    /// `geof:difference` — registered, unimplemented, hard-errors by name.
    Difference,
    /// `geof:symDifference` — registered, unimplemented, hard-errors by name.
    SymDifference,
    /// `geof:asGML` — registered, unimplemented, hard-errors by name.
    AsGml,
    /// `geof:asKML` — registered, unimplemented, hard-errors by name.
    AsKml,
    /// `geof:asDGGS` — registered, unimplemented, hard-errors by name.
    AsDggs,
}

/// Everything after the twenty-four topological relations, in registration order.
///
/// Split out from [`GeofFunction::ALL`] so that the relation prefix can be copied
/// straight from [`SpatialRelation::ALL`] in a `const fn` rather than retyped.
const TAIL: [GeofFunction; 44] = [
    GeofFunction::Relate,
    GeofFunction::Dimension,
    GeofFunction::CoordinateDimension,
    GeofFunction::SpatialDimension,
    GeofFunction::GeometryType,
    GeofFunction::IsEmpty,
    GeofFunction::IsSimple,
    GeofFunction::Is3D,
    GeofFunction::IsMeasured,
    GeofFunction::GetSrid,
    GeofFunction::NumGeometries,
    GeofFunction::GeometryN,
    GeofFunction::MinX,
    GeofFunction::MaxX,
    GeofFunction::MinY,
    GeofFunction::MaxY,
    GeofFunction::MinZ,
    GeofFunction::MaxZ,
    GeofFunction::Envelope,
    GeofFunction::Boundary,
    GeofFunction::ConvexHull,
    GeofFunction::Centroid,
    GeofFunction::Area,
    GeofFunction::MetricArea,
    GeofFunction::Length,
    GeofFunction::MetricLength,
    GeofFunction::Perimeter,
    GeofFunction::MetricPerimeter,
    GeofFunction::Distance,
    GeofFunction::MetricDistance,
    GeofFunction::AsWkt,
    GeofFunction::AsGeoJson,
    GeofFunction::Transform,
    GeofFunction::Buffer,
    GeofFunction::MetricBuffer,
    GeofFunction::BoundingCircle,
    GeofFunction::ConcaveHull,
    GeofFunction::Intersection,
    GeofFunction::Union,
    GeofFunction::Difference,
    GeofFunction::SymDifference,
    GeofFunction::AsGml,
    GeofFunction::AsKml,
    GeofFunction::AsDggs,
];

/// Build [`GeofFunction::ALL`] at compile time: the twenty-four relations in
/// [`SpatialRelation::ALL`]'s order, then [`TAIL`].
const fn all_functions() -> [GeofFunction; GeofFunction::COUNT] {
    let mut out = [GeofFunction::Relate; GeofFunction::COUNT];
    let mut index = 0;
    while index < SpatialRelation::ALL.len() {
        out[index] = GeofFunction::Relation(SpatialRelation::ALL[index]);
        index += 1;
    }
    let mut tail = 0;
    while tail < TAIL.len() {
        out[SpatialRelation::ALL.len() + tail] = TAIL[tail];
        tail += 1;
    }
    out
}

impl GeofFunction {
    /// How many `geof:` scalar functions this crate registers.
    ///
    /// Twenty-four topological relations, plus `geof:relate`, plus the
    /// thirty-one implemented accessors, measures, constructors and
    /// serializations, plus the twelve that are registered so that they can fail
    /// by name.
    pub const COUNT: usize = 68;

    /// Every registered function, in a fixed order.
    pub const ALL: [Self; Self::COUNT] = all_functions();

    /// The six GeoSPARQL aggregates, which are **deliberately not registered
    /// here**.
    ///
    /// `geof:aggBoundingBox`, `geof:aggBoundingCircle`, `geof:aggCentroid`,
    /// `geof:aggConcaveHull`, `geof:aggConvexHull` and `geof:aggUnion` are SPARQL
    /// **aggregates**: each folds a whole solution group into one value, and a
    /// call is written `AGG(<iri>, ?g)` against
    /// [`AggregateRegistry`](purrdf_sparql_eval::AggregateRegistry), not
    /// `<iri>(?g)` against this one. Registering them as scalar functions would
    /// make `geof:aggUnion(?g)` resolve, accept one row's geometry, and answer
    /// something — which is the silent-wrong-answer channel this crate exists to
    /// keep closed, dressed up as a working feature. They belong on the aggregate
    /// seam and are listed here only so that a reader can see the whole `geof:`
    /// surface in one place and see that these six are accounted for rather than
    /// forgotten.
    pub const AGGREGATES: [&'static str; 6] = [
        "aggBoundingBox",
        "aggBoundingCircle",
        "aggCentroid",
        "aggConcaveHull",
        "aggConvexHull",
        "aggUnion",
    ];

    /// This function's local name within the `geof:` namespace, byte-exact.
    #[must_use]
    pub const fn local_name(self) -> &'static str {
        match self {
            Self::Relation(relation) => relation.local_name(),
            Self::Relate => "relate",
            Self::Dimension => "dimension",
            Self::CoordinateDimension => "coordinateDimension",
            Self::SpatialDimension => "spatialDimension",
            Self::GeometryType => "geometryType",
            Self::IsEmpty => "isEmpty",
            Self::IsSimple => "isSimple",
            Self::Is3D => "is3D",
            Self::IsMeasured => "isMeasured",
            Self::GetSrid => "getSRID",
            Self::NumGeometries => "numGeometries",
            Self::GeometryN => "geometryN",
            Self::MinX => "minX",
            Self::MaxX => "maxX",
            Self::MinY => "minY",
            Self::MaxY => "maxY",
            Self::MinZ => "minZ",
            Self::MaxZ => "maxZ",
            Self::Envelope => "envelope",
            Self::Boundary => "boundary",
            Self::ConvexHull => "convexHull",
            Self::Centroid => "centroid",
            Self::Area => "area",
            Self::MetricArea => "metricArea",
            Self::Length => "length",
            Self::MetricLength => "metricLength",
            Self::Perimeter => "perimeter",
            Self::MetricPerimeter => "metricPerimeter",
            Self::Distance => "distance",
            Self::MetricDistance => "metricDistance",
            Self::AsWkt => "asWKT",
            Self::AsGeoJson => "asGeoJSON",
            Self::Transform => "transform",
            Self::Buffer => "buffer",
            Self::MetricBuffer => "metricBuffer",
            Self::BoundingCircle => "boundingCircle",
            Self::ConcaveHull => "concaveHull",
            Self::Intersection => "intersection",
            Self::Union => "union",
            Self::Difference => "difference",
            Self::SymDifference => "symDifference",
            Self::AsGml => "asGML",
            Self::AsKml => "asKML",
            Self::AsDggs => "asDGGS",
        }
    }

    /// The function with this local name, or `None`.
    #[must_use]
    pub fn from_local_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.local_name() == name)
    }

    /// The declared argument count, which the seam checks **before** the body
    /// runs.
    ///
    /// `geof:asGML` and `geof:asDGGS` are [`Arity::AtLeast`] rather than exact
    /// because the standard gives each an optional trailing namespace/version
    /// argument. Both are unimplemented, and the wide arity is deliberate: it
    /// makes the refusal name the *function* rather than the argument count, so a
    /// caller learns that the serialization is missing rather than that they
    /// miscounted.
    #[must_use]
    pub const fn arity(self) -> Arity {
        match self {
            Self::Relation(_)
            | Self::GeometryN
            | Self::Area
            | Self::Length
            | Self::Perimeter
            | Self::MetricDistance
            | Self::Transform
            | Self::MetricBuffer
            | Self::Intersection
            | Self::Union
            | Self::Difference
            | Self::SymDifference => Arity::Exact(2),
            Self::Relate | Self::Distance | Self::Buffer | Self::ConcaveHull => Arity::Exact(3),
            Self::AsGml | Self::AsDggs => Arity::AtLeast(1),
            Self::Dimension
            | Self::CoordinateDimension
            | Self::SpatialDimension
            | Self::GeometryType
            | Self::IsEmpty
            | Self::IsSimple
            | Self::Is3D
            | Self::IsMeasured
            | Self::GetSrid
            | Self::NumGeometries
            | Self::MinX
            | Self::MaxX
            | Self::MinY
            | Self::MaxY
            | Self::MinZ
            | Self::MaxZ
            | Self::Envelope
            | Self::Boundary
            | Self::ConvexHull
            | Self::Centroid
            | Self::MetricArea
            | Self::MetricLength
            | Self::MetricPerimeter
            | Self::AsWkt
            | Self::AsGeoJson
            | Self::BoundingCircle
            | Self::AsKml => Arity::Exact(1),
        }
    }

    /// Why this function is not implemented, or `None` when it is.
    ///
    /// This is the single table the refusal message is built from, so the reason a
    /// caller reads and the reason a maintainer reads cannot drift apart. Each one
    /// names a *facility this crate deliberately does not have*, not a task nobody
    /// got to: the honest content of the refusal is what the caller needs in order
    /// to choose another engine for that query.
    #[must_use]
    pub const fn unsupported_reason(self) -> Option<&'static str> {
        Some(match self {
            Self::Transform => {
                "reprojecting coordinates between reference systems requires a \
                 coordinate-reference-system database, and purrdf-geo ships none — it carries, \
                 compares and reports a CRS but converts no ordinate, so any geometry returned \
                 here would be the input's numbers wearing another system's name"
            }
            Self::Buffer => {
                "a buffer's boundary is a curve, and rendering it as a polygon requires a segment \
                 count and a join style that OGC 22-047r1 leaves implementation-defined; the \
                 shape returned would be this implementation's invention rather than the \
                 specified answer"
            }
            Self::MetricBuffer => {
                "a metric buffer needs both the curve approximation geof:buffer needs (whose \
                 parameters OGC 22-047r1 leaves implementation-defined) and a geodesic in metres; \
                 purrdf-geo has neither, and an approximated ring would be an invented shape"
            }
            Self::BoundingCircle => {
                "the smallest enclosing circle is a curve, and rendering it as a geometry \
                 requires a segment count OGC 22-047r1 leaves implementation-defined; the polygon \
                 returned would be this implementation's invention rather than the circle"
            }
            Self::ConcaveHull => {
                "OGC 22-047r1 states explicitly that the concave hull's parameters are \
                 implementation-defined, so any hull returned here would be purrdf-geo's own \
                 answer rather than the specified one — and a plausible wrong polygon is exactly \
                 the failure this crate refuses to produce"
            }
            Self::Intersection | Self::Union | Self::Difference | Self::SymDifference => {
                "a set operation on two geometries requires a planar overlay — a full noding, \
                 labelling and ring-assembly engine — which purrdf-geo does not implement in this \
                 pass; there is no partial answer that would not be a wrong geometry"
            }
            Self::AsGml | Self::AsKml | Self::AsDggs => {
                "purrdf-geo implements the WKT and GeoJSON serializations only; this encoding is \
                 not implemented, and emitting a different encoding under this name would give \
                 the result a datatype its lexical form does not satisfy"
            }
            Self::Relation(_)
            | Self::Relate
            | Self::Dimension
            | Self::CoordinateDimension
            | Self::SpatialDimension
            | Self::GeometryType
            | Self::IsEmpty
            | Self::IsSimple
            | Self::Is3D
            | Self::IsMeasured
            | Self::GetSrid
            | Self::NumGeometries
            | Self::GeometryN
            | Self::MinX
            | Self::MaxX
            | Self::MinY
            | Self::MaxY
            | Self::MinZ
            | Self::MaxZ
            | Self::Envelope
            | Self::Boundary
            | Self::ConvexHull
            | Self::Centroid
            | Self::Area
            | Self::MetricArea
            | Self::Length
            | Self::MetricLength
            | Self::Perimeter
            | Self::MetricPerimeter
            | Self::Distance
            | Self::MetricDistance
            | Self::AsWkt
            | Self::AsGeoJson => return None,
        })
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register every implemented and every deliberately-unsupported `geof:`
/// function against `vocab`'s function namespace.
///
/// The walk is over [`GeofFunction::ALL`] in its fixed order, so the set and the
/// order of registrations are a pure function of the table above rather than of
/// any map's iteration order. Each closure captures its own clone of `vocab` —
/// the body must be `'static` inside the `Arc`, and a `GeoVocab` is a handful of
/// owned strings, cloned once per function at wiring time and never on a
/// per-row path.
///
/// Every function is registered [`Volatility::Stable`]: see the module docs for
/// the justification.
pub fn register(registry: &mut UserFunctionRegistry, vocab: &GeoVocab) {
    for function in GeofFunction::ALL {
        let iri = vocab.function(function.local_name());
        let captured = vocab.clone();
        let body: NativeFnBody =
            Arc::new(move |args: &[&TermValue]| evaluate(function, &captured, args));
        registry.register_native(iri, function.arity(), Volatility::Stable, body);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Run one `geof:` function over already-evaluated argument terms, in the
/// evaluator's own three-exit shape.
///
/// This is literally the body [`register`] installs, exposed so a caller (and
/// this module's own tests) can exercise a function without standing up an
/// evaluator. Use [`compute`] instead when you want the refusal itself rather than
/// what the SPARQL seam does with it.
///
/// The native seam checks [`GeofFunction::arity`] *before* it invokes a body and
/// short-circuits an unbound argument before that, so a registered call always
/// arrives with the right number of bound terms. This function re-checks the
/// count anyway, because it is public and a direct caller has no such guarantee —
/// and an out-of-bounds index would be a panic, which is precisely what a seam
/// body must never do.
///
/// # Which refusals travel how far
///
/// The seam has two failure distances and this function is where a [`GeoError`]
/// picks one, by asking [`GeoError::is_expression_error`] (which is where the
/// reasoning lives):
///
/// * `Ok(None)` — a **per-solution** SPARQL expression error, for a malformed or
///   wrongly-typed geometry literal and for a domain refusal (mixed coordinate
///   reference systems, a measure of an empty geometry, an out-of-range member
///   index, an undeclared unit). SPARQL 1.1 §17.2: "Functions invoked with an
///   argument of the wrong type will produce a type error." The enclosing operator
///   then decides the outcome — a `FILTER` drops that solution, a `BIND` or
///   `SELECT` expression leaves the variable unbound (§10, algebra §18.5 `Extend`)
///   — and the rest of the query is unaffected. One bad geometry in one row is not
///   a failed query.
/// * `Err` — **query-fatal**, and deliberately still so for the three kinds that
///   hold for every solution alike: a wrong argument count, an unimplemented
///   function, and an unusable vocabulary. An unimplemented topological predicate
///   that answered "no value" would be dropped by a `FILTER` exactly as an honest
///   `false` is, which is the silent wrong answer this crate refuses to produce;
///   the other two would empty a result set and present that as the answer.
///
/// # Errors
///
/// [`EvalError::Function`] for an unimplemented function or a wrong argument count;
/// [`EvalError::Config`] for a vocabulary that is missing a term the call needs. See
/// [`GeoError`] for which kind means whose mistake it was, and [`compute`] for the
/// refusal itself rather than its seam-level effect.
pub fn evaluate(
    function: GeofFunction,
    vocab: &GeoVocab,
    args: &[&TermValue],
) -> Result<Option<TermValue>, EvalError> {
    match compute(function, vocab, args) {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.is_expression_error() => Ok(None),
        Err(err) => Err(EvalError::from(err)),
    }
}

/// Run one `geof:` function over already-evaluated argument terms, in this crate's
/// own error space.
///
/// The same computation [`evaluate`] performs, stopping one step earlier: every
/// refusal arrives as the [`GeoError`] that was raised, with its detail message
/// intact and its kind still legible. [`evaluate`] is the seam shape — it maps two
/// of the five kinds onto SPARQL's per-solution `Ok(None)`, which by construction
/// carries no message, because the specification gives an expression error nowhere
/// to put one. A caller that wants to *report* why a geometry was rejected (a
/// loader, a linter, a diagnostic pass) must call this; a caller that wants what a
/// query would do must call [`evaluate`].
///
/// # Errors
///
/// [`GeoError::Arity`] for a wrong argument count, [`GeoError::Unsupported`] for a
/// function this crate does not implement, [`GeoError::Literal`] for a malformed or
/// wrongly-typed geometry literal, [`GeoError::Domain`] for well-formed arguments the
/// operation is undefined on, and [`GeoError::Config`] for a vocabulary missing a
/// term the call needs.
pub fn compute(
    function: GeofFunction,
    vocab: &GeoVocab,
    args: &[&TermValue],
) -> Result<TermValue, GeoError> {
    if !arity_accepts(function.arity(), args.len()) {
        // Not an expression error: a call written with the wrong number of
        // arguments cannot be evaluated for ANY solution, so softening it would
        // turn a defect in the query text into a silently empty answer.
        return Err(GeoError::arity(format!(
            "geof:{} expects {} argument(s), got {}",
            function.local_name(),
            function.arity(),
            args.len()
        )));
    }
    dispatch(function, vocab, args)
}

/// Whether `count` arguments satisfies `arity`.
///
/// A local re-statement rather than a call, because
/// [`Arity::accepts`](purrdf_sparql_eval::Arity) is `pub(crate)` to the evaluator.
/// The three variants are total, so this cannot silently disagree with the seam's
/// own check by omission.
const fn arity_accepts(arity: Arity, count: usize) -> bool {
    match arity {
        Arity::Exact(n) => count == n,
        Arity::Range { min, max } => min <= count && count <= max,
        Arity::AtLeast(n) => count >= n,
    }
}

/// The body of [`evaluate`], in this crate's own error space.
fn dispatch(
    function: GeofFunction,
    vocab: &GeoVocab,
    args: &[&TermValue],
) -> Result<TermValue, GeoError> {
    match function {
        GeofFunction::Relation(relation) => {
            let (a, b) = geometry_pair(vocab, args)?;
            let matrix = relate(a.geometry(), b.geometry());
            Ok(bool_term(relation.holds(
                &matrix,
                topological_dimension(a.geometry()),
                topological_dimension(b.geometry()),
            )))
        }
        GeofFunction::Relate => {
            let (a, b) = geometry_pair(vocab, args)?;
            let slots = parse_relate_pattern(string_arg(args[2])?)?;
            let matrix = relate(a.geometry(), b.geometry());
            Ok(bool_term(matrix_matches(&matrix, &slots)))
        }

        GeofFunction::Dimension => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(integer_term(i64::from(topological_dimension(
                literal.geometry(),
            ))))
        }
        GeofFunction::CoordinateDimension => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(count_term(literal.geometry().dim().ordinates()))
        }
        GeofFunction::SpatialDimension => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(count_term(spatial_dimension(literal.geometry().dim())))
        }
        GeofFunction::GeometryType => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(any_uri_term(
                vocab.simple_features_type(literal.geometry().kind())?,
            ))
        }
        GeofFunction::IsEmpty => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(bool_term(literal.geometry().is_empty()))
        }
        GeofFunction::IsSimple => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(bool_term(measure::is_simple(literal.geometry())))
        }
        GeofFunction::Is3D => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(bool_term(literal.geometry().dim().has_z()))
        }
        GeofFunction::IsMeasured => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(bool_term(literal.geometry().dim().has_m()))
        }
        GeofFunction::GetSrid => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(any_uri_term(literal.crs().as_str().to_owned()))
        }
        GeofFunction::NumGeometries => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(count_term(measure::num_geometries(literal.geometry())))
        }
        GeofFunction::GeometryN => {
            let literal = geometry_arg(vocab, args[0])?;
            let index = integer_arg(args[1])?;
            let member = usize::try_from(index)
                .ok()
                .and_then(|n| measure::geometry_n(literal.geometry(), n))
                .ok_or_else(|| {
                    GeoError::domain(format!(
                        "geof:geometryN was asked for member {index} of a geometry with {} \
                         member(s); the index is 1-based, and outside that range there is no \
                         member — answering with an empty geometry would be a different question's \
                         answer",
                        measure::num_geometries(literal.geometry())
                    ))
                })?;
            Ok(wkt_term(
                &GeometryLiteral::new(literal.crs().clone(), member),
                vocab,
            ))
        }

        GeofFunction::MinX => planar_bound(vocab, args[0], "minX", measure::min_x),
        GeofFunction::MaxX => planar_bound(vocab, args[0], "maxX", measure::max_x),
        GeofFunction::MinY => planar_bound(vocab, args[0], "minY", measure::min_y),
        GeofFunction::MaxY => planar_bound(vocab, args[0], "maxY", measure::max_y),
        GeofFunction::MinZ => elevation_bound(vocab, args[0], "minZ", measure::min_z),
        GeofFunction::MaxZ => elevation_bound(vocab, args[0], "maxZ", measure::max_z),

        GeofFunction::Envelope => derived_geometry(vocab, args[0], construct::envelope),
        GeofFunction::Boundary => derived_geometry(vocab, args[0], construct::boundary),
        GeofFunction::ConvexHull => derived_geometry(vocab, args[0], construct::convex_hull),
        GeofFunction::Centroid => {
            let literal = geometry_arg(vocab, args[0])?;
            // `POINT EMPTY` for an empty input is a real answer, not a fabricated
            // one: the centroid of the empty set is the empty point set, and it
            // states no location. Inventing an origin would.
            let body = GeometryBody::Point(construct::centroid(literal.geometry()));
            let geometry = Geometry::new(CoordDim::Xy, body)?;
            Ok(wkt_term(
                &GeometryLiteral::new(literal.crs().clone(), geometry),
                vocab,
            ))
        }

        GeofFunction::Area => unit_measure(vocab, args, measure::area),
        GeofFunction::MetricArea => metric_measure(vocab, args, measure::area),
        GeofFunction::Length => unit_measure(vocab, args, measure::length),
        GeofFunction::MetricLength => metric_measure(vocab, args, measure::length),
        GeofFunction::Perimeter => unit_measure(vocab, args, measure::perimeter),
        GeofFunction::MetricPerimeter => metric_measure(vocab, args, measure::perimeter),
        GeofFunction::Distance => {
            let (a, b) = geometry_pair(vocab, args)?;
            vocab.require_unit(a.crs(), unit_arg(args[2])?)?;
            distance_term(&a, &b)
        }
        GeofFunction::MetricDistance => {
            let (a, b) = geometry_pair(vocab, args)?;
            vocab.require_metre(a.crs())?;
            distance_term(&a, &b)
        }

        GeofFunction::AsWkt => {
            let literal = geometry_arg(vocab, args[0])?;
            Ok(wkt_term(&literal, vocab))
        }
        GeofFunction::AsGeoJson => {
            let literal = geometry_arg(vocab, args[0])?;
            // `geojson::write` is the refusal site for a geometry outside the
            // GeoJSON system: RFC 7946 admits exactly one, this crate reprojects
            // nothing, and its message names both systems.
            let lexical = geojson::write(&literal, vocab.geojson_crs(), vocab.coordinate_scale())?;
            Ok(TermValue::typed_literal(
                lexical,
                vocab.term(GeoTerm::GeoJsonLiteral),
            ))
        }

        GeofFunction::Transform
        | GeofFunction::Buffer
        | GeofFunction::MetricBuffer
        | GeofFunction::BoundingCircle
        | GeofFunction::ConcaveHull
        | GeofFunction::Intersection
        | GeofFunction::Union
        | GeofFunction::Difference
        | GeofFunction::SymDifference
        | GeofFunction::AsGml
        | GeofFunction::AsKml
        | GeofFunction::AsDggs => Err(unsupported_error(function)),
    }
}

/// The refusal an unimplemented function answers with, built from the single
/// reason table on [`GeofFunction::unsupported_reason`].
fn unsupported_error(function: GeofFunction) -> GeoError {
    match function.unsupported_reason() {
        Some(reason) => GeoError::unsupported(format!(
            "geof:{} is defined by OGC 22-047r1 and purrdf-geo does not implement it: {reason}",
            function.local_name()
        )),
        // Unreachable as written — the dispatch arm that calls this lists exactly
        // the variants the table answers `Some` for — but an error rather than a
        // panic, so that a future edit which breaks that pairing surfaces as a
        // named bug report instead of a caught panic with no detail.
        None => GeoError::unsupported(format!(
            "geof:{} reached purrdf-geo's unimplemented branch although it is implemented; this \
             is a purrdf-geo bug rather than a property of the arguments",
            function.local_name()
        )),
    }
}

// ---------------------------------------------------------------------------
// Argument readers
// ---------------------------------------------------------------------------

/// Read a geometry-literal argument.
///
/// The datatype decides the codec, and it decides it against the **caller's**
/// vocabulary: `geo:wktLiteral` is parsed as WKT against the vocabulary's default
/// coordinate reference system (a literal may override it with an explicit
/// `<IRI>` prefix), and `geo:geoJSONLiteral` as GeoJSON against the one system
/// RFC 7946 admits.
///
/// A plain `xsd:string` is **not** accepted, and that is not pedantry: a geometry
/// literal's datatype is what makes its lexical form a geometry rather than text
/// that happens to look like one, and a store that has lost the datatype has lost
/// the fact. The refusal names the datatype that did arrive so the gap is
/// diagnosable.
///
/// # Errors
///
/// [`GeoError::Unsupported`] for `geo:gmlLiteral`, `geo:kmlLiteral` and
/// `geo:dggsLiteral` — spec-defined datatypes whose codecs this crate does not
/// have — naming the datatype. [`GeoError::Literal`] for any other datatype, for
/// a non-literal term, and for a well-typed literal whose lexical form is
/// malformed.
pub fn geometry_arg(vocab: &GeoVocab, value: &TermValue) -> Result<GeometryLiteral, GeoError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    else {
        return Err(GeoError::literal(format!(
            "a geof: function takes a geometry literal and {} arrived; a geometry's datatype is \
             what makes its lexical form a geometry rather than text, so purrdf-geo refuses \
             rather than guessing a codec",
            describe(value)
        )));
    };
    // `term_of` is a prefix strip plus a table scan over fixed strings, so this
    // dispatch allocates nothing on the per-row path.
    match vocab.term_of(datatype) {
        Some(GeoTerm::WktLiteral) => wkt::parse(lexical_form, vocab.default_wkt_crs()),
        Some(GeoTerm::GeoJsonLiteral) => geojson::parse(lexical_form, vocab.geojson_crs()),
        Some(term @ (GeoTerm::GmlLiteral | GeoTerm::KmlLiteral | GeoTerm::DggsLiteral)) => {
            Err(GeoError::unsupported(format!(
                "<{datatype}> is the geo:{} datatype, and purrdf-geo implements the WKT and \
                 GeoJSON codecs only; it refuses rather than reading the lexical form as a \
                 serialization it is not",
                term.local_name()
            )))
        }
        _ => Err(GeoError::literal(format!(
            "<{datatype}> is not a geometry datatype; a geof: function takes a geo:wktLiteral or \
             a geo:geoJSONLiteral, and an xsd:string carrying the same characters is text rather \
             than a geometry"
        ))),
    }
}

/// Read the two geometry arguments of a binary function and check that they share
/// a coordinate reference system.
///
/// One helper rather than three lines repeated in each of the four binary arms
/// (`Relation`, `Relate`, `Distance`, `MetricDistance`), so the same-CRS check
/// cannot be present in three of them and absent from the fourth — an omission
/// that would silently compare ordinates from two different spaces.
fn geometry_pair(
    vocab: &GeoVocab,
    args: &[&TermValue],
) -> Result<(GeometryLiteral, GeometryLiteral), GeoError> {
    let a = geometry_arg(vocab, args[0])?;
    let b = geometry_arg(vocab, args[1])?;
    a.require_same_crs(&b)?;
    Ok((a, b))
}

/// Read a units argument.
///
/// A units argument is `xsd:anyURI` in the standard's signature, but a SPARQL
/// query that writes `geof:area(?g, unit:metre)` passes an **IRI term**, not a
/// literal, and both spellings mean the same unit. So both are accepted, as is an
/// `xsd:string` carrying the IRI — the value is compared against the caller's own
/// declaration by [`GeoVocab::require_unit`], which refuses a unit that was never
/// declared, so a permissive read here cannot turn into a wrong measurement.
///
/// # Errors
///
/// [`GeoError::Literal`] naming what arrived, for a blank node, a triple term, a
/// language-tagged literal, or a literal of any other datatype.
fn unit_arg(value: &TermValue) -> Result<&str, GeoError> {
    match value {
        TermValue::Iri(iri) => Ok(iri),
        TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            ..
        } if is_any_uri(datatype) || datatype == XsdDatatype::String.iri() => Ok(lexical_form),
        other => Err(GeoError::literal(format!(
            "a units argument is a unit IRI (as an IRI term, an xsd:anyURI or an xsd:string) and \
             {} arrived",
            describe(other)
        ))),
    }
}

/// Read an `xsd:integer`-family argument.
///
/// # Errors
///
/// [`GeoError::Literal`] naming what arrived, for a non-literal, a literal whose
/// datatype is not in the XSD integer family, or a lexical form outside `i64`.
fn integer_arg(value: &TermValue) -> Result<i64, GeoError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        ..
    } = value
    else {
        return Err(GeoError::literal(format!(
            "an integer argument was expected and {} arrived",
            describe(value)
        )));
    };
    let is_integer_family = XsdDatatype::from_iri(datatype)
        .and_then(XsdDatatype::integer_range)
        .is_some();
    if !is_integer_family {
        return Err(GeoError::literal(format!(
            "an integer argument was expected and a literal of datatype <{datatype}> arrived"
        )));
    }
    lexical_form.parse::<i64>().map_err(|_| {
        GeoError::literal(format!(
            "{lexical_form:?} is not an integer purrdf-geo can index with"
        ))
    })
}

/// Read an `xsd:string` argument.
///
/// # Errors
///
/// [`GeoError::Literal`] naming what arrived, for anything that is not a
/// plain (non-language-tagged) `xsd:string` literal.
fn string_arg(value: &TermValue) -> Result<&str, GeoError> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            ..
        } if datatype == XsdDatatype::String.iri() => Ok(lexical_form),
        other => Err(GeoError::literal(format!(
            "an xsd:string argument was expected and {} arrived",
            describe(other)
        ))),
    }
}

/// Name what a term is, for a refusal message.
fn describe(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("the IRI <{iri}>"),
        TermValue::Blank { label, .. } => format!("the blank node _:{label}"),
        TermValue::Triple { .. } => "a triple term".to_owned(),
        TermValue::Literal {
            datatype,
            language: Some(tag),
            ..
        } => format!("a literal tagged @{tag} (datatype <{datatype}>)"),
        TermValue::Literal { datatype, .. } => format!("a literal of datatype <{datatype}>"),
    }
}

// ---------------------------------------------------------------------------
// Result builders
// ---------------------------------------------------------------------------

/// The `xsd:anyURI` datatype IRI.
///
/// Assembled from [`purrdf_xsd::XSD_NS`] rather than written out, so the
/// namespace has exactly one spelling in the workspace. [`XsdDatatype`] has no
/// `anyURI` variant — it models the *value space* datatypes, and `xsd:anyURI` has
/// no value-space behaviour there — so this is the one datatype IRI this module
/// builds by hand.
fn any_uri_iri() -> String {
    format!("{XSD_NS}anyURI")
}

/// Whether `datatype` is `xsd:anyURI`, decided without building the IRI — this
/// sits on the per-row measurement path, and a comparison is not worth an
/// allocation.
fn is_any_uri(datatype: &str) -> bool {
    datatype.strip_prefix(XSD_NS) == Some("anyURI")
}

/// An `xsd:boolean` result.
fn bool_term(value: bool) -> TermValue {
    TermValue::typed_literal(
        if value { "true" } else { "false" },
        XsdDatatype::Boolean.iri(),
    )
}

/// An `xsd:integer` result from a signed value (`geof:dimension` answers `-1` for
/// an empty geometry, which is the standard's own convention).
fn integer_term(value: i64) -> TermValue {
    TermValue::typed_literal(value.to_string(), XsdDatatype::Integer.iri())
}

/// An `xsd:integer` result from a count.
///
/// Separate from [`integer_term`] so a `usize` reaches the lexical form as its own
/// decimal text: `xsd:integer` is unbounded, so there is no width to cast to and
/// no truncation to reason about.
fn count_term(value: usize) -> TermValue {
    TermValue::typed_literal(value.to_string(), XsdDatatype::Integer.iri())
}

/// An `xsd:double` result.
///
/// This is the crate's single float boundary: [`Rat::to_f64`] computes the
/// correctly rounded nearest double with integer arithmetic and
/// [`f64::from_bits`], and [`purrdf_xsd::numeric::canonical_double`] renders it. No
/// arithmetic is performed on the `f64`, which is why this compiles under the
/// crate root's `deny(clippy::float_arithmetic)`.
fn double_term(value: &Rat) -> TermValue {
    TermValue::typed_literal(canonical_double(value.to_f64()), XsdDatatype::Double.iri())
}

/// An `xsd:anyURI` result.
fn any_uri_term(iri: String) -> TermValue {
    TermValue::typed_literal(iri, any_uri_iri())
}

/// A `geo:wktLiteral` result, carrying the geometry's own coordinate reference
/// system (`wkt::write` always writes the `<IRI>` prefix, so the system survives
/// the round trip rather than being re-derived from a default).
fn wkt_term(literal: &GeometryLiteral, vocab: &GeoVocab) -> TermValue {
    TermValue::typed_literal(
        wkt::write(literal, vocab.coordinate_scale()),
        vocab.term(GeoTerm::WktLiteral),
    )
}

// ---------------------------------------------------------------------------
// Shared bodies
// ---------------------------------------------------------------------------

/// `coordinateDimension` minus the measure axis: the number of ordinates that
/// describe a *place*, which is what "spatial" means here.
const fn spatial_dimension(dim: CoordDim) -> usize {
    dim.ordinates() - if dim.has_m() { 1 } else { 0 }
}

/// One of `minX`/`maxX`/`minY`/`maxY`.
///
/// An empty geometry is a refusal rather than `0`: the empty set has no extent,
/// and `0.0` is a coordinate the data never contained and that a caller cannot
/// tell apart from a real one.
fn planar_bound(
    vocab: &GeoVocab,
    value: &TermValue,
    name: &str,
    pick: impl Fn(&Geometry) -> Option<Rat>,
) -> Result<TermValue, GeoError> {
    let literal = geometry_arg(vocab, value)?;
    let picked = pick(literal.geometry()).ok_or_else(|| {
        GeoError::domain(format!(
            "geof:{name} has no answer for an empty geometry: the empty set has no extent, and 0 \
             would be a coordinate the data never contained"
        ))
    })?;
    Ok(double_term(&picked))
}

/// One of `minZ`/`maxZ`.
///
/// Two different absences, two different messages — and neither is `0`. A 2D
/// geometry has no elevation at all, which is a different fact from an empty
/// geometry having no positions, and `0` would be an elevation the data never
/// stated in either case.
fn elevation_bound(
    vocab: &GeoVocab,
    value: &TermValue,
    name: &str,
    pick: impl Fn(&Geometry) -> Option<Rat>,
) -> Result<TermValue, GeoError> {
    let literal = geometry_arg(vocab, value)?;
    let geometry = literal.geometry();
    let picked = pick(geometry).ok_or_else(|| {
        GeoError::domain(if geometry.is_empty() {
            format!(
                "geof:{name} has no answer for an empty geometry: the empty set has no positions, \
                 so it has no elevation, and 0 would be one the data never stated"
            )
        } else {
            format!(
                "geof:{name} has no answer for a geometry with no Z ordinate: this geometry is \
                 planar, so it has no elevation at all, and 0 would be an elevation the data \
                 never stated"
            )
        })
    })?;
    Ok(double_term(&picked))
}

/// `envelope`, `boundary` and `convexHull`: a derived geometry in the input's own
/// coordinate reference system.
///
/// The system is carried through unchanged because the construction is computed
/// in the input's own coordinates; nothing here reprojects, so relabelling would
/// be a claim the numbers do not support.
fn derived_geometry(
    vocab: &GeoVocab,
    value: &TermValue,
    build: impl Fn(&Geometry) -> Geometry,
) -> Result<TermValue, GeoError> {
    let literal = geometry_arg(vocab, value)?;
    let derived = build(literal.geometry());
    Ok(wkt_term(
        &GeometryLiteral::new(literal.crs().clone(), derived),
        vocab,
    ))
}

/// `area`, `length` and `perimeter`: a planar measure reported in a caller-named
/// unit, refused unless that unit is the one the caller declared for the
/// geometry's own system.
fn unit_measure(
    vocab: &GeoVocab,
    args: &[&TermValue],
    measure: impl Fn(&Geometry) -> Rat,
) -> Result<TermValue, GeoError> {
    let literal = geometry_arg(vocab, args[0])?;
    vocab.require_unit(literal.crs(), unit_arg(args[1])?)?;
    Ok(double_term(&measure(literal.geometry())))
}

/// `metricArea`, `metricLength` and `metricPerimeter`: the same measure with the
/// unit fixed to the metre, refused unless the caller declared the geometry's
/// system to be measured in metres.
fn metric_measure(
    vocab: &GeoVocab,
    args: &[&TermValue],
    measure: impl Fn(&Geometry) -> Rat,
) -> Result<TermValue, GeoError> {
    let literal = geometry_arg(vocab, args[0])?;
    vocab.require_metre(literal.crs())?;
    Ok(double_term(&measure(literal.geometry())))
}

/// The shared tail of `distance` and `metricDistance`.
fn distance_term(a: &GeometryLiteral, b: &GeometryLiteral) -> Result<TermValue, GeoError> {
    let value = measure::distance(a.geometry(), b.geometry()).ok_or_else(|| {
        GeoError::domain(
            "the distance to an empty geometry is undefined: there is no point of the empty set \
             for the distance to be measured to, and 0 would say the two geometries touch",
        )
    })?;
    Ok(double_term(&value))
}

// ---------------------------------------------------------------------------
// The runtime DE-9IM pattern
// ---------------------------------------------------------------------------

/// Parse the nine-character pattern argument of `geof:relate`.
///
/// [`crate::de9im::Pattern::new`] is a `const fn` that **panics** on a bad
/// pattern, which is right for the relation tables (a mistyped constant is a build
/// failure at its definition site) and wrong for a runtime string: `geof:relate`'s
/// third argument is user input, and a panicking function on user input is a bug
/// even when the seam catches it. So this is a fallible parser producing the
/// [`Slot`] row directly.
///
/// # Errors
///
/// [`GeoError::Literal`] for a string that is not exactly nine characters, or that
/// contains a character outside `F`, `T`, `0`, `1`, `2`, `*`.
fn parse_relate_pattern(text: &str) -> Result<[Slot; 9], GeoError> {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() != 9 {
        return Err(GeoError::literal(format!(
            "a DE-9IM pattern has exactly nine positions; {text:?} has {}",
            characters.len()
        )));
    }
    let mut slots = [Slot::Any; 9];
    for (slot, character) in slots.iter_mut().zip(characters) {
        *slot = match character {
            'F' => Slot::Empty,
            'T' => Slot::Present,
            '0' => Slot::Exactly(Dim::Zero),
            '1' => Slot::Exactly(Dim::One),
            '2' => Slot::Exactly(Dim::Two),
            '*' => Slot::Any,
            other => {
                return Err(GeoError::literal(format!(
                    "{other:?} is not a DE-9IM pattern character; a pattern is written with F, T, \
                     0, 1, 2 and * only"
                )));
            }
        };
    }
    Ok(slots)
}

/// Whether `matrix` satisfies a runtime-parsed pattern row.
fn matrix_matches(matrix: &IntersectionMatrix, slots: &[Slot; 9]) -> bool {
    matrix
        .cells()
        .iter()
        .zip(slots)
        .all(|(&dim, &slot)| slot.accepts(dim))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
    use purrdf_sparql_eval::{
        Arity, EvalError, NativeSparqlEngine, QueryOptions, UserFunctionRegistry,
    };

    use super::{GeoError, GeofFunction, compute, evaluate, register};
    use crate::geom::Crs;
    use crate::relations::SpatialRelation;
    use crate::vocab::{GeoTerm, GeoVocab, GeoVocabBuilder};

    const CORE: &str = "http://example.org/geo#";
    const FUNC: &str = "http://example.org/geof/";
    /// The default WKT system, declared to be measured in metres.
    const CRS_METRIC: &str = "http://example.org/crs/metric";
    /// The GeoJSON system, declared to be measured in degrees.
    const CRS_GEOJSON: &str = "http://example.org/crs/geojson";
    /// A system with no declared unit at all.
    const CRS_UNDECLARED: &str = "http://example.org/crs/undeclared";
    const METRE: &str = "http://example.org/unit/metre";
    const DEGREE: &str = "http://example.org/unit/degree";
    const SF: &str = "http://example.org/sf#";

    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
    const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
    const XSD_ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";

    fn crs(iri: &str) -> Crs {
        Crs::new(iri).expect("a non-empty IRI")
    }

    /// A fully declared vocabulary: both systems have units, the metre is named,
    /// and the Simple Features namespace is supplied.
    fn vocab() -> GeoVocab {
        GeoVocabBuilder::new(CORE, FUNC, crs(CRS_METRIC), crs(CRS_GEOJSON))
            .expect("non-empty namespaces")
            .declare_crs_unit(&crs(CRS_METRIC), METRE)
            .expect("a fresh declaration")
            .declare_crs_unit(&crs(CRS_GEOJSON), DEGREE)
            .expect("a fresh declaration")
            .declare_metre(METRE)
            .expect("a non-empty metre IRI")
            .declare_simple_features_namespace(SF)
            .expect("a non-empty namespace")
            .build()
    }

    fn registry_of(vocab: &GeoVocab) -> UserFunctionRegistry {
        let mut registry = UserFunctionRegistry::new();
        register(&mut registry, vocab);
        registry
    }

    fn wkt(vocab: &GeoVocab, lexical: &str) -> TermValue {
        TermValue::typed_literal(lexical, vocab.term(GeoTerm::WktLiteral))
    }

    fn geojson(vocab: &GeoVocab, lexical: &str) -> TermValue {
        TermValue::typed_literal(lexical, vocab.term(GeoTerm::GeoJsonLiteral))
    }

    fn string(lexical: &str) -> TermValue {
        TermValue::typed_literal(lexical, XSD_STRING)
    }

    fn integer(value: i64) -> TermValue {
        TermValue::typed_literal(value.to_string(), XSD_INTEGER)
    }

    /// Invoke a registered function **through the registry**.
    ///
    /// The registration is resolved first and the assertion fails if it is
    /// missing, so every golden answer below also proves that the IRI is wired.
    /// The resolved [`NativeFunction`](purrdf_sparql_eval::NativeFunction)'s
    /// closure field is `pub(crate)` to `purrdf-sparql-eval` and therefore cannot
    /// be called from this crate; [`compute`] is the computation [`evaluate`] — the
    /// body [`register`] installs (see its one construction site) — performs, so
    /// calling it after the resolve exercises the same code the seam runs. The end-to-end
    /// tests at the bottom of this module close the remaining gap by running real
    /// SPARQL through the engine.
    ///
    /// It answers in [`GeoError`] space rather than through [`evaluate`] on purpose:
    /// `evaluate` maps an expression error onto `Ok(None)`, which by construction
    /// carries no message, so asserting *why* a call was refused is only possible on
    /// this side of the mapping. Which side of it each refusal lands on is asserted
    /// separately, by [`expression_error`]/[`fatal_refusal`] here and by real query
    /// text in `tests/scalar_functions_e2e.rs`.
    fn call(
        registry: &UserFunctionRegistry,
        vocab: &GeoVocab,
        function: GeofFunction,
        args: &[TermValue],
    ) -> Result<TermValue, GeoError> {
        let iri = vocab.function(function.local_name());
        assert!(
            registry.resolve_native(&iri).is_some(),
            "geof:{} must be registered as a native function at <{iri}>",
            function.local_name()
        );
        let borrowed: Vec<&TermValue> = args.iter().collect();
        compute(function, vocab, &borrowed)
    }

    /// [`call`] for the one-argument functions, which are most of them.
    fn call1(
        registry: &UserFunctionRegistry,
        vocab: &GeoVocab,
        function: GeofFunction,
        arg: &TermValue,
    ) -> Result<TermValue, GeoError> {
        call(registry, vocab, function, std::slice::from_ref(arg))
    }

    fn named(local_name: &str) -> GeofFunction {
        GeofFunction::from_local_name(local_name)
            .unwrap_or_else(|| panic!("geof:{local_name} must be a declared function"))
    }

    /// The lexical form and datatype of a successful call.
    fn ok_literal(result: Result<TermValue, GeoError>, what: &str) -> (String, String) {
        match result {
            Ok(TermValue::Literal {
                lexical_form,
                datatype,
                ..
            }) => (lexical_form, datatype),
            Ok(other) => panic!("{what} must answer with a literal, got {other:?}"),
            Err(err) => panic!("{what} must succeed, but was refused: {err}"),
        }
    }

    /// Assert a call succeeded with an exact lexical form and datatype.
    fn assert_answer(
        result: Result<TermValue, GeoError>,
        what: &str,
        lexical: &str,
        datatype: &str,
    ) {
        let (got_lexical, got_datatype) = ok_literal(result, what);
        assert_eq!(got_lexical, lexical, "{what} answered the wrong value");
        assert_eq!(
            got_datatype, datatype,
            "{what} answered with the wrong datatype"
        );
    }

    /// Assert a call was refused **for these arguments only** — a SPARQL expression
    /// error, which the seam reports as `Ok(None)` and which a `FILTER` turns into a
    /// dropped solution and a `BIND` into an unbound variable — and return the
    /// rendered message.
    ///
    /// The distance is asserted, not assumed: a refusal that quietly became fatal
    /// would turn one bad literal into a failed query, and every caller of this
    /// helper is a case where that must not happen.
    fn expression_error(result: Result<TermValue, GeoError>, what: &str) -> String {
        match result {
            Ok(value) => panic!("{what} must be refused, but answered {value:?}"),
            Err(err) => {
                assert!(
                    err.is_expression_error(),
                    "{what} is a refusal about these arguments, so it must be a per-solution \
                     expression error rather than a query-fatal one: {err}"
                );
                err.to_string()
            }
        }
    }

    /// Assert a call was refused **fatally** — the query cannot be answered at all —
    /// and return the rendered message.
    ///
    /// The mirror of [`expression_error`], and the assertion that matters more: an
    /// unimplemented function or an unusable vocabulary that softened into `Ok(None)`
    /// would be dropped by a `FILTER` exactly as an honest `false` is, with nothing
    /// downstream able to tell the two apart.
    fn fatal_refusal(result: Result<TermValue, GeoError>, what: &str) -> String {
        match result {
            Ok(value) => panic!("{what} must be refused, but answered {value:?}"),
            Err(err) => {
                assert!(
                    !err.is_expression_error(),
                    "{what} must abort the query rather than leaving one solution without a \
                     value: {err}"
                );
                err.to_string()
            }
        }
    }

    // -----------------------------------------------------------------------
    // 1 & 2 — the inventory
    // -----------------------------------------------------------------------

    /// Every function in the table is registered, and the registry holds exactly
    /// them: an extra registration would be a `geof:` IRI this table does not
    /// account for, and a missing one would be a function that silently resolves
    /// to nothing.
    #[test]
    fn every_declared_function_is_registered_and_the_registry_holds_nothing_else() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        assert_eq!(
            GeofFunction::ALL.len(),
            68,
            "the inventory is 24 relations + relate + 31 implemented + 12 unimplemented"
        );
        assert_eq!(
            GeofFunction::COUNT,
            GeofFunction::ALL.len(),
            "COUNT must be the length of ALL"
        );
        for function in GeofFunction::ALL {
            let iri = vocab.function(function.local_name());
            assert!(
                registry.resolve_native(&iri).is_some(),
                "geof:{} must be registered at <{iri}>",
                function.local_name()
            );
        }
        assert_eq!(
            registry.len(),
            68,
            "the registry must hold exactly the 68 declared functions and nothing else"
        );
    }

    /// The local names are distinct and round-trip, so no two functions can be
    /// registered at the same IRI (which would silently make one shadow the other,
    /// since `register_native` is last-write-wins).
    #[test]
    fn every_local_name_is_distinct_and_round_trips_through_from_local_name() {
        let mut names: Vec<&str> = GeofFunction::ALL
            .iter()
            .map(|function| function.local_name())
            .collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "no two geof: functions share a name");

        for function in GeofFunction::ALL {
            assert_eq!(
                GeofFunction::from_local_name(function.local_name()),
                Some(function),
                "geof:{} must round-trip through its local name",
                function.local_name()
            );
        }
        assert_eq!(
            GeofFunction::from_local_name("notAFunction"),
            None,
            "an unknown local name resolves to nothing rather than to a default"
        );
        // An aggregate is NOT a scalar function and must not resolve as one.
        for aggregate in GeofFunction::AGGREGATES {
            assert_eq!(
                GeofFunction::from_local_name(aggregate),
                None,
                "geof:{aggregate} is an aggregate and must not be a scalar function"
            );
        }
    }

    /// The topological family is exactly the twenty-four relations, reusing their
    /// own local-name table rather than a retyped copy.
    #[test]
    fn the_topological_family_is_exactly_the_twenty_four_relations() {
        let registered: Vec<&str> = GeofFunction::ALL
            .iter()
            .filter_map(|function| match function {
                GeofFunction::Relation(relation) => Some(relation.local_name()),
                _ => None,
            })
            .collect();
        let expected: Vec<&str> = SpatialRelation::ALL
            .iter()
            .map(|relation| relation.local_name())
            .collect();
        assert_eq!(
            registered, expected,
            "the geof: topological names are the geo: relation names, in the same order"
        );
        assert_eq!(registered.len(), 24, "there are twenty-four of them");
    }

    /// Every registration is `Stable`: the bodies are pure arithmetic over their
    /// arguments, and the fork-join gate reads this declaration alone.
    #[test]
    fn every_registration_declares_the_stable_volatility_the_bodies_actually_have() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        for function in GeofFunction::ALL {
            let iri = vocab.function(function.local_name());
            let native = registry
                .resolve_native(&iri)
                .unwrap_or_else(|| panic!("geof:{} is registered", function.local_name()));
            // `NativeFunction` exposes its descriptor only through `Debug` (the
            // fields are `pub(crate)` to `purrdf-sparql-eval`), and that rendering
            // carries exactly the two things the fork-join gate reads.
            let rendered = format!("{native:?}");
            assert!(
                rendered.contains("volatility: Stable"),
                "geof:{} must be registered Stable, got {rendered}",
                function.local_name()
            );
            assert!(
                rendered.contains(&format!("arity: {:?}", function.arity())),
                "geof:{} must be registered with its declared arity, got {rendered}",
                function.local_name()
            );
        }
    }

    // -----------------------------------------------------------------------
    // 3 & 4 — golden answers
    // -----------------------------------------------------------------------

    /// A point strictly inside a polygon is `sfWithin` it and does **not**
    /// `sfContains` it. The ordered pair is the point of the assertion: a
    /// symmetric implementation would pass the first half and fail the second.
    #[test]
    fn a_point_inside_a_polygon_is_within_it_and_does_not_contain_it() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let point = wkt(&vocab, "POINT(0.5 0.5)");
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");

        assert_answer(
            call(
                &registry,
                &vocab,
                named("sfWithin"),
                &[point.clone(), square.clone()],
            ),
            "geof:sfWithin(point, square)",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("sfContains"),
                &[point.clone(), square.clone()],
            ),
            "geof:sfContains(point, square)",
            "false",
            XSD_BOOLEAN,
        );
        // The converse pair, so the two answers above are about the argument order
        // rather than about the geometries never relating.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("sfContains"),
                &[square.clone(), point.clone()],
            ),
            "geof:sfContains(square, point)",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call(&registry, &vocab, named("sfIntersects"), &[point, square]),
            "geof:sfIntersects(point, square)",
            "true",
            XSD_BOOLEAN,
        );
    }

    /// The regression for the standard's printed `TFFFTFFFT`: a point has an empty
    /// boundary, so a literal reading of that pattern makes two identical points
    /// unequal. See `crate::relations`' module docs.
    #[test]
    fn sf_equals_holds_between_two_identical_points() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        for name in ["sfEquals", "ehEquals", "rcc8eq"] {
            assert_answer(
                call(
                    &registry,
                    &vocab,
                    named(name),
                    &[wkt(&vocab, "POINT(1 1)"), wkt(&vocab, "POINT(1 1)")],
                ),
                &format!("geof:{name} between two identical points"),
                "true",
                XSD_BOOLEAN,
            );
        }
        // The neighbouring case that must still be false.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("sfEquals"),
                &[wkt(&vocab, "POINT(1 1)"), wkt(&vocab, "POINT(2 2)")],
            ),
            "geof:sfEquals between two different points",
            "false",
            XSD_BOOLEAN,
        );
    }

    /// The accessors report the geometry's own properties rather than a default.
    #[test]
    fn the_accessors_report_the_geometrys_own_properties() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");
        let line = wkt(&vocab, "LINESTRING(0 0,1 1)");
        let point_zm = wkt(&vocab, "POINT ZM (1 2 3 4)");
        let multi = wkt(&vocab, "MULTIPOINT((0 0),(1 1),(2 2))");

        assert_answer(
            call1(&registry, &vocab, named("dimension"), &square),
            "geof:dimension of a polygon",
            "2",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("dimension"), &line),
            "geof:dimension of a line",
            "1",
            XSD_INTEGER,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("dimension"),
                &[wkt(&vocab, "POINT EMPTY")],
            ),
            "geof:dimension of an empty geometry",
            "-1",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("coordinateDimension"), &square),
            "geof:coordinateDimension of an XY polygon",
            "2",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("coordinateDimension"), &point_zm),
            "geof:coordinateDimension of an XYZM point",
            "4",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("spatialDimension"), &point_zm),
            "geof:spatialDimension drops the measure axis",
            "3",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("geometryType"), &square),
            "geof:geometryType of a polygon",
            "http://example.org/sf#Polygon",
            XSD_ANY_URI,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[wkt(&vocab, "POINT EMPTY")],
            ),
            "geof:isEmpty of POINT EMPTY",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("isEmpty"), &square),
            "geof:isEmpty of a populated polygon",
            "false",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("isSimple"), &line),
            "geof:isSimple of a straight line",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isSimple"),
                &[wkt(&vocab, "MULTIPOINT((0 0),(0 0))")],
            ),
            "geof:isSimple of a multipoint with a duplicate",
            "false",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("is3D"), &point_zm),
            "geof:is3D of an XYZM point",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("is3D"), &square),
            "geof:is3D of an XY polygon",
            "false",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("isMeasured"), &point_zm),
            "geof:isMeasured of an XYZM point",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("isMeasured"), &square),
            "geof:isMeasured of an XY polygon",
            "false",
            XSD_BOOLEAN,
        );
        assert_answer(
            call1(&registry, &vocab, named("getSRID"), &square),
            "geof:getSRID of a literal with no explicit prefix",
            CRS_METRIC,
            XSD_ANY_URI,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("getSRID"),
                &[wkt(&vocab, &format!("<{CRS_GEOJSON}> POINT(1 2)"))],
            ),
            "geof:getSRID of a literal with an explicit prefix",
            CRS_GEOJSON,
            XSD_ANY_URI,
        );
        assert_answer(
            call1(&registry, &vocab, named("numGeometries"), &multi),
            "geof:numGeometries of a three-member multipoint",
            "3",
            XSD_INTEGER,
        );
        assert_answer(
            call1(&registry, &vocab, named("numGeometries"), &square),
            "geof:numGeometries of a simple geometry",
            "1",
            XSD_INTEGER,
        );
        assert_answer(
            call(&registry, &vocab, named("geometryN"), &[multi, integer(2)]),
            "geof:geometryN(multipoint, 2)",
            &format!("<{CRS_METRIC}> POINT(1 1)"),
            &vocab.term(GeoTerm::WktLiteral),
        );
    }

    /// The bounding ordinates, the constructors and the serializations.
    #[test]
    fn the_constructors_and_serializations_answer_with_the_geometry_they_name() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let square = wkt(&vocab, "POLYGON((0 0,2 0,2 2,0 2,0 0))");
        let wkt_datatype = vocab.term(GeoTerm::WktLiteral);

        assert_answer(
            call1(&registry, &vocab, named("minX"), &square),
            "geof:minX of the square",
            "0.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call1(&registry, &vocab, named("maxX"), &square),
            "geof:maxX of the square",
            "2.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call1(&registry, &vocab, named("minY"), &square),
            "geof:minY of the square",
            "0.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call1(&registry, &vocab, named("maxY"), &square),
            "geof:maxY of the square",
            "2.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("envelope"),
                &[wkt(&vocab, "LINESTRING(0 0,2 1,1 2)")],
            ),
            "geof:envelope of a bent line",
            &format!("<{CRS_METRIC}> POLYGON((0 0,2 0,2 2,0 2,0 0))"),
            &wkt_datatype,
        );
        assert_answer(
            call1(&registry, &vocab, named("boundary"), &square),
            "geof:boundary of a square",
            &format!("<{CRS_METRIC}> MULTILINESTRING((0 0,2 0,2 2,0 2,0 0))"),
            &wkt_datatype,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("convexHull"),
                &[wkt(&vocab, "MULTIPOINT((0 0),(2 0),(2 2),(0 2),(1 1))")],
            ),
            "geof:convexHull of five points, the fifth interior",
            &format!("<{CRS_METRIC}> POLYGON((0 0,2 0,2 2,0 2,0 0))"),
            &wkt_datatype,
        );
        assert_answer(
            call1(&registry, &vocab, named("centroid"), &square),
            "geof:centroid of the square",
            &format!("<{CRS_METRIC}> POINT(1 1)"),
            &wkt_datatype,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("asWKT"),
                &[wkt(&vocab, "  polygon (( 0 0, 2 0, 2 2, 0 2, 0 0 ))  ")],
            ),
            "geof:asWKT normalizes a sloppy spelling",
            &format!("<{CRS_METRIC}> POLYGON((0 0,2 0,2 2,0 2,0 0))"),
            &wkt_datatype,
        );
        // asWKT is a round trip: feeding its own output back gives the same bytes.
        let once = ok_literal(
            call1(&registry, &vocab, named("asWKT"), &square),
            "geof:asWKT",
        );
        let twice = ok_literal(
            call(
                &registry,
                &vocab,
                named("asWKT"),
                &[TermValue::typed_literal(once.0.clone(), once.1.clone())],
            ),
            "geof:asWKT of its own output",
        );
        assert_eq!(once, twice, "geof:asWKT must be idempotent");
    }

    /// The measures answer in the declared unit and are exact where the geometry
    /// is exact.
    #[test]
    fn the_measures_answer_in_the_declared_unit() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let unit_square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");
        let metre = TermValue::Iri(METRE.to_owned());

        assert_answer(
            call(
                &registry,
                &vocab,
                named("area"),
                &[unit_square.clone(), metre.clone()],
            ),
            "geof:area of the unit square",
            "1.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call1(&registry, &vocab, named("metricArea"), &unit_square),
            "geof:metricArea of the unit square",
            "1.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("perimeter"),
                &[unit_square.clone(), metre.clone()],
            ),
            "geof:perimeter of the unit square",
            "4.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call1(&registry, &vocab, named("metricPerimeter"), &unit_square),
            "geof:metricPerimeter of the unit square",
            "4.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("length"),
                &[wkt(&vocab, "LINESTRING(0 0,3 4)"), metre.clone()],
            ),
            "geof:length of a 3-4-5 segment",
            "5.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("metricLength"),
                &[wkt(&vocab, "LINESTRING(0 0,3 4)")],
            ),
            "geof:metricLength of a 3-4-5 segment",
            "5.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("distance"),
                &[wkt(&vocab, "POINT(0 0)"), wkt(&vocab, "POINT(3 4)"), metre],
            ),
            "geof:distance between two points",
            "5.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("metricDistance"),
                &[wkt(&vocab, "POINT(0 0)"), wkt(&vocab, "POINT(3 4)")],
            ),
            "geof:metricDistance between two points",
            "5.0E0",
            XSD_DOUBLE,
        );
        // The units argument is also accepted as an xsd:anyURI literal, which is
        // the spelling the standard's own signature gives.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("area"),
                &[
                    wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))"),
                    TermValue::typed_literal(METRE, XSD_ANY_URI),
                ],
            ),
            "geof:area with an xsd:anyURI unit",
            "1.0E0",
            XSD_DOUBLE,
        );
    }

    // -----------------------------------------------------------------------
    // 5 — every unimplemented function fails by name
    // -----------------------------------------------------------------------

    /// Every registered-but-unimplemented function refuses by name, as an
    /// `EvalError::Function`, and never returns `Ok`. A default answer here — an
    /// unchanged geometry from `geof:transform`, a `false` from anything — would
    /// be indistinguishable from a real answer.
    #[test]
    fn every_unimplemented_function_fails_by_name_rather_than_answering() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let geometry = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");
        let mut checked = 0usize;

        for function in GeofFunction::ALL {
            let Some(reason) = function.unsupported_reason() else {
                continue;
            };
            checked += 1;
            // Enough arguments for the widest declared arity; the seam checks
            // arity before the body, and `evaluate` re-checks it, so the call must
            // present a satisfying count for the refusal to be about the function.
            let args: Vec<TermValue> = match function.arity() {
                Arity::Exact(n) => (0..n).map(|_| geometry.clone()).collect(),
                Arity::AtLeast(n) => (0..n.max(1)).map(|_| geometry.clone()).collect(),
                Arity::Range { min, .. } => (0..min).map(|_| geometry.clone()).collect(),
            };
            let result = call(&registry, &vocab, function, &args);
            let err = match result {
                Ok(value) => panic!(
                    "geof:{} is not implemented and must refuse, but answered {value:?}",
                    function.local_name()
                ),
                Err(err) => err,
            };
            assert!(
                matches!(err, GeoError::Unsupported(_)),
                "geof:{} must refuse as GeoError::Unsupported, got {err:?}",
                function.local_name()
            );
            assert!(
                !err.is_expression_error(),
                "geof:{} is unimplemented, so the refusal must abort the query rather than \
                 leaving one solution without a value — a FILTER would drop that solution \
                 exactly as it drops an honest false: {err:?}",
                function.local_name()
            );
            assert!(
                matches!(EvalError::from(err.clone()), EvalError::Function(_)),
                "geof:{} must reach the evaluator as EvalError::Function, got {err:?}",
                function.local_name()
            );
            let rendered = err.to_string();
            assert!(
                rendered.contains(function.local_name()),
                "the refusal must name geof:{}, got {rendered}",
                function.local_name()
            );
            assert!(
                rendered.contains(reason),
                "the refusal must carry the declared reason for geof:{}, got {rendered}",
                function.local_name()
            );
        }
        assert_eq!(
            checked, 12,
            "twelve spec-defined functions are registered and unimplemented"
        );
    }

    /// The six aggregates are listed but not registered as scalar functions: a
    /// `geof:aggUnion(?g)` call must fail to resolve rather than look like it
    /// works.
    #[test]
    fn the_aggregates_are_listed_but_never_registered_as_scalar_functions() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        assert_eq!(GeofFunction::AGGREGATES.len(), 6, "there are six of them");
        for aggregate in GeofFunction::AGGREGATES {
            let iri = vocab.function(aggregate);
            assert!(
                registry.resolve_native(&iri).is_none(),
                "geof:{aggregate} is an aggregate and must not resolve on the scalar seam"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 6 — refusals, each with its neighbouring valid case
    // -----------------------------------------------------------------------

    /// A non-literal argument is refused; the same call with a literal is not.
    #[test]
    fn a_non_literal_argument_is_refused_and_a_literal_one_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let rendered = expression_error(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[TermValue::Iri("http://example.org/a-geometry".to_owned())],
            ),
            "geof:isEmpty of an IRI",
        );
        assert!(
            rendered.contains("http://example.org/a-geometry"),
            "the refusal must name what arrived, got {rendered}"
        );
        // The neighbouring VALID case.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[wkt(&vocab, "POINT(1 2)")],
            ),
            "geof:isEmpty of a wktLiteral",
            "false",
            XSD_BOOLEAN,
        );
    }

    /// An `xsd:string` carrying WKT text is refused; the identical characters
    /// under `geo:wktLiteral` are not. The datatype is what makes it a geometry.
    #[test]
    fn an_xsd_string_argument_is_refused_and_a_wkt_literal_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let rendered = expression_error(
            call(&registry, &vocab, named("isEmpty"), &[string("POINT(1 2)")]),
            "geof:isEmpty of an xsd:string",
        );
        assert!(
            rendered.contains(XSD_STRING),
            "the refusal must name the datatype that arrived, got {rendered}"
        );
        // The neighbouring VALID case: the very same characters, correctly typed.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[wkt(&vocab, "POINT(1 2)")],
            ),
            "geof:isEmpty of the same text as a wktLiteral",
            "false",
            XSD_BOOLEAN,
        );
        // A geoJSONLiteral is the other accepted datatype, and still works.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[geojson(&vocab, r#"{"type":"Point","coordinates":[1,2]}"#)],
            ),
            "geof:isEmpty of a geoJSONLiteral",
            "false",
            XSD_BOOLEAN,
        );
    }

    /// The three serialization datatypes this crate has no codec for are refused
    /// by name; the two it does have are not.
    #[test]
    fn the_gml_kml_and_dggs_datatypes_are_refused_by_name_and_the_two_codecs_are_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        for term in [
            GeoTerm::GmlLiteral,
            GeoTerm::KmlLiteral,
            GeoTerm::DggsLiteral,
        ] {
            // Fatal, not per-solution. This is the *codec* that is missing, not the
            // literal that is bad: a dataset mixing WKT and GML geometries would
            // otherwise have its GML rows quietly filtered away, and a query that
            // silently answered about half a dataset is the failure this crate keeps
            // closed. The gap has to be named so the caller learns which
            // serialization purrdf-geo cannot read.
            let rendered = fatal_refusal(
                call(
                    &registry,
                    &vocab,
                    named("isEmpty"),
                    &[TermValue::typed_literal("anything", vocab.term(term))],
                ),
                &format!("geof:isEmpty of a geo:{}", term.local_name()),
            );
            assert!(
                rendered.contains(term.local_name()),
                "the refusal must name geo:{}, got {rendered}",
                term.local_name()
            );
        }
        // The neighbouring VALID cases.
        for value in [
            wkt(&vocab, "POINT(1 2)"),
            geojson(&vocab, r#"{"type":"Point","coordinates":[1,2]}"#),
        ] {
            assert_answer(
                call1(&registry, &vocab, named("isEmpty"), &value),
                "geof:isEmpty of a supported datatype",
                "false",
                XSD_BOOLEAN,
            );
        }
    }

    /// Malformed WKT is refused as data; the well-formed neighbour is not.
    #[test]
    fn malformed_wkt_is_refused_and_well_formed_wkt_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        // NOT in this list: the empty lexical form. OGC 22-047r1
        // /req/geometry-extension/wkt-literal-empty makes it the empty geometry,
        // so refusing it would abort every query touching an empty geo:asWKT
        // rather than answering about it. It is asserted as VALID below.
        for bad in ["POINT(1", "POIN(1 2)", "POINT(1 2) trailing"] {
            let result = call1(&registry, &vocab, named("isEmpty"), &wkt(&vocab, bad));
            let err = match result {
                Ok(value) => panic!("{bad:?} must be refused, but answered {value:?}"),
                Err(err) => err,
            };
            assert!(
                matches!(err, GeoError::Literal(_)),
                "a malformed geometry literal is bad DATA, got {err:?} for {bad:?}"
            );
            assert!(
                err.is_expression_error(),
                "a malformed literal is a statement about ONE row's argument, so it must be a \
                 per-solution expression error — otherwise one bad geometry anywhere in a \
                 dataset fails every query that scans past it: {err:?} for {bad:?}"
            );
        }
        // The neighbouring VALID case.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("isEmpty"),
                &[wkt(&vocab, "POINT(1 2)")],
            ),
            "geof:isEmpty of well-formed WKT",
            "false",
            XSD_BOOLEAN,
        );
        // The other neighbouring VALID case: the empty lexical form is the empty
        // geometry, so it ANSWERS rather than aborting the query.
        for empty in ["", " ", "\t\n"] {
            assert_answer(
                call(&registry, &vocab, named("isEmpty"), &[wkt(&vocab, empty)]),
                "geof:isEmpty of the empty wktLiteral",
                "true",
                XSD_BOOLEAN,
            );
        }
    }

    /// A mixed-CRS pair is refused; the same-CRS pair answers.
    #[test]
    fn a_mixed_crs_pair_is_refused_and_a_same_crs_pair_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let here = wkt(&vocab, "POINT(0 0)");
        let elsewhere = wkt(&vocab, &format!("<{CRS_GEOJSON}> POINT(0 0)"));
        let rendered = expression_error(
            call(
                &registry,
                &vocab,
                named("sfEquals"),
                &[here.clone(), elsewhere],
            ),
            "geof:sfEquals across two systems",
        );
        assert!(
            rendered.contains(CRS_METRIC) && rendered.contains(CRS_GEOJSON),
            "the refusal must name both systems, got {rendered}"
        );
        // The neighbouring VALID case.
        assert_answer(
            call(&registry, &vocab, named("sfEquals"), &[here.clone(), here]),
            "geof:sfEquals within one system",
            "true",
            XSD_BOOLEAN,
        );
    }

    /// `minZ` on a planar geometry is refused rather than answering `0`; on a 3D
    /// geometry it answers.
    #[test]
    fn min_z_is_refused_on_a_planar_geometry_and_answered_on_a_three_dimensional_one() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        for name in ["minZ", "maxZ"] {
            let rendered = expression_error(
                call(&registry, &vocab, named(name), &[wkt(&vocab, "POINT(1 2)")]),
                &format!("geof:{name} of a planar point"),
            );
            assert!(
                rendered.contains("no Z ordinate"),
                "the refusal must say the geometry has no elevation, got {rendered}"
            );
        }
        // The neighbouring VALID case.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("minZ"),
                &[wkt(&vocab, "LINESTRING Z (0 0 3,1 1 9)")],
            ),
            "geof:minZ of a 3D line",
            "3.0E0",
            XSD_DOUBLE,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("maxZ"),
                &[wkt(&vocab, "LINESTRING Z (0 0 3,1 1 9)")],
            ),
            "geof:maxZ of a 3D line",
            "9.0E0",
            XSD_DOUBLE,
        );
    }

    /// `minX` on an empty geometry is refused rather than answering `0`; on a
    /// populated one it answers.
    #[test]
    fn min_x_is_refused_on_an_empty_geometry_and_answered_on_a_populated_one() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        for name in ["minX", "maxX", "minY", "maxY"] {
            let rendered = expression_error(
                call(
                    &registry,
                    &vocab,
                    named(name),
                    &[wkt(&vocab, "POLYGON EMPTY")],
                ),
                &format!("geof:{name} of an empty polygon"),
            );
            assert!(
                rendered.contains("empty"),
                "the refusal must say the geometry is empty, got {rendered}"
            );
        }
        // The neighbouring VALID case: the same function on a populated geometry.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("minX"),
                &[wkt(&vocab, "POLYGON((3 0,5 0,5 2,3 2,3 0))")],
            ),
            "geof:minX of a populated polygon",
            "3.0E0",
            XSD_DOUBLE,
        );
    }

    /// An out-of-range member index is refused rather than answering an empty
    /// geometry; an in-range one answers with the member.
    #[test]
    fn geometry_n_out_of_range_is_refused_and_an_in_range_index_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let multi = wkt(&vocab, "MULTIPOINT((0 0),(1 1))");
        for index in [0_i64, 3, -1] {
            let rendered = expression_error(
                call(
                    &registry,
                    &vocab,
                    named("geometryN"),
                    &[multi.clone(), integer(index)],
                ),
                &format!("geof:geometryN(multipoint, {index})"),
            );
            assert!(
                rendered.contains("1-based"),
                "the refusal must explain the index space, got {rendered}"
            );
        }
        // The neighbouring VALID cases: both ends of the range.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("geometryN"),
                &[multi.clone(), integer(1)],
            ),
            "geof:geometryN(multipoint, 1)",
            &format!("<{CRS_METRIC}> POINT(0 0)"),
            &vocab.term(GeoTerm::WktLiteral),
        );
        assert_answer(
            call(&registry, &vocab, named("geometryN"), &[multi, integer(2)]),
            "geof:geometryN(multipoint, 2)",
            &format!("<{CRS_METRIC}> POINT(1 1)"),
            &vocab.term(GeoTerm::WktLiteral),
        );
    }

    /// A measurement asked in a unit the caller never declared for the geometry's
    /// system is refused; the declared unit answers.
    #[test]
    fn an_undeclared_unit_is_refused_and_the_declared_one_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");

        // Declared for this system, but a different unit was asked for.
        let rendered = expression_error(
            call(
                &registry,
                &vocab,
                named("area"),
                &[square.clone(), TermValue::Iri(DEGREE.to_owned())],
            ),
            "geof:area asked in degrees of a metre system",
        );
        assert!(
            rendered.contains(DEGREE) && rendered.contains(METRE),
            "the refusal must name both units, got {rendered}"
        );

        // A system with no declared unit at all — fatal, unlike the unit MISMATCH
        // just above. The mismatch is a statement about this call's two arguments;
        // this is a declaration the host never made, and PurRDF ships no
        // coordinate-reference-system database to fabricate one from. Leaving the
        // measure unbound would report "this geometry has no area" and hide the
        // wiring gap the host is the only party able to close.
        let rendered = fatal_refusal(
            call(
                &registry,
                &vocab,
                named("area"),
                &[
                    wkt(
                        &vocab,
                        &format!("<{CRS_UNDECLARED}> POLYGON((0 0,1 0,1 1,0 1,0 0))"),
                    ),
                    TermValue::Iri(METRE.to_owned()),
                ],
            ),
            "geof:area of a geometry in an undeclared system",
        );
        assert!(
            rendered.contains(CRS_UNDECLARED),
            "the refusal must name the undeclared system, got {rendered}"
        );

        // The neighbouring VALID case.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("area"),
                &[square, TermValue::Iri(METRE.to_owned())],
            ),
            "geof:area in the declared unit",
            "1.0E0",
            XSD_DOUBLE,
        );
    }

    /// The `metric*` family is refused on a system the caller declared in degrees
    /// and answers on one declared in metres.
    #[test]
    fn metric_area_is_refused_in_degrees_and_answered_in_metres() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let rendered = expression_error(
            call(
                &registry,
                &vocab,
                named("metricArea"),
                &[wkt(
                    &vocab,
                    &format!("<{CRS_GEOJSON}> POLYGON((0 0,1 0,1 1,0 1,0 0))"),
                )],
            ),
            "geof:metricArea of a geometry in a degree system",
        );
        assert!(
            rendered.contains(DEGREE),
            "the refusal must name the system's declared unit, got {rendered}"
        );
        // The neighbouring VALID case: the identical shape in a metre system.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("metricArea"),
                &[wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))")],
            ),
            "geof:metricArea of the same shape in a metre system",
            "1.0E0",
            XSD_DOUBLE,
        );
        // And the whole metric family behaves the same way.
        let degrees = wkt(&vocab, &format!("<{CRS_GEOJSON}> LINESTRING(0 0,3 4)"));
        for name in ["metricLength", "metricPerimeter"] {
            let _ = expression_error(
                call1(&registry, &vocab, named(name), &degrees),
                &format!("geof:{name} in a degree system"),
            );
        }
        assert_answer(
            call(
                &registry,
                &vocab,
                named("metricLength"),
                &[wkt(&vocab, "LINESTRING(0 0,3 4)")],
            ),
            "geof:metricLength in a metre system",
            "5.0E0",
            XSD_DOUBLE,
        );
    }

    /// `geof:geometryType` answers from a third vocabulary and is inert until the
    /// caller declares it; with the declaration it answers.
    #[test]
    fn geometry_type_is_refused_without_the_simple_features_namespace_and_answered_with_it() {
        let undeclared = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_METRIC), crs(CRS_GEOJSON))
            .expect("non-empty namespaces")
            .build();
        let registry = registry_of(&undeclared);
        // Fatal, not per-solution: an undeclared vocabulary is the host's wiring,
        // which no row is responsible for and no row can repair, so every solution
        // would be refused for the same reason. Leaving the variable unbound would
        // report "no geometry type" for geometries that plainly have one.
        let rendered = fatal_refusal(
            call(
                &registry,
                &undeclared,
                named("geometryType"),
                &[wkt(&undeclared, "POINT(1 2)")],
            ),
            "geof:geometryType with no sf: namespace",
        );
        assert!(
            rendered.contains("Simple Features"),
            "the refusal must name the missing vocabulary, got {rendered}"
        );

        // The neighbouring VALID case: the same call against a vocabulary that
        // declares the namespace.
        let vocab = vocab();
        let declared = registry_of(&vocab);
        assert_answer(
            call(
                &declared,
                &vocab,
                named("geometryType"),
                &[wkt(&vocab, "POINT(1 2)")],
            ),
            "geof:geometryType with the sf: namespace declared",
            "http://example.org/sf#Point",
            XSD_ANY_URI,
        );
    }

    /// `geof:asGeoJSON` is refused for a geometry outside the one system RFC 7946
    /// admits, naming both systems, and answers for a geometry inside it.
    #[test]
    fn as_geojson_is_refused_outside_the_geojson_crs_and_answered_inside_it() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let rendered = expression_error(
            call(
                &registry,
                &vocab,
                named("asGeoJSON"),
                &[wkt(&vocab, "POINT(1 2)")],
            ),
            "geof:asGeoJSON of a geometry in the metric system",
        );
        assert!(
            rendered.contains(CRS_METRIC) && rendered.contains(CRS_GEOJSON),
            "the refusal must name both systems, got {rendered}"
        );
        // The neighbouring VALID case: the same geometry in the GeoJSON system.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("asGeoJSON"),
                &[wkt(&vocab, &format!("<{CRS_GEOJSON}> POINT(1 2)"))],
            ),
            "geof:asGeoJSON of a geometry in the GeoJSON system",
            r#"{"type":"Point","coordinates":[1,2]}"#,
            &vocab.term(GeoTerm::GeoJsonLiteral),
        );
    }

    /// A malformed `geof:relate` pattern is refused; a well-formed one answers.
    #[test]
    fn a_malformed_relate_pattern_is_refused_and_a_valid_one_is_not() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let point = wkt(&vocab, "POINT(0.5 0.5)");
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");

        for bad in ["T*F**F**", "T*F**F****", "T*F**F**X", "", "TFFFTFFF3"] {
            let rendered = expression_error(
                call(
                    &registry,
                    &vocab,
                    named("relate"),
                    &[point.clone(), square.clone(), string(bad)],
                ),
                &format!("geof:relate with the pattern {bad:?}"),
            );
            assert!(
                rendered.contains("DE-9IM"),
                "the refusal must name what a pattern is, got {rendered}"
            );
        }
        // A non-string pattern argument is refused too.
        let _ = expression_error(
            call(
                &registry,
                &vocab,
                named("relate"),
                &[point.clone(), square.clone(), integer(9)],
            ),
            "geof:relate with an integer pattern",
        );

        // The neighbouring VALID cases: `within` and its negation, spelled out.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("relate"),
                &[point.clone(), square.clone(), string("T*F**F***")],
            ),
            "geof:relate(point, square, within)",
            "true",
            XSD_BOOLEAN,
        );
        assert_answer(
            call(
                &registry,
                &vocab,
                named("relate"),
                &[point, square, string("FF*FF****")],
            ),
            "geof:relate(point, square, disjoint)",
            "false",
            XSD_BOOLEAN,
        );
    }

    /// No pattern string reaches a panic. `Pattern::new` is a `const fn` that
    /// panics on a bad pattern; `geof:relate`'s third argument is user input, so
    /// this module parses it fallibly instead. Every shape that would have
    /// panicked is exercised here.
    #[test]
    fn no_relate_pattern_string_can_panic_the_seam() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let point = wkt(&vocab, "POINT(0 0)");
        for bad in [
            "",
            "T",
            "TTTTTTTT",
            "TTTTTTTTTT",
            "XXXXXXXXX",
            "         ",
            "T*F**F**\n",
            "ttttttttt",
            "😀😀😀😀😀😀😀😀😀",
            "T*F**F**é",
        ] {
            let result = call(
                &registry,
                &vocab,
                named("relate"),
                &[point.clone(), point.clone(), string(bad)],
            );
            assert!(
                result.is_err(),
                "geof:relate must refuse the pattern {bad:?} rather than answering"
            );
        }
        // The neighbouring VALID case, so the loop above is about the patterns.
        assert_answer(
            call(
                &registry,
                &vocab,
                named("relate"),
                &[point.clone(), point, string("*********")],
            ),
            "geof:relate with the always-true pattern",
            "true",
            XSD_BOOLEAN,
        );
    }

    /// A wrong argument count is a clean error, not an out-of-bounds panic —
    /// `evaluate` is public, so it cannot rely on the seam's own arity check — and
    /// it stays **fatal**: a call written with the wrong number of arguments cannot
    /// be evaluated for any solution, so softening it would turn a defect in the
    /// query text into a silently empty answer.
    #[test]
    fn a_direct_call_with_the_wrong_argument_count_is_refused_rather_than_panicking() {
        let vocab = vocab();
        let point = wkt(&vocab, "POINT(0 0)");
        let err = evaluate(named("sfEquals"), &vocab, &[&point])
            .expect_err("a one-argument sfEquals must be refused");
        assert!(
            matches!(err, EvalError::Function(_)),
            "a wrong argument count is a function error, got {err:?}"
        );
        assert!(
            matches!(
                compute(named("sfEquals"), &vocab, &[&point]),
                Err(GeoError::Arity(_))
            ),
            "and it is its own kind: nobody's data and nobody's wiring is wrong, the call is"
        );
        // The neighbouring VALID case.
        assert!(
            matches!(
                evaluate(named("sfEquals"), &vocab, &[&point, &point]),
                Ok(Some(_))
            ),
            "the two-argument call must still answer with a value"
        );
    }

    // -----------------------------------------------------------------------
    // 7 — determinism
    // -----------------------------------------------------------------------

    /// The same call twice returns byte-identical literals, and two spellings of
    /// the same coordinate produce the same bytes. Coordinates are read as exact
    /// rationals, so `1.5` and `15e-1` are literally the same value on the way in.
    #[test]
    fn the_same_call_twice_and_two_spellings_of_one_coordinate_agree_byte_for_byte() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let plain = wkt(&vocab, "POLYGON((0 0,1.5 0,1.5 1.5,0 1.5,0 0))");
        let exponential = wkt(&vocab, "POLYGON((0 0,15e-1 0,15e-1 1.50,0 1.50,0 0))");

        for name in [
            "asWKT",
            "envelope",
            "boundary",
            "convexHull",
            "centroid",
            "metricArea",
            "metricPerimeter",
            "minX",
            "maxY",
        ] {
            let first = ok_literal(
                call1(&registry, &vocab, named(name), &plain),
                &format!("geof:{name}"),
            );
            let second = ok_literal(
                call1(&registry, &vocab, named(name), &plain),
                &format!("geof:{name} again"),
            );
            assert_eq!(
                first, second,
                "geof:{name} must return byte-identical literals on repeated calls"
            );
            let other_spelling = ok_literal(
                call1(&registry, &vocab, named(name), &exponential),
                &format!("geof:{name} of the exponential spelling"),
            );
            assert_eq!(
                first, other_spelling,
                "geof:{name} must not distinguish 1.5 from 15e-1"
            );
        }
    }

    // -----------------------------------------------------------------------
    // The seam, end to end
    // -----------------------------------------------------------------------

    fn empty_dataset() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    /// Render a term as SPARQL source text.
    fn sparql_term(term: &TermValue) -> String {
        match term {
            TermValue::Iri(iri) => format!("<{iri}>"),
            TermValue::Literal {
                lexical_form,
                datatype,
                ..
            } => {
                let escaped = lexical_form.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\"^^<{datatype}>")
            }
            other => panic!("the fixtures pass only IRIs and literals, got {other:?}"),
        }
    }

    /// Evaluate `geof:<name>(args…)` as a real SPARQL projection over an empty
    /// dataset, with the registry injected through `QueryOptions::functions` and
    /// **no** `ParserOptions` configuration — which is the module doc's claim
    /// about this seam, stated as a test.
    fn engine_call(
        vocab: &GeoVocab,
        registry: &UserFunctionRegistry,
        local_name: &str,
        args: &[TermValue],
    ) -> Result<Option<TermValue>, String> {
        let rendered: Vec<String> = args.iter().map(sparql_term).collect();
        let query = format!(
            "SELECT ((<{}>({})) AS ?v) WHERE {{}}",
            vocab.function(local_name),
            rendered.join(", ")
        );
        let dataset = empty_dataset();
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: registry,
                    ..QueryOptions::EMPTY
                },
            )
            .map_err(|err| err.to_string())?;
        match result {
            SparqlResult::Solutions { rows, .. } => Ok(rows
                .into_iter()
                .next()
                .and_then(|row| row.into_iter().next().flatten())),
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// The registered functions answer a real SPARQL query, with the `geof:` IRI
    /// lowered to a custom function call and resolved at eval time — no parser
    /// configuration anywhere.
    #[test]
    fn the_registered_seam_answers_a_real_sparql_query() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let point = wkt(&vocab, "POINT(0.5 0.5)");
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");

        let value = engine_call(&vocab, &registry, "sfWithin", &[point, square.clone()])
            .expect("the query must run")
            .expect("the projection must be bound");
        assert!(
            matches!(&value, TermValue::Literal { lexical_form, datatype, .. }
                if lexical_form == "true" && datatype == XSD_BOOLEAN),
            "geof:sfWithin must answer true through the engine, got {value:?}"
        );

        let value = engine_call(
            &vocab,
            &registry,
            "area",
            &[square, TermValue::Iri(METRE.to_owned())],
        )
        .expect("the query must run")
        .expect("the projection must be bound");
        assert!(
            matches!(&value, TermValue::Literal { lexical_form, datatype, .. }
                if lexical_form == "1.0E0" && datatype == XSD_DOUBLE),
            "geof:area must answer 1.0E0 through the engine, got {value:?}"
        );
    }

    /// An unimplemented function aborts a real query by name rather than binding a
    /// default — and does so without a panic reaching the seam's `catch_unwind`.
    #[test]
    fn an_unimplemented_function_aborts_a_real_sparql_query() {
        let vocab = vocab();
        let registry = registry_of(&vocab);
        let square = wkt(&vocab, "POLYGON((0 0,1 0,1 1,0 1,0 0))");
        let err = engine_call(
            &vocab,
            &registry,
            "union",
            &[square.clone(), square.clone()],
        )
        .expect_err("an unimplemented function must fail the query");
        assert!(
            err.contains("union"),
            "the query failure must name geof:union, got {err}"
        );
        assert!(
            !err.contains("panicked"),
            "the refusal must be a returned error, not a caught panic: {err}"
        );
        // The neighbouring VALID case: an implemented function on the same
        // arguments still answers, so the failure above is about `union`.
        let value = engine_call(&vocab, &registry, "sfEquals", &[square.clone(), square])
            .expect("the query must run")
            .expect("the projection must be bound");
        assert!(
            matches!(&value, TermValue::Literal { lexical_form, .. } if lexical_form == "true"),
            "geof:sfEquals must still answer, got {value:?}"
        );
    }
}
