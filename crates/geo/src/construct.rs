// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The constructors that derive a new geometry from an old one: boundary,
//! envelope, convex hull and centroid.
//!
//! # Everything here is planar, and every output is `XY`
//!
//! OGC GeoSPARQL 1.1, Clause 10.2, verbatim:
//!
//! > Geometric functions working with Geometries that have Z values will ignore
//! > Z values in calculations and first project geometry onto the Z=0 level. …
//! > Like Z values in coordinates, M values are to be ignored.
//!
//! So every function here reads only `x` and `y`, and every geometry it builds
//! carries [`CoordDim::Xy`]. That is not a simplification, it is what "first
//! project geometry onto the Z=0 level" means: the [`boundary`] of a `POLYGON Z`
//! is a **two-dimensional** `MULTILINESTRING`, because the object whose boundary
//! was taken is the *projected* polygon, and the projected polygon has no
//! elevation to hand on. A constructor that copied the input's `Z` through would
//! be claiming to have worked in three dimensions, which it did not.
//!
//! # The empty answers, spelled out
//!
//! Three of these constructors can produce nothing, and each spells it the way
//! the *kind* of the answer requires rather than by echoing the input's kind:
//!
//! * [`boundary`] returns the container its answer belongs in — an empty
//!   `MULTIPOINT` for a closed curve (a curve's boundary is a point set), an
//!   empty `GEOMETRYCOLLECTION` for a point (a point's boundary is empty of any
//!   type at all).
//! * [`envelope`] and [`convex_hull`] return an empty `GEOMETRYCOLLECTION` for
//!   an empty input, which is the canonical empty geometry.
//!
//! # Exactness
//!
//! [`boundary`], [`envelope`] and [`convex_hull`] are exact with nothing to
//! round: they select and reorder ordinates that were already in the input, and
//! the hull's only decision is [`crate::topology::orientation`], the
//! sign of an exact rational cross product. [`centroid`] is exact for areal and
//! point geometries and carries the fixed-scale truncation of
//! [`crate::measure::LENGTH_SCALE_DIGITS`] for the length-weighted linear case,
//! because segment lengths are irrational. See each function for the specifics.

use crate::exact::{Int, Rat};
use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind};
use crate::measure::{Parts, bounds, from_scaled, scaled_segment_length, signed_ring_area};
use crate::topology::{curve_boundary_points, orientation};

/// The canonical empty answer: a geometry that denotes the empty set and claims
/// no kind it does not have.
fn nothing() -> Geometry {
    Geometry::empty(CoordDim::Xy, GeometryKind::GeometryCollection)
}

/// A planar geometry, built from a body whose coordinates are all `XY`.
fn planar(body: GeometryBody) -> Geometry {
    Geometry::new(CoordDim::Xy, body)
        .expect("a body assembled from projected coordinates is XY and structurally well-formed")
}

/// A vertex run as a coordinate sequence.
fn run(points: &[Coord]) -> CoordSeq {
    CoordSeq::from(points)
}

// ---------------------------------------------------------------------------
// Boundary
// ---------------------------------------------------------------------------

/// The closure of the boundary of `geometry`, as a geometry.
///
/// Per OGC Simple Features, by kind:
///
/// * **Point**, **MultiPoint** — the empty set. A point has no boundary, so the
///   answer is an empty `GEOMETRYCOLLECTION`: it is not a degenerate point set,
///   it is nothing.
/// * **LineString**, **MultiLineString** — the endpoints under the **mod-2
///   rule**, from [`crate::topology::curve_boundary_points`]: a position
///   is in the boundary when it is an endpoint of an odd number of the member
///   curves. A closed curve writes its one endpoint twice, so it cancels and a
///   ring has an empty boundary; two curves joined end to end contribute their
///   shared join twice, so it cancels too and the boundary is the two far ends.
///   The answer is a `MULTIPOINT`, possibly empty, whose members are in
///   lexicographic `(x, y)` order — a total order on positions, so the output is
///   a pure function of the geometry rather than of its written order.
/// * **Polygon**, **MultiPolygon** — every ring of every component, exterior and
///   holes alike, as a `MULTILINESTRING` in written order.
/// * **GeometryCollection** — a `GEOMETRYCOLLECTION` of the members' boundaries,
///   member for member.
///
/// The result is always `XY`. The boundary of a `POLYGON Z` is therefore a
/// two-dimensional `MULTILINESTRING`: Clause 10.2 projects the polygon onto
/// `Z=0` before anything is computed, and the projected polygon's boundary has
/// no elevation.
#[must_use]
pub fn boundary(geometry: &Geometry) -> Geometry {
    match geometry.body() {
        GeometryBody::Point(_) | GeometryBody::MultiPoint(_) => nothing(),
        GeometryBody::LineString(_) | GeometryBody::MultiLineString(_) => {
            planar(GeometryBody::MultiPoint(
                curve_boundary_points(geometry)
                    .into_iter()
                    .map(Some)
                    .collect(),
            ))
        }
        GeometryBody::Polygon(_) | GeometryBody::MultiPolygon(_) => {
            surface_boundary(&Parts::of(geometry))
        }
        GeometryBody::GeometryCollection(members) => planar(GeometryBody::GeometryCollection(
            members.iter().map(boundary).collect(),
        )),
    }
}

/// Every ring of every polygon component, as a `MULTILINESTRING`.
fn surface_boundary(parts: &Parts) -> Geometry {
    let mut rings: Vec<CoordSeq> = Vec::new();
    for polygon in &parts.polygons {
        for ring in polygon {
            rings.push(run(ring));
        }
    }
    planar(GeometryBody::MultiLineString(rings))
}

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/// The minimum bounding box of `geometry`, as a geometry.
///
/// The four cases are decided by the box's extent, not by the input's kind, and
/// each degenerate one is a real answer rather than a padded rectangle:
///
/// * **Zero extent in both axes** — a single distinct position, however many
///   times it was written. The envelope is a `POINT` at that position. Emitting a
///   rectangle here would invent four corners the data does not have.
/// * **Zero extent in one axis** — a horizontal or vertical extent. The envelope
///   is a two-position `LINESTRING` from `(min_x, min_y)` to `(max_x, max_y)`,
///   **not** a zero-width `POLYGON`: a ring with zero area is a degenerate
///   surface that every downstream areal predicate would have to special-case.
/// * **Non-zero extent in both axes** — a `POLYGON` whose single ring is the
///   rectangle, wound counter-clockwise from the lower-left corner and closed:
///   `(min_x, min_y), (max_x, min_y), (max_x, max_y), (min_x, max_y),
///   (min_x, min_y)`. That order is fixed, so the output is byte-deterministic.
/// * **Empty input** — an empty `GEOMETRYCOLLECTION`. An empty geometry has no
///   extent, and a box around nothing is nothing.
///
/// The box is planar and the result is `XY`, per Clause 10.2.
#[must_use]
pub fn envelope(geometry: &Geometry) -> Geometry {
    let Some((min_x, min_y, max_x, max_y)) = bounds(geometry) else {
        return nothing();
    };
    let flat_x = min_x == max_x;
    let flat_y = min_y == max_y;
    if flat_x && flat_y {
        return planar(GeometryBody::Point(Some(Coord::xy(min_x, min_y))));
    }
    if flat_x || flat_y {
        return planar(GeometryBody::LineString(
            [Coord::xy(min_x, min_y), Coord::xy(max_x, max_y)]
                .into_iter()
                .collect(),
        ));
    }
    let ring: CoordSeq = [
        Coord::xy(min_x.clone(), min_y.clone()),
        Coord::xy(max_x.clone(), min_y.clone()),
        Coord::xy(max_x, max_y.clone()),
        Coord::xy(min_x.clone(), max_y),
        Coord::xy(min_x, min_y),
    ]
    .into_iter()
    .collect();
    planar(GeometryBody::Polygon(vec![ring]))
}

// ---------------------------------------------------------------------------
// Convex hull
// ---------------------------------------------------------------------------

/// Andrew's monotone chain over positions already sorted by `(x, y)` and
/// deduplicated, returning the strictly convex hull counter-clockwise from the
/// lexicographically smallest position.
///
/// Collinear positions are dropped — the orientation test pops on `0` as well as
/// on a clockwise turn — so an all-collinear input returns exactly its two
/// extremes.
fn monotone_chain(points: &[Coord]) -> Vec<Coord> {
    let mut lower = half_chain(points.iter());
    let mut upper = half_chain(points.iter().rev());
    lower.pop();
    upper.pop();
    lower.append(&mut upper);
    lower
}

/// One half of the chain, in the order `points` is walked.
fn half_chain<'a>(points: impl Iterator<Item = &'a Coord>) -> Vec<Coord> {
    let mut chain: Vec<Coord> = Vec::new();
    for point in points {
        while chain.len() >= 2
            && orientation(&chain[chain.len() - 2], &chain[chain.len() - 1], point) <= 0
        {
            chain.pop();
        }
        chain.push(point.clone());
    }
    chain
}

/// The convex hull of `geometry`, computed exactly with orientation tests.
///
/// Every distinct position in the input is a candidate — the vertices of curves
/// and rings as well as point members — because the hull of a geometry is the
/// hull of its positions.
///
/// The shape of the answer follows the hull's dimension, and each degenerate
/// case is an honest answer rather than a padded one:
///
/// * **Two-dimensional hull** — a `POLYGON` with one ring, wound
///   counter-clockwise and **starting at the lexicographically smallest
///   position** (smallest `x`, then smallest `y`). Positions that lie on a hull
///   edge without being a corner are dropped, so the ring is strictly convex.
///   Both the start and the direction are fixed, which is what makes the output
///   byte-deterministic rather than merely correct.
/// * **All positions collinear** — a two-position `LINESTRING` from the
///   lexicographically smallest position to the largest. The hull of a set of
///   collinear points genuinely is a segment; a zero-area ring would be a surface
///   that it is not.
/// * **One distinct position** — a `POINT`, however many times that position was
///   written. Duplicates collapse before the hull is built.
/// * **Empty input** — an empty `GEOMETRYCOLLECTION`.
///
/// The hull is planar and the result is `XY`, per Clause 10.2.
#[must_use]
pub fn convex_hull(geometry: &Geometry) -> Geometry {
    let mut points: Vec<Coord> = geometry
        .coords()
        .map(|c| Coord::xy(c.x().clone(), c.y().clone()))
        .collect();
    points.sort_by(|a, b| a.x().cmp(b.x()).then_with(|| a.y().cmp(b.y())));
    points.dedup();
    match points.len() {
        0 => return nothing(),
        1 => return planar(GeometryBody::Point(Some(points.swap_remove(0)))),
        _ => {}
    }
    let hull = monotone_chain(&points);
    if hull.len() < 3 {
        return planar(GeometryBody::LineString(run(&hull)));
    }
    let mut ring = run(&hull);
    ring.push(hull[0].clone());
    planar(GeometryBody::Polygon(vec![ring]))
}

// ---------------------------------------------------------------------------
// Centroid
// ---------------------------------------------------------------------------

/// The mathematical centroid of `geometry`, as a planar [`Coord`]; `None` when
/// the geometry is empty.
///
/// The highest dimension present wins, as OGC defines:
///
/// 1. **Areal** — the area-weighted centroid of the polygon components. Each ring
///    contributes `Σ (x_i + x_{i+1}) * cross_i / 6` against its own signed area,
///    holes with the opposite sign, and the answer is the ratio of those two
///    sums. Every term is a rational function of rational coordinates, so **this
///    case is exact with nothing rounded at all**; the returned `Rat`s are the
///    true centroid and the caller decides how many digits to serialize. Winding
///    does not matter: each ring's contribution is normalized by the sign of its
///    own area, so a clockwise exterior gives the same answer as a
///    counter-clockwise one.
/// 2. **Linear** — the length-weighted centroid of the segments: each segment's
///    midpoint weighted by its length. Segment lengths are irrational, so the
///    weights are the fixed-scale integers of
///    [`crate::measure::LENGTH_SCALE_DIGITS`] and the answer carries their
///    truncation. It is still exactly reproducible — integer weights, exact
///    rational midpoints, one exact division at the end — just not exactly the
///    true centroid. This case is also where a **zero-area** polygon lands,
///    because a degenerate surface has no area to weight by while its rings still
///    have length.
/// 3. **Point** — the arithmetic mean of the point members.
/// 4. **Every named position** — the arithmetic mean of every vertex, reached
///    only by a geometry whose every segment has zero length (a curve written as
///    one repeated position). Without it such a geometry would have no centroid
///    even though it plainly has a place.
///
/// The result is planar, per Clause 10.2: the centroid of a `POLYGON Z` is the
/// centroid of its shadow on `Z=0` and carries no elevation.
#[must_use]
pub fn centroid(geometry: &Geometry) -> Option<Coord> {
    let parts = Parts::of(geometry);
    areal_centroid(&parts)
        .or_else(|| linear_centroid(&parts))
        .or_else(|| mean(&parts.points.iter().collect::<Vec<&Coord>>()))
        .or_else(|| mean(&parts.vertices()))
}

/// The arithmetic mean of `points`, or `None` when there are none.
fn mean(points: &[&Coord]) -> Option<Coord> {
    let count = i64::try_from(points.len()).ok()?;
    if count == 0 {
        return None;
    }
    let divisor = Rat::from_i64(count);
    let mut sum_x = Rat::zero();
    let mut sum_y = Rat::zero();
    for point in points {
        sum_x = sum_x.add(point.x());
        sum_y = sum_y.add(point.y());
    }
    Some(Coord::xy(
        sum_x.div(&divisor).expect("a positive count is non-zero"),
        sum_y.div(&divisor).expect("a positive count is non-zero"),
    ))
}

/// The area-weighted centroid, or `None` when the total signed area is zero.
fn areal_centroid(parts: &Parts) -> Option<Coord> {
    let mut total_area = Rat::zero();
    let mut moment_x = Rat::zero();
    let mut moment_y = Rat::zero();
    for polygon in &parts.polygons {
        let Some((exterior, holes)) = polygon.split_first() else {
            continue;
        };
        accumulate_ring(
            exterior,
            true,
            &mut total_area,
            &mut moment_x,
            &mut moment_y,
        );
        for hole in holes {
            accumulate_ring(hole, false, &mut total_area, &mut moment_x, &mut moment_y);
        }
    }
    if total_area.is_zero() {
        return None;
    }
    Some(Coord::xy(
        moment_x
            .div(&total_area)
            .expect("the branch above proved the area non-zero"),
        moment_y
            .div(&total_area)
            .expect("the branch above proved the area non-zero"),
    ))
}

/// Add one ring's contribution to the running area and first moments.
///
/// `additive` is `true` for an exterior ring and `false` for a hole. The ring's
/// own winding is normalized away first — its signed area and its moments flip
/// together, so multiplying both by the sign of the area yields the
/// positively-wound contribution regardless of how the ring was written.
fn accumulate_ring(
    ring: &[Coord],
    additive: bool,
    total_area: &mut Rat,
    moment_x: &mut Rat,
    moment_y: &mut Rat,
) {
    let signed = signed_ring_area(ring);
    if signed.is_zero() {
        return;
    }
    let winding = i64::from(signed.signum());
    let factor = Rat::from_i64(if additive { winding } else { -winding });
    let six = Rat::from_i64(6);
    let mut sum_x = Rat::zero();
    let mut sum_y = Rat::zero();
    for pair in ring.windows(2) {
        let (p, q) = (&pair[0], &pair[1]);
        let cross = p.x().mul(q.y()).sub(&q.x().mul(p.y()));
        sum_x = sum_x.add(&p.x().add(q.x()).mul(&cross));
        sum_y = sum_y.add(&p.y().add(q.y()).mul(&cross));
    }
    *total_area = total_area.add(&factor.mul(&signed));
    *moment_x = moment_x.add(&factor.mul(&sum_x.div(&six).expect("six is not zero")));
    *moment_y = moment_y.add(&factor.mul(&sum_y.div(&six).expect("six is not zero")));
}

/// The length-weighted centroid of every segment, or `None` when every segment
/// has zero length.
fn linear_centroid(parts: &Parts) -> Option<Coord> {
    let half = Rat::new(Int::one(), Int::from_i64(2)).expect("two is not zero");
    let mut weight = Int::zero();
    let mut moment_x = Rat::zero();
    let mut moment_y = Rat::zero();
    for segment in &parts.segments() {
        let (p, q) = (&segment[0], &segment[1]);
        let scaled = scaled_segment_length(p, q);
        if scaled.is_zero() {
            continue;
        }
        let w = from_scaled(&scaled);
        moment_x = moment_x.add(&w.mul(&p.x().add(q.x()).mul(&half)));
        moment_y = moment_y.add(&w.mul(&p.y().add(q.y()).mul(&half)));
        weight = weight.add(&scaled);
    }
    if weight.is_zero() {
        return None;
    }
    let total = from_scaled(&weight);
    Some(Coord::xy(
        moment_x
            .div(&total)
            .expect("the branch above proved the weight non-zero"),
        moment_y
            .div(&total)
            .expect("the branch above proved the weight non-zero"),
    ))
}

#[cfg(test)]
mod tests {
    use super::{boundary, centroid, convex_hull, envelope};
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

    /// The written positions of a geometry, for pinning an exact vertex order.
    fn written(geometry: &Geometry) -> Vec<(Rat, Rat)> {
        geometry
            .coords()
            .map(|c| (c.x().clone(), c.y().clone()))
            .collect()
    }

    fn pairs(points: &[(i64, i64)]) -> Vec<(Rat, Rat)> {
        points.iter().map(|&(x, y)| (r(x), r(y))).collect()
    }

    // ---- boundary --------------------------------------------------------

    #[test]
    fn a_points_boundary_is_nothing_at_all() {
        for g in [point(1, 2), multipoint(&[(0, 0), (5, 5)])] {
            let b = boundary(&g);
            assert!(b.is_empty(), "a point set has an empty boundary");
            assert_eq!(
                b.kind(),
                GeometryKind::GeometryCollection,
                "and it is empty of any kind, not a degenerate point set"
            );
        }
    }

    #[test]
    fn an_open_curves_boundary_is_its_two_ends_and_a_closed_ones_is_empty() {
        let open = boundary(&line(&[(0, 0), (1, 1), (4, 0)]));
        assert_eq!(
            open.kind(),
            GeometryKind::MultiPoint,
            "a curve's boundary is a point set"
        );
        assert_eq!(
            written(&open),
            pairs(&[(0, 0), (4, 0)]),
            "the two endpoints, in lexicographic order"
        );
        let closed = boundary(&line(UNIT_SQUARE));
        assert!(
            closed.is_empty(),
            "a ring writes its endpoint twice, so the mod-2 rule cancels it"
        );
        assert_eq!(
            closed.kind(),
            GeometryKind::MultiPoint,
            "and the empty answer is still a point set"
        );
    }

    /// The mod-2 rule with teeth: a shared join is an endpoint of two members, so
    /// it cancels and only the far ends survive; a three-way join is an endpoint
    /// of three, an odd count, so it does not.
    #[test]
    fn a_multicurve_boundary_follows_the_mod_two_rule() {
        let joined = boundary(&multiline(&[&[(0, 0), (1, 0)], &[(1, 0), (2, 0)]]));
        assert_eq!(
            written(&joined),
            pairs(&[(0, 0), (2, 0)]),
            "the shared (1,0) is an endpoint of both members and cancels"
        );
        let disjoint = boundary(&multiline(&[&[(0, 0), (1, 0)], &[(5, 0), (6, 0)]]));
        assert_eq!(
            written(&disjoint),
            pairs(&[(0, 0), (1, 0), (5, 0), (6, 0)]),
            "nothing cancels when nothing is shared"
        );
        let three_way = boundary(&multiline(&[
            &[(0, 0), (1, 0)],
            &[(1, 0), (2, 0)],
            &[(1, 0), (1, 5)],
        ]));
        assert_eq!(
            written(&three_way),
            pairs(&[(0, 0), (1, 0), (1, 5), (2, 0)]),
            "(1,0) is an endpoint of three members, an odd count, so it survives"
        );
    }

    #[test]
    fn a_polygons_boundary_is_every_ring_exterior_first() {
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
        ]);
        let b = boundary(&with_hole);
        assert_eq!(
            b.kind(),
            GeometryKind::MultiLineString,
            "a surface's boundary is a curve set"
        );
        let GeometryBody::MultiLineString(lines) = b.body() else {
            panic!("the boundary of a polygon must be a MultiLineString");
        };
        assert_eq!(lines.len(), 2, "the exterior ring and the one hole");
        assert_eq!(
            written(&b),
            pairs(&[
                (0, 0),
                (4, 0),
                (4, 4),
                (0, 4),
                (0, 0),
                (1, 1),
                (3, 1),
                (3, 3),
                (1, 3),
                (1, 1)
            ]),
            "in written order, each ring closed"
        );
        let two = boundary(&multipolygon(&[
            &[UNIT_SQUARE],
            &[&[(5, 5), (6, 5), (6, 6), (5, 6), (5, 5)]],
        ]));
        let GeometryBody::MultiLineString(members) = two.body() else {
            panic!("the boundary of a multipolygon must be a MultiLineString");
        };
        assert_eq!(members.len(), 2, "one ring per component");
    }

    /// "First project geometry onto the Z=0 level" is what makes this a 2D
    /// answer: the boundary of a 3D polygon carries no elevation.
    #[test]
    fn the_boundary_of_a_three_dimensional_polygon_is_a_two_dimensional_curve() {
        let raised = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::Polygon(vec![
                [
                    Coord::new(r(0), r(0), Some(r(7)), None),
                    Coord::new(r(1), r(0), Some(r(7)), None),
                    Coord::new(r(1), r(1), Some(r(7)), None),
                    Coord::new(r(0), r(0), Some(r(7)), None),
                ]
                .into_iter()
                .collect(),
            ]),
        )
        .expect("a closed 3D ring");
        let b = boundary(&raised);
        assert_eq!(b.dim(), CoordDim::Xy, "the answer is planar");
        assert!(
            b.coords().all(|c| c.z().is_none()),
            "and carries no elevation at all"
        );
    }

    #[test]
    fn a_collections_boundary_is_its_members_boundaries() {
        let b = boundary(&collection(vec![
            point(9, 9),
            line(&[(0, 0), (3, 0)]),
            polygon(&[UNIT_SQUARE]),
        ]));
        let GeometryBody::GeometryCollection(members) = b.body() else {
            panic!("the boundary of a collection must be a collection");
        };
        assert_eq!(members.len(), 3, "member for member");
        assert_eq!(
            members[0].kind(),
            GeometryKind::GeometryCollection,
            "the point's boundary is nothing"
        );
        assert_eq!(
            members[1].kind(),
            GeometryKind::MultiPoint,
            "the curve's is a point set"
        );
        assert_eq!(
            members[2].kind(),
            GeometryKind::MultiLineString,
            "the surface's is a curve set"
        );
    }

    // ---- envelope --------------------------------------------------------

    #[test]
    fn a_degenerate_envelope_is_a_point_or_a_curve_rather_than_a_padded_rectangle() {
        let single = envelope(&point(3, 4));
        assert_eq!(
            single.kind(),
            GeometryKind::Point,
            "no extent in either axis"
        );
        assert_eq!(written(&single), pairs(&[(3, 4)]), "at the position itself");
        assert_eq!(
            envelope(&multipoint(&[(3, 4), (3, 4), (3, 4)])).kind(),
            GeometryKind::Point,
            "duplicates give no extent either"
        );

        let horizontal = envelope(&line(&[(1, 5), (9, 5), (4, 5)]));
        assert_eq!(
            horizontal.kind(),
            GeometryKind::LineString,
            "zero extent in y is a segment, not a zero-width polygon"
        );
        assert_eq!(
            written(&horizontal),
            pairs(&[(1, 5), (9, 5)]),
            "from (min_x, min_y) to (max_x, max_y)"
        );
        let vertical = envelope(&line(&[(2, 0), (2, 8)]));
        assert_eq!(
            vertical.kind(),
            GeometryKind::LineString,
            "and so is zero extent in x"
        );
        assert_eq!(written(&vertical), pairs(&[(2, 0), (2, 8)]));
    }

    #[test]
    fn a_normal_envelope_is_the_rectangle_wound_counter_clockwise_from_the_lower_left() {
        let e = envelope(&multipoint(&[(2, 3), (7, 11), (5, 5)]));
        assert_eq!(e.kind(), GeometryKind::Polygon, "two axes of extent");
        assert_eq!(
            written(&e),
            pairs(&[(2, 3), (7, 3), (7, 11), (2, 11), (2, 3)]),
            "the vertex order is pinned so the output is byte-deterministic"
        );
    }

    #[test]
    fn the_envelope_of_an_empty_geometry_is_the_empty_geometry() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            let e = envelope(&Geometry::empty(CoordDim::Xy, kind));
            assert!(e.is_empty(), "{kind:?} EMPTY has no extent to box");
            assert_eq!(
                e.kind(),
                GeometryKind::GeometryCollection,
                "and the empty answer is the canonical empty geometry"
            );
        }
    }

    // ---- convex hull -----------------------------------------------------

    #[test]
    fn the_hull_of_a_square_with_an_interior_point_is_the_square_with_a_pinned_vertex_order() {
        let hull = convex_hull(&multipoint(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 2)]));
        assert_eq!(hull.kind(), GeometryKind::Polygon, "a 2-dimensional hull");
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)]),
            "counter-clockwise from the lexicographically smallest vertex, closed; the interior \
             point is gone"
        );
    }

    #[test]
    fn a_position_on_a_hull_edge_is_not_a_hull_vertex() {
        let hull = convex_hull(&multipoint(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 0)]));
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)]),
            "(2,0) sits on the bottom edge, so the ring stays strictly convex"
        );
    }

    #[test]
    fn a_collinear_input_hulls_to_a_curve_and_a_single_position_to_a_point() {
        let collinear = convex_hull(&multipoint(&[(0, 0), (1, 1), (2, 2)]));
        assert_eq!(
            collinear.kind(),
            GeometryKind::LineString,
            "the hull of collinear points is a segment, not a zero-area ring"
        );
        assert_eq!(
            written(&collinear),
            pairs(&[(0, 0), (2, 2)]),
            "from the smallest position to the largest"
        );

        let duplicated = convex_hull(&multipoint(&[(1, 1), (1, 1), (1, 1)]));
        assert_eq!(
            duplicated.kind(),
            GeometryKind::Point,
            "duplicates collapse to one distinct position"
        );
        assert_eq!(written(&duplicated), pairs(&[(1, 1)]));

        let single = convex_hull(&point(6, 7));
        assert_eq!(
            single.kind(),
            GeometryKind::Point,
            "one position, one point"
        );
        assert_eq!(written(&single), pairs(&[(6, 7)]));

        let two = convex_hull(&multipoint(&[(1, 0), (0, 0)]));
        assert_eq!(
            two.kind(),
            GeometryKind::LineString,
            "two positions, a segment"
        );
        assert_eq!(
            written(&two),
            pairs(&[(0, 0), (1, 0)]),
            "sorted, so the output does not depend on the written order"
        );
    }

    #[test]
    fn the_hull_of_an_empty_geometry_is_the_empty_geometry() {
        let hull = convex_hull(&Geometry::empty(CoordDim::Xy, GeometryKind::MultiPolygon));
        assert!(hull.is_empty(), "there is nothing to hull");
        assert_eq!(hull.kind(), GeometryKind::GeometryCollection);
    }

    #[test]
    fn the_hull_takes_every_position_including_ring_and_curve_vertices() {
        let hull = convex_hull(&collection(vec![
            polygon(&[UNIT_SQUARE]),
            line(&[(3, 0), (3, 1)]),
        ]));
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (3, 0), (3, 1), (0, 1), (0, 0)]),
            "ring vertices and curve vertices are both candidates"
        );
    }

    // ---- centroid --------------------------------------------------------

    #[test]
    fn the_areal_centroid_is_exact_with_nothing_rounded() {
        assert_eq!(
            centroid(&polygon(&[UNIT_SQUARE])),
            Some(Coord::xy(ratio(1, 2), ratio(1, 2))),
            "the unit square's centre is exactly (1/2, 1/2)"
        );
        // An L: a 2x1 base and a 1x2 upright, total area 4, worked by hand as the
        // area-weighted mean of the two rectangles' centres —
        // Cx = (2*1 + 2*(1/2))/4 = 3/4, Cy = (2*(1/2) + 2*2)/4 = 5/4.
        let l_shape = polygon(&[&[(0, 0), (2, 0), (2, 1), (1, 1), (1, 3), (0, 3), (0, 0)]]);
        assert_eq!(
            centroid(&l_shape),
            Some(Coord::xy(ratio(3, 4), ratio(5, 4))),
            "the L-shape's centroid, hand-computed"
        );
        assert_eq!(
            centroid(&polygon(&[&[(0, 0), (3, 0), (0, 3), (0, 0)]])),
            Some(Coord::xy(r(1), r(1))),
            "a triangle's centroid is the mean of its corners"
        );
    }

    /// Winding is a spelling choice: the same ring written the other way round
    /// has the same centroid, because each ring is normalized by the sign of its
    /// own area before it is combined.
    #[test]
    fn the_areal_centroid_does_not_depend_on_ring_winding() {
        let counter_clockwise = polygon(&[UNIT_SQUARE]);
        let clockwise = polygon(&[&[(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]]);
        assert_eq!(
            centroid(&counter_clockwise),
            centroid(&clockwise),
            "the two windings must agree"
        );
    }

    #[test]
    fn a_hole_pulls_the_areal_centroid_away_from_it() {
        // A 4x4 square, area 16, centroid (2,2); minus a 1x1 hole spanning
        // x in [2,3] and y in [2,3], area 1, centroid (5/2, 5/2).
        // Cx = (16*2 - 1*(5/2)) / 15 = (59/2)/15 = 59/30.
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(2, 2), (3, 2), (3, 3), (2, 3), (2, 2)],
        ]);
        assert_eq!(
            centroid(&with_hole),
            Some(Coord::xy(ratio(59, 30), ratio(59, 30))),
            "the hole subtracts both its area and its moment"
        );
    }

    #[test]
    fn a_linear_centroid_is_length_weighted() {
        assert_eq!(
            centroid(&line(&[(0, 0), (2, 0)])),
            Some(Coord::xy(r(1), Rat::zero())),
            "one segment's centroid is its midpoint"
        );
        assert_eq!(
            centroid(&line(&[(0, 0), (1, 0), (1, 1)])),
            Some(Coord::xy(ratio(3, 4), ratio(1, 4))),
            "two equal unit segments, midpoints (1/2,0) and (1,1/2)"
        );
        // Unequal weights: a length-3 segment and a length-1 segment.
        // Cx = (3*(3/2) + 1*(7/2))/4 = (9/2 + 7/2)/4 = 2.
        assert_eq!(
            centroid(&line(&[(0, 0), (3, 0), (4, 0)])),
            Some(Coord::xy(r(2), Rat::zero())),
            "the longer segment pulls harder, which a plain vertex mean would miss"
        );
    }

    #[test]
    fn a_point_centroid_is_the_arithmetic_mean() {
        assert_eq!(
            centroid(&multipoint(&[(0, 0), (2, 4)])),
            Some(Coord::xy(r(1), r(2))),
            "a two-point multipoint sits halfway between them"
        );
        assert_eq!(
            centroid(&point(5, 6)),
            Some(Coord::xy(r(5), r(6))),
            "a point is its own centroid"
        );
        assert_eq!(
            centroid(&multipoint(&[(0, 0), (1, 0), (2, 0)])),
            Some(Coord::xy(r(1), Rat::zero())),
            "three points, equally weighted"
        );
    }

    /// The highest dimension present wins, so a point beside a polygon does not
    /// move the polygon's centroid at all.
    #[test]
    fn the_highest_dimension_present_decides_the_centroid() {
        let areal_and_more = collection(vec![
            polygon(&[UNIT_SQUARE]),
            point(1000, 1000),
            line(&[(500, 500), (600, 600)]),
        ]);
        assert_eq!(
            centroid(&areal_and_more),
            Some(Coord::xy(ratio(1, 2), ratio(1, 2))),
            "the areal component alone decides it"
        );
        let linear_and_points = collection(vec![line(&[(0, 0), (2, 0)]), point(1000, 1000)]);
        assert_eq!(
            centroid(&linear_and_points),
            Some(Coord::xy(r(1), Rat::zero())),
            "and with no area, the curve decides it"
        );
    }

    /// A zero-area polygon has no area to weight by, so it falls to the linear
    /// case rather than having no centroid at all.
    #[test]
    fn a_degenerate_geometry_still_has_a_centroid() {
        let flat = polygon(&[&[(0, 0), (2, 0), (4, 0), (0, 0)]]);
        assert!(
            centroid(&flat).is_some(),
            "a collapsed ring still has rings with length"
        );
        assert_eq!(
            centroid(&line(&[(3, 4), (3, 4)])),
            Some(Coord::xy(r(3), r(4))),
            "a curve written as one repeated position is at that position"
        );
    }

    #[test]
    fn only_an_empty_geometry_has_no_centroid() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            assert_eq!(
                centroid(&Geometry::empty(CoordDim::Xy, kind)),
                None,
                "{kind:?} EMPTY has no position"
            );
        }
        assert!(
            centroid(&point(0, 0)).is_some(),
            "and the neighbouring non-empty case does have one"
        );
    }

    #[test]
    fn every_constructed_output_is_planar() {
        let raised = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::MultiPoint(vec![
                Some(Coord::new(r(0), r(0), Some(r(1)), None)),
                Some(Coord::new(r(4), r(0), Some(r(2)), None)),
                Some(Coord::new(r(4), r(4), Some(r(3)), None)),
                Some(Coord::new(r(0), r(4), Some(r(4)), None)),
            ]),
        )
        .expect("a well-formed 3D multipoint");
        for built in [boundary(&raised), envelope(&raised), convex_hull(&raised)] {
            assert_eq!(built.dim(), CoordDim::Xy, "Clause 10.2 projects onto Z=0");
            assert!(
                built.coords().all(|c| c.z().is_none() && c.m().is_none()),
                "so no ordinate beyond x and y survives"
            );
        }
        assert_eq!(
            centroid(&raised),
            Some(Coord::xy(r(2), r(2))),
            "and the centroid is planar too"
        );
    }

    // ---- over-refusal control -------------------------------------------

    /// None of these constructors may refuse a well-formed geometry.
    /// `boundary`, `envelope` and `convex_hull` are total — they have no failure
    /// channel at all — and `centroid` returns `None` exactly for the empty
    /// geometry and nothing else. This is the assertion that keeps a future
    /// "guard" from quietly turning a degenerate input into a refusal.
    #[test]
    fn the_constructors_answer_for_every_well_formed_geometry_including_degenerate_ones() {
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
            assert_eq!(
                boundary(g).dim(),
                CoordDim::Xy,
                "boundary answers, planar: {:?}",
                g.kind()
            );
            let e = envelope(g);
            assert_eq!(e.dim(), CoordDim::Xy, "envelope answers, planar");
            assert_eq!(
                e.is_empty(),
                g.is_empty(),
                "an envelope is empty exactly when its input is: {:?}",
                g.kind()
            );
            let h = convex_hull(g);
            assert_eq!(h.dim(), CoordDim::Xy, "convex_hull answers, planar");
            assert_eq!(
                h.is_empty(),
                g.is_empty(),
                "a hull is empty exactly when its input is: {:?}",
                g.kind()
            );
            assert_eq!(
                centroid(g).is_some(),
                !g.is_empty(),
                "a centroid exists exactly when the geometry is non-empty: {:?}",
                g.kind()
            );
        }
    }
}
