// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The geometry model: exact coordinates, the seven OGC geometry kinds, and the
//! coordinate reference system a literal was written in.
//!
//! # Coordinates are exact, and they never pass through a float
//!
//! Every ordinate is a [`Rat`] — an exact rational — built by reading the
//! literal's decimal text **directly** into a numerator and a denominator. There
//! is no `str::parse::<f64>()` anywhere on the ingest path, so a coordinate is
//! never rounded on the way in and two spellings of the same number
//! (`1.5`, `1.50`, `15e-1`) produce the identical value. Together with
//! [`crate::exact`]'s integer-only arithmetic this is the whole of the crate's
//! determinism argument: every geometric decision is a comparison of exact
//! rationals, integer arithmetic is fully specified by Rust on every target, and
//! so a native answer and a `wasm32-unknown-unknown` answer are equal by
//! construction rather than by luck.
//!
//! # Dimension is a property of the geometry, not of a coordinate
//!
//! WKT writes the dimension once, in the tag (`POINT Z (1 2 3)`), and it governs
//! every coordinate in the geometry — including a geometry with no coordinates at
//! all (`POINT Z EMPTY`). So [`Geometry`] carries a [`CoordDim`] beside its body
//! rather than letting each [`Coord`] decide for itself, and
//! [`Geometry::new`] refuses a body whose coordinates disagree with it. A model
//! that let one coordinate be `XY` and its neighbour `XYZ` would make
//! `geof:coordinateDimension` a question with no answer.
//!
//! # Structural validity is checked; geometric validity is not
//!
//! Construction refuses what the OGC Simple Features grammar refuses *structurally*
//! — a line with one point, an unclosed ring, a polygon ring with fewer than four
//! positions — because those are not geometries at all and every downstream
//! algorithm would have to invent a meaning for them. It deliberately does NOT
//! refuse a geometry that is merely *invalid*: a self-intersecting polygon, a ring
//! wound the wrong way, a line that doubles back. Those are well-formed geometries
//! that the specification has predicates *about* ([`crate::measure`]'s
//! `is_simple`), and refusing them at parse time would reject data a conforming
//! store is required to carry.

use core::fmt;

use crate::error::GeoError;
use crate::exact::Rat;

// ---------------------------------------------------------------------------
// Coordinate reference system
// ---------------------------------------------------------------------------

/// The coordinate reference system a geometry's ordinates are expressed in,
/// as the IRI that named it.
///
/// This crate **reprojects nothing**: a CRS is carried, compared, and reported,
/// never converted. Two geometries in different systems are refused by every
/// binary operation ([`GeoError::Domain`]) rather than silently treated as
/// comparable, because coordinates in two systems are two different numbers
/// describing the same place and arithmetic across them is meaningless.
///
/// The IRI is caller-supplied in every case — including the one a WKT literal
/// omits, which [`crate::vocab::GeoVocab`] supplies. PurRDF mints no vocabulary
/// IRIs, so there is no fabricated default here to fall back on.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Crs(String);

impl Crs {
    /// The system named by `iri`.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `iri` is empty. An empty IRI names no system and
    /// would make every CRS comparison below succeed against every other empty
    /// one, which is the silent-agreement failure this type exists to prevent.
    ///
    /// [`GeoError::Config`] if `iri` is not an absolute IRI reference: OGC
    /// 22-047r1's `wktLiteral` requirement is that the prefix be "a valid
    /// absolute IRI, as defined in [RFC 3987], enclosed in angled brackets", so a
    /// value with no scheme, or one carrying whitespace or control characters, is
    /// refused here rather than stored.
    ///
    /// Emptiness alone was not enough, and the gap was one space wide. `< >` and
    /// `< http://example.org/crs/planar >` were accepted verbatim, and each
    /// caused a failure that pointed somewhere else: the padded form never
    /// compares equal to the identical-looking declared system, so
    /// [`unit_of`](crate::vocab::GeoVocab::unit_of) reports a unit the caller
    /// plainly *did* declare as undeclared; and the writer re-emits the stored
    /// bytes, producing a `geo:wktLiteral` no conformant consumer can read.
    pub fn new(iri: impl Into<String>) -> Result<Self, GeoError> {
        let iri = iri.into();
        if iri.is_empty() {
            return Err(GeoError::config(
                "a coordinate reference system IRI may not be empty; PurRDF mints no vocabulary \
                 IRIs, so the CRS a geometry literal omits must be supplied by the caller's \
                 GeoVocab",
            ));
        }
        // Validated by the workspace's single IRI layer rather than by a local
        // predicate: `BaseIri::parse` is "a well-formed RFC 3987 IRI that carries
        // a scheme", which is exactly OGC's "valid absolute IRI" requirement, and
        // routing through it means this crate cannot drift from the resolver's
        // idea of what an absolute IRI is.
        if let Err(error) = purrdf_iri::BaseIri::parse(&iri) {
            return Err(GeoError::config(format!(
                "a coordinate reference system must be named by an ABSOLUTE IRI, and {iri:?} is \
                 not one ({error}); OGC 22-047r1 requires the wktLiteral prefix to be \"a valid \
                 absolute IRI, as defined in [RFC 3987], enclosed in angled brackets\". Storing an \
                 invalid one verbatim would both fail to match an identical-looking declared \
                 system — reporting a unit the caller did declare as undeclared — and render a \
                 wktLiteral no conformant consumer can read"
            )));
        }
        Ok(Self(iri))
    }

    /// The IRI, byte-exact.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Crs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Dimension
// ---------------------------------------------------------------------------

/// Which ordinates every coordinate of a geometry carries.
///
/// The four combinations OGC Simple Features admits: the two planar ordinates
/// always, plus an optional elevation `Z` and an optional measure `M`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum CoordDim {
    /// `x y` — the planar ordinates alone.
    #[default]
    Xy,
    /// `x y z` — with elevation.
    Xyz,
    /// `x y m` — with a measure but no elevation.
    Xym,
    /// `x y z m` — with both.
    Xyzm,
}

impl CoordDim {
    /// Whether an elevation ordinate is present.
    #[must_use]
    pub const fn has_z(self) -> bool {
        matches!(self, Self::Xyz | Self::Xyzm)
    }

    /// Whether a measure ordinate is present.
    #[must_use]
    pub const fn has_m(self) -> bool {
        matches!(self, Self::Xym | Self::Xyzm)
    }

    /// The number of ordinates one coordinate carries (2, 3 or 4).
    #[must_use]
    pub const fn ordinates(self) -> usize {
        2 + (self.has_z() as usize) + (self.has_m() as usize)
    }

    /// The dimension with the given optional ordinates present.
    #[must_use]
    pub const fn new(z: bool, m: bool) -> Self {
        match (z, m) {
            (false, false) => Self::Xy,
            (true, false) => Self::Xyz,
            (false, true) => Self::Xym,
            (true, true) => Self::Xyzm,
        }
    }

    /// The WKT dimension tag: `""`, `"Z"`, `"M"` or `"ZM"`.
    #[must_use]
    pub const fn wkt_tag(self) -> &'static str {
        match self {
            Self::Xy => "",
            Self::Xyz => "Z",
            Self::Xym => "M",
            Self::Xyzm => "ZM",
        }
    }
}

// ---------------------------------------------------------------------------
// Coordinate
// ---------------------------------------------------------------------------

/// One position: two exact planar ordinates plus the optional elevation and
/// measure its geometry's [`CoordDim`] declares.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Coord {
    x: Rat,
    y: Rat,
    z: Option<Rat>,
    m: Option<Rat>,
}

impl Coord {
    /// A planar position.
    #[must_use]
    pub const fn xy(x: Rat, y: Rat) -> Self {
        Self {
            x,
            y,
            z: None,
            m: None,
        }
    }

    /// A position with whichever optional ordinates are supplied.
    #[must_use]
    pub const fn new(x: Rat, y: Rat, z: Option<Rat>, m: Option<Rat>) -> Self {
        Self { x, y, z, m }
    }

    /// The first planar ordinate.
    #[must_use]
    pub const fn x(&self) -> &Rat {
        &self.x
    }

    /// The second planar ordinate.
    #[must_use]
    pub const fn y(&self) -> &Rat {
        &self.y
    }

    /// The elevation, when present.
    #[must_use]
    pub const fn z(&self) -> Option<&Rat> {
        self.z.as_ref()
    }

    /// The measure, when present.
    #[must_use]
    pub const fn m(&self) -> Option<&Rat> {
        self.m.as_ref()
    }

    /// This position's own dimension — which optional ordinates it actually
    /// carries, as opposed to the one its geometry declares.
    #[must_use]
    pub const fn dim(&self) -> CoordDim {
        CoordDim::new(self.z.is_some(), self.m.is_some())
    }

    /// Whether two positions agree on the **planar** ordinates alone.
    ///
    /// This is the equality every topological decision uses: OGC Simple Features
    /// topology is defined on the planar projection, so two positions that differ
    /// only in elevation or measure are the same point of the topological space.
    /// Full structural equality is [`PartialEq`], and the two are deliberately
    /// different functions so a caller cannot pick one by accident.
    #[must_use]
    pub fn same_planar(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

/// A run of positions — a line's vertices, or a polygon ring.
///
/// A plain [`Vec`], deliberately, and this was measured rather than assumed. An
/// exact [`Coord`] is four arbitrary-precision rationals and is **384 bytes**
/// (see `crates/geo/tests/layout.rs`, which pins it), so a small-vector with an
/// inline capacity of four made every [`GeometryBody`] 1552 bytes — a `POINT`
/// carried a kilobyte and a half of unused ring storage, and a `Vec<Geometry>`
/// paid it per member. The inline storage bought nothing either: a sequence with
/// any positions at all is heap-allocated in the `Vec` case and would spill in
/// the small-vector case as soon as a ring exceeded a triangle. So the
/// indirection is the cheaper of the two, by a factor of four on the whole
/// model.
pub type CoordSeq = Vec<Coord>;

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// The seven geometry kinds OGC Simple Features names, as a tag with no payload.
///
/// Returned by `geof:geometryType` and used wherever a kind must be reported or
/// compared without carrying the geometry itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GeometryKind {
    /// A single position.
    Point,
    /// A connected run of straight segments.
    LineString,
    /// One exterior ring and zero or more interior rings.
    Polygon,
    /// A set of points.
    MultiPoint,
    /// A set of line strings.
    MultiLineString,
    /// A set of polygons.
    MultiPolygon,
    /// A heterogeneous set of geometries.
    GeometryCollection,
}

impl GeometryKind {
    /// The uppercase WKT keyword for this kind.
    #[must_use]
    pub const fn wkt_keyword(self) -> &'static str {
        match self {
            Self::Point => "POINT",
            Self::LineString => "LINESTRING",
            Self::Polygon => "POLYGON",
            Self::MultiPoint => "MULTIPOINT",
            Self::MultiLineString => "MULTILINESTRING",
            Self::MultiPolygon => "MULTIPOLYGON",
            Self::GeometryCollection => "GEOMETRYCOLLECTION",
        }
    }

    /// The RFC 7946 GeoJSON `type` member for this kind.
    #[must_use]
    pub const fn geojson_type(self) -> &'static str {
        match self {
            Self::Point => "Point",
            Self::LineString => "LineString",
            Self::Polygon => "Polygon",
            Self::MultiPoint => "MultiPoint",
            Self::MultiLineString => "MultiLineString",
            Self::MultiPolygon => "MultiPolygon",
            Self::GeometryCollection => "GeometryCollection",
        }
    }
}

/// A polygon: its exterior ring first, then its interior rings in written order.
///
/// Every ring is **closed** — its last position repeats its first — and has at
/// least four positions, which [`Geometry::new`] enforces. An empty `rings`
/// vector is the empty polygon.
pub type Rings = Vec<CoordSeq>;

/// The coordinates of a geometry, discriminated by kind.
///
/// Constructed only through [`Geometry::new`], which is where the structural
/// checks live; the variants are public so that pattern matching over a geometry
/// is exhaustive and total in every consumer.
// MEASURED, not assumed — `crates/geo/tests/layout.rs` pins every number here.
// After `CoordSeq` was changed from an inline small-vector to a `Vec` (see its
// docs), the only large variant left is `Point`, at 392 bytes, because a `Coord`
// is four exact rationals and is 384 bytes by construction. Clippy's remaining
// advice is to box it. That is refused deliberately: a `POINT` whose ordinates
// fit an `Int`'s inline limbs — which is every coordinate a real dataset holds —
// currently allocates ZERO times, and boxing would put one allocation on the
// commonest geometry in the commonest corpus to move 384 bytes that the
// coordinate itself already occupies either way. The 4x waste was the sequence
// variants, and that is fixed; this residue is the size of the value.
#[allow(
    clippy::large_enum_variant,
    reason = "the Point variant's size is a Coord's, which is four exact rationals; boxing it \
              would add an allocation to the commonest geometry to relocate bytes the value \
              occupies regardless. Pinned by crates/geo/tests/layout.rs."
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryBody {
    /// A single position, or `None` for `POINT EMPTY`.
    Point(Option<Coord>),
    /// Two or more positions, or an empty sequence for `LINESTRING EMPTY`.
    LineString(CoordSeq),
    /// Closed rings, exterior first, or an empty vector for `POLYGON EMPTY`.
    Polygon(Rings),
    /// Zero or more member positions, each of which may itself be empty.
    MultiPoint(Vec<Option<Coord>>),
    /// Zero or more member line strings.
    MultiLineString(Vec<CoordSeq>),
    /// Zero or more member polygons.
    MultiPolygon(Vec<Rings>),
    /// Zero or more member geometries of any kind, including nested collections.
    GeometryCollection(Vec<Geometry>),
}

/// A geometry: a dimension every coordinate in it honours, and a body.
///
/// The coordinate reference system is NOT part of a geometry — it belongs to the
/// literal that carried it (see [`GeometryLiteral`]), because a
/// `GEOMETRYCOLLECTION`'s members share the collection's system and a nested
/// member carrying its own would be a contradiction the model should not be able
/// to express.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Geometry {
    dim: CoordDim,
    body: GeometryBody,
}

impl Geometry {
    /// A geometry of dimension `dim` with `body`, structurally checked.
    ///
    /// # Errors
    ///
    /// [`GeoError::Literal`] when the body is not a geometry:
    ///
    /// * a coordinate whose own ordinates disagree with `dim`;
    /// * a `LineString` with exactly one position (a curve needs two ends);
    /// * a polygon ring with fewer than four positions, or whose last position
    ///   does not repeat its first;
    /// * a `GeometryCollection` member whose dimension differs from `dim`.
    ///
    /// It does **not** refuse a geometry that is merely invalid — a
    /// self-intersecting polygon, a ring wound either way, repeated consecutive
    /// positions — because those are well-formed geometries the specification has
    /// predicates about, and refusing them would reject data a conforming store
    /// must carry.
    pub fn new(dim: CoordDim, body: GeometryBody) -> Result<Self, GeoError> {
        check_body(dim, &body)?;
        Ok(Self { dim, body })
    }

    /// The dimension every coordinate in this geometry carries.
    #[must_use]
    pub const fn dim(&self) -> CoordDim {
        self.dim
    }

    /// The coordinates, discriminated by kind.
    #[must_use]
    pub const fn body(&self) -> &GeometryBody {
        &self.body
    }

    /// This geometry's kind.
    #[must_use]
    pub const fn kind(&self) -> GeometryKind {
        match &self.body {
            GeometryBody::Point(_) => GeometryKind::Point,
            GeometryBody::LineString(_) => GeometryKind::LineString,
            GeometryBody::Polygon(_) => GeometryKind::Polygon,
            GeometryBody::MultiPoint(_) => GeometryKind::MultiPoint,
            GeometryBody::MultiLineString(_) => GeometryKind::MultiLineString,
            GeometryBody::MultiPolygon(_) => GeometryKind::MultiPolygon,
            GeometryBody::GeometryCollection(_) => GeometryKind::GeometryCollection,
        }
    }

    /// The empty geometry of this kind and dimension.
    #[must_use]
    pub fn empty(dim: CoordDim, kind: GeometryKind) -> Self {
        let body = match kind {
            GeometryKind::Point => GeometryBody::Point(None),
            GeometryKind::LineString => GeometryBody::LineString(CoordSeq::new()),
            GeometryKind::Polygon => GeometryBody::Polygon(Rings::new()),
            GeometryKind::MultiPoint => GeometryBody::MultiPoint(Vec::new()),
            GeometryKind::MultiLineString => GeometryBody::MultiLineString(Vec::new()),
            GeometryKind::MultiPolygon => GeometryBody::MultiPolygon(Vec::new()),
            GeometryKind::GeometryCollection => GeometryBody::GeometryCollection(Vec::new()),
        };
        Self { dim, body }
    }

    /// Whether this geometry is the empty set.
    ///
    /// A multi-geometry or collection is empty when it has no members **or** when
    /// every member is itself empty — `MULTIPOINT(EMPTY)` denotes the empty set
    /// exactly as `MULTIPOINT EMPTY` does, and reporting the first as non-empty
    /// would be a classification the geometry does not support.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match &self.body {
            GeometryBody::Point(point) => point.is_none(),
            GeometryBody::LineString(coords) => coords.is_empty(),
            GeometryBody::Polygon(rings) => rings.is_empty(),
            GeometryBody::MultiPoint(points) => points.iter().all(Option::is_none),
            GeometryBody::MultiLineString(lines) => lines.iter().all(Vec::is_empty),
            GeometryBody::MultiPolygon(polygons) => polygons.iter().all(Vec::is_empty),
            GeometryBody::GeometryCollection(members) => members.iter().all(Self::is_empty),
        }
    }

    /// Every position in this geometry, in written order.
    ///
    /// Ring closing positions are included, exactly as written — a consumer that
    /// wants each vertex once drops the last position of each ring itself, and one
    /// that wants a bounding box does not care.
    pub fn coords(&self) -> impl Iterator<Item = &Coord> + '_ {
        // A boxed iterator because the recursion through GeometryCollection makes
        // the concrete type infinite; a collection walk is not a per-row hot path
        // (the predicate engine indexes into the rings directly), so one
        // allocation per collection level is the right trade against an explicit
        // hand-rolled stack machine that would have to be verified separately.
        let iter: Box<dyn Iterator<Item = &Coord> + '_> = match &self.body {
            GeometryBody::Point(point) => Box::new(point.iter()),
            GeometryBody::LineString(coords) => Box::new(coords.iter()),
            GeometryBody::Polygon(rings) => Box::new(rings.iter().flat_map(|r| r.iter())),
            GeometryBody::MultiPoint(points) => Box::new(points.iter().flatten()),
            GeometryBody::MultiLineString(lines) => Box::new(lines.iter().flat_map(|l| l.iter())),
            GeometryBody::MultiPolygon(polygons) => Box::new(
                polygons
                    .iter()
                    .flat_map(|p| p.iter().flat_map(|r| r.iter())),
            ),
            GeometryBody::GeometryCollection(members) => {
                Box::new(members.iter().flat_map(Self::coords))
            }
        };
        iter
    }

    /// The number of positions [`Self::coords`] yields.
    #[must_use]
    pub fn coord_count(&self) -> usize {
        self.coords().count()
    }
}

/// The structural checks [`Geometry::new`] applies, split out so the recursion
/// through `GeometryCollection` is one function rather than a closure.
fn check_body(dim: CoordDim, body: &GeometryBody) -> Result<(), GeoError> {
    match body {
        GeometryBody::Point(point) => point.as_ref().map_or(Ok(()), |c| check_coord(dim, c)),
        GeometryBody::LineString(coords) => check_line(dim, coords),
        GeometryBody::Polygon(rings) => rings.iter().try_for_each(|ring| check_ring(dim, ring)),
        GeometryBody::MultiPoint(points) => points
            .iter()
            .flatten()
            .try_for_each(|c| check_coord(dim, c)),
        GeometryBody::MultiLineString(lines) => lines.iter().try_for_each(|l| check_line(dim, l)),
        GeometryBody::MultiPolygon(polygons) => polygons
            .iter()
            .flat_map(|p| p.iter())
            .try_for_each(|ring| check_ring(dim, ring)),
        GeometryBody::GeometryCollection(members) => members.iter().try_for_each(|member| {
            if member.dim == dim {
                Ok(())
            } else {
                Err(GeoError::literal(format!(
                    "a geometry collection declared {} but holds a {} member; WKT writes the \
                     dimension once and it governs every member",
                    dim_name(dim),
                    dim_name(member.dim)
                )))
            }
        }),
    }
}

fn dim_name(dim: CoordDim) -> &'static str {
    match dim {
        CoordDim::Xy => "XY",
        CoordDim::Xyz => "XYZ",
        CoordDim::Xym => "XYM",
        CoordDim::Xyzm => "XYZM",
    }
}

fn check_coord(dim: CoordDim, coord: &Coord) -> Result<(), GeoError> {
    if coord.dim() == dim {
        return Ok(());
    }
    Err(GeoError::literal(format!(
        "a {} geometry holds a {} coordinate; every coordinate of a geometry carries the \
         dimension the geometry declares",
        dim_name(dim),
        dim_name(coord.dim())
    )))
}

fn check_line(dim: CoordDim, coords: &CoordSeq) -> Result<(), GeoError> {
    if coords.len() == 1 {
        return Err(GeoError::literal(
            "a line string has either no positions (it is empty) or at least two; a single \
             position is a point, not a curve",
        ));
    }
    coords.iter().try_for_each(|c| check_coord(dim, c))
}

fn check_ring(dim: CoordDim, ring: &CoordSeq) -> Result<(), GeoError> {
    if ring.len() < 4 {
        return Err(GeoError::literal(format!(
            "a polygon ring needs at least four positions (three corners and the repeat that \
             closes it); this one has {}",
            ring.len()
        )));
    }
    ring.iter().try_for_each(|c| check_coord(dim, c))?;
    let first = &ring[0];
    let last = &ring[ring.len() - 1];
    // Closure is decided on the PLANAR ordinates: OGC closes a ring in the plane,
    // and a ring whose endpoints differ only in elevation or measure is closed as
    // a ring while still carrying two distinct 3D positions. Refusing it would
    // reject data a conforming store must carry.
    if first.same_planar(last) {
        Ok(())
    } else {
        Err(GeoError::literal(
            "a polygon ring must close: its last position must repeat the first in the plane",
        ))
    }
}

// ---------------------------------------------------------------------------
// Literal
// ---------------------------------------------------------------------------

/// A geometry together with the coordinate reference system its ordinates are
/// expressed in — what a `geo:wktLiteral` or `geo:geoJSONLiteral` actually
/// denotes.
///
/// Every binary operation in this crate compares the two operands' systems and
/// refuses a mismatch. See [`Crs`] for why that is a refusal rather than a
/// conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryLiteral {
    crs: Crs,
    geometry: Geometry,
}

impl GeometryLiteral {
    /// A geometry expressed in `crs`.
    #[must_use]
    pub const fn new(crs: Crs, geometry: Geometry) -> Self {
        Self { crs, geometry }
    }

    /// The coordinate reference system.
    #[must_use]
    pub const fn crs(&self) -> &Crs {
        &self.crs
    }

    /// The geometry.
    #[must_use]
    pub const fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// Take the geometry, dropping the system.
    #[must_use]
    pub fn into_geometry(self) -> Geometry {
        self.geometry
    }

    /// Check that `self` and `other` are expressed in the same system.
    ///
    /// # Errors
    ///
    /// [`GeoError::Domain`] naming both systems when they differ. This crate
    /// reprojects nothing, so comparing coordinates across two systems would be
    /// arithmetic on numbers that do not describe the same space — the refusal is
    /// the only honest answer available to it.
    pub fn require_same_crs(&self, other: &Self) -> Result<(), GeoError> {
        if self.crs == other.crs {
            return Ok(());
        }
        Err(GeoError::domain(format!(
            "the two geometries are in different coordinate reference systems (<{}> and <{}>); \
             purrdf-geo reprojects nothing, so it refuses rather than treating their ordinates \
             as comparable",
            self.crs, other.crs
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral,
    };
    use crate::error::GeoError;
    use crate::exact::Rat;

    fn r(value: i64) -> Rat {
        Rat::from_i64(value)
    }

    fn xy(x: i64, y: i64) -> Coord {
        Coord::xy(r(x), r(y))
    }

    fn seq(points: &[(i64, i64)]) -> CoordSeq {
        points.iter().map(|&(x, y)| xy(x, y)).collect()
    }

    fn crs() -> Crs {
        Crs::new("http://example.org/crs/planar").expect("a non-empty IRI")
    }

    // ---- CRS -------------------------------------------------------------

    #[test]
    fn a_crs_iri_must_be_an_absolute_iri_and_ordinary_ones_still_are() {
        // Empty names no system.
        assert!(matches!(Crs::new(""), Err(GeoError::Config(_))));
        // Whitespace-padded. This is the one that mattered: it was accepted
        // verbatim, never compared equal to the identical-looking declared
        // system, and was re-emitted into an unreadable wktLiteral.
        for bad in [
            " ",
            "   ",
            " http://example.org/crs/planar ",
            "http://example.org/crs/a b",
            "not an iri at all",
            "{\"a\":1}",
            "crs/planar",
            "//example.org/crs",
            "1http://example.org/",
            "http://example.org/<x>",
        ] {
            assert!(
                matches!(Crs::new(bad), Err(GeoError::Config(_))),
                "{bad:?} is not an absolute IRI and must be refused"
            );
        }
        // The neighbouring VALID cases. This type still mints nothing and still
        // takes the caller's IRI byte-exactly; the check must not have narrowed
        // which real systems can be named.
        for good in [
            "http://example.org/crs/1",
            "https://example.org/crs/1",
            "urn:x:1",
            "urn:ogc:def:crs:OGC:1.3:CRS84",
            "http://www.opengis.net/def/crs/EPSG/0/4326",
            "x-custom.scheme+v2:anything/at#all?q=1",
        ] {
            assert_eq!(
                Crs::new(good)
                    .expect("an absolute IRI is accepted")
                    .as_str(),
                good,
                "{good:?} must be accepted byte-exactly"
            );
        }
    }

    #[test]
    fn a_crs_mismatch_is_refused_and_a_match_is_not() {
        let a = GeometryLiteral::new(crs(), Geometry::empty(CoordDim::Xy, GeometryKind::Point));
        let b = GeometryLiteral::new(
            Crs::new("http://example.org/crs/other").expect("iri"),
            Geometry::empty(CoordDim::Xy, GeometryKind::Point),
        );
        assert!(matches!(a.require_same_crs(&b), Err(GeoError::Domain(_))));
        // The neighbouring VALID case.
        assert!(a.require_same_crs(&a.clone()).is_ok());
    }

    // ---- dimension -------------------------------------------------------

    #[test]
    fn the_dimension_tags_and_ordinate_counts_are_the_wkt_ones() {
        for (dim, tag, ordinates) in [
            (CoordDim::Xy, "", 2),
            (CoordDim::Xyz, "Z", 3),
            (CoordDim::Xym, "M", 3),
            (CoordDim::Xyzm, "ZM", 4),
        ] {
            assert_eq!(dim.wkt_tag(), tag);
            assert_eq!(dim.ordinates(), ordinates);
            assert_eq!(CoordDim::new(dim.has_z(), dim.has_m()), dim);
        }
    }

    #[test]
    fn a_coordinate_must_carry_its_geometrys_dimension() {
        let three_d = Coord::new(r(1), r(2), Some(r(3)), None);
        assert!(matches!(
            Geometry::new(CoordDim::Xy, GeometryBody::Point(Some(three_d.clone()))),
            Err(GeoError::Literal(_))
        ));
        // The neighbouring VALID case: the same coordinate under the dimension it
        // actually carries.
        assert!(Geometry::new(CoordDim::Xyz, GeometryBody::Point(Some(three_d))).is_ok());
    }

    // ---- structural checks, each with its valid neighbour -----------------

    #[test]
    fn a_one_position_line_is_refused_but_two_and_zero_are_not() {
        assert!(matches!(
            Geometry::new(CoordDim::Xy, GeometryBody::LineString(seq(&[(0, 0)]))),
            Err(GeoError::Literal(_))
        ));
        assert!(
            Geometry::new(
                CoordDim::Xy,
                GeometryBody::LineString(seq(&[(0, 0), (1, 1)]))
            )
            .is_ok(),
            "two positions is the smallest curve"
        );
        assert!(
            Geometry::new(CoordDim::Xy, GeometryBody::LineString(CoordSeq::new())).is_ok(),
            "no positions is LINESTRING EMPTY"
        );
    }

    #[test]
    fn a_short_or_unclosed_ring_is_refused_but_a_closed_triangle_is_not() {
        let short = vec![seq(&[(0, 0), (1, 0), (0, 0)])];
        assert!(matches!(
            Geometry::new(CoordDim::Xy, GeometryBody::Polygon(short)),
            Err(GeoError::Literal(_))
        ));
        let unclosed = vec![seq(&[(0, 0), (1, 0), (0, 1), (2, 2)])];
        assert!(matches!(
            Geometry::new(CoordDim::Xy, GeometryBody::Polygon(unclosed)),
            Err(GeoError::Literal(_))
        ));
        // The neighbouring VALID case.
        let closed = vec![seq(&[(0, 0), (1, 0), (0, 1), (0, 0)])];
        assert!(Geometry::new(CoordDim::Xy, GeometryBody::Polygon(closed)).is_ok());
    }

    /// The structural checks must NOT reject a geometry that is merely invalid:
    /// a self-intersecting polygon and a line that doubles back are well-formed
    /// and a conforming store has to carry them.
    #[test]
    fn geometric_invalidity_is_not_a_structural_refusal() {
        let bowtie = vec![seq(&[(0, 0), (2, 2), (2, 0), (0, 2), (0, 0)])];
        assert!(
            Geometry::new(CoordDim::Xy, GeometryBody::Polygon(bowtie)).is_ok(),
            "a self-intersecting ring is a well-formed geometry with predicates about it"
        );
        let doubles_back = seq(&[(0, 0), (1, 0), (0, 0)]);
        assert!(Geometry::new(CoordDim::Xy, GeometryBody::LineString(doubles_back)).is_ok());
        let repeated = seq(&[(0, 0), (0, 0), (1, 1)]);
        assert!(
            Geometry::new(CoordDim::Xy, GeometryBody::LineString(repeated)).is_ok(),
            "repeated consecutive positions are legal"
        );
    }

    /// A ring closes in the PLANE: endpoints that differ only in elevation still
    /// close it, and refusing them would reject valid 3D data.
    #[test]
    fn a_ring_closes_on_the_planar_ordinates() {
        let ring: CoordSeq = [
            Coord::new(r(0), r(0), Some(r(0)), None),
            Coord::new(r(1), r(0), Some(r(1)), None),
            Coord::new(r(0), r(1), Some(r(2)), None),
            Coord::new(r(0), r(0), Some(r(9)), None),
        ]
        .into_iter()
        .collect();
        assert!(Geometry::new(CoordDim::Xyz, GeometryBody::Polygon(vec![ring])).is_ok());
    }

    #[test]
    fn a_collection_member_must_share_the_collections_dimension() {
        let flat = Geometry::new(CoordDim::Xy, GeometryBody::Point(Some(xy(1, 2)))).expect("point");
        assert!(matches!(
            Geometry::new(
                CoordDim::Xyz,
                GeometryBody::GeometryCollection(vec![flat.clone()])
            ),
            Err(GeoError::Literal(_))
        ));
        // The neighbouring VALID case.
        assert!(Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(vec![flat])).is_ok());
    }

    // ---- emptiness -------------------------------------------------------

    #[test]
    fn a_multi_geometry_of_empties_is_empty() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            let empty = Geometry::empty(CoordDim::Xy, kind);
            assert!(empty.is_empty(), "{kind:?} EMPTY is empty");
            assert_eq!(empty.kind(), kind);
            assert_eq!(empty.coord_count(), 0);
        }

        let multipoint_of_empty =
            Geometry::new(CoordDim::Xy, GeometryBody::MultiPoint(vec![None, None]))
                .expect("well formed");
        assert!(
            multipoint_of_empty.is_empty(),
            "MULTIPOINT(EMPTY, EMPTY) denotes the empty set exactly as MULTIPOINT EMPTY does"
        );

        let nested = Geometry::new(
            CoordDim::Xy,
            GeometryBody::GeometryCollection(vec![Geometry::empty(
                CoordDim::Xy,
                GeometryKind::GeometryCollection,
            )]),
        )
        .expect("well formed");
        assert!(nested.is_empty(), "emptiness recurses through collections");

        // The neighbouring NON-empty case, so the assertions above turn on
        // emptiness rather than on everything reporting empty.
        let one_point = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(vec![None, Some(xy(1, 1))]),
        )
        .expect("well formed");
        assert!(!one_point.is_empty());
        assert_eq!(one_point.coord_count(), 1);
    }

    // ---- traversal -------------------------------------------------------

    #[test]
    fn coords_yields_every_position_in_written_order_including_ring_closures() {
        let polygon = Geometry::new(
            CoordDim::Xy,
            GeometryBody::Polygon(vec![
                seq(&[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)]),
                seq(&[(1, 1), (2, 1), (2, 2), (1, 1)]),
            ]),
        )
        .expect("closed rings");
        let seen: Vec<(&Rat, &Rat)> = polygon.coords().map(|c| (c.x(), c.y())).collect();
        assert_eq!(seen.len(), 9, "5 exterior + 4 interior, closures included");
        assert_eq!(*seen[0].0, r(0));
        assert_eq!(*seen[5].0, r(1), "the interior ring follows the exterior");

        let collection = Geometry::new(
            CoordDim::Xy,
            GeometryBody::GeometryCollection(vec![
                Geometry::new(CoordDim::Xy, GeometryBody::Point(Some(xy(7, 8)))).expect("point"),
                polygon,
            ]),
        )
        .expect("uniform dimension");
        assert_eq!(collection.coord_count(), 10);
        assert_eq!(
            *collection.coords().next().expect("first").x(),
            r(7),
            "collection members are walked in written order"
        );
    }

    #[test]
    fn the_keywords_are_the_serialization_ones() {
        assert_eq!(
            GeometryKind::MultiLineString.wkt_keyword(),
            "MULTILINESTRING"
        );
        assert_eq!(
            GeometryKind::MultiLineString.geojson_type(),
            "MultiLineString"
        );
        assert_eq!(
            GeometryKind::GeometryCollection.wkt_keyword(),
            "GEOMETRYCOLLECTION"
        );
        assert_eq!(
            GeometryKind::GeometryCollection.geojson_type(),
            "GeometryCollection"
        );
    }

    #[test]
    fn planar_equality_ignores_elevation_and_measure_but_structural_equality_does_not() {
        let a = Coord::new(r(1), r(2), Some(r(3)), Some(r(4)));
        let b = Coord::new(r(1), r(2), Some(r(9)), Some(r(9)));
        assert!(a.same_planar(&b));
        assert_ne!(a, b, "structural equality keeps every ordinate");
        let c = Coord::new(r(1), r(5), Some(r(3)), Some(r(4)));
        assert!(!a.same_planar(&c));
    }
}
