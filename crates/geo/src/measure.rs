// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The non-topological measures: bounding ordinates, area, length, perimeter,
//! distance, member counts and simplicity.
//!
//! # Everything here is planar, because the specification says so
//!
//! OGC GeoSPARQL 1.1, Clause 10.2, verbatim:
//!
//! > Geometric functions working with Geometries that have Z values will ignore
//! > Z values in calculations and first project geometry onto the Z=0 level. …
//! > Like Z values in coordinates, M values are to be ignored.
//!
//! So every computation in this module reads [`Coord::x`] and [`Coord::y`] and
//! nothing else. A `POLYGON Z` has the area of its shadow on the `Z=0` plane, a
//! `LINESTRING Z` has the length of its shadow, and two geometries that differ
//! only in elevation are at distance zero from one another. The only two
//! functions that look at an elevation at all are [`min_z`] and [`max_z`], and
//! they **report** the ordinate rather than computing with it — which is exactly
//! what `geof:minZ`/`geof:maxZ` are for.
//!
//! The projection is applied once, on the way in: `Parts` holds
//! [`crate::geom::CoordDim::Xy`] positions produced by
//! [`crate::topology`]'s primitives, so no measurement below is in a position to
//! read a `Z` even by mistake.
//!
//! # The exact primitives are shared, not re-derived
//!
//! Orientation, point-on-segment and segment-intersection live in
//! [`crate::topology`] and are used from here rather than reimplemented.
//! Two independent implementations of an exact geometric predicate is exactly the
//! drift this crate exists to prevent: a `distance` that decided "these touch"
//! differently from the way `relate` decides it would make `geof:distance(a, b) =
//! 0` and `geo:sfIntersects(a, b)` disagree about the same pair of geometries.
//!
//! # Irrational measures are exact integer square roots at a fixed scale
//!
//! [`length`], [`perimeter`] and [`distance`] are sums of square roots, and a
//! square root of a rational is almost never rational. That is the one place in
//! this crate where an answer cannot be exact, so it is the one place where the
//! rounding has to be *specified* rather than inherited from the hardware.
//!
//! The naive implementation — round each term to some precision and add the
//! rounded terms — makes the total depend on the rounding of every term, and
//! makes a differently-ordered traversal give a different answer, because
//! rounded addition is not associative. That is precisely the `f64` failure mode
//! this crate exists to close.
//!
//! So each segment's length is computed as an **exact integer** at the fixed
//! internal scale [`LENGTH_SCALE_DIGITS`], and the terms are summed as integers.
//! Integer addition *is* associative and *is* fully specified by Rust on every
//! target, so the sum is a pure function of the multiset of segments: it cannot
//! depend on traversal order, and a native build and a `wasm32-unknown-unknown`
//! build produce the same digits by construction rather than by luck.
//!
//! ## The derivation
//!
//! For a segment from `p` to `q`, the exact squared length is the rational
//! `d2 = (qx - px)^2 + (qy - py)^2`. Write it in lowest terms as `n / m`, with
//! `n >= 0` and `m > 0` ([`Rat`] is always canonical, so this is free). The
//! quantity wanted is the true length scaled by `10^18` and truncated:
//!
//! ```text
//! sqrt(n / m) * 10^18
//!   = sqrt( (n / m) * 10^36 )        (10^18 = sqrt(10^36), and both are >= 0)
//!   = sqrt( n * 10^36 / m )
//!   = sqrt( n * m * 10^36 / m^2 )    (multiply inside the root by m/m)
//!   = sqrt( n * m * 10^36 ) / m      (m > 0, so it leaves the root as +m)
//! ```
//!
//! So with `t = n * m * 10^36` and `s = floor(sqrt(t))` — an exact
//! arbitrary-precision integer square root, [`Int::sqrt_floor`] — the scaled
//! length is `floor(s / m)`. That truncated division is the *same* value as
//! `floor(sqrt(t) / m)`, because `floor(floor(x) / m) == floor(x / m)` for every
//! real `x >= 0` and integer `m > 0`; no second rounding sneaks in.
//!
//! ## The honest error bound
//!
//! Each segment contributes `floor(len * 10^18)`, which understates that
//! segment by strictly less than `10^-18`. A total over `k` segments is
//! therefore a **truncation of the true measure**, never an overstatement, with
//! absolute error strictly less than `k * 10^-18`. Nothing here is a rounding
//! *estimate*: the digits above `10^-18` are correct, the sign of the error is
//! known, and zero floating-point operations were performed to obtain it.
//!
//! [`LENGTH_SCALE_DIGITS`] is a **constant of the contract**, not a tuning
//! parameter. Changing it changes every emitted length, so it is documented and
//! pinned exactly as `purrdf-text`'s `SERIES_TERMS` is.

use crate::de9im::Set;
use crate::exact::{Int, Rat};
use crate::geom::{Coord, CoordSeq, Geometry, GeometryBody};
use crate::topology::{SegmentIntersection, intersect, locate, on_segment};

/// The Clause 10.2 projection of `coord`: its `x` and `y`, with elevation and
/// measure dropped.
///
/// Applied once, at the boundary of every decomposition below, so that no
/// measurement is in a position to read a `Z` even by mistake.
fn plane(coord: &Coord) -> Coord {
    Coord::xy(coord.x().clone(), coord.y().clone())
}

// ---------------------------------------------------------------------------
// The internal scale
// ---------------------------------------------------------------------------

/// The number of fraction digits every irrational measure in this crate is
/// computed to, before any caller-facing rounding.
///
/// [`length`], [`perimeter`], [`distance`] and the length-weighted centroid in
/// [`crate::construct`] all accumulate exact integers at this scale and divide
/// once, at the end. See the module documentation for the derivation and the
/// error bound.
///
/// This is part of the crate's contract and not a knob: two runs, two builds and
/// two targets agree on a length because they agree on this constant and on
/// integer arithmetic. Eighteen digits is well past the roughly fifteen
/// significant decimal digits an `xsd:double` result can carry, so the truncation
/// here is never what limits a caller's answer.
pub const LENGTH_SCALE_DIGITS: u32 = 18;

// ---------------------------------------------------------------------------
// Decomposition
// ---------------------------------------------------------------------------

/// A straight segment between two planar positions. Its endpoints may coincide,
/// in which case it denotes a single point.
pub(crate) type Segment = [Coord; 2];

/// A geometry taken apart into the three kinds of planar piece every measure
/// here is defined over, with collections flattened.
///
/// Every position in it has already been through
/// [`plane`], so it is `XY` and no consumer is in a
/// position to read an elevation.
///
/// Empty pieces are dropped on the way in — an empty member contributes no
/// point, no curve and no ring — so a non-empty geometry always yields at least
/// one piece and an empty one always yields none.
#[derive(Clone, Debug, Default)]
pub(crate) struct Parts {
    /// Every `Point` and `MultiPoint` member, projected.
    pub(crate) points: Vec<Coord>,
    /// Every curve component's vertex run, in written order.
    pub(crate) lines: Vec<Vec<Coord>>,
    /// Every polygon component, as its rings: exterior first, then holes.
    pub(crate) polygons: Vec<Vec<Vec<Coord>>>,
}

impl Parts {
    /// Take `geometry` apart, recursing through collections.
    pub(crate) fn of(geometry: &Geometry) -> Self {
        let mut parts = Self::default();
        parts.absorb(geometry);
        parts
    }

    /// Add `geometry`'s pieces to `self`.
    fn absorb(&mut self, geometry: &Geometry) {
        match geometry.body() {
            GeometryBody::Point(point) => {
                self.points.extend(point.iter().map(plane));
            }
            GeometryBody::LineString(coords) => self.push_line(coords),
            GeometryBody::Polygon(rings) => self.push_polygon(rings),
            GeometryBody::MultiPoint(points) => {
                self.points.extend(points.iter().flatten().map(plane));
            }
            GeometryBody::MultiLineString(lines) => {
                for line in lines {
                    self.push_line(line);
                }
            }
            GeometryBody::MultiPolygon(polygons) => {
                for rings in polygons {
                    self.push_polygon(rings);
                }
            }
            GeometryBody::GeometryCollection(members) => {
                for member in members {
                    self.absorb(member);
                }
            }
        }
    }

    /// Record a curve component, unless it is empty.
    fn push_line(&mut self, coords: &CoordSeq) {
        if coords.len() >= 2 {
            self.lines.push(coords.iter().map(plane).collect());
        }
    }

    /// Record a polygon component, unless it is empty.
    fn push_polygon(&mut self, rings: &[CoordSeq]) {
        if rings.is_empty() {
            return;
        }
        self.polygons.push(
            rings
                .iter()
                .map(|ring| ring.iter().map(plane).collect())
                .collect(),
        );
    }

    /// Every straight segment in the geometry: the curve components' segments
    /// and every polygon ring's segments.
    pub(crate) fn segments(&self) -> Vec<Segment> {
        let mut out = Vec::new();
        for line in &self.lines {
            push_segments(line, &mut out);
        }
        for polygon in &self.polygons {
            for ring in polygon {
                push_segments(ring, &mut out);
            }
        }
        out
    }

    /// Every position the geometry names: its point members and every vertex of
    /// every curve and ring.
    pub(crate) fn vertices(&self) -> Vec<&Coord> {
        let mut out: Vec<&Coord> = self.points.iter().collect();
        for line in &self.lines {
            out.extend(line.iter());
        }
        for polygon in &self.polygons {
            for ring in polygon {
                out.extend(ring.iter());
            }
        }
        out
    }
}

/// Append the consecutive-pair segments of `path` to `out`.
fn push_segments(path: &[Coord], out: &mut Vec<Segment>) {
    for pair in path.windows(2) {
        out.push([pair[0].clone(), pair[1].clone()]);
    }
}

// ---------------------------------------------------------------------------
// Bounding ordinates
// ---------------------------------------------------------------------------

/// The axis-aligned bounding box of `geometry`: `(min_x, min_y, max_x, max_y)`,
/// or `None` when the geometry is empty.
///
/// The box is planar: a `POLYGON Z` has the bounding box of its projection onto
/// `Z=0`, per Clause 10.2. Every ordinate is exact — the box is a comparison of
/// rationals, never a widened float — so the corners are literally ordinates
/// that appeared in the input.
#[must_use]
pub fn bounds(geometry: &Geometry) -> Option<(Rat, Rat, Rat, Rat)> {
    let mut coords = geometry.coords();
    let first = coords.next()?;
    let mut min_x = first.x().clone();
    let mut max_x = min_x.clone();
    let mut min_y = first.y().clone();
    let mut max_y = min_y.clone();
    for coord in coords {
        // `min_x <= max_x` always holds, so a value below the minimum cannot
        // also be above the maximum and the `else` costs nothing.
        if *coord.x() < min_x {
            min_x = coord.x().clone();
        } else if *coord.x() > max_x {
            max_x = coord.x().clone();
        }
        if *coord.y() < min_y {
            min_y = coord.y().clone();
        } else if *coord.y() > max_y {
            max_y = coord.y().clone();
        }
    }
    Some((min_x, min_y, max_x, max_y))
}

/// The smallest `x` ordinate, or `None` when the geometry is empty.
#[must_use]
pub fn min_x(geometry: &Geometry) -> Option<Rat> {
    bounds(geometry).map(|(value, _, _, _)| value)
}

/// The largest `x` ordinate, or `None` when the geometry is empty.
#[must_use]
pub fn max_x(geometry: &Geometry) -> Option<Rat> {
    bounds(geometry).map(|(_, _, value, _)| value)
}

/// The smallest `y` ordinate, or `None` when the geometry is empty.
#[must_use]
pub fn min_y(geometry: &Geometry) -> Option<Rat> {
    bounds(geometry).map(|(_, value, _, _)| value)
}

/// The largest `y` ordinate, or `None` when the geometry is empty.
#[must_use]
pub fn max_y(geometry: &Geometry) -> Option<Rat> {
    bounds(geometry).map(|(_, _, _, value)| value)
}

/// The smallest elevation, or `None` when the geometry carries no `Z` ordinate
/// (because it is empty, or because it is not a `Z` geometry).
///
/// This and [`max_z`] are the module's only two exceptions to Clause 10.2's
/// projection rule, and they are exceptions in the harmless direction: they
/// **report** the elevation rather than computing with it. No length, area or
/// distance anywhere in this crate reads a `Z`.
#[must_use]
pub fn min_z(geometry: &Geometry) -> Option<Rat> {
    geometry.coords().filter_map(Coord::z).min().cloned()
}

/// The largest elevation, or `None` when the geometry carries no `Z` ordinate.
///
/// See [`min_z`] for why reporting `Z` does not contradict the projection rule.
#[must_use]
pub fn max_z(geometry: &Geometry) -> Option<Rat> {
    geometry.coords().filter_map(Coord::z).max().cloned()
}

// ---------------------------------------------------------------------------
// Area
// ---------------------------------------------------------------------------

/// The exact planar area: the sum over polygon components of the absolute
/// shoelace area of the exterior ring minus that of each hole.
///
/// The result is a plain [`Rat`] with no rounding whatsoever — a polygon's area
/// is a rational function of rational coordinates, so there is nothing to round.
///
/// It is exactly zero for every geometry with no polygon component, which is
/// what the specification requires rather than an omission: `geof:area` "must
/// return zero for all geometry types other than Polygon". A point has no area,
/// a curve has no area, and reporting anything else would be inventing one.
///
/// Winding does not matter: each ring's shoelace sum is taken in absolute value,
/// so a clockwise and a counter-clockwise spelling of the same ring give the
/// same positive area.
#[must_use]
pub fn area(geometry: &Geometry) -> Rat {
    let mut total = Rat::zero();
    for polygon in &Parts::of(geometry).polygons {
        let Some((exterior, holes)) = polygon.split_first() else {
            continue;
        };
        let mut component = signed_ring_area(exterior).abs();
        for hole in holes {
            component = component.sub(&signed_ring_area(hole).abs());
        }
        total = total.add(&component);
    }
    total
}

/// The signed shoelace area of a closed ring: positive when the ring is wound
/// counter-clockwise, negative when clockwise, zero when degenerate.
pub(crate) fn signed_ring_area(ring: &[Coord]) -> Rat {
    let mut twice = Rat::zero();
    for pair in ring.windows(2) {
        let (p, q) = (&pair[0], &pair[1]);
        twice = twice.add(&p.x().mul(q.y()).sub(&q.x().mul(p.y())));
    }
    twice
        .div(&Rat::from_i64(2))
        .expect("two is not zero, so the halving cannot fail")
}

// ---------------------------------------------------------------------------
// Length, perimeter and the scaled square root
// ---------------------------------------------------------------------------

/// `floor(|q - p| * 10^LENGTH_SCALE_DIGITS)`, exactly, with no floating point.
///
/// See the module documentation for the derivation: with the exact squared
/// length reduced to `n / m`, this is `floor(floor(sqrt(n * m * 10^36)) / m)`.
pub(crate) fn scaled_segment_length(p: &Coord, q: &Coord) -> Int {
    let dx = q.x().sub(p.x());
    let dy = q.y().sub(p.y());
    let squared = dx.mul(&dx).add(&dy.mul(&dy));
    if squared.is_zero() {
        return Int::zero();
    }
    let numerator = squared.numerator();
    let denominator = squared.denominator();
    let radicand = numerator
        .mul(denominator)
        .mul(&Int::pow10(2 * LENGTH_SCALE_DIGITS));
    let root = radicand
        .sqrt_floor()
        .expect("a sum of two squares is never negative");
    root.div_rem(denominator)
        .expect("a canonical denominator is strictly positive")
        .0
}

/// The rational a scaled-integer accumulator denotes: `scaled / 10^18`.
pub(crate) fn from_scaled(scaled: &Int) -> Rat {
    Rat::new(scaled.clone(), Int::pow10(LENGTH_SCALE_DIGITS)).expect("a power of ten is not zero")
}

/// The scaled length of one vertex run.
fn scaled_path_length(path: &[Coord]) -> Int {
    let mut total = Int::zero();
    for pair in path.windows(2) {
        total = total.add(&scaled_segment_length(&pair[0], &pair[1]));
    }
    total
}

/// The scaled length of a set of vertex runs.
fn scaled_paths_length(paths: &[Vec<Coord>]) -> Int {
    let mut total = Int::zero();
    for path in paths {
        total = total.add(&scaled_path_length(path));
    }
    total
}

/// The length of every curve component, at the fixed internal precision
/// [`LENGTH_SCALE_DIGITS`].
///
/// "Curve component" means a `LineString`, a `MultiLineString` member, or either
/// of those reached through a `GeometryCollection`. It is exactly zero for a
/// geometry with no curve component — a point has no length, and a polygon's
/// ring length is [`perimeter`], which is the function the specification gives
/// that measurement to.
///
/// The value is planar (Clause 10.2: a `LINESTRING Z` has the length of its
/// shadow on `Z=0`) and is a truncation of the true length with error strictly
/// below `segment_count * 10^-18`. It is computed with zero floating-point
/// operations and is therefore bit-identical on a native and a
/// `wasm32-unknown-unknown` build, and independent of the order the segments are
/// visited in.
#[must_use]
pub fn length(geometry: &Geometry) -> Rat {
    from_scaled(&scaled_paths_length(&Parts::of(geometry).lines))
}

/// The length of every polygon component's rings, at the fixed internal
/// precision [`LENGTH_SCALE_DIGITS`].
///
/// Every ring counts, exterior and holes alike: the perimeter of a surface is
/// the length of its whole boundary.
///
/// For a geometry with **no** polygon component this is exactly [`length`],
/// which is what the specification says: `geof:perimeter` "for non-areal
/// geometries the result equals the length". For a geometry that mixes a polygon
/// with a curve, the polygon rings are what is measured — the curve is not part
/// of any perimeter — so a `GEOMETRYCOLLECTION(POLYGON(...), LINESTRING(...))`
/// reports the polygon's ring length alone.
///
/// The determinism and error argument is [`length`]'s, unchanged.
#[must_use]
pub fn perimeter(geometry: &Geometry) -> Rat {
    let parts = Parts::of(geometry);
    if parts.polygons.is_empty() {
        return from_scaled(&scaled_paths_length(&parts.lines));
    }
    let mut total = Int::zero();
    for polygon in &parts.polygons {
        total = total.add(&scaled_paths_length(polygon));
    }
    from_scaled(&total)
}

// ---------------------------------------------------------------------------
// Distance
// ---------------------------------------------------------------------------

/// `floor(dist(p, segment) * 10^LENGTH_SCALE_DIGITS)`.
///
/// The foot of the perpendicular is found exactly — the parameter
/// `dot(p - a, r) / dot(r, r)` is a rational — and clamped to `[0, 1]` so that a
/// foot beyond an end of the segment falls back to that endpoint. Only the final
/// square root is scaled and truncated.
fn scaled_point_segment(p: &Coord, segment: &Segment) -> Int {
    let (a, b) = (&segment[0], &segment[1]);
    let rx = b.x().sub(a.x());
    let ry = b.y().sub(a.y());
    let squared = rx.mul(&rx).add(&ry.mul(&ry));
    if squared.is_zero() {
        return scaled_segment_length(p, a);
    }
    let raw = p
        .x()
        .sub(a.x())
        .mul(&rx)
        .add(&p.y().sub(a.y()).mul(&ry))
        .div(&squared)
        .expect("a non-degenerate segment has a non-zero squared length");
    let parameter = if raw < Rat::zero() {
        Rat::zero()
    } else if raw > Rat::one() {
        Rat::one()
    } else {
        raw
    };
    let foot = Coord::xy(
        a.x().add(&parameter.mul(&rx)),
        a.y().add(&parameter.mul(&ry)),
    );
    scaled_segment_length(p, &foot)
}

/// `floor(dist(s, t) * 10^LENGTH_SCALE_DIGITS)` for two segments **known not to
/// meet**: the smallest of the four endpoint-to-other-segment distances.
///
/// That is the correct answer only because they are disjoint; the caller
/// establishes that with [`crate::topology::intersect`] rather than by
/// comparing a computed value against zero.
fn scaled_segment_segment(s: &Segment, t: &Segment) -> Int {
    let mut best = scaled_point_segment(&s[0], t);
    for candidate in [
        scaled_point_segment(&s[1], t),
        scaled_point_segment(&t[0], s),
        scaled_point_segment(&t[1], s),
    ] {
        if candidate < best {
            best = candidate;
        }
    }
    best
}

/// Whether `a` and `b` share any point of the plane, decided exactly.
///
/// Two geometries meet exactly when some segment of one meets some segment of
/// the other, **or** some named position of one is non-exterior to the other.
/// The second clause is what catches containment without a crossing — a point
/// inside a polygon, or a polygon wholly inside another, whose ring vertices are
/// named positions — and it is answered by [`crate::topology::locate`],
/// the same classifier the DE-9IM relate uses, so `distance(a, b) == 0` and
/// `sfIntersects(a, b)` cannot disagree.
fn geometries_meet(a: &Geometry, parts_a: &Parts, b: &Geometry, parts_b: &Parts) -> bool {
    for s in &parts_a.segments() {
        for t in &parts_b.segments() {
            if intersect(&s[0], &s[1], &t[0], &t[1]) != SegmentIntersection::None {
                return true;
            }
        }
    }
    parts_a
        .vertices()
        .into_iter()
        .any(|p| locate(p, b) != Set::Exterior)
        || parts_b
            .vertices()
            .into_iter()
            .any(|p| locate(p, a) != Set::Exterior)
}

/// The shortest distance between any point of `a` and any point of `b`, at the
/// fixed internal precision [`LENGTH_SCALE_DIGITS`]; `None` when either geometry
/// is empty.
///
/// The distance is planar, per Clause 10.2: two geometries that differ only in
/// elevation are at distance zero.
///
/// **Zero is decided topologically, not numerically.** When the two geometries
/// share any point the answer is exactly `0`, and that case is detected with
/// exact segment-intersection and point-location tests rather than by computing a
/// small number and comparing it against a tolerance. So a point inside a
/// polygon, two touching polygons and two crossing curves all report `0` — not
/// "approximately 0" — and no epsilon appears anywhere.
///
/// Otherwise the answer is the minimum over every (point, point),
/// (point, segment) and (segment, segment) pair of the exact distance, scaled and
/// truncated once per pair by `scaled_segment_length`. Because a minimum of
/// truncated values is the truncation of the minimum, the result carries the
/// single-segment error bound — strictly under `10^-18`, never an overstatement —
/// rather than the whole-sum bound [`length`] carries.
#[must_use]
pub fn distance(a: &Geometry, b: &Geometry) -> Option<Rat> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let parts_a = Parts::of(a);
    let parts_b = Parts::of(b);
    if geometries_meet(a, &parts_a, b, &parts_b) {
        return Some(Rat::zero());
    }
    let segments_a = parts_a.segments();
    let segments_b = parts_b.segments();
    let mut best: Option<Int> = None;
    let mut offer = |candidate: Int| {
        if best.as_ref().is_none_or(|current| candidate < *current) {
            best = Some(candidate);
        }
    };
    for p in &parts_a.points {
        for q in &parts_b.points {
            offer(scaled_segment_length(p, q));
        }
        for t in &segments_b {
            offer(scaled_point_segment(p, t));
        }
    }
    for q in &parts_b.points {
        for s in &segments_a {
            offer(scaled_point_segment(q, s));
        }
    }
    for s in &segments_a {
        for t in &segments_b {
            offer(scaled_segment_segment(s, t));
        }
    }
    Some(from_scaled(&best.expect(
        "a non-empty geometry always decomposes into at least one piece",
    )))
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

/// The number of top-level geometries: 1 for a simple geometry, the member count
/// for a multi-geometry or collection.
///
/// The count is structural, so `MULTIPOINT(EMPTY, EMPTY)` has two members even
/// though it denotes the empty set: `geof:geometryN` must be able to reach both
/// of them, and a count that hid them would make the accessor unusable.
#[must_use]
pub fn num_geometries(geometry: &Geometry) -> usize {
    match geometry.body() {
        GeometryBody::MultiPoint(points) => points.len(),
        GeometryBody::MultiLineString(lines) => lines.len(),
        GeometryBody::MultiPolygon(polygons) => polygons.len(),
        GeometryBody::GeometryCollection(members) => members.len(),
        GeometryBody::Point(_) | GeometryBody::LineString(_) | GeometryBody::Polygon(_) => 1,
    }
}

/// The 1-based `n`-th member, or the geometry itself when it is not a collection
/// and `n` is 1.
///
/// `None` when `n` is zero or past [`num_geometries`]. A multi-geometry's member
/// comes back as the corresponding simple kind — a `MultiPolygon` member is a
/// `Polygon` — carrying the parent's dimension, because that is the geometry the
/// parent actually holds.
#[must_use]
pub fn geometry_n(geometry: &Geometry, n: usize) -> Option<Geometry> {
    let index = n.checked_sub(1)?;
    let dim = geometry.dim();
    let built =
        |body| Geometry::new(dim, body).expect("a member of a well-formed geometry is well-formed");
    match geometry.body() {
        GeometryBody::MultiPoint(points) => points
            .get(index)
            .map(|point| built(GeometryBody::Point(point.clone()))),
        GeometryBody::MultiLineString(lines) => lines
            .get(index)
            .map(|line| built(GeometryBody::LineString(line.clone()))),
        GeometryBody::MultiPolygon(polygons) => polygons
            .get(index)
            .map(|rings| built(GeometryBody::Polygon(rings.clone()))),
        GeometryBody::GeometryCollection(members) => members.get(index).cloned(),
        GeometryBody::Point(_) | GeometryBody::LineString(_) | GeometryBody::Polygon(_) => {
            (index == 0).then(|| geometry.clone())
        }
    }
}

// ---------------------------------------------------------------------------
// Simplicity
// ---------------------------------------------------------------------------

/// Whether the geometry has no anomalous points — self-intersection or
/// self-tangency — as OGC Simple Features defines simplicity per geometry kind.
///
/// The rules, each straight from the Simple Features definition:
///
/// * **Point** — always simple. A single position cannot meet itself.
/// * **MultiPoint** — simple exactly when no two members are equal in the plane.
///   Elevation does not rescue a duplicate: Clause 10.2 projects first, so two
///   members that differ only in `Z` are the same point.
/// * **LineString** — simple when it does not pass through the same point twice,
///   with one exception: a **closed** curve (first position equal to last) is
///   simple when that repetition is its only one. Interior self-intersections,
///   repeated consecutive vertices and collinear doubling-back are all anomalous.
/// * **MultiLineString** — simple when every member is simple and any two members
///   meet only at points that are boundary points of **both**. A shared interior
///   point is an anomaly even though neither member is anomalous alone.
/// * **Polygon** and **MultiPolygon** — **always simple, by definition**. This is
///   not an unimplemented case: OGC Simple Features states that surfaces are
///   always simple, because what constrains a polygon's rings is *validity*
///   (rings must not cross, holes must lie inside the exterior), a different
///   predicate from simplicity. A reader looking for the ring checks wants a
///   validity function, and this is not it.
/// * **GeometryCollection** — every member simple is **necessary and not
///   sufficient**, so the members' points and curves are gathered across the whole
///   collection (recursively) and run through exactly the checks the `MULTIPOINT`
///   and `MULTILINESTRING` arms run, plus the point-against-curve check neither
///   homogeneous arm can express: **no two positions equal**, **any two curves
///   meet only at points bounding both**, and **no point member lies anywhere on
///   a curve member**. That last clause is the same rule as the second one — an
///   intersection is permitted only where it bounds *both* operands — applied to
///   a pair whose first operand is a `Point`, whose boundary is empty; so no
///   meeting at all is permitted. A collection holding `POINT(1 1)` and
///   `LINESTRING(0 0, 2 2)` is not simple, exactly as one holding that point
///   twice is not.
///
///   Surfaces contribute nothing, because OGC Simple Features makes a surface
///   always simple (see the `Polygon` clause above) — what constrains a polygon
///   against another polygon is validity, not simplicity.
///
///   OGC leaves collection simplicity under-specified; this is the reading taken,
///   and it is the one that makes `is_simple` agree with itself on a collection
///   and on the multi-geometry with the same members.
///
/// An empty geometry of any kind is simple: it has no points, so it has no
/// anomalous ones.
#[must_use]
pub fn is_simple(geometry: &Geometry) -> bool {
    match geometry.body() {
        GeometryBody::Point(_) | GeometryBody::Polygon(_) | GeometryBody::MultiPolygon(_) => true,
        GeometryBody::MultiPoint(points) => {
            let projected: Vec<Coord> = points.iter().flatten().map(plane).collect();
            !projected
                .iter()
                .enumerate()
                .any(|(index, p)| projected[..index].iter().any(|seen| seen.same_planar(p)))
        }
        GeometryBody::LineString(coords) => {
            curve_is_simple(&coords.iter().map(plane).collect::<Vec<Coord>>())
        }
        GeometryBody::MultiLineString(lines) => {
            let curves: Vec<Vec<Coord>> = lines
                .iter()
                .filter(|line| line.len() >= 2)
                .map(|line| line.iter().map(plane).collect())
                .collect();
            curves.iter().all(|curve| curve_is_simple(curve)) && curves_meet_only_at_ends(&curves)
        }
        GeometryBody::GeometryCollection(_) => {
            // Every member simple is NECESSARY but not SUFFICIENT. Stopping there
            // returned `true` over a branch that had evaluated nothing about how
            // the members meet each other, so a collection holding two curves that
            // cross — the textbook non-simple figure — answered `true` while the
            // MULTILINESTRING with the identical members answered `false`. That is
            // the disagreement the doc above promises does not happen.
            //
            // So the members' points and curves are gathered across the whole
            // collection (recursively, since a collection may nest) and run
            // through exactly the checks the MULTIPOINT and MULTILINESTRING arms
            // run. Polygons contribute nothing because surfaces are always simple.
            //
            // Comparing the points with each other and the curves with each other
            // is still not the whole of it: a point lying ON a curve is an
            // anomalous position of the collection by the very rule the curve
            // pairs are judged by — an intersection is permitted only where it
            // bounds BOTH operands, and a Point's boundary is empty. Leaving that
            // pair out returned `true` for GEOMETRYCOLLECTION(POINT(1 1),
            // LINESTRING(0 0, 2 2)) having evaluated nothing about how the point
            // and the curve meet, which is the same unearned `true` the curve
            // pairs above were added to close.
            let mut points: Vec<Coord> = Vec::new();
            let mut curves: Vec<Vec<Coord>> = Vec::new();
            collect_simplicity_parts(geometry, &mut points, &mut curves);
            curves.iter().all(|curve| curve_is_simple(curve))
                && !points
                    .iter()
                    .enumerate()
                    .any(|(index, p)| points[..index].iter().any(|seen| seen.same_planar(p)))
                && curves_meet_only_at_ends(&curves)
                && no_point_lies_on_a_curve(&points, &curves)
        }
    }
}

/// Gather, from `geometry` and every geometry nested inside it, the planar points
/// and the planar curves that [`is_simple`] compares against one another.
///
/// Surfaces are skipped: OGC Simple Features makes them always simple, so they
/// constrain nothing here. A curve of fewer than two positions is skipped for the
/// same reason the `MultiLineString` arm skips it — it has no segment to meet
/// anything with.
fn collect_simplicity_parts(
    geometry: &Geometry,
    points: &mut Vec<Coord>,
    curves: &mut Vec<Vec<Coord>>,
) {
    match geometry.body() {
        GeometryBody::Point(point) => points.extend(point.iter().map(plane)),
        GeometryBody::MultiPoint(members) => points.extend(members.iter().flatten().map(plane)),
        GeometryBody::LineString(coords) => {
            if coords.len() >= 2 {
                curves.push(coords.iter().map(plane).collect());
            }
        }
        GeometryBody::MultiLineString(lines) => {
            for line in lines {
                if line.len() >= 2 {
                    curves.push(line.iter().map(plane).collect());
                }
            }
        }
        GeometryBody::Polygon(_) | GeometryBody::MultiPolygon(_) => {}
        GeometryBody::GeometryCollection(members) => {
            for member in members {
                collect_simplicity_parts(member, points, curves);
            }
        }
    }
}

/// Whether one vertex run is a simple curve.
fn curve_is_simple(path: &[Coord]) -> bool {
    if path.len() < 2 {
        return true;
    }
    let mut segments = Vec::with_capacity(path.len() - 1);
    push_segments(path, &mut segments);
    // A zero-length segment is a vertex written twice in a row: the curve
    // occupies the same point at two distinct parameters, which is the
    // definition of an anomalous point.
    if segments
        .iter()
        .any(|segment| segment[0].same_planar(&segment[1]))
    {
        return false;
    }
    let closed = path[0].same_planar(&path[path.len() - 1]);
    let last = segments.len() - 1;
    for (i, s) in segments.iter().enumerate() {
        for (j, t) in segments.iter().enumerate().skip(i + 1) {
            // Two consecutive segments are entitled to their shared vertex, and
            // a closed curve's first and last segments are entitled to the
            // closure. Nothing else is.
            let mut allowed: Vec<&Coord> = Vec::with_capacity(2);
            if j == i + 1 {
                allowed.push(&path[j]);
            }
            if closed && i == 0 && j == last {
                allowed.push(&path[0]);
            }
            match intersect(&s[0], &s[1], &t[0], &t[1]) {
                SegmentIntersection::None => {}
                SegmentIntersection::Point(shared) => {
                    if !allowed.iter().any(|point| point.same_planar(&shared)) {
                        return false;
                    }
                }
                SegmentIntersection::Collinear { .. } => return false,
            }
        }
    }
    true
}

/// The mod-2 boundary of one vertex run: its two endpoints when it is open, and
/// nothing when it is closed (the shared endpoint is written twice and cancels).
fn chain_boundary(path: &[Coord]) -> Vec<Coord> {
    if path.len() < 2 || path[0].same_planar(&path[path.len() - 1]) {
        return Vec::new();
    }
    vec![path[0].clone(), path[path.len() - 1].clone()]
}

/// Whether no point member of a collection lies anywhere on a curve member.
///
/// The rule [`curves_meet_only_at_ends`] applies — a meeting is permitted only
/// where it bounds *both* operands — with the first operand a `Point`. A Point's
/// boundary is empty (Simple Features, Clause 6.1.6), so no position bounds it and
/// therefore no meeting at all is permitted: a point on a curve's endpoint is as
/// anomalous as one in its interior.
fn no_point_lies_on_a_curve(points: &[Coord], curves: &[Vec<Coord>]) -> bool {
    !points.iter().any(|point| {
        curves.iter().any(|curve| {
            curve
                .windows(2)
                .any(|edge| on_segment(point, &edge[0], &edge[1]))
        })
    })
}

/// Whether every pair of curves meets only at points that bound both of them.
fn curves_meet_only_at_ends(curves: &[Vec<Coord>]) -> bool {
    let boundaries: Vec<Vec<Coord>> = curves.iter().map(|c| chain_boundary(c)).collect();
    let mut segments: Vec<Vec<Segment>> = Vec::with_capacity(curves.len());
    for curve in curves {
        let mut run = Vec::new();
        push_segments(curve, &mut run);
        segments.push(run);
    }
    for (i, first) in segments.iter().enumerate() {
        for (j, second) in segments.iter().enumerate().skip(i + 1) {
            for s in first {
                for t in second {
                    match intersect(&s[0], &s[1], &t[0], &t[1]) {
                        SegmentIntersection::None => {}
                        SegmentIntersection::Collinear { .. } => return false,
                        SegmentIntersection::Point(shared) => {
                            let bounds_both =
                                boundaries[i].iter().any(|point| point.same_planar(&shared))
                                    && boundaries[j].iter().any(|point| point.same_planar(&shared));
                            if !bounds_both {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        LENGTH_SCALE_DIGITS, Parts, area, bounds, distance, geometry_n, is_simple, length, max_x,
        max_y, max_z, min_x, min_y, min_z, num_geometries, perimeter,
    };
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind};

    // ---- fixtures --------------------------------------------------------

    fn r(value: i64) -> Rat {
        Rat::from_i64(value)
    }

    fn ratio(numerator: i64, denominator: i64) -> Rat {
        Rat::new(Int::from_i64(numerator), Int::from_i64(denominator))
            .expect("a non-zero denominator")
    }

    fn seq(points: &[(i64, i64)]) -> CoordSeq {
        points.iter().map(|&(x, y)| Coord::xy(r(x), r(y))).collect()
    }

    fn point(x: i64, y: i64) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(r(x), r(y)))),
        )
        .expect("a well-formed point")
    }

    fn multipoint(points: &[(i64, i64)]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(
                points
                    .iter()
                    .map(|&(x, y)| Some(Coord::xy(r(x), r(y))))
                    .collect(),
            ),
        )
        .expect("a well-formed multipoint")
    }

    fn line(points: &[(i64, i64)]) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::LineString(seq(points)))
            .expect("a well-formed line")
    }

    fn multiline(lines: &[&[(i64, i64)]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(lines.iter().map(|l| seq(l)).collect()),
        )
        .expect("a well-formed multiline")
    }

    fn polygon(rings: &[&[(i64, i64)]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::Polygon(rings.iter().map(|ring| seq(ring)).collect()),
        )
        .expect("a well-formed polygon")
    }

    fn multipolygon(polygons: &[&[&[(i64, i64)]]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPolygon(
                polygons
                    .iter()
                    .map(|rings| rings.iter().map(|ring| seq(ring)).collect())
                    .collect(),
            ),
        )
        .expect("a well-formed multipolygon")
    }

    fn collection(members: Vec<Geometry>) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(members))
            .expect("uniform dimension")
    }

    const UNIT_SQUARE: &[(i64, i64)] = &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)];

    /// The scaled integer a measure at the internal precision denotes.
    fn scaled(value: &Rat) -> Rat {
        value.mul(&Rat::from_int(Int::pow10(LENGTH_SCALE_DIGITS)))
    }

    // ---- bounds ----------------------------------------------------------

    #[test]
    fn the_bounding_box_is_the_exact_extreme_ordinates_and_is_none_for_an_empty_geometry() {
        let g = polygon(&[&[(2, 3), (7, 3), (7, 11), (2, 11), (2, 3)]]);
        assert_eq!(
            bounds(&g),
            Some((r(2), r(3), r(7), r(11))),
            "the box is (min_x, min_y, max_x, max_y)"
        );
        assert_eq!(min_x(&g), Some(r(2)), "min_x reads the box");
        assert_eq!(min_y(&g), Some(r(3)), "min_y reads the box");
        assert_eq!(max_x(&g), Some(r(7)), "max_x reads the box");
        assert_eq!(max_y(&g), Some(r(11)), "max_y reads the box");
        let empty = Geometry::empty(CoordDim::Xy, GeometryKind::Polygon);
        assert_eq!(bounds(&empty), None, "an empty geometry has no box");
        assert_eq!(min_x(&empty), None, "and so no minimum x");
    }

    /// Clause 10.2 projects onto `Z=0` for every *calculation*, but `minZ`/`maxZ`
    /// exist to report the ordinate — so they, and only they, read it.
    #[test]
    fn the_z_extremes_are_reported_when_present_and_absent_otherwise() {
        let with_z = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::LineString(
                [
                    Coord::new(r(0), r(0), Some(r(5)), None),
                    Coord::new(r(1), r(1), Some(r(-2)), None),
                    Coord::new(r(2), r(2), Some(r(9)), None),
                ]
                .into_iter()
                .collect(),
            ),
        )
        .expect("a well-formed 3D line");
        assert_eq!(min_z(&with_z), Some(r(-2)), "the smallest elevation");
        assert_eq!(max_z(&with_z), Some(r(9)), "the largest elevation");
        assert_eq!(
            min_z(&line(&[(0, 0), (1, 1)])),
            None,
            "a 2D geometry carries no elevation to report"
        );
        assert_eq!(
            min_z(&Geometry::empty(CoordDim::Xyz, GeometryKind::Point)),
            None,
            "an empty 3D geometry has no coordinate to read an elevation from"
        );
    }

    /// The planar rule with teeth: a 3D polygon has the area of its shadow, and
    /// the elevations do not enter the number at all.
    #[test]
    fn a_three_dimensional_geometry_is_measured_on_its_projection_onto_z_zero() {
        let tilted = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::Polygon(vec![
                [
                    Coord::new(r(0), r(0), Some(r(0)), None),
                    Coord::new(r(1), r(0), Some(r(100)), None),
                    Coord::new(r(1), r(1), Some(r(-40)), None),
                    Coord::new(r(0), r(1), Some(r(7)), None),
                    Coord::new(r(0), r(0), Some(r(0)), None),
                ]
                .into_iter()
                .collect(),
            ]),
        )
        .expect("a closed 3D ring");
        assert_eq!(
            area(&tilted),
            r(1),
            "the area is the unit square's, because Z is projected away"
        );
        assert_eq!(
            perimeter(&tilted),
            r(4),
            "and so is the perimeter, for the same reason"
        );
    }

    // ---- area ------------------------------------------------------------

    #[test]
    fn the_area_goldens_are_exact_rationals_not_approximations() {
        assert_eq!(area(&polygon(&[UNIT_SQUARE])), r(1), "the unit square is 1");
        assert_eq!(
            area(&polygon(&[&[(0, 0), (1, 0), (0, 1), (0, 0)]])),
            ratio(1, 2),
            "the unit right triangle is exactly one half, not 0.4999..."
        );
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
        ]);
        assert_eq!(
            area(&with_hole),
            r(12),
            "a 4x4 square minus a 2x2 hole is 16 - 4"
        );
        let two_squares =
            multipolygon(&[&[UNIT_SQUARE], &[&[(5, 5), (6, 5), (6, 6), (5, 6), (5, 5)]]]);
        assert_eq!(
            area(&two_squares),
            r(2),
            "a multipolygon sums its components"
        );
        assert_eq!(
            area(&polygon(&[&[(0, 0), (1, 0), (1, 3), (0, 3), (0, 0)]])),
            r(3),
            "a 1x3 rectangle is 3"
        );
    }

    /// Winding is a spelling choice, not a measurement: the same ring written the
    /// other way round has the same positive area.
    #[test]
    fn area_is_orientation_independent() {
        let counter_clockwise = polygon(&[UNIT_SQUARE]);
        let clockwise = polygon(&[&[(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]]);
        assert_eq!(
            area(&counter_clockwise),
            area(&clockwise),
            "the two windings must agree"
        );
        assert_eq!(area(&clockwise), r(1), "and both must be positive");
    }

    /// `geof:area` "must return zero for all geometry types other than Polygon",
    /// and the neighbouring polygon case must still be non-zero — otherwise the
    /// assertion above would pass on a function that always answers zero.
    #[test]
    fn area_is_zero_for_every_non_areal_geometry_and_non_zero_for_a_polygon() {
        for g in [
            point(3, 4),
            multipoint(&[(0, 0), (5, 5)]),
            line(&[(0, 0), (10, 0), (10, 10)]),
            multiline(&[&[(0, 0), (1, 1)], &[(2, 2), (3, 3)]]),
            Geometry::empty(CoordDim::Xy, GeometryKind::Polygon),
        ] {
            assert_eq!(area(&g), Rat::zero(), "a {:?} has no area at all", g.kind());
        }
        assert_eq!(
            area(&polygon(&[UNIT_SQUARE])),
            r(1),
            "the neighbouring areal case is emphatically not zero"
        );
    }

    // ---- length and perimeter -------------------------------------------

    /// A 3-4-5 hypotenuse is a perfect square under the radical, so the scaled
    /// integer square root is exact and the truncation costs nothing.
    #[test]
    fn a_perfect_square_length_is_exact_at_the_internal_scale() {
        assert_eq!(
            length(&line(&[(0, 0), (3, 4)])),
            r(5),
            "sqrt(9 + 16) is exactly 5"
        );
        assert_eq!(
            length(&line(UNIT_SQUARE)),
            r(4),
            "the closed unit-square path is four unit segments"
        );
    }

    /// The unit diagonal is irrational, so this pins the exact scaled integer:
    /// the first eighteen fraction digits of sqrt(2), truncated, never rounded up.
    #[test]
    fn an_irrational_length_is_the_truncated_scaled_integer_square_root() {
        let diagonal = length(&line(&[(0, 0), (1, 1)]));
        let expected_scaled = Int::from_i64(1_414_213_562_373_095_048);
        assert_eq!(
            scaled(&diagonal),
            Rat::from_int(expected_scaled.clone()),
            "sqrt(2) = 1.414213562373095048801688..., truncated at 18 fraction digits"
        );
        assert_eq!(
            diagonal,
            Rat::new(expected_scaled, Int::pow10(LENGTH_SCALE_DIGITS)).expect("non-zero"),
            "and the returned rational is exactly that integer over 10^18"
        );
        assert!(
            diagonal.mul(&diagonal) < r(2),
            "a truncation never overstates: the square of the answer stays below 2"
        );
    }

    /// The property `f64` would break. Rounded addition is not associative, so a
    /// float implementation gives a different total for a reversed traversal;
    /// integer addition is associative, so this must hold exactly.
    #[test]
    fn length_does_not_depend_on_the_order_the_segments_are_summed_in() {
        let forwards = line(&[(0, 0), (1, 1), (4, 5), (7, 2), (11, 13), (0, 3)]);
        let backwards = line(&[(0, 3), (11, 13), (7, 2), (4, 5), (1, 1), (0, 0)]);
        assert_eq!(
            length(&forwards),
            length(&backwards),
            "the reversed polyline must measure identically, digit for digit"
        );
        assert!(
            length(&forwards) > r(0),
            "and the length must be a real measurement, not a shared zero"
        );
    }

    #[test]
    fn length_counts_curve_components_only() {
        assert_eq!(
            length(&polygon(&[UNIT_SQUARE])),
            Rat::zero(),
            "a polygon has no length; its ring measurement is its perimeter"
        );
        assert_eq!(length(&point(1, 1)), Rat::zero(), "a point has no length");
        assert_eq!(
            length(&multiline(&[&[(0, 0), (3, 4)], &[(0, 0), (0, 7)]])),
            r(12),
            "5 + 7, summed as scaled integers"
        );
        assert_eq!(
            length(&collection(vec![point(9, 9), line(&[(0, 0), (3, 4)])])),
            r(5),
            "a collection contributes its curve members"
        );
    }

    #[test]
    fn perimeter_measures_rings_and_falls_back_to_length_without_a_polygon() {
        assert_eq!(
            perimeter(&polygon(&[UNIT_SQUARE])),
            r(4),
            "the unit square's boundary is 4"
        );
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
        ]);
        assert_eq!(
            perimeter(&with_hole),
            r(24),
            "16 around the exterior plus 8 around the hole"
        );
        let curve = line(&[(0, 0), (3, 4)]);
        assert_eq!(
            perimeter(&curve),
            length(&curve),
            "for a non-areal geometry the perimeter is the length"
        );
        assert_eq!(
            perimeter(&point(1, 2)),
            Rat::zero(),
            "and a point's is zero, exactly as its length is"
        );
    }

    // ---- distance --------------------------------------------------------

    #[test]
    fn the_distance_goldens_are_exact() {
        assert_eq!(
            distance(&point(0, 0), &point(3, 4)),
            Some(r(5)),
            "two points, a perfect square apart"
        );
        assert_eq!(
            distance(&point(0, 1), &line(&[(-5, 0), (5, 0)])),
            Some(r(1)),
            "the perpendicular foot falls inside the segment"
        );
        assert_eq!(
            distance(&point(0, 4), &line(&[(3, 0), (9, 0)])),
            Some(r(5)),
            "the foot falls outside, so the nearest endpoint (3,0) wins"
        );
        assert_eq!(
            distance(
                &polygon(&[UNIT_SQUARE]),
                &polygon(&[&[(3, 0), (4, 0), (4, 1), (3, 1), (3, 0)]])
            ),
            Some(r(2)),
            "two disjoint squares, gap 2"
        );
        assert_eq!(
            distance(&point(0, 0), &point(1, 1)),
            Some(
                Rat::new(
                    Int::from_i64(1_414_213_562_373_095_048),
                    Int::pow10(LENGTH_SCALE_DIGITS)
                )
                .expect("non-zero")
            ),
            "an irrational distance is the same truncated scaled integer a length is"
        );
    }

    /// Zero is decided by an exact intersection test, never by comparing a
    /// computed value against a tolerance.
    #[test]
    fn intersecting_geometries_are_at_distance_exactly_zero() {
        assert_eq!(
            distance(&line(&[(0, 0), (2, 2)]), &line(&[(0, 2), (2, 0)])),
            Some(Rat::zero()),
            "two crossing segments"
        );
        assert_eq!(
            distance(
                &polygon(&[UNIT_SQUARE]),
                &polygon(&[&[(1, 0), (2, 0), (2, 1), (1, 1), (1, 0)]])
            ),
            Some(Rat::zero()),
            "two polygons sharing an edge"
        );
        let inside = Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(ratio(1, 2), ratio(1, 2)))),
        )
        .expect("a well-formed point");
        assert_eq!(
            distance(&inside, &polygon(&[UNIT_SQUARE])),
            Some(Rat::zero()),
            "a point strictly inside a polygon is at distance zero from it"
        );
        assert_eq!(
            distance(&polygon(&[UNIT_SQUARE]), &inside),
            Some(Rat::zero()),
            "and the same the other way round"
        );
        let with_hole = polygon(&[
            &[(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)],
            &[(2, 2), (8, 2), (8, 8), (2, 8), (2, 2)],
        ]);
        assert_eq!(
            distance(&point(5, 5), &with_hole),
            Some(r(3)),
            "a point in the hole is outside the polygon, three units from the hole's wall"
        );
        // The neighbouring case, so the zeros above are not the answers of a
        // function that always says zero.
        assert_eq!(
            distance(&point(0, 0), &point(0, 7)),
            Some(r(7)),
            "disjoint geometries still measure"
        );
    }

    #[test]
    fn distance_is_none_exactly_when_an_operand_is_empty() {
        let empty = Geometry::empty(CoordDim::Xy, GeometryKind::LineString);
        assert_eq!(distance(&empty, &point(0, 0)), None, "empty on the left");
        assert_eq!(distance(&point(0, 0), &empty), None, "empty on the right");
        assert_eq!(distance(&empty, &empty), None, "empty on both sides");
        assert!(
            distance(&point(0, 0), &point(1, 0)).is_some(),
            "and two non-empty operands always measure"
        );
    }

    #[test]
    fn distance_is_symmetric_and_planar() {
        let a = line(&[(0, 0), (0, 10)]);
        let b = point(4, 3);
        assert_eq!(distance(&a, &b), distance(&b, &a), "distance is symmetric");
        assert_eq!(distance(&a, &b), Some(r(4)), "and is the perpendicular");
    }

    // ---- members ---------------------------------------------------------

    #[test]
    fn the_member_count_is_one_for_a_simple_geometry_and_the_length_otherwise() {
        assert_eq!(num_geometries(&point(1, 1)), 1, "a point is one geometry");
        assert_eq!(num_geometries(&line(&[(0, 0), (1, 1)])), 1, "so is a curve");
        assert_eq!(
            num_geometries(&polygon(&[UNIT_SQUARE])),
            1,
            "and so is a polygon, holes and all"
        );
        assert_eq!(
            num_geometries(&multipoint(&[(0, 0), (1, 1), (2, 2)])),
            3,
            "a multipoint counts its members"
        );
        assert_eq!(
            num_geometries(&collection(vec![point(0, 0), polygon(&[UNIT_SQUARE])])),
            2,
            "a collection counts its members"
        );
        assert_eq!(
            num_geometries(&Geometry::empty(CoordDim::Xy, GeometryKind::MultiPoint)),
            0,
            "an empty multi-geometry has no members"
        );
    }

    #[test]
    fn the_nth_member_is_one_based_and_out_of_range_is_none() {
        let mp = multipoint(&[(0, 0), (7, 8)]);
        assert_eq!(
            geometry_n(&mp, 2),
            Some(point(7, 8)),
            "the second member is a Point, not a MultiPoint"
        );
        assert_eq!(geometry_n(&mp, 0), None, "there is no zeroth member");
        assert_eq!(geometry_n(&mp, 3), None, "and none past the end");
        let single = line(&[(0, 0), (1, 1)]);
        assert_eq!(
            geometry_n(&single, 1),
            Some(single.clone()),
            "a non-collection is its own first member"
        );
        assert_eq!(geometry_n(&single, 2), None, "and has no second");
        assert_eq!(
            geometry_n(&multipolygon(&[&[UNIT_SQUARE]]), 1).map(|g| g.kind()),
            Some(GeometryKind::Polygon),
            "a multipolygon member comes back as a Polygon"
        );
    }

    // ---- simplicity ------------------------------------------------------

    #[test]
    fn a_simple_curve_is_simple_and_a_self_crossing_one_is_not() {
        assert!(
            is_simple(&line(&[(0, 0), (1, 0), (1, 1)])),
            "an injective open path is simple"
        );
        assert!(
            !is_simple(&line(&[(0, 0), (2, 2), (2, 0), (0, 2)])),
            "an X-shaped path crosses itself at (1,1)"
        );
    }

    #[test]
    fn a_closed_ring_is_simple_but_a_ring_that_also_crosses_itself_is_not() {
        assert!(
            is_simple(&line(UNIT_SQUARE)),
            "a closed ring's single repetition is the closure, which is allowed"
        );
        assert!(
            !is_simple(&line(&[(0, 0), (2, 2), (2, 0), (0, 2), (0, 0)])),
            "a closed bow-tie repeats its start AND crosses itself"
        );
    }

    #[test]
    fn a_repeated_vertex_makes_a_curve_non_simple_but_collinear_vertices_do_not() {
        assert!(
            !is_simple(&line(&[(0, 0), (1, 1), (1, 1), (2, 2)])),
            "a vertex written twice in a row is an anomalous point"
        );
        assert!(
            !is_simple(&line(&[(0, 0), (2, 0), (2, 2), (0, 0), (0, -2)])),
            "revisiting the start part-way through is an anomalous point"
        );
        assert!(
            !is_simple(&line(&[(0, 0), (2, 2), (1, 1)])),
            "doubling back along the same line re-traces points"
        );
        // The neighbouring VALID cases: collinearity and mere direction changes
        // are not anomalies, and refusing them would be over-refusal.
        assert!(
            is_simple(&line(&[(0, 0), (1, 1), (2, 2)])),
            "three collinear vertices are still an injective path"
        );
        assert!(
            is_simple(&line(&[(0, 0), (2, 0), (2, 2), (0, 2)])),
            "an open three-segment path is simple"
        );
    }

    #[test]
    fn a_multipoint_with_a_duplicate_is_not_simple_but_distinct_members_are() {
        assert!(
            !is_simple(&multipoint(&[(1, 1), (2, 2), (1, 1)])),
            "two members at the same place are an anomalous point"
        );
        assert!(
            is_simple(&multipoint(&[(1, 1), (2, 2), (3, 3)])),
            "distinct members are simple"
        );
        let planar_duplicate = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::MultiPoint(vec![
                Some(Coord::new(r(1), r(1), Some(r(0)), None)),
                Some(Coord::new(r(1), r(1), Some(r(50)), None)),
            ]),
        )
        .expect("a well-formed 3D multipoint");
        assert!(
            !is_simple(&planar_duplicate),
            "Clause 10.2 projects first, so a different elevation does not rescue a duplicate"
        );
    }

    #[test]
    fn multiline_members_may_touch_only_where_both_have_a_boundary() {
        assert!(
            is_simple(&multiline(&[&[(0, 0), (1, 0)], &[(1, 0), (2, 0)]])),
            "two curves meeting end to end meet at a boundary point of both"
        );
        assert!(
            !is_simple(&multiline(&[&[(0, 0), (2, 0)], &[(1, -1), (1, 1)]])),
            "a crossing at an interior point of both is an anomaly"
        );
        assert!(
            !is_simple(&multiline(&[&[(0, 0), (2, 0)], &[(1, 0), (1, 1)]])),
            "a T-junction touches the interior of the first member"
        );
        assert!(
            !is_simple(&multiline(&[&[(0, 0), (2, 2)], &[(1, 1), (3, 3)]])),
            "a collinear overlap is an anomaly of positive length"
        );
        // The neighbouring VALID case.
        assert!(
            is_simple(&multiline(&[&[(0, 0), (1, 0)], &[(5, 5), (6, 6)]])),
            "disjoint members are simple"
        );
    }

    /// OGC defines surfaces as always simple: what constrains a polygon's rings
    /// is validity, a different predicate. This is the specification's answer,
    /// not an unimplemented case.
    #[test]
    fn every_polygon_is_simple_by_the_specifications_definition() {
        assert!(is_simple(&polygon(&[UNIT_SQUARE])), "a well-formed square");
        assert!(
            is_simple(&polygon(&[&[(0, 0), (2, 2), (2, 0), (0, 2), (0, 0)]])),
            "and a self-intersecting ring too: that is invalidity, not non-simplicity"
        );
        assert!(
            is_simple(&multipolygon(&[&[UNIT_SQUARE], &[UNIT_SQUARE]])),
            "overlapping components are likewise invalid rather than non-simple"
        );
    }

    #[test]
    fn a_collection_is_simple_exactly_when_every_member_is() {
        assert!(
            is_simple(&collection(vec![point(0, 0), line(&[(5, 5), (6, 6)])])),
            "simple members make a simple collection"
        );
        assert!(
            !is_simple(&collection(vec![
                point(0, 0),
                line(&[(0, 0), (2, 2), (2, 0), (0, 2)]),
            ])),
            "one non-simple member is enough"
        );
    }

    /// A point member lying on a curve member is an anomalous position of the
    /// collection, and each of the three ways it can lie there is graded against
    /// the neighbouring position that must stay simple.
    ///
    /// Comparing the points with each other and the curves with each other left
    /// this pair unevaluated, so the answer was `true` — a `true` earned by
    /// nothing, and indistinguishable from a collection whose members genuinely
    /// do not meet. There is no homogeneous multi-geometry spelling for a
    /// point-and-curve collection, so the agreement test above cannot reach it.
    #[test]
    fn a_point_member_lying_on_a_curve_member_makes_the_collection_non_simple() {
        let curve = || line(&[(0, 0), (4, 0)]);

        // 1. On the curve's INTERIOR, at a vertex-free position.
        assert!(
            !is_simple(&collection(vec![point(2, 0), curve()])),
            "a point in the curve's interior is an anomalous position"
        );
        // 2. On an interior VERTEX of the curve.
        assert!(
            !is_simple(&collection(vec![
                point(2, 0),
                line(&[(0, 0), (2, 0), (4, 0)])
            ])),
            "a point on an interior vertex is anomalous for the same reason"
        );
        // 3. On the curve's BOUNDARY. A Point's own boundary is empty, so the
        //    meeting bounds only one of the two operands and is not permitted —
        //    the same rule two curves are judged by.
        assert!(
            !is_simple(&collection(vec![point(0, 0), curve()])),
            "a point on the curve's endpoint bounds the curve but not the point"
        );

        // The NEIGHBOURING VALID cases, which must all still be simple: the same
        // curve with the point moved off it — beside it, beyond its end on the
        // same line, and in a nested collection.
        assert!(
            is_simple(&collection(vec![point(2, 1), curve()])),
            "a point beside the curve meets nothing"
        );
        assert!(
            is_simple(&collection(vec![point(5, 0), curve()])),
            "collinear with the curve but past its end is still not ON it"
        );
        assert!(
            is_simple(&collection(vec![
                point(2, 1),
                collection(vec![curve(), line(&[(4, 0), (6, 2)])]),
            ])),
            "the rule reaches through nesting without refusing a legitimate figure"
        );
        // And it reaches through nesting in the failing direction too, or the
        // recursion would be a silent hole rather than a rule.
        assert!(
            !is_simple(&collection(vec![
                point(2, 0),
                collection(vec![curve(), line(&[(4, 0), (6, 2)])]),
            ])),
            "a point on a curve nested one level down is still anomalous"
        );
    }

    /// A collection answers the same as the multi-geometry holding the same
    /// members — which is the reading [`is_simple`]'s docs promise.
    ///
    /// Every member being simple is necessary but not sufficient: two curves that
    /// individually do not self-cross can still cross *each other*, and a
    /// collection that only checked its members one at a time reported `true` for
    /// exactly that figure while `MULTILINESTRING` with the identical members
    /// reported `false`. Each case below pairs the two spellings and requires
    /// them to agree, so the two arms cannot drift apart again.
    #[test]
    fn a_collection_agrees_with_the_multi_geometry_holding_the_same_members() {
        // Curves that cross in their interiors.
        let crossing: &[&[(i64, i64)]] = &[&[(0, 0), (2, 2)], &[(0, 2), (2, 0)]];
        // Curves that overlap along a shared stretch.
        let collinear: &[&[(i64, i64)]] = &[&[(0, 0), (4, 0)], &[(1, 0), (3, 0)]];
        // A T-junction: one curve's end lands in the other's interior.
        let tee: &[&[(i64, i64)]] = &[&[(0, 0), (4, 0)], &[(2, -1), (2, 1)]];
        // The VALID neighbour: two curves that meet only at their endpoints.
        let end_to_end: &[&[(i64, i64)]] = &[&[(0, 0), (1, 1)], &[(1, 1), (2, 0)]];
        for lines in [crossing, collinear, tee, end_to_end] {
            let members: Vec<Geometry> = lines.iter().map(|line| self::line(line)).collect();
            let as_multi = is_simple(&multiline(lines));
            let as_collection = is_simple(&collection(members));
            assert_eq!(
                as_collection, as_multi,
                "the collection and the multi-geometry must agree for {lines:?}"
            );
        }
        // And the agreement is not vacuous: the four cases are not all the same
        // answer, so an `is_simple` that returned a constant would fail here.
        assert!(
            !is_simple(&multiline(crossing)),
            "crossing curves are not simple"
        );
        assert!(
            is_simple(&multiline(end_to_end)),
            "curves meeting only at endpoints ARE simple"
        );
        // Duplicate points behave the same way across the two spellings.
        assert_eq!(
            is_simple(&collection(vec![point(1, 1), point(1, 1)])),
            is_simple(&multipoint(&[(1, 1), (1, 1)])),
            "a repeated point is non-simple in both spellings"
        );
        assert!(
            is_simple(&collection(vec![point(1, 1), point(2, 2)])),
            "distinct points are simple — the neighbouring valid case"
        );
    }

    #[test]
    fn every_empty_geometry_is_simple() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            assert!(
                is_simple(&Geometry::empty(CoordDim::Xy, kind)),
                "{kind:?} EMPTY has no points, so it has no anomalous ones"
            );
        }
    }

    // ---- decomposition ---------------------------------------------------

    #[test]
    fn decomposition_drops_empty_pieces_and_flattens_collections() {
        let nested = collection(vec![
            Geometry::empty(CoordDim::Xy, GeometryKind::LineString),
            collection(vec![point(1, 1), polygon(&[UNIT_SQUARE])]),
            line(&[(0, 0), (1, 0)]),
        ]);
        let parts = Parts::of(&nested);
        assert_eq!(parts.points.len(), 1, "one point member survives");
        assert_eq!(parts.lines.len(), 1, "the empty curve contributes nothing");
        assert_eq!(parts.polygons.len(), 1, "the nested polygon is reached");
        assert_eq!(
            parts.points[0],
            Coord::xy(r(1), r(1)),
            "and it is the one that was written, projected"
        );
        assert!(
            Parts::of(&Geometry::empty(CoordDim::Xy, GeometryKind::MultiPolygon))
                .polygons
                .is_empty(),
            "an empty geometry decomposes into nothing"
        );
        assert_eq!(
            parts.segments().len(),
            1 + 4,
            "one curve segment and the square's four ring segments"
        );
    }

    // ---- over-refusal control -------------------------------------------

    /// Every total function here must answer for every well-formed geometry,
    /// degenerate and empty ones included. None of them has a refusal channel,
    /// and this is the assertion that keeps it that way: a future "guard" that
    /// started returning a sentinel for a degenerate input would have to break
    /// one of these.
    #[test]
    fn the_total_measures_answer_for_every_well_formed_geometry_including_degenerate_ones() {
        let mut corpus = vec![
            point(0, 0),
            multipoint(&[(1, 1), (1, 1)]),
            line(&[(0, 0), (0, 0)]),
            line(&[(0, 0), (1, 1), (2, 2)]),
            line(UNIT_SQUARE),
            polygon(&[UNIT_SQUARE]),
            polygon(&[&[(0, 0), (1, 0), (2, 0), (0, 0)]]),
            polygon(&[
                &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
                &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
            ]),
            multipolygon(&[&[UNIT_SQUARE]]),
            multiline(&[&[(0, 0), (1, 1)]]),
            collection(vec![point(0, 0), line(&[(1, 1), (2, 2)])]),
            Geometry::new(CoordDim::Xy, GeometryBody::MultiPoint(vec![None]))
                .expect("MULTIPOINT(EMPTY)"),
        ];
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            corpus.push(Geometry::empty(CoordDim::Xy, kind));
        }

        for g in &corpus {
            assert!(
                area(g) >= Rat::zero(),
                "area answers and never goes negative: {:?}",
                g.kind()
            );
            assert!(
                length(g) >= Rat::zero(),
                "length answers and never goes negative: {:?}",
                g.kind()
            );
            assert!(
                perimeter(g) >= Rat::zero(),
                "perimeter answers and never goes negative: {:?}",
                g.kind()
            );
            let _simple = is_simple(g);
            assert!(
                num_geometries(g) == 0 || geometry_n(g, 1).is_some(),
                "a geometry with members must yield its first: {:?}",
                g.kind()
            );
            assert_eq!(
                bounds(g).is_some(),
                g.coord_count() > 0,
                "a box exists exactly when a coordinate does: {:?}",
                g.kind()
            );
        }

        // And the binary one, over every ordered pair.
        for a in &corpus {
            for b in &corpus {
                assert_eq!(
                    distance(a, b).is_some(),
                    !a.is_empty() && !b.is_empty(),
                    "distance answers for every non-empty pair and refuses only for empties: \
                     {:?} vs {:?}",
                    a.kind(),
                    b.kind()
                );
            }
        }
    }
}
