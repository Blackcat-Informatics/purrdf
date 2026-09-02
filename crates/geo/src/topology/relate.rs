// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The DE-9IM intersection matrix of an ordered pair of geometries, computed
//! exactly.
//!
//! Everything GeoSPARQL calls a topological relation — all twenty-four of them
//! across the Simple Features, Egenhofer and RCC8 families — is a pattern read off
//! the matrix this file produces. There is one algorithm here and no per-relation
//! code anywhere, which is the point: twenty-four hand-written predicates would be
//! twenty-four independent chances to answer `false` for the wrong reason, and a
//! wrong `false` from a topological predicate is indistinguishable from a right
//! one.
//!
//! # Everything is planar
//!
//! OGC GeoSPARQL 1.1 Clause 10.2: *"Geometric functions working with Geometries
//! that have Z values will ignore Z values in calculations and first project
//! geometry onto the Z=0 level. … Like Z values in coordinates, M values are to be
//! ignored."* Nothing in this module tree reads `z` or `m`; every coordinate it
//! handles internally has already been projected by
//! `super::segment::plane`.
//!
//! # The algorithm, and why each step is complete
//!
//! An entry of the matrix is the *dimension* of the intersection of one of `a`'s
//! three point-sets with one of `b`'s. Computing those intersections as sets would
//! mean building an overlay — the hardest and least reliable thing in
//! computational geometry. Instead this file produces a finite set of **witnesses**
//! whose classification is provably enough to determine all nine entries, and
//! raises each entry to the largest dimension any witness supports. Because
//! [`IntersectionMatrix::raise`] is a maximum, the accumulation is independent of
//! the order witnesses are visited in, and so the answer is a pure function of the
//! two geometries.
//!
//! **Step 0 — empties.** No special case is needed, and that is a stronger
//! statement than a special case would be. An empty geometry puts *every* position
//! in its exterior, so every witness classified against it lands in the exterior
//! row or column; its own interior and boundary rows therefore stay `F` on their
//! own, and the non-empty operand's dimensions still reach the exterior column
//! through the ordinary node and fragment passes. The single unconditional
//! assignment is `E/E = 2`: the exteriors of two geometries in the plane always
//! share a two-dimensional region, including when both geometries are empty, in
//! which case the whole plane is that region. The both-empty case returns
//! immediately only because there is provably no work to do.
//!
//! **Step 1 — collect.** From each geometry: its segments (the edges of every
//! `LineString` and of every polygon ring). Isolated points need no separate
//! collection because they are already vertices, and every vertex becomes an event
//! point below.
//!
//! **Step 2 — node.** Intersect every segment with every other segment — not only
//! the A-against-B pairs, but the A-against-A and B-against-B ones too. The event
//! set is every vertex of A and of B, plus every intersection point — both
//! extremes of a collinear overlap, so that an overlap's ends are nodes like any
//! other crossing. Every segment is then split at every event point lying on it,
//! producing **fragments**.
//!
//! *Why self-intersections have to be nodes as well.* [`crate::geom`] deliberately
//! accepts geometries that are well-formed but not *valid* — a self-intersecting
//! ring, a multi-polygon whose members overlap — because a conforming store has to
//! carry them. Step 4's completeness argument rests on every face of the
//! arrangement having its extreme `y` values at event points, and a
//! self-intersection can be a face's extreme: in the bow-tie ring
//! `(0 0, 2 0, 0 2, 2 2, 0 0)` the even-odd interior is the pair of wedges above
//! and below the waist at `(1 1)`, and the lower wedge's maximum `y` is the waist
//! itself. Noding only across the two geometries would leave `y = 1` off the band
//! list, both wedges unsampled, and `I/E` reported as `F` when it is `2`. Noding
//! every pair costs the same asymptotic quadratic and closes the hole.
//!
//! **Step 3 — the 0- and 1-dimensional entries.** Each event point `p` is a
//! zero-dimensional witness for the cell `(locate(p, a), locate(p, b))`. Each
//! fragment is a one-dimensional witness for the cell its midpoint falls in.
//!
//! *Soundness of the fragment step*: a fragment's interior contains no event
//! point, and in particular no vertex of either geometry and no crossing with any
//! segment of either geometry. A location can only change where a boundary is
//! crossed, and every boundary of either geometry is a union of those segments. So
//! `locate(·, a)` and `locate(·, b)` are constant along the open fragment, and the
//! midpoint — which is in that open fragment, since the fragment has positive
//! length — reports the location of the whole of it.
//!
//! *Completeness of the fragment step*: any one-dimensional piece of an
//! intersection of two of these point-sets is a union of pieces of the geometries'
//! edges (an interior of a surface has no one-dimensional pieces of its own that
//! are not covered by a two-dimensional entry, and a curve is its edges). Every
//! edge is covered by the fragments it was split into, so every one-dimensional
//! entry has a fragment witnessing it.
//!
//! **Step 4 — the 2-dimensional entries.** `E/E` is always `2`. Of the remaining
//! eight cells only `I/I`, `I/E` and `E/I` can ever be `2`, because a boundary is
//! at most one-dimensional. They are decided by an exact **horizontal scan line**:
//!
//! * Let `ys` be the sorted, deduplicated `y` of every event point, plus a
//!   sentinel one below the minimum and one above the maximum.
//! * For each consecutive pair take `ym = (y_i + y_{i+1}) / 2` — exact, and
//!   strictly between, so **no vertex and no intersection point lies on the line
//!   `y = ym`**.
//! * Compute the exact abscissa at which each segment of A or B crosses that
//!   line. Sort and deduplicate them. Sample the midpoint of each consecutive
//!   pair, plus one abscissa below the smallest and one above the largest.
//! * Raise `(locate(s, a), locate(s, b))` to `2` — but only into `I/I`, `I/E` and
//!   `E/I`. A sample can only witness a two-dimensional intersection where both
//!   sides are open sets.
//!
//! *Why a sample is never on a boundary*: `ym` is strictly between two consecutive
//! event `y` values, so no vertex and no isolated point sits on the line. A
//! segment meets the line either not at all, or in exactly one abscissa (it cannot
//! be horizontal *at* `ym`, since a horizontal segment's `y` is an event `y`). The
//! sampled abscissae are strictly between consecutive *distinct* crossings, so
//! they avoid every one of them. A `debug_assert!` states this, because if the
//! construction ever failed the wrong entry would be raised silently.
//!
//! *Completeness of the scan*: consider a face of the arrangement of A's and B's
//! edges — a maximal connected open region on which both locations are constant.
//! Its extreme `y` values are attained at vertices or at edge crossings, all of
//! which are event points; therefore its open `y`-extent contains at least one
//! whole band `(y_i, y_{i+1})`, and in particular the line `y = ym` for that band.
//! The face's cross-section at that line is a non-empty open interval bounded by
//! two consecutive crossing abscissae (or unbounded, which the below-smallest and
//! above-largest samples cover), and the sampled midpoint of that interval is in
//! the face. So every face is sampled, and any face lying in `I/I`, `I/E` or `E/I`
//! raises its cell.
//!
//! The scan is skipped entirely when neither geometry has area, because then no
//! entry other than `E/E` can be `2`. It also stops as soon as every cell it could
//! still decide has reached `2`.

use super::locate::{has_area, horizontal_crossing, locate};
use super::segment::{SegmentIntersection, cmp_xy, half, intersect, midpoint, on_segment, plane};
use crate::de9im::{Dim, IntersectionMatrix, Pattern, Set};
use crate::exact::Rat;
use crate::geom::{Coord, CoordSeq, Geometry, GeometryBody};

/// One planar edge: an ordered pair of projected positions.
type Edge = (Coord, Coord);

/// The DE-9IM intersection matrix of the ordered pair `(a, b)`, computed exactly.
///
/// Total: every well-formed pair of geometries has a matrix, and this function
/// returns one for all of them. It has no failure channel and no refusal — an
/// unanswerable pair would have to be reported as such rather than as a wrong
/// matrix, and there is none.
///
/// Only `x` and `y` are read, per GeoSPARQL 1.1 Clause 10.2. See the module docs
/// for the algorithm and for why each of its four steps is complete.
pub fn relate(a: &Geometry, b: &Geometry) -> IntersectionMatrix {
    let mut matrix = IntersectionMatrix::new();
    // Step 0. Two exteriors in the plane always share a two-dimensional region.
    matrix.set(Set::Exterior, Set::Exterior, Dim::Two);
    if a.is_empty() && b.is_empty() {
        return matrix;
    }

    // Step 1. The two geometries' edges are kept in one list because every later
    // pass wants all of them: the noder pairs them all against one another, the
    // fragment pass walks all of them, and the scan line crosses all of them.
    let mut edges = Vec::new();
    collect_edges(a, &mut edges);
    collect_edges(b, &mut edges);

    // Step 2.
    let events = event_points(a, b, &edges);

    // Step 3, nodes.
    for point in &events {
        matrix.raise(locate(point, a), locate(point, b), Dim::Zero);
    }

    // Step 3, fragments.
    for edge in &edges {
        for (from, to) in fragments(edge, &events) {
            let mid = midpoint(&from, &to);
            matrix.raise(locate(&mid, a), locate(&mid, b), Dim::One);
        }
    }

    // Step 4.
    if has_area(a) || has_area(b) {
        scan_for_areas(a, b, &events, &edges, &mut matrix);
    }

    matrix
}

/// Whether the ordered pair `(a, b)` satisfies `pattern`.
///
/// A convenience over [`relate`]; it computes the whole matrix because there is
/// no cheaper way to decide a pattern that is honest about the cells the pattern
/// constrains.
pub fn relate_pattern(a: &Geometry, b: &Geometry, pattern: &Pattern) -> bool {
    relate(a, b).matches(pattern)
}

// ---------------------------------------------------------------------------
// Step 1 — collection
// ---------------------------------------------------------------------------

/// Append every planar edge of `geometry`: the edges of each curve and of each
/// polygon ring.
fn collect_edges(geometry: &Geometry, out: &mut Vec<Edge>) {
    match geometry.body() {
        GeometryBody::LineString(coords) => push_chain(coords, out),
        GeometryBody::Polygon(rings) => {
            for ring in rings {
                push_chain(ring, out);
            }
        }
        GeometryBody::MultiLineString(members) => {
            for member in members {
                push_chain(member, out);
            }
        }
        GeometryBody::MultiPolygon(members) => {
            for rings in members {
                for ring in rings {
                    push_chain(ring, out);
                }
            }
        }
        GeometryBody::GeometryCollection(members) => {
            for member in members {
                collect_edges(member, out);
            }
        }
        GeometryBody::Point(_) | GeometryBody::MultiPoint(_) => {}
    }
}

/// Append the consecutive edges of one coordinate chain, planar-projected.
fn push_chain(coords: &CoordSeq, out: &mut Vec<Edge>) {
    for pair in coords.windows(2) {
        out.push((plane(&pair[0]), plane(&pair[1])));
    }
}

// ---------------------------------------------------------------------------
// Step 2 — noding
// ---------------------------------------------------------------------------

/// The event set: every vertex of either geometry (which covers every isolated
/// point, since a point *is* a vertex) plus every intersection of any two of the
/// collected segments — including two segments of the *same* geometry, for the
/// reason the module docs give — sorted lexicographically and deduplicated.
fn event_points(a: &Geometry, b: &Geometry, edges: &[Edge]) -> Vec<Coord> {
    let mut events: Vec<Coord> = a.coords().chain(b.coords()).map(plane).collect();
    for (index, (a1, a2)) in edges.iter().enumerate() {
        for (b1, b2) in &edges[index + 1..] {
            match intersect(a1, a2, b1, b2) {
                SegmentIntersection::None => {}
                SegmentIntersection::Point(point) => events.push(point),
                SegmentIntersection::Collinear { from, to } => {
                    events.push(from);
                    events.push(to);
                }
            }
        }
    }
    events.sort_by(cmp_xy);
    events.dedup_by(|left, right| cmp_xy(left, right) == core::cmp::Ordering::Equal);
    events
}

/// Split `edge` at every event point lying on it, yielding its fragments in order
/// along the edge.
///
/// A zero-length edge has no fragments: it is a point, and the point is already an
/// event, so nothing about it is lost. Ordering is by the exact projection
/// parameter `(p - start) · (end - start)`, which is strictly monotone along the
/// edge and therefore a total order on the (already deduplicated) points on it.
fn fragments(edge: &Edge, events: &[Coord]) -> Vec<Edge> {
    let (start, end) = (&edge.0, &edge.1);
    if start.same_planar(end) {
        return Vec::new();
    }
    let dx = end.x().sub(start.x());
    let dy = end.y().sub(start.y());
    let mut along: Vec<(Rat, &Coord)> = events
        .iter()
        .filter(|point| on_segment(point, start, end))
        .map(|point| {
            let parameter = point
                .x()
                .sub(start.x())
                .mul(&dx)
                .add(&point.y().sub(start.y()).mul(&dy));
            (parameter, point)
        })
        .collect();
    along.sort_by(|left, right| left.0.cmp(&right.0));
    along
        .windows(2)
        .map(|pair| (pair[0].1.clone(), pair[1].1.clone()))
        .collect()
}

// ---------------------------------------------------------------------------
// Step 4 — the horizontal scan line
// ---------------------------------------------------------------------------

/// Which of the three possibly-two-dimensional cells this pair could still put a
/// `2` into.
///
/// `I/I` needs both operands to have area; `I/E` needs `a` to; `E/I` needs `b`
/// to. Knowing this up front is what makes the early exit sound: the scan may stop
/// as soon as every *reachable* cell has been raised.
#[derive(Clone, Copy, Debug)]
struct AreaTargets {
    /// Whether `I/I` could be two-dimensional.
    interior_interior: bool,
    /// Whether `I/E` could be two-dimensional.
    interior_exterior: bool,
    /// Whether `E/I` could be two-dimensional.
    exterior_interior: bool,
}

impl AreaTargets {
    /// The targets for this ordered pair.
    fn of(a: &Geometry, b: &Geometry) -> Self {
        let area_a = has_area(a);
        let area_b = has_area(b);
        Self {
            interior_interior: area_a && area_b,
            interior_exterior: area_a,
            exterior_interior: area_b,
        }
    }

    /// Whether every reachable target has already been raised to `2`.
    fn settled(self, matrix: &IntersectionMatrix) -> bool {
        let done =
            |wanted: bool, row: Set, column: Set| !wanted || matrix.get(row, column) == Dim::Two;
        done(self.interior_interior, Set::Interior, Set::Interior)
            && done(self.interior_exterior, Set::Interior, Set::Exterior)
            && done(self.exterior_interior, Set::Exterior, Set::Interior)
    }
}

/// Decide `I/I`, `I/E` and `E/I` with the exact horizontal scan line described in
/// the module docs.
fn scan_for_areas(
    a: &Geometry,
    b: &Geometry,
    events: &[Coord],
    edges: &[Edge],
    matrix: &mut IntersectionMatrix,
) {
    let targets = AreaTargets::of(a, b);
    let Some(bands) = band_boundaries(events) else {
        return;
    };
    let one = Rat::one();
    let scale = half();

    for pair in bands.windows(2) {
        if targets.settled(matrix) {
            return;
        }
        let height = pair[0].add(&pair[1]).mul(&scale);
        let crossings = sorted_crossings(edges, &height);
        let (Some(leftmost), Some(rightmost)) = (crossings.first(), crossings.last()) else {
            continue;
        };
        let mut abscissae = Vec::with_capacity(crossings.len() + 1);
        abscissae.push(leftmost.sub(&one));
        for window in crossings.windows(2) {
            abscissae.push(window[0].add(&window[1]).mul(&scale));
        }
        abscissae.push(rightmost.add(&one));
        for abscissa in abscissae {
            let sample = Coord::xy(abscissa, height.clone());
            let in_a = locate(&sample, a);
            let in_b = locate(&sample, b);
            debug_assert!(
                in_a != Set::Boundary && in_b != Set::Boundary,
                "a scan-line sample landed on a boundary, which the band and abscissa \
                 construction is supposed to make impossible; the noding is incomplete"
            );
            match (in_a, in_b) {
                (Set::Interior, Set::Interior) => {
                    matrix.raise(Set::Interior, Set::Interior, Dim::Two);
                }
                (Set::Interior, Set::Exterior) => {
                    matrix.raise(Set::Interior, Set::Exterior, Dim::Two);
                }
                (Set::Exterior, Set::Interior) => {
                    matrix.raise(Set::Exterior, Set::Interior, Dim::Two);
                }
                _ => {}
            }
        }
    }
}

/// The band boundaries: every event `y`, sorted and deduplicated, with a sentinel
/// one unit below the smallest and one above the largest so that the unbounded
/// regions above and below the geometries are sampled too.
fn band_boundaries(events: &[Coord]) -> Option<Vec<Rat>> {
    let mut ordinates: Vec<Rat> = events.iter().map(|point| point.y().clone()).collect();
    ordinates.sort();
    ordinates.dedup();
    let first = ordinates.first()?.sub(&Rat::one());
    let last = ordinates.last()?.add(&Rat::one());
    let mut bands = Vec::with_capacity(ordinates.len() + 2);
    bands.push(first);
    bands.append(&mut ordinates);
    bands.push(last);
    Some(bands)
}

/// The sorted, deduplicated abscissae at which any edge of either geometry
/// strictly crosses the horizontal line `y = height`.
///
/// Deduplication is what makes the sampled midpoints safe: two edges meeting the
/// line at the same abscissa would otherwise produce a "midpoint" equal to that
/// abscissa, which lies *on* both edges.
fn sorted_crossings(edges: &[Edge], height: &Rat) -> Vec<Rat> {
    let mut crossings: Vec<Rat> = edges
        .iter()
        .filter_map(|(from, to)| horizontal_crossing(from, to, height))
        .collect();
    crossings.sort();
    crossings.dedup();
    crossings
}

#[cfg(test)]
mod tests {
    use super::super::locate::{locate, topological_dimension};
    use super::{relate, relate_pattern};
    use crate::de9im::{Dim, IntersectionMatrix, Pattern, Set};
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, Rings};
    use crate::relations::{SpatialRelation, transpose};

    // ---- builders --------------------------------------------------------

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

    /// The axis-aligned box `[x0, x1] × [y0, y1]`, written counter-clockwise.
    fn boxed(x0: i64, y0: i64, x1: i64, y1: i64) -> Geometry {
        polygon(&[&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]])
    }

    fn multi_polygon(shapes: &[&[&[(i64, i64)]]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPolygon(shapes.iter().map(|shape| rings(shape)).collect()),
        )
        .expect("well-formed closed rings")
    }

    fn empty(kind: GeometryKind) -> Geometry {
        Geometry::empty(CoordDim::Xy, kind)
    }

    /// Assert that `relate(a, b)` renders exactly as `expected`.
    fn assert_matrix(label: &str, a: &Geometry, b: &Geometry, expected: &str) {
        let actual = relate(a, b).to_string();
        assert_eq!(
            actual, expected,
            "{label}: expected the DE-9IM matrix {expected}, computed {actual}"
        );
    }

    // ================================================================
    // 1. The golden table of hand-computed matrices
    // ================================================================

    /// A point against a polygon, in all three positions relative to it.
    #[test]
    fn the_golden_matrices_for_a_point_against_a_polygon_hold() {
        let square = boxed(0, 0, 4, 4);
        assert_matrix(
            "point strictly inside a polygon",
            &point(2, 2),
            &square,
            "0FFFFF212",
        );
        assert_matrix(
            "point on a polygon's boundary",
            &point(0, 2),
            &square,
            "F0FFFF212",
        );
        assert_matrix(
            "point on a polygon's corner",
            &point(0, 0),
            &square,
            "F0FFFF212",
        );
        assert_matrix(
            "point outside a polygon",
            &point(9, 9),
            &square,
            "FF0FFF212",
        );
    }

    /// The five polygon-against-polygon configurations, plus the two orders of
    /// containment.
    #[test]
    fn the_golden_matrices_for_two_polygons_hold() {
        let unit = boxed(0, 0, 2, 2);
        assert_matrix("two identical polygons", &unit, &unit, "2FFF1FFF2");

        // The prompt's `212FF1FF2` is the CONTAINS orientation: it has `I/B = 1`
        // and `E/I = F`, which can only be the outer polygon in the row position.
        // Both orders are asserted so the labelling cannot be ambiguous.
        let outer = boxed(0, 0, 6, 6);
        let inner = boxed(2, 2, 4, 4);
        assert_matrix(
            "the containing polygon against the one strictly inside it",
            &outer,
            &inner,
            "212FF1FF2",
        );
        assert_matrix(
            "the strictly-inside polygon against its container",
            &inner,
            &outer,
            "2FF1FF212",
        );

        assert_matrix(
            "two polygons sharing exactly one boundary point",
            &boxed(0, 0, 1, 1),
            &boxed(1, 1, 2, 2),
            "FF2F01212",
        );
        assert_matrix(
            "two polygons sharing a boundary edge, interiors disjoint",
            &boxed(0, 0, 1, 1),
            &boxed(1, 0, 2, 1),
            "FF2F11212",
        );
        assert_matrix(
            "two polygons properly overlapping",
            &boxed(0, 0, 2, 2),
            &boxed(1, 1, 3, 3),
            "212101212",
        );
        assert_matrix(
            "two disjoint polygons",
            &boxed(0, 0, 1, 1),
            &boxed(5, 5, 6, 6),
            "FF2FF1212",
        );
    }

    /// Two points, equal and distinct. A point has no boundary, so the whole
    /// middle row and column are `F` except where the exteriors meet.
    #[test]
    fn the_golden_matrices_for_two_points_hold() {
        assert_matrix(
            "two identical points",
            &point(1, 1),
            &point(1, 1),
            "0FFFFFFF2",
        );
        assert_matrix(
            "two distinct points",
            &point(1, 1),
            &point(2, 2),
            "FF0FFF0F2",
        );
    }

    /// Lines against lines, and lines against a polygon.
    ///
    /// The crossing-line-against-polygon entry is derived rather than quoted:
    /// the line's interior meets the polygon's interior in a segment (`1`), its
    /// boundary in the two crossing points (`0`) and its exterior in the two
    /// outside stubs (`1`); the line's two endpoints are outside the polygon, so
    /// `B/I` and `B/B` are `F` and `B/E` is `0`; the polygon's interior and
    /// boundary minus a one-dimensional set are still `2` and `1`. That is
    /// `101FF0212`.
    #[test]
    fn the_golden_matrices_for_lines_hold() {
        let square = boxed(0, 0, 2, 2);
        assert_matrix(
            "a line crossing a polygon",
            &line(&[(-1, 1), (3, 1)]),
            &square,
            "101FF0212",
        );
        let big = boxed(0, 0, 4, 4);
        assert_matrix(
            "a line inside a polygon's interior",
            &line(&[(1, 2), (3, 2)]),
            &big,
            "1FF0FF212",
        );
        assert_matrix(
            "two lines crossing at a point",
            &line(&[(0, 0), (2, 2)]),
            &line(&[(0, 2), (2, 0)]),
            "0F1FF0102",
        );
        assert_matrix(
            "two identical lines",
            &line(&[(0, 0), (2, 2)]),
            &line(&[(0, 0), (2, 2)]),
            "1FFF0FFF2",
        );
        assert_matrix(
            "two disjoint lines",
            &line(&[(0, 0), (1, 0)]),
            &line(&[(0, 1), (1, 1)]),
            "FF1FF0102",
        );
    }

    /// A line whose endpoint touches a polygon's boundary without entering, and
    /// one that runs along an edge — the tangential cases where a crossing test
    /// alone would be wrong.
    #[test]
    fn the_golden_matrices_for_tangential_lines_hold() {
        let square = boxed(0, 0, 4, 4);
        assert_matrix(
            "a line touching a polygon's boundary at its own endpoint, from outside",
            &line(&[(-2, 2), (0, 2)]),
            &square,
            "FF1F00212",
        );
        assert_matrix(
            "a line lying along a polygon's edge",
            &line(&[(0, 1), (0, 3)]),
            &square,
            "F1FF0F212",
        );
    }

    // ================================================================
    // The corpus every law below is quantified over
    // ================================================================

    /// A wide corpus of ordered pairs: every kind, every configuration used in
    /// the golden table, and every degenerate case this file has to survive.
    fn corpus() -> Vec<(&'static str, Geometry, Geometry)> {
        let square = boxed(0, 0, 4, 4);
        let holed = polygon(&[
            &[(0, 0), (6, 0), (6, 6), (0, 6), (0, 0)],
            &[(2, 2), (4, 2), (4, 4), (2, 4), (2, 2)],
        ]);
        vec![
            ("point inside polygon", point(2, 2), square.clone()),
            ("point on polygon edge", point(0, 2), square.clone()),
            ("point on polygon corner", point(4, 4), square.clone()),
            ("point outside polygon", point(9, 9), square.clone()),
            ("polygon against point inside", square.clone(), point(2, 2)),
            ("identical points", point(1, 1), point(1, 1)),
            ("distinct points", point(1, 1), point(3, 3)),
            (
                "point on line interior",
                point(1, 0),
                line(&[(0, 0), (2, 0)]),
            ),
            (
                "point on line endpoint",
                point(0, 0),
                line(&[(0, 0), (2, 0)]),
            ),
            ("point off line", point(1, 1), line(&[(0, 0), (2, 0)])),
            ("identical polygons", square.clone(), square.clone()),
            (
                "polygon contains polygon",
                boxed(0, 0, 6, 6),
                boxed(2, 2, 4, 4),
            ),
            (
                "polygon inside polygon",
                boxed(2, 2, 4, 4),
                boxed(0, 0, 6, 6),
            ),
            (
                "polygons touching at a corner",
                boxed(0, 0, 1, 1),
                boxed(1, 1, 2, 2),
            ),
            (
                "polygons sharing an edge",
                boxed(0, 0, 1, 1),
                boxed(1, 0, 2, 1),
            ),
            ("polygons overlapping", boxed(0, 0, 2, 2), boxed(1, 1, 3, 3)),
            ("polygons disjoint", boxed(0, 0, 1, 1), boxed(5, 5, 6, 6)),
            (
                "line crossing polygon",
                line(&[(-1, 2), (5, 2)]),
                square.clone(),
            ),
            (
                "line inside polygon",
                line(&[(1, 2), (3, 2)]),
                square.clone(),
            ),
            (
                "line along a polygon edge",
                line(&[(0, 1), (0, 3)]),
                square.clone(),
            ),
            (
                "line touching a polygon boundary from outside",
                line(&[(-2, 2), (0, 2)]),
                square.clone(),
            ),
            (
                "lines crossing",
                line(&[(0, 0), (2, 2)]),
                line(&[(0, 2), (2, 0)]),
            ),
            (
                "identical lines",
                line(&[(0, 0), (2, 2)]),
                line(&[(0, 0), (2, 2)]),
            ),
            (
                "disjoint lines",
                line(&[(0, 0), (1, 0)]),
                line(&[(0, 1), (1, 1)]),
            ),
            (
                "collinear overlapping lines",
                line(&[(0, 0), (3, 0)]),
                line(&[(1, 0), (5, 0)]),
            ),
            (
                "lines meeting end to end",
                line(&[(0, 0), (1, 0)]),
                line(&[(1, 0), (2, 0)]),
            ),
            (
                "closed line against polygon",
                line(&[(0, 0), (4, 0), (4, 4), (0, 0)]),
                square.clone(),
            ),
            (
                "polygon with a hole against a point in the hole",
                holed.clone(),
                point(3, 3),
            ),
            (
                "polygon with a hole against a point on the hole boundary",
                holed.clone(),
                point(3, 2),
            ),
            (
                "polygon with a hole against itself",
                holed.clone(),
                holed.clone(),
            ),
            (
                "polygon with a hole against a polygon filling the hole",
                holed,
                boxed(2, 2, 4, 4),
            ),
            (
                "multipolygon against a polygon straddling both members",
                multi_polygon(&[
                    &[&[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]],
                    &[&[(2, 0), (3, 0), (3, 1), (2, 1), (2, 0)]],
                ]),
                boxed(0, 0, 3, 1),
            ),
            (
                "multipoint against a polygon",
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::MultiPoint(vec![Some(c(2, 2)), Some(c(9, 9)), None]),
                )
                .expect("well formed"),
                square.clone(),
            ),
            (
                "multilinestring joined end to end against a polygon",
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::MultiLineString(vec![
                        seq(&[(-1, 2), (2, 2)]),
                        seq(&[(2, 2), (5, 2)]),
                    ]),
                )
                .expect("well formed"),
                square.clone(),
            ),
            (
                "geometry collection against a polygon",
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::GeometryCollection(vec![point(2, 2), line(&[(-1, 1), (5, 1)])]),
                )
                .expect("uniform dimension"),
                square.clone(),
            ),
            (
                "zero-length line against a polygon",
                line(&[(2, 2), (2, 2)]),
                square.clone(),
            ),
            (
                "zero-length line against a zero-length line",
                line(&[(2, 2), (2, 2)]),
                line(&[(2, 2), (2, 2)]),
            ),
            (
                "empty point against a polygon",
                empty(GeometryKind::Point),
                square.clone(),
            ),
            (
                "polygon against an empty polygon",
                square,
                empty(GeometryKind::Polygon),
            ),
            (
                "empty against empty",
                empty(GeometryKind::LineString),
                empty(GeometryKind::Point),
            ),
            (
                "empty collection against a line",
                empty(GeometryKind::GeometryCollection),
                line(&[(0, 0), (1, 1)]),
            ),
            (
                "point against an empty line",
                point(0, 0),
                empty(GeometryKind::LineString),
            ),
            (
                "closed line against itself",
                line(&[(0, 0), (2, 0), (2, 2), (0, 0)]),
                line(&[(0, 0), (2, 0), (2, 2), (0, 0)]),
            ),
            (
                "bowtie polygon against a point at its waist",
                polygon(&[&[(0, 0), (2, 2), (2, 0), (0, 2), (0, 0)]]),
                point(1, 1),
            ),
        ]
    }

    // ================================================================
    // 2. The symmetry law
    // ================================================================

    /// Transposing `relate(a, b)` must give `relate(b, a)` exactly. This is an
    /// independent check on the whole algorithm: nothing in the implementation
    /// computes the reverse pair, so an asymmetric bug in noding, in fragment
    /// classification or in the scan line would show up here.
    #[test]
    fn relate_is_symmetric_under_transposition_across_the_corpus() {
        for (label, a, b) in corpus() {
            let forward = relate(&a, &b);
            let backward = relate(&b, &a);
            assert_eq!(
                transpose(&forward),
                backward,
                "{label}: transpose of {forward} should equal {backward}"
            );
            assert_eq!(
                transpose(&backward),
                forward,
                "{label}: transposition must be an involution here too"
            );
        }
    }

    // ================================================================
    // 3. The GeoSPARQL relation patterns
    // ================================================================

    /// `sfWithin(a, b)` and `sfContains(b, a)` are the same statement read from
    /// the two orders of the same matrix, so they must agree everywhere.
    #[test]
    fn within_and_contains_are_converses_across_the_corpus() {
        for (label, a, b) in corpus() {
            let forward = relate(&a, &b);
            let backward = relate(&b, &a);
            let (dim_a, dim_b) = (topological_dimension(&a), topological_dimension(&b));
            assert_eq!(
                SpatialRelation::SfWithin.holds(&forward, dim_a, dim_b),
                SpatialRelation::SfContains.holds(&backward, dim_b, dim_a),
                "{label}: sfWithin(a, b) must equal sfContains(b, a)"
            );
            assert_eq!(
                SpatialRelation::SfContains.holds(&forward, dim_a, dim_b),
                SpatialRelation::SfWithin.holds(&backward, dim_b, dim_a),
                "{label}: sfContains(a, b) must equal sfWithin(b, a)"
            );
        }
    }

    /// `sfDisjoint` is the exact negation of `sfIntersects`, and the two
    /// symmetric relations really are symmetric.
    #[test]
    fn disjoint_is_the_negation_of_intersects_and_the_symmetric_relations_are_symmetric() {
        for (label, a, b) in corpus() {
            let forward = relate(&a, &b);
            let backward = relate(&b, &a);
            let (dim_a, dim_b) = (topological_dimension(&a), topological_dimension(&b));
            assert_eq!(
                SpatialRelation::SfDisjoint.holds(&forward, dim_a, dim_b),
                !SpatialRelation::SfIntersects.holds(&forward, dim_a, dim_b),
                "{label}: disjoint and intersects partition every pair"
            );
            for relation in [
                SpatialRelation::SfEquals,
                SpatialRelation::SfDisjoint,
                SpatialRelation::SfIntersects,
                SpatialRelation::SfTouches,
                SpatialRelation::SfOverlaps,
            ] {
                assert_eq!(
                    relation.holds(&forward, dim_a, dim_b),
                    relation.holds(&backward, dim_b, dim_a),
                    "{label}: {relation:?} is a symmetric relation"
                );
            }
        }
    }

    /// Equality is reflexive: relating a non-empty geometry to itself gives a
    /// matrix in which nothing of either operand escapes the other.
    ///
    /// The pattern asserted is OGC Simple Features' `Equals`, `T*F**FFF*` — the
    /// interiors meet, and neither interior nor boundary reaches the other's
    /// exterior. GeoSPARQL Table 2 renders `sfEquals` as `TFFFTFFFT` instead,
    /// which additionally demands a **non-empty boundary/boundary** entry; that
    /// rendering is therefore not reflexive on any geometry whose boundary is
    /// empty — every point, every multi-point, every closed curve. The last
    /// assertion below executes that difference on a point rather than leaving it
    /// as a claim, because it is a property of the published pattern and not of
    /// the matrix computed here.
    ///
    /// Empty geometries are excluded: every pattern in the family demands a
    /// non-empty interior/interior entry and an empty geometry has no interior.
    #[test]
    fn equality_is_reflexive_on_every_non_empty_corpus_member() {
        let equals = Pattern::new("T*F**FFF*");
        let mut checked = 0_usize;
        for (label, a, b) in corpus() {
            for geometry in [&a, &b] {
                if geometry.is_empty() {
                    continue;
                }
                let matrix = relate(geometry, geometry);
                let dim = topological_dimension(geometry);
                assert!(
                    matrix.matches(&equals),
                    "{label}: every non-empty geometry equals itself, but got {matrix}"
                );
                assert!(
                    !SpatialRelation::SfDisjoint.holds(&matrix, dim, dim),
                    "{label}: a non-empty geometry is not disjoint from itself"
                );
                assert!(
                    SpatialRelation::SfIntersects.holds(&matrix, dim, dim),
                    "{label}: a non-empty geometry intersects itself"
                );
                checked += 1;
            }
        }
        assert!(checked > 40, "the reflexivity claim must not be vacuous");

        // The boundary-less kinds, executed rather than left as prose: a point
        // and a closed curve both relate to themselves with an EMPTY
        // boundary/boundary entry, which is why the literal Table 2 rendering
        // `TFFFTFFFT` cannot be reflexive and why `crate::relations` does not use
        // it. The neighbouring open curve, which does have a boundary, is
        // asserted beside them so the point is about the pattern rather than
        // about self-relation being broken.
        let dot = relate(&point(1, 1), &point(1, 1));
        assert_eq!(dot.to_string(), "0FFFFFFF2", "two identical points");
        assert_eq!(
            dot.get(Set::Boundary, Set::Boundary),
            Dim::Empty,
            "a point has no boundary to share with itself"
        );
        let ring = line(&[(0, 0), (2, 0), (2, 2), (0, 0)]);
        assert_eq!(
            relate(&ring, &ring).get(Set::Boundary, Set::Boundary),
            Dim::Empty,
            "nor has a closed curve"
        );
        let open = line(&[(0, 0), (1, 1)]);
        assert_eq!(
            relate(&open, &open).get(Set::Boundary, Set::Boundary),
            Dim::Zero,
            "but an open curve shares its two endpoints with itself"
        );
        for geometry in [&point(1, 1), &ring, &open] {
            let dim = topological_dimension(geometry);
            assert!(
                SpatialRelation::SfEquals.holds(&relate(geometry, geometry), dim, dim),
                "sfEquals as this crate renders it stays reflexive on all three"
            );
        }
    }

    // ================================================================
    // 4. The independent sampling oracle
    // ================================================================

    /// Classify a lattice of exact rational positions and derive, for each of the
    /// nine cells, whether *some* position witnesses it as non-empty.
    ///
    /// This is a **one-directional** oracle and deliberately so. A finite set of
    /// sample points can witness that two sets meet; it can never witness that
    /// they do not, and it can never establish that a meeting is one- or
    /// two-dimensional rather than merely non-empty. So the only thing it is
    /// entitled to assert is `matrix[cell] != F` wherever a sample landed in that
    /// cell — a lower bound. That bound is still worth having, because the failure
    /// mode it catches is precisely the one the exact algorithm is most likely to
    /// have: a witness that the noder never generated, leaving a cell at `F` when
    /// the two sets really do meet. A sampling oracle finds that immediately and
    /// finds it without sharing any code with the noder.
    fn sampled_lower_bound(a: &Geometry, b: &Geometry) -> [[bool; 3]; 3] {
        let mut witnessed = [[false; 3]; 3];
        let Some((min_x, min_y, max_x, max_y)) = joint_box(a, b) else {
            return witnessed;
        };
        // A quarter-unit lattice: fine enough to land exactly on the integer
        // vertices and edge midpoints every corpus geometry is built from, so
        // boundary cells really do get witnessed.
        let step = q(1, 4);
        let mut y = min_y;
        while y <= max_y {
            let mut x = min_x.clone();
            while x <= max_x {
                let sample = Coord::xy(x.clone(), y.clone());
                let row = locate(&sample, a) as usize;
                let column = locate(&sample, b) as usize;
                witnessed[row][column] = true;
                x = x.add(&step);
            }
            y = y.add(&step);
        }
        witnessed
    }

    /// The joint bounding box of the two geometries, widened by one unit so that
    /// the exterior of both is sampled, or `None` when both are empty.
    fn joint_box(a: &Geometry, b: &Geometry) -> Option<(Rat, Rat, Rat, Rat)> {
        let mut coords = a.coords().chain(b.coords());
        let first = coords.next()?;
        let mut min_x = first.x().clone();
        let mut min_y = first.y().clone();
        let mut max_x = min_x.clone();
        let mut max_y = min_y.clone();
        for coord in coords {
            if *coord.x() < min_x {
                min_x = coord.x().clone();
            }
            if *coord.x() > max_x {
                max_x = coord.x().clone();
            }
            if *coord.y() < min_y {
                min_y = coord.y().clone();
            }
            if *coord.y() > max_y {
                max_y = coord.y().clone();
            }
        }
        let one = Rat::one();
        Some((
            min_x.sub(&one),
            min_y.sub(&one),
            max_x.add(&one),
            max_y.add(&one),
        ))
    }

    /// The exact matrix must dominate the sampled lower bound at every cell.
    #[test]
    fn the_exact_matrix_dominates_the_sampling_oracle_across_the_corpus() {
        let mut witnessed_cells = 0_usize;
        for (label, a, b) in corpus() {
            let matrix = relate(&a, &b);
            let bound = sampled_lower_bound(&a, &b);
            for (row_index, row) in Set::ALL.into_iter().enumerate() {
                for (column_index, column) in Set::ALL.into_iter().enumerate() {
                    if !bound[row_index][column_index] {
                        continue;
                    }
                    witnessed_cells += 1;
                    assert_ne!(
                        matrix.get(row, column),
                        Dim::Empty,
                        "{label}: a sample position lies in {row:?}(a) and {column:?}(b), so the \
                         matrix {matrix} may not report that cell empty"
                    );
                }
            }
        }
        assert!(
            witnessed_cells > 100,
            "the oracle must actually witness cells; it only witnessed {witnessed_cells}"
        );
    }

    // ================================================================
    // 5. Determinism
    // ================================================================

    /// The same pair gives the same matrix, coordinates spelled three different
    /// ways give the same matrix, and reordering a multi-polygon's members gives
    /// the same matrix.
    #[test]
    fn relate_is_a_pure_function_of_the_two_geometries() {
        let a = boxed(0, 0, 2, 2);
        let b = boxed(1, 1, 3, 3);
        let once = relate(&a, &b);
        for _ in 0..5 {
            assert_eq!(
                relate(&a, &b),
                once,
                "repeated calls on the same pair must agree"
            );
        }

        // `1.5`, `15e-1` and the rational 3/2 are the same exact number, so the
        // three spellings must produce byte-identical geometry and matrix.
        let decimal = Rat::parse_decimal("1.5").expect("a decimal literal");
        let exponent = Rat::parse_decimal("15e-1").expect("an exponent literal");
        let ratio = q(3, 2);
        assert_eq!(decimal, exponent, "1.5 and 15e-1 are the same number");
        assert_eq!(decimal, ratio, "and both are three halves");
        let spellings = [decimal, exponent, ratio];
        let mut matrices = Vec::new();
        for spelling in spellings {
            let probe = Geometry::new(
                CoordDim::Xy,
                GeometryBody::Point(Some(Coord::xy(spelling.clone(), spelling))),
            )
            .expect("a well-formed point");
            matrices.push(relate(&probe, &a).to_string());
        }
        assert_eq!(
            matrices,
            vec![
                "0FFFFF212".to_owned(),
                "0FFFFF212".to_owned(),
                "0FFFFF212".to_owned()
            ],
            "three spellings of the same coordinate must give the same matrix"
        );

        let forward = multi_polygon(&[
            &[&[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]],
            &[&[(2, 0), (3, 0), (3, 1), (2, 1), (2, 0)]],
        ]);
        let reversed = multi_polygon(&[
            &[&[(2, 0), (3, 0), (3, 1), (2, 1), (2, 0)]],
            &[&[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]],
        ]);
        let probe = boxed(0, 0, 3, 1);
        assert_eq!(
            relate(&forward, &probe),
            relate(&reversed, &probe),
            "member order is not part of a multi-polygon's meaning"
        );
        assert_eq!(
            relate(&probe, &forward),
            relate(&probe, &reversed),
            "nor in the reversed pair"
        );
    }

    /// Elevation and measure never reach the matrix: the same footprint with wild
    /// `z` and `m` values relates identically. This is Clause 10.2 executed.
    #[test]
    fn elevation_and_measure_never_reach_the_matrix() {
        let flat = boxed(0, 0, 4, 4);
        let raised = Geometry::new(
            CoordDim::Xyzm,
            GeometryBody::Polygon(vec![
                [
                    Coord::new(r(0), r(0), Some(r(50)), Some(r(-50))),
                    Coord::new(r(4), r(0), Some(r(-7)), Some(r(7))),
                    Coord::new(r(4), r(4), Some(r(900)), Some(r(1))),
                    Coord::new(r(0), r(4), Some(r(0)), Some(r(0))),
                    Coord::new(r(0), r(0), Some(r(3)), Some(r(3))),
                ]
                .into_iter()
                .collect(),
            ]),
        )
        .expect("a ring that closes in the plane");
        let probe = point(2, 2);
        assert_eq!(
            relate(&probe, &flat),
            relate(&probe, &raised),
            "z and m are projected away before anything is decided"
        );
        assert_eq!(
            relate(&flat, &flat),
            relate(&raised, &raised),
            "and the projection is consistent on both sides"
        );
    }

    // ================================================================
    // 6. Degenerate inputs
    // ================================================================

    /// Empty geometries in every position produce the specification's matrices:
    /// the empty side's interior and boundary rows or columns are entirely `F`,
    /// and the non-empty side's own dimensions appear against the empty side's
    /// exterior.
    #[test]
    fn empty_geometries_are_answered_in_every_position() {
        let nothing = empty(GeometryKind::Point);
        let nothing_else = empty(GeometryKind::GeometryCollection);
        assert_matrix("both empty", &nothing, &nothing_else, "FFFFFFFF2");

        assert_matrix("empty against a point", &nothing, &point(1, 1), "FFFFFF0F2");
        assert_matrix("point against empty", &point(1, 1), &nothing, "FF0FFFFF2");
        assert_matrix(
            "empty against an open line",
            &nothing,
            &line(&[(0, 0), (1, 1)]),
            "FFFFFF102",
        );
        assert_matrix(
            "open line against empty",
            &line(&[(0, 0), (1, 1)]),
            &nothing,
            "FF1FF0FF2",
        );
        assert_matrix(
            "empty against a closed line, which has no boundary",
            &nothing,
            &line(&[(0, 0), (1, 0), (1, 1), (0, 0)]),
            "FFFFFF1F2",
        );
        assert_matrix(
            "empty against a polygon",
            &nothing,
            &boxed(0, 0, 2, 2),
            "FFFFFF212",
        );
        assert_matrix(
            "polygon against empty",
            &boxed(0, 0, 2, 2),
            &nothing,
            "FF2FF1FF2",
        );
    }

    /// A zero-length line segment is a degenerate curve; it must be located and
    /// related without dividing by zero and without vanishing.
    #[test]
    fn a_zero_length_line_is_related_rather_than_dropped() {
        let dot = line(&[(2, 2), (2, 2)]);
        assert_matrix(
            "a zero-length line inside a polygon",
            &dot,
            &boxed(0, 0, 4, 4),
            "0FFFFF212",
        );
        assert_matrix(
            "a zero-length line against a coincident point",
            &dot,
            &point(2, 2),
            "0FFFFFFF2",
        );
        assert_matrix(
            "a zero-length line against a distant point",
            &dot,
            &point(9, 9),
            "FF0FFF0F2",
        );
    }

    /// A polygon with a hole: a point in the hole, on the hole's boundary, and in
    /// the material between shell and hole.
    #[test]
    fn a_polygon_with_a_hole_is_related_correctly_in_all_three_regions() {
        let holed = polygon(&[
            &[(0, 0), (6, 0), (6, 6), (0, 6), (0, 0)],
            &[(2, 2), (4, 2), (4, 4), (2, 4), (2, 2)],
        ]);
        assert_matrix(
            "a point inside the hole is outside the polygon",
            &point(3, 3),
            &holed,
            "FF0FFF212",
        );
        assert_matrix(
            "a point exactly on the hole's boundary",
            &point(3, 2),
            &holed,
            "F0FFFF212",
        );
        assert_matrix(
            "a point on the hole's corner",
            &point(2, 2),
            &holed,
            "F0FFFF212",
        );
        assert_matrix(
            "a point in the material between shell and hole",
            &point(1, 3),
            &holed,
            "0FFFFF212",
        );
        // `E/B` is `F` here, not `1`: the filling polygon's whole boundary IS the
        // hole ring, which belongs to the holed polygon's boundary rather than to
        // its exterior. `E/I` is `2` because the holed polygon's exterior
        // contains the open hole, which is exactly the filler's interior.
        assert_matrix(
            "a polygon exactly filling the hole touches only the boundary",
            &holed,
            &boxed(2, 2, 4, 4),
            "FF2F112F2",
        );
    }

    /// Collinear overlapping lines share a one-dimensional interior, which is a
    /// case a crossing-only noder would miss entirely.
    #[test]
    fn collinear_overlapping_lines_share_a_one_dimensional_interior() {
        assert_matrix(
            "two collinear lines overlapping in a sub-segment",
            &line(&[(0, 0), (3, 0)]),
            &line(&[(1, 0), (5, 0)]),
            "1010F0102",
        );
        // Derived, not quoted: the two curves share the single position (1 0),
        // which is a boundary point of each, so `B/B` is `0`. `B/E` is `0` too —
        // `a`'s other endpoint (0 0) is in `b`'s exterior — and symmetrically for
        // `E/B`.
        assert_matrix(
            "two collinear lines meeting at exactly one endpoint",
            &line(&[(0, 0), (1, 0)]),
            &line(&[(1, 0), (2, 0)]),
            "FF1F00102",
        );
    }

    /// The self-noding claim from the module docs, executed.
    ///
    /// The bow-tie ring `(0 0, 2 0, 0 2, 2 2, 0 0)` is well-formed but not valid,
    /// and its even-odd interior is the pair of wedges above and below the waist
    /// at `(1 1)`. The waist is a self-intersection, not a vertex, so its `y` only
    /// reaches the scan line's band list because the noder intersects a
    /// geometry's segments against its own. If it did not, the only band covering
    /// the wedges would be `(0, 2)`, whose mid-line `y = 1` passes exactly through
    /// the waist and samples only the two exterior side wedges — and `I/E` would
    /// come back `F` for a polygon that plainly has area.
    #[test]
    fn a_self_intersecting_rings_interior_is_still_found_by_the_scan_line() {
        let bowtie = polygon(&[&[(0, 0), (2, 0), (0, 2), (2, 2), (0, 0)]]);
        assert_eq!(
            locate(&c(1, 1), &bowtie),
            Set::Boundary,
            "the waist is on the ring"
        );
        assert_eq!(
            locate(&Coord::xy(r(1), q(19, 10)), &bowtie),
            Set::Interior,
            "the upper wedge is interior under the even-odd rule"
        );
        assert_eq!(
            locate(&Coord::xy(r(1), q(1, 10)), &bowtie),
            Set::Interior,
            "and so is the lower wedge, whose maximum y IS the waist"
        );
        assert_eq!(
            locate(&Coord::xy(q(1, 10), r(1)), &bowtie),
            Set::Exterior,
            "while the left side wedge is exterior"
        );
        assert_matrix(
            "a bow-tie ring against a distant polygon",
            &bowtie,
            &boxed(10, 10, 11, 11),
            "FF2FF1212",
        );
        let matrix = relate(&bowtie, &boxed(10, 10, 11, 11));
        assert_eq!(
            matrix.get(Set::Interior, Set::Exterior),
            Dim::Two,
            "the bow-tie's interior is two-dimensional and must be witnessed"
        );
    }

    /// A line that touches a polygon's boundary tangentially without entering it.
    #[test]
    fn a_line_touching_a_polygon_boundary_without_entering_is_not_inside_it() {
        let square = boxed(0, 0, 4, 4);
        // The segment (5 3)-(3 5) passes through the corner (4 4) and is outside
        // the square on both sides of it, so the corner is met by the line's
        // INTERIOR: `I/B` is `0`, `I/I` is `F`, and both line endpoints are
        // exterior so `B/E` is `0`.
        let grazing = line(&[(5, 3), (3, 5)]);
        assert_matrix(
            "a line that grazes a corner from outside",
            &grazing,
            &square,
            "F01FF0212",
        );
        let matrix = relate(&grazing, &square);
        assert!(
            !SpatialRelation::SfWithin.holds(&matrix, 1, 2),
            "grazing a corner is not being within the polygon"
        );
        assert!(
            SpatialRelation::SfTouches.holds(&matrix, 1, 2),
            "but it is touching it"
        );
    }

    // ================================================================
    // 7. The over-refusal control
    // ================================================================

    /// `relate` answers every well-formed pair. There is no failure channel and
    /// no refusal: the signature returns a matrix, and the whole corpus —
    /// including every degenerate and empty case — produces one whose rendering is
    /// nine legal characters and whose `E/E` entry is always `2`.
    ///
    /// This is the over-refusal control. The mirror of a silently dropped witness
    /// is a silently refused input, and a refusal here would look exactly like
    /// correct strictness until a user asked a question that should have been
    /// answerable.
    #[test]
    fn every_well_formed_pair_is_answered_and_none_is_refused() {
        let mut answered = 0_usize;
        for (label, a, b) in corpus() {
            for (left, right) in [(&a, &b), (&b, &a), (&a, &a), (&b, &b)] {
                let matrix = relate(left, right);
                let rendered = matrix.to_string();
                assert_eq!(rendered.len(), 9, "{label}: a matrix has nine entries");
                assert!(
                    rendered
                        .chars()
                        .all(|ch| matches!(ch, 'F' | '0' | '1' | '2')),
                    "{label}: a matrix is written with F, 0, 1 and 2 only, got {rendered}"
                );
                assert_eq!(
                    matrix.get(Set::Exterior, Set::Exterior),
                    Dim::Two,
                    "{label}: two exteriors in the plane always share an area"
                );
                assert_eq!(
                    IntersectionMatrix::parse(&rendered).expect("a legal rendering"),
                    matrix,
                    "{label}: the rendering must round-trip"
                );
                answered += 1;
            }
        }
        assert!(
            answered > 120,
            "the corpus must actually be exercised; only {answered} pairs were related"
        );
    }

    /// `relate_pattern` is exactly `relate` followed by a pattern match, and it
    /// accepts the neighbouring pattern it should while rejecting the one it
    /// should not.
    #[test]
    fn relate_pattern_agrees_with_matching_the_computed_matrix() {
        let inside = point(2, 2);
        let square = boxed(0, 0, 4, 4);
        let within = Pattern::new("T*F**F***");
        let contains = Pattern::new("T*****FF*");
        assert!(
            relate_pattern(&inside, &square, &within),
            "a point inside a polygon is within it"
        );
        assert!(
            !relate_pattern(&inside, &square, &contains),
            "and it does not also contain it"
        );
        assert!(
            relate_pattern(&square, &inside, &contains),
            "the reversed pair is the containment, so the pattern is not simply unsatisfiable"
        );
        for (_, a, b) in corpus() {
            let matrix = relate(&a, &b);
            assert_eq!(
                relate_pattern(&a, &b, &within),
                matrix.matches(&within),
                "relate_pattern must be relate plus a match, with no second opinion"
            );
        }
    }
}
