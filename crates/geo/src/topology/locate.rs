// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact point location: which of a geometry's three point-sets — interior,
//! boundary, exterior — a given position belongs to.
//!
//! # Why this is the whole of the DE-9IM computation
//!
//! A DE-9IM entry is the dimension of the intersection of one of `a`'s three sets
//! with one of `b`'s. [`super::relate`] never computes those intersections as
//! *sets*: it produces a finite collection of witnesses (nodes and edge
//! fragments), classifies each witness against both geometries with this function,
//! and raises the corresponding entry. So the correctness of the matrix rests
//! almost entirely on this file being right about what "interior" and "boundary"
//! mean, kind by kind. Everything else is bookkeeping.
//!
//! # Everything is planar
//!
//! OGC GeoSPARQL 1.1 Clause 10.2: *"Geometric functions working with Geometries
//! that have Z values will ignore Z values in calculations and first project
//! geometry onto the Z=0 level. … Like Z values in coordinates, M values are to be
//! ignored."* Nothing here reads [`Coord::z`] or [`Coord::m`]; a position is its
//! `(x, y)` pair and nothing else.
//!
//! # The boundary is not "the edge you can see"
//!
//! OGC Simple Features defines the boundary per geometry kind, and the definitions
//! are less obvious than the pictures suggest:
//!
//! * A **point** has an *empty* boundary. A point is its own interior.
//! * A **curve** has its two endpoints as its boundary — *unless it is closed*
//!   (first position equal to last in the plane), in which case its boundary is
//!   empty. A closed ring drawn as a `LINESTRING` therefore has no boundary at
//!   all, and a point anywhere on it is in its interior.
//! * A **multi-curve**'s boundary is the **mod-2** combination of its members'
//!   boundaries: a position is on the boundary exactly when it is an endpoint of an
//!   *odd* number of member curves. Two curves joined end to end form one longer
//!   curve whose join is interior, and the mod-2 rule is what produces that
//!   without having to merge the members first.
//! * A **surface**'s boundary is all of its rings — exterior *and* holes.
//!
//! # The combination rule, and why boundary is tested before interior
//!
//! For every multi-geometry and every collection this file uses the JTS
//! `PointLocator` combination: walk the components, count how many report
//! `Boundary` and remember whether any reported `Interior`, then
//!
//! ```text
//! if the boundary count is odd            -> Boundary
//! else if the count is > 0, or any Interior -> Interior
//! else                                     -> Exterior
//! ```
//!
//! Two parts of that deserve their reasons written down.
//!
//! The `count > 0 and even -> Interior` branch is what makes a position on the
//! shared boundary of two touching polygons come out as **interior** of the
//! multi-polygon: the multi-geometry denotes the *union* of its members, and the
//! shared edge is in the interior of that union even though it is on the boundary
//! of each member separately. Reporting it as `Boundary` would make a
//! `MULTIPOLYGON` of two tiles behave differently from the single polygon covering
//! the same area, which is the bug this rule exists to prevent.
//!
//! The ordering — odd count wins *even when some component reported interior* — is
//! forced by the multi-curve rule above. In
//! `MULTILINESTRING((0 0, 2 0), (1 0, 1 1))` the position `(1 0)` is interior to
//! the first member and an endpoint of the second. The mod-2 boundary of the
//! multi-curve contains it (one member contributes it), so by the SFS definition of
//! interior as "the geometry minus its boundary" it is *not* interior. Testing
//! interior first would contradict the mod-2 rule this same file implements for
//! [`curve_boundary_points`], so the two would disagree with each other. They must
//! not.
//!
//! The price is that a geometry collection mixing dimensions can report `Boundary`
//! for a position that is also inside an area member. `GEOMETRYCOLLECTION`
//! topology is not well defined by SFS in the first place — the specification's
//! relational operators are stated for the six homogeneous kinds — so this file
//! follows the reference implementation rather than inventing a rule of its own.

use core::cmp::Ordering;

use super::segment::{cmp_xy, on_segment, plane};
use crate::de9im::Set;
use crate::exact::Rat;
use crate::geom::{Coord, CoordSeq, Geometry, GeometryBody, Rings};

/// Where `point` lies relative to `geometry`: its interior, its boundary, or its
/// exterior.
///
/// Exact and total: every position is in exactly one of the three sets, and an
/// empty geometry puts every position in its exterior. Only `x` and `y` are read,
/// per Clause 10.2.
pub fn locate(point: &Coord, geometry: &Geometry) -> Set {
    match geometry.body() {
        GeometryBody::Point(member) => locate_in_point(point, member.as_ref()),
        GeometryBody::LineString(coords) => locate_in_curve(point, coords),
        GeometryBody::Polygon(rings) => locate_in_surface(point, rings),
        GeometryBody::MultiPoint(members) => combine(
            members
                .iter()
                .map(|member| locate_in_point(point, member.as_ref())),
        ),
        GeometryBody::MultiLineString(members) => {
            combine(members.iter().map(|member| locate_in_curve(point, member)))
        }
        GeometryBody::MultiPolygon(members) => combine(
            members
                .iter()
                .map(|member| locate_in_surface(point, member)),
        ),
        GeometryBody::GeometryCollection(members) => {
            combine(members.iter().map(|member| locate(point, member)))
        }
    }
}

/// The JTS `PointLocator` combination rule. See the module docs for why the
/// boundary test comes first and why an even, non-zero count is interior.
fn combine(parts: impl Iterator<Item = Set>) -> Set {
    let mut any_interior = false;
    let mut boundaries = 0_usize;
    for part in parts {
        match part {
            Set::Interior => any_interior = true,
            Set::Boundary => boundaries += 1,
            Set::Exterior => {}
        }
    }
    if boundaries % 2 == 1 {
        Set::Boundary
    } else if boundaries > 0 || any_interior {
        Set::Interior
    } else {
        Set::Exterior
    }
}

/// Location against a single point member. A point's boundary is empty, so the
/// only two answers are interior and exterior.
fn locate_in_point(point: &Coord, member: Option<&Coord>) -> Set {
    match member {
        Some(member) if point.same_planar(member) => Set::Interior,
        _ => Set::Exterior,
    }
}

/// Location against a single curve: its two endpoints are its boundary unless it
/// is closed, and every other position on it is interior.
fn locate_in_curve(point: &Coord, coords: &CoordSeq) -> Set {
    let (Some(first), Some(last)) = (coords.first(), coords.last()) else {
        return Set::Exterior;
    };
    if !first.same_planar(last) && (point.same_planar(first) || point.same_planar(last)) {
        return Set::Boundary;
    }
    if coords
        .windows(2)
        .any(|edge| on_segment(point, &edge[0], &edge[1]))
    {
        return Set::Interior;
    }
    Set::Exterior
}

/// Location against a single surface: on any ring is boundary, otherwise strictly
/// inside the exterior ring and strictly outside every hole is interior.
fn locate_in_surface(point: &Coord, rings: &Rings) -> Set {
    let Some((shell, holes)) = rings.split_first() else {
        return Set::Exterior;
    };
    if rings.iter().any(|ring| on_ring(point, ring)) {
        return Set::Boundary;
    }
    if !crossing_number_is_odd(point, shell) {
        return Set::Exterior;
    }
    if holes.iter().any(|hole| crossing_number_is_odd(point, hole)) {
        return Set::Exterior;
    }
    Set::Interior
}

/// Whether `point` lies on any edge of `ring`.
fn on_ring(point: &Coord, ring: &CoordSeq) -> bool {
    ring.windows(2)
        .any(|edge| on_segment(point, &edge[0], &edge[1]))
}

/// The exact crossing-number (even-odd) test for a closed ring, for a `point`
/// already known not to lie on the ring.
///
/// The rule is the standard robust one: an edge `(i, j)` is counted when
/// `(y_i > py) != (y_j > py)` and the edge's abscissa at `py` is strictly greater
/// than `px`. The half-open `>` comparison is what makes a vertex count exactly
/// once rather than zero or twice, and it makes a horizontal edge — for which the
/// two comparisons agree — contribute nothing at all, which is correct because a
/// horizontal edge cannot be crossed by a horizontal ray. The guard also
/// guarantees `y_j != y_i`, so the division below has a non-zero denominator by
/// construction.
fn crossing_number_is_odd(point: &Coord, ring: &CoordSeq) -> bool {
    let mut inside = false;
    for edge in ring.windows(2) {
        let (from, to) = (&edge[0], &edge[1]);
        if (from.y() > point.y()) == (to.y() > point.y()) {
            continue;
        }
        let dy = to.y().sub(from.y());
        let t = point
            .y()
            .sub(from.y())
            .div(&dy)
            .expect("the straddle test guarantees the two ordinates differ");
        let abscissa = from.x().add(&t.mul(&to.x().sub(from.x())));
        if *point.x() < abscissa {
            inside = !inside;
        }
    }
    inside
}

/// The boundary points of a geometry's curve components, under the mod-2 rule.
///
/// A position is included exactly when it is an endpoint of an odd number of the
/// geometry's `LineString` components; a closed member contributes nothing,
/// because a closed curve has an empty boundary. Surfaces contribute nothing
/// either — their boundary is one-dimensional, not a point set.
///
/// The result is sorted in lexicographic `(x, y)` order and deduplicated, so it is
/// a pure function of the geometry rather than of any traversal or hashing order.
pub fn curve_boundary_points(geometry: &Geometry) -> Vec<Coord> {
    let mut endpoints = Vec::new();
    push_curve_endpoints(geometry, &mut endpoints);
    endpoints.sort_by(cmp_xy);

    let mut boundary = Vec::new();
    let mut index = 0;
    while index < endpoints.len() {
        let mut run = index + 1;
        while run < endpoints.len() && cmp_xy(&endpoints[index], &endpoints[run]) == Ordering::Equal
        {
            run += 1;
        }
        if (run - index) % 2 == 1 {
            boundary.push(endpoints[index].clone());
        }
        index = run;
    }
    boundary
}

/// Append the endpoints of every non-closed curve component, planar-projected.
fn push_curve_endpoints(geometry: &Geometry, out: &mut Vec<Coord>) {
    match geometry.body() {
        GeometryBody::LineString(coords) => push_chain_endpoints(coords, out),
        GeometryBody::MultiLineString(members) => {
            for member in members {
                push_chain_endpoints(member, out);
            }
        }
        GeometryBody::GeometryCollection(members) => {
            for member in members {
                push_curve_endpoints(member, out);
            }
        }
        GeometryBody::Point(_)
        | GeometryBody::Polygon(_)
        | GeometryBody::MultiPoint(_)
        | GeometryBody::MultiPolygon(_) => {}
    }
}

/// Append one chain's two endpoints, unless it is empty or closed.
fn push_chain_endpoints(coords: &CoordSeq, out: &mut Vec<Coord>) {
    let (Some(first), Some(last)) = (coords.first(), coords.last()) else {
        return;
    };
    if first.same_planar(last) {
        return;
    }
    out.push(plane(first));
    out.push(plane(last));
}

/// Whether `geometry` has any 2-dimensional component.
///
/// "Has area" means a non-empty surface is present. An empty polygon is *not*
/// counted: it contributes no interior for a scan line to find, and counting it
/// would make [`super::relate`]'s area pass run over geometries that provably
/// cannot raise a two-dimensional entry.
pub fn has_area(geometry: &Geometry) -> bool {
    match geometry.body() {
        GeometryBody::Polygon(rings) => !rings.is_empty(),
        GeometryBody::MultiPolygon(members) => members.iter().any(|rings| !rings.is_empty()),
        GeometryBody::GeometryCollection(members) => members.iter().any(has_area),
        GeometryBody::Point(_)
        | GeometryBody::LineString(_)
        | GeometryBody::MultiPoint(_)
        | GeometryBody::MultiLineString(_) => false,
    }
}

/// The largest topological dimension present: `-1` for empty, else `0`, `1` or
/// `2`.
///
/// This is the value the dimension-dependent GeoSPARQL relations (`sfOverlaps`,
/// `sfCrosses`) branch on, and `-1` for empty is the specification's own
/// convention rather than a sentinel invented here.
pub fn topological_dimension(geometry: &Geometry) -> i32 {
    match geometry.body() {
        GeometryBody::Point(member) => {
            if member.is_some() {
                0
            } else {
                -1
            }
        }
        GeometryBody::LineString(coords) => {
            if coords.is_empty() {
                -1
            } else {
                1
            }
        }
        GeometryBody::Polygon(rings) => {
            if rings.is_empty() {
                -1
            } else {
                2
            }
        }
        GeometryBody::MultiPoint(members) => {
            if members.iter().any(Option::is_some) {
                0
            } else {
                -1
            }
        }
        GeometryBody::MultiLineString(members) => {
            if members.iter().any(|member| !member.is_empty()) {
                1
            } else {
                -1
            }
        }
        GeometryBody::MultiPolygon(members) => {
            if members.iter().any(|rings| !rings.is_empty()) {
                2
            } else {
                -1
            }
        }
        GeometryBody::GeometryCollection(members) => members
            .iter()
            .map(topological_dimension)
            .max()
            .unwrap_or(-1),
    }
}

/// The exact abscissa where a positive-height segment crosses the horizontal line
/// `y = height`, or `None` when it does not strictly straddle it.
///
/// Shared with [`super::relate`]'s scan line, and kept here beside
/// [`crossing_number_is_odd`] because it is the same computation: both ask where a
/// segment meets a horizontal line, and having one of them drift from the other
/// would put the scan line's sample points somewhere this file does not agree
/// they are.
pub(crate) fn horizontal_crossing(from: &Coord, to: &Coord, height: &Rat) -> Option<Rat> {
    let (low, high) = if from.y() <= to.y() {
        (from.y(), to.y())
    } else {
        (to.y(), from.y())
    };
    if !(low < height && height < high) {
        return None;
    }
    let dy = to.y().sub(from.y());
    let t = height
        .sub(from.y())
        .div(&dy)
        .expect("a strict straddle implies the two ordinates differ");
    Some(from.x().add(&t.mul(&to.x().sub(from.x()))))
}

#[cfg(test)]
mod tests {
    use super::{
        curve_boundary_points, has_area, horizontal_crossing, locate, topological_dimension,
    };
    use crate::de9im::Set;
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, Rings};

    fn r(value: i64) -> Rat {
        Rat::from_i64(value)
    }

    fn q(numerator: i64, denominator: i64) -> Rat {
        Rat::new(Int::from_i64(numerator), Int::from_i64(denominator))
            .expect("a non-zero denominator")
    }

    fn c(x: i64, y: i64) -> Coord {
        Coord::xy(r(x), r(y))
    }

    fn seq(points: &[(i64, i64)]) -> CoordSeq {
        points.iter().map(|&(x, y)| c(x, y)).collect()
    }

    fn point(x: i64, y: i64) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::Point(Some(c(x, y))))
            .expect("a well-formed point")
    }

    fn line(points: &[(i64, i64)]) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::LineString(seq(points)))
            .expect("a well-formed line")
    }

    fn rings(shape: &[&[(i64, i64)]]) -> Rings {
        shape.iter().map(|ring| seq(ring)).collect()
    }

    fn polygon(shape: &[&[(i64, i64)]]) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::Polygon(rings(shape)))
            .expect("well-formed closed rings")
    }

    /// The unit square scaled to `[0, size]²`, written counter-clockwise.
    fn square(size: i64) -> Geometry {
        polygon(&[&[(0, 0), (size, 0), (size, size), (0, size), (0, 0)]])
    }

    // ---- points ----------------------------------------------------------

    /// A point's boundary is empty: a position is either the point (interior) or
    /// not (exterior), and `Boundary` is never returned.
    #[test]
    fn a_point_has_an_empty_boundary() {
        let p = point(1, 2);
        assert_eq!(locate(&c(1, 2), &p), Set::Interior, "the point itself");
        assert_eq!(locate(&c(1, 3), &p), Set::Exterior, "anywhere else");
        assert_eq!(
            locate(
                &c(1, 2),
                &Geometry::empty(CoordDim::Xy, GeometryKind::Point)
            ),
            Set::Exterior,
            "every position is exterior to POINT EMPTY"
        );
    }

    /// A position that equals a member of a multi-point is interior; the mod-2
    /// combination never turns a point set into a boundary.
    #[test]
    fn a_multi_point_is_interior_at_its_members_and_never_a_boundary() {
        let multi = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(vec![Some(c(0, 0)), None, Some(c(5, 5))]),
        )
        .expect("well formed");
        assert_eq!(locate(&c(0, 0), &multi), Set::Interior, "the first member");
        assert_eq!(locate(&c(5, 5), &multi), Set::Interior, "the last member");
        assert_eq!(locate(&c(1, 1), &multi), Set::Exterior, "no member here");
    }

    // ---- curves ----------------------------------------------------------

    /// An open curve's two endpoints are its boundary and everything else on it
    /// is interior; a CLOSED curve has no boundary at all.
    #[test]
    fn an_open_curve_has_two_boundary_points_and_a_closed_one_has_none() {
        let open = line(&[(0, 0), (2, 0), (2, 2)]);
        assert_eq!(locate(&c(0, 0), &open), Set::Boundary, "the first endpoint");
        assert_eq!(locate(&c(2, 2), &open), Set::Boundary, "the last endpoint");
        assert_eq!(locate(&c(1, 0), &open), Set::Interior, "along an edge");
        assert_eq!(locate(&c(2, 0), &open), Set::Interior, "the middle vertex");
        assert_eq!(locate(&c(1, 1), &open), Set::Exterior, "off the curve");

        let closed = line(&[(0, 0), (2, 0), (2, 2), (0, 0)]);
        assert_eq!(
            locate(&c(0, 0), &closed),
            Set::Interior,
            "a closed curve's join is interior, not boundary"
        );
        assert_eq!(locate(&c(1, 0), &closed), Set::Interior, "along an edge");
        assert_eq!(
            locate(&c(1, 1), &closed),
            Set::Interior,
            "the closing edge runs along y = x, so (1, 1) is ON the curve"
        );
        assert_eq!(
            locate(&c(1, 2), &closed),
            Set::Exterior,
            "a position on no edge of the closed curve"
        );
        assert_eq!(
            locate(&c(0, 1), &closed),
            Set::Exterior,
            "and another, inside the curve's convex hull but off every edge"
        );
    }

    /// A zero-length curve is degenerate but must still be answered rather than
    /// dividing by zero or reporting nonsense.
    #[test]
    fn a_zero_length_curve_is_located_without_dividing_by_zero() {
        let degenerate = line(&[(3, 4), (3, 4)]);
        assert_eq!(
            locate(&c(3, 4), &degenerate),
            Set::Interior,
            "a zero-length curve is closed, so it has no boundary"
        );
        assert_eq!(
            locate(&c(3, 5), &degenerate),
            Set::Exterior,
            "and nothing else is on it"
        );
    }

    /// The mod-2 rule: two curves joined end to end make the join INTERIOR, and
    /// the two far ends the boundary.
    #[test]
    fn two_curves_joined_end_to_end_have_an_interior_join() {
        let joined = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(vec![seq(&[(0, 0), (1, 0)]), seq(&[(1, 0), (2, 0)])]),
        )
        .expect("well formed");
        assert_eq!(
            locate(&c(1, 0), &joined),
            Set::Interior,
            "the join is an endpoint of two members, so mod-2 removes it"
        );
        assert_eq!(locate(&c(0, 0), &joined), Set::Boundary, "the far left end");
        assert_eq!(
            locate(&c(2, 0), &joined),
            Set::Boundary,
            "the far right end"
        );
    }

    /// The ordering claim from the module docs, executed: a position interior to
    /// one member and an endpoint of another is BOUNDARY, because the mod-2
    /// boundary contains it and the interior is the geometry minus its boundary.
    #[test]
    fn an_endpoint_landing_inside_another_member_is_boundary_not_interior() {
        let tee = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(vec![seq(&[(0, 0), (2, 0)]), seq(&[(1, 0), (1, 1)])]),
        )
        .expect("well formed");
        assert_eq!(
            locate(&c(1, 0), &tee),
            Set::Boundary,
            "one member contributes the position to the mod-2 boundary"
        );
        assert_eq!(
            curve_boundary_points(&tee),
            vec![c(0, 0), c(1, 0), c(1, 1), c(2, 0)],
            "and curve_boundary_points must agree with locate about it"
        );
    }

    /// `curve_boundary_points` implements mod-2 and is sorted, deduplicated and
    /// independent of member order.
    #[test]
    fn curve_boundary_points_is_mod_two_sorted_and_order_independent() {
        let forward = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(vec![
                seq(&[(0, 0), (1, 0)]),
                seq(&[(1, 0), (2, 0)]),
                seq(&[(2, 0), (3, 0)]),
            ]),
        )
        .expect("well formed");
        assert_eq!(
            curve_boundary_points(&forward),
            vec![c(0, 0), c(3, 0)],
            "three chained curves have two boundary points"
        );

        let reversed = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(vec![
                seq(&[(2, 0), (3, 0)]),
                seq(&[(0, 0), (1, 0)]),
                seq(&[(1, 0), (2, 0)]),
            ]),
        )
        .expect("well formed");
        assert_eq!(
            curve_boundary_points(&reversed),
            curve_boundary_points(&forward),
            "member order cannot change the answer"
        );

        let ring = line(&[(0, 0), (1, 0), (1, 1), (0, 0)]);
        assert!(
            curve_boundary_points(&ring).is_empty(),
            "a closed curve contributes nothing"
        );
        assert!(
            curve_boundary_points(&square(2)).is_empty(),
            "a surface contributes nothing: its boundary is not a point set"
        );
    }

    // ---- surfaces --------------------------------------------------------

    /// The three answers for a simple square, including every vertex and a
    /// horizontal-edge position, which is where a naive ray test goes wrong.
    #[test]
    fn a_square_locates_its_interior_boundary_and_exterior_exactly() {
        let sq = square(4);
        assert_eq!(locate(&c(2, 2), &sq), Set::Interior, "the middle");
        assert_eq!(locate(&c(0, 0), &sq), Set::Boundary, "a corner");
        assert_eq!(locate(&c(4, 4), &sq), Set::Boundary, "the far corner");
        assert_eq!(locate(&c(2, 0), &sq), Set::Boundary, "a horizontal edge");
        assert_eq!(locate(&c(0, 2), &sq), Set::Boundary, "a vertical edge");
        assert_eq!(
            locate(&c(5, 2), &sq),
            Set::Exterior,
            "beyond the right edge"
        );
        assert_eq!(
            locate(&c(-1, 2), &sq),
            Set::Exterior,
            "before the left edge"
        );
        assert_eq!(locate(&c(2, 5), &sq), Set::Exterior, "above");
        assert_eq!(
            locate(&c(5, 0), &sq),
            Set::Exterior,
            "level with a horizontal edge but outside it"
        );
        assert_eq!(
            locate(&c(5, 4), &sq),
            Set::Exterior,
            "level with the top edge but outside it"
        );
    }

    /// A ray that passes exactly through a vertex must count that vertex once,
    /// not zero times and not twice. The diamond puts vertices at the extreme
    /// `y` values on purpose.
    #[test]
    fn a_ray_through_a_vertex_counts_it_exactly_once() {
        let diamond = polygon(&[&[(2, 0), (4, 2), (2, 4), (0, 2), (2, 0)]]);
        assert_eq!(
            locate(&c(2, 2), &diamond),
            Set::Interior,
            "the centre, on a ray through the left and right vertices"
        );
        assert_eq!(
            locate(&c(-1, 2), &diamond),
            Set::Exterior,
            "the same ray, outside on the left"
        );
        assert_eq!(
            locate(&c(5, 2), &diamond),
            Set::Exterior,
            "the same ray, outside on the right"
        );
        assert_eq!(
            locate(&c(2, 0), &diamond),
            Set::Boundary,
            "the bottom vertex, an extremum of y"
        );
        assert_eq!(
            locate(&c(1, 1), &diamond),
            Set::Boundary,
            "a position on a slanted edge"
        );
        assert_eq!(
            locate(&c(0, 0), &diamond),
            Set::Exterior,
            "the bounding-box corner is outside the diamond"
        );
    }

    /// A hole is exterior inside, boundary on its ring, and does not disturb the
    /// surrounding interior.
    #[test]
    fn a_hole_is_exterior_inside_and_boundary_on_its_ring() {
        let with_hole = polygon(&[
            &[(0, 0), (6, 0), (6, 6), (0, 6), (0, 0)],
            &[(2, 2), (4, 2), (4, 4), (2, 4), (2, 2)],
        ]);
        assert_eq!(
            locate(&c(3, 3), &with_hole),
            Set::Exterior,
            "inside the hole"
        );
        assert_eq!(
            locate(&c(2, 2), &with_hole),
            Set::Boundary,
            "a hole corner is on the boundary"
        );
        assert_eq!(
            locate(&c(3, 2), &with_hole),
            Set::Boundary,
            "a hole edge is on the boundary"
        );
        assert_eq!(
            locate(&c(1, 3), &with_hole),
            Set::Interior,
            "between the shell and the hole"
        );
        assert_eq!(
            locate(&c(7, 3), &with_hole),
            Set::Exterior,
            "outside the shell"
        );
    }

    /// A position on the shared edge of two touching polygons of a MULTIPOLYGON
    /// is INTERIOR to the multi-polygon: it is inside the union those members
    /// denote. This is the even-count branch of the combination rule.
    #[test]
    fn a_shared_edge_of_a_multi_polygon_is_interior_to_the_union() {
        let touching = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPolygon(vec![
                rings(&[&[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]]),
                rings(&[&[(1, 0), (2, 0), (2, 1), (1, 1), (1, 0)]]),
            ]),
        )
        .expect("well formed");
        assert_eq!(
            locate(&c(1, 0), &touching),
            Set::Interior,
            "a corner shared by both members: two boundaries, an even count"
        );
        assert_eq!(
            locate(&c(0, 0), &touching),
            Set::Boundary,
            "a corner belonging to one member only"
        );
        assert_eq!(
            locate(&c(1, 1), &touching),
            Set::Interior,
            "the other shared corner"
        );
        assert_eq!(locate(&c(3, 0), &touching), Set::Exterior, "well outside");
    }

    /// Empty bodies of every kind put every position in the exterior.
    #[test]
    fn every_empty_geometry_puts_every_position_in_its_exterior() {
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
            assert_eq!(
                locate(&c(0, 0), &empty),
                Set::Exterior,
                "{kind:?} EMPTY has no interior and no boundary"
            );
        }
    }

    /// A geometry collection combines its members with the same rule, and
    /// recursion through a nested collection does not change the answer.
    #[test]
    fn a_geometry_collection_combines_and_nests() {
        let flat = Geometry::new(
            CoordDim::Xy,
            GeometryBody::GeometryCollection(vec![point(9, 9), square(2)]),
        )
        .expect("uniform dimension");
        assert_eq!(locate(&c(1, 1), &flat), Set::Interior, "inside the square");
        assert_eq!(locate(&c(9, 9), &flat), Set::Interior, "at the point");
        assert_eq!(
            locate(&c(0, 1), &flat),
            Set::Boundary,
            "on the square's edge"
        );
        assert_eq!(
            locate(&c(5, 5), &flat),
            Set::Exterior,
            "nowhere near either"
        );

        let nested = Geometry::new(
            CoordDim::Xy,
            GeometryBody::GeometryCollection(vec![
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::GeometryCollection(vec![square(2)]),
                )
                .expect("uniform dimension"),
            ]),
        )
        .expect("uniform dimension");
        assert_eq!(
            locate(&c(1, 1), &nested),
            Set::Interior,
            "nesting must not change the location"
        );
        assert_eq!(locate(&c(0, 1), &nested), Set::Boundary, "nor the boundary");
    }

    // ---- Clause 10.2 -----------------------------------------------------

    /// Elevation and measure are ignored: a geometry that differs only in `z`
    /// locates identically, and a position with a wild `z` still locates by its
    /// projection.
    #[test]
    fn elevation_and_measure_are_ignored_in_every_location() {
        let flat = square(4);
        let raised = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::Polygon(vec![
                [
                    Coord::new(r(0), r(0), Some(r(7)), None),
                    Coord::new(r(4), r(0), Some(r(-3)), None),
                    Coord::new(r(4), r(4), Some(r(99)), None),
                    Coord::new(r(0), r(4), Some(r(0)), None),
                    Coord::new(r(0), r(0), Some(r(1)), None),
                ]
                .into_iter()
                .collect(),
            ]),
        )
        .expect("a ring that closes in the plane");
        for probe in [c(2, 2), c(0, 0), c(2, 0), c(9, 9)] {
            assert_eq!(
                locate(&probe, &flat),
                locate(&probe, &raised),
                "Clause 10.2 projects onto z = 0 before deciding anything"
            );
        }
        let high = Coord::new(r(2), r(2), Some(r(1000)), Some(r(-1000)));
        assert_eq!(
            locate(&high, &flat),
            Set::Interior,
            "the probe's own z and m are ignored too"
        );
    }

    // ---- dimension and area ---------------------------------------------

    /// `topological_dimension` is `-1` for every empty geometry and the largest
    /// present dimension otherwise, and `has_area` agrees with it exactly.
    #[test]
    fn dimension_is_minus_one_when_empty_and_agrees_with_has_area() {
        let cases: Vec<(Geometry, i32)> = vec![
            (Geometry::empty(CoordDim::Xy, GeometryKind::Point), -1),
            (Geometry::empty(CoordDim::Xy, GeometryKind::Polygon), -1),
            (
                Geometry::empty(CoordDim::Xy, GeometryKind::GeometryCollection),
                -1,
            ),
            (point(0, 0), 0),
            (line(&[(0, 0), (1, 1)]), 1),
            (square(1), 2),
            (
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::MultiPoint(vec![None, Some(c(1, 1))]),
                )
                .expect("well formed"),
                0,
            ),
            (
                Geometry::new(CoordDim::Xy, GeometryBody::MultiPoint(vec![None])).expect("ok"),
                -1,
            ),
            (
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::GeometryCollection(vec![point(0, 0), line(&[(0, 0), (1, 1)])]),
                )
                .expect("uniform dimension"),
                1,
            ),
            (
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::GeometryCollection(vec![
                        line(&[(0, 0), (1, 1)]),
                        square(1),
                        point(4, 4),
                    ]),
                )
                .expect("uniform dimension"),
                2,
            ),
        ];
        for (geometry, expected) in cases {
            assert_eq!(
                topological_dimension(&geometry),
                expected,
                "dimension of {:?}",
                geometry.kind()
            );
            assert_eq!(
                has_area(&geometry),
                expected == 2,
                "has_area must be exactly 'the dimension is two' for {:?}",
                geometry.kind()
            );
        }
    }

    // ---- the shared horizontal crossing ---------------------------------

    /// The scan line's crossing helper is exact, strict at both ends, and never
    /// reports a horizontal segment.
    #[test]
    fn the_horizontal_crossing_is_exact_and_strictly_straddling() {
        let from = c(0, 0);
        let to = c(2, 4);
        assert_eq!(
            horizontal_crossing(&from, &to, &r(2)),
            Some(r(1)),
            "halfway up is halfway across"
        );
        assert_eq!(
            horizontal_crossing(&from, &to, &r(1)),
            Some(q(1, 2)),
            "a quarter of the way up is an exact half across"
        );
        assert_eq!(
            horizontal_crossing(&from, &to, &r(0)),
            None,
            "an endpoint height does not strictly straddle"
        );
        assert_eq!(
            horizontal_crossing(&from, &to, &r(4)),
            None,
            "nor does the other endpoint height"
        );
        assert_eq!(
            horizontal_crossing(&from, &to, &r(9)),
            None,
            "nor a height above the segment"
        );
        assert_eq!(
            horizontal_crossing(&c(0, 5), &c(9, 5), &r(5)),
            None,
            "a horizontal segment never strictly straddles its own height"
        );
        assert_eq!(
            horizontal_crossing(&to, &from, &r(2)),
            Some(r(1)),
            "the answer does not depend on the endpoint order"
        );
    }
}
