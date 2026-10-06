// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The four exact primitives every other file in this module tree is built from:
//! which side of a line a point falls on, whether a point lies on a segment, the
//! midpoint of a segment, and how two closed segments meet.
//!
//! # Why these four, and why they are exact
//!
//! Every topological question this crate answers reduces to a *sign*. "Is the
//! point inside the ring" is a parity of crossing signs; "do these two edges
//! cross" is a pair of orientation signs; "is this vertex on that edge" is a zero
//! sign plus an interval test. A sign is a discrete answer extracted from a
//! continuous computation, which is exactly the place where floating point stops
//! being an approximation and starts being a *different answer*: a cross product
//! that should be `0` evaluates to `-1.2e-17`, the sign flips, and the predicate
//! reports that a vertex is off its own edge. There is no tolerance that repairs
//! that in general — a tolerance large enough to absorb the error merges genuinely
//! distinct points, and one small enough to keep them apart does not absorb it.
//!
//! So the sign is computed exactly. [`orientation`] is the sign of a cross product
//! of exact rationals; it is `0` when and only when the three points really are
//! collinear. [`intersect`] computes the crossing point as
//! `a1 + t·(a2 - a1)` with `t` an exact rational quotient, so the point it returns
//! is *on both segments* rather than near them, and feeding it back into
//! [`on_segment`] is guaranteed to answer `true`. That round-trip property is what
//! the noder in [`super::relate`] depends on: it splits segments at points these
//! functions produced, and a fragment endpoint that had drifted off its own
//! segment would silently corrupt the fragment's midpoint classification.
//!
//! # Everything here is planar
//!
//! OGC GeoSPARQL 1.1 Clause 10.2: *"Geometric functions working with Geometries
//! that have Z values will ignore Z values in calculations and first project
//! geometry onto the Z=0 level. … Like Z values in coordinates, M values are to be
//! ignored."* No function in this file reads [`Coord::z`] or [`Coord::m`], and
//! every [`Coord`] any of them *returns* is [`crate::geom::CoordDim::Xy`] — the
//! projection is performed once, on the way out, so a caller cannot accidentally
//! propagate an elevation into a planar result.
//!
//! # Canonical output
//!
//! [`intersect`] orders the two extremes of a collinear overlap by the
//! lexicographic `(x, y)` order. That order is a *linear* order along any line
//! (compare by `x`; for a vertical line every `x` ties and `y` breaks it), so the
//! two extremes are well defined and the result is a pure function of the two
//! segments rather than of the order they were passed in.

use core::cmp::Ordering;

use crate::GeoError;
use crate::exact::{Int, Rat};
use crate::geom::Coord;
use crate::numerical::{ExactAdmission, exact_rational};
use purrdf_xsd::integer::ExactOperation::{
    Linear, RationalAdd, RationalCompare, RationalDivide, RationalMultiply,
};

/// One half, exactly.
///
/// Built as a rational with denominator two rather than by dividing, so no
/// fallible division appears on the midpoint path at all.
pub(crate) fn half() -> Rat {
    Rat::new(Int::one(), Int::from_i64(2)).expect("two is a non-zero denominator")
}

/// The planar projection of `coord`: its `x` and `y`, with elevation and measure
/// dropped.
///
/// This is the Clause 10.2 projection, applied once at the boundary of the
/// topology engine and of every measurement decomposition, so that nothing
/// downstream has to remember to ignore `z`. It is the crate's one planar
/// projection; the measurement module reads it through `crate::topology`.
pub(crate) fn plane(coord: &Coord) -> Coord {
    Coord::xy(coord.x().clone(), coord.y().clone())
}

/// The lexicographic `(x, y)` order on the planar projection.
///
/// A total order derived entirely from [`Rat`]'s [`Ord`], which is what every
/// sort in this module tree uses: no hash iteration ever reaches a result, so the
/// answer cannot depend on a hasher's seed or on insertion order.
pub(crate) fn cmp_xy(left: &Coord, right: &Coord) -> Ordering {
    left.x()
        .cmp(right.x())
        .then_with(|| left.y().cmp(right.y()))
}

/// Admit the exact two-dimensional determinant before its original products.
pub(crate) fn determinant2_admitted(
    ax: &Rat,
    ay: &Rat,
    bx: &Rat,
    by: &Rat,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Rat, GeoError> {
    determinant2_inner(ax, ay, bx, by, Some(admission))
}
fn determinant2_inner(
    ax: &Rat,
    ay: &Rat,
    bx: &Rat,
    by: &Rat,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Rat, GeoError> {
    let first = exact_rational(
        admission.as_deref_mut(),
        RationalMultiply,
        &[ax, by],
        || ax.mul(by),
    )?;
    let second = exact_rational(
        admission.as_deref_mut(),
        RationalMultiply,
        &[ay, bx],
        || ay.mul(bx),
    )?;
    exact_rational(admission, RationalAdd, &[&first, &second], || {
        first.sub(&second)
    })
}
fn cross_inner(
    a: &Coord,
    b: &Coord,
    c: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Rat, GeoError> {
    let abx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b.x(), a.x()],
        || b.x().sub(a.x()),
    )?;
    let aby = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b.y(), a.y()],
        || b.y().sub(a.y()),
    )?;
    let acx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[c.x(), a.x()],
        || c.x().sub(a.x()),
    )?;
    let acy = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[c.y(), a.y()],
        || c.y().sub(a.y()),
    )?;
    determinant2_inner(&abx, &aby, &acx, &acy, admission)
}

/// The sign of the cross product `(b - a) × (c - a)`: `+1` counter-clockwise, `0`
/// collinear, `-1` clockwise.
///
/// Exact, so `0` means the three points are *genuinely* collinear rather than
/// collinear to within a tolerance nobody chose.
pub fn orientation(a: &Coord, b: &Coord, c: &Coord) -> i32 {
    orientation_inner(a, b, c, None).expect("pure exact orientation has no admission refusal")
}
pub(crate) fn orientation_admitted(
    a: &Coord,
    b: &Coord,
    c: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<i32, GeoError> {
    orientation_inner(a, b, c, Some(admission))
}
fn orientation_inner(
    a: &Coord,
    b: &Coord,
    c: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<i32, GeoError> {
    if let Some((ordinate, factor)) = axis_direction_inner(a, b, admission.as_deref_mut())? {
        if factor == 0 {
            return Ok(0);
        }
        let (value, origin) = if ordinate == 0 {
            (c.x(), a.x())
        } else {
            (c.y(), a.y())
        };
        let sign = exact_rational(admission, RationalCompare, &[value, origin], || {
            value.cmp(origin) as i32
        })?;
        return Ok(factor * sign);
    }
    Ok(cross_inner(a, b, c, admission)?.signum())
}

/// For an original axis edge, its cross product is factor times the selected
/// displacement ordinate. Canonical equality and comparisons precede any body.
fn axis_direction_inner(
    a: &Coord,
    b: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Option<(usize, i32)>, GeoError> {
    if exact_rational(admission.as_deref_mut(), Linear, &[a.y(), b.y()], || {
        a.y() == b.y()
    })? {
        let factor = exact_rational(admission, RationalCompare, &[b.x(), a.x()], || {
            b.x().cmp(a.x()) as i32
        })?;
        // Δy=0 gives Δx*dy, including a collapsed original edge.
        return Ok(Some((1, factor)));
    }
    if exact_rational(admission.as_deref_mut(), Linear, &[a.x(), b.x()], || {
        a.x() == b.x()
    })? {
        let factor = exact_rational(admission, RationalCompare, &[b.y(), a.y()], || {
            -(b.y().cmp(a.y()) as i32)
        })?;
        // Δx=0 gives -Δy*dx.
        return Ok(Some((0, factor)));
    }
    Ok(None)
}

/// Whether `value` lies in the closed interval spanned by `one` and `other`, in
/// either order.
fn between_inner(
    value: &Rat,
    one: &Rat,
    other: &Rat,
    admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    let evaluate = || {
        let (lo, hi) = if one <= other {
            (one, other)
        } else {
            (other, one)
        };
        lo <= value && value <= hi
    };
    if let Some(admission) = admission {
        admission.rational(RationalCompare, &[value, one, other], 3, || Ok(evaluate()))
    } else {
        Ok(evaluate())
    }
}

/// Whether `p` lies on the closed segment `a`-`b` (collinear **and** within the
/// bounding box).
///
/// The bounding-box half of the test is what turns "on the infinite line" into
/// "on the segment", and it is inclusive because a segment is closed: its own
/// endpoints lie on it. A zero-length segment (`a == b` in the plane) is a point,
/// and this answers `true` exactly for `p == a`.
pub fn on_segment(p: &Coord, a: &Coord, b: &Coord) -> bool {
    on_segment_inner(p, a, b, None).expect("pure exact segment membership has no admission refusal")
}
pub(crate) fn on_segment_admitted(
    p: &Coord,
    a: &Coord,
    b: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    on_segment_inner(p, a, b, Some(admission))
}
pub(crate) fn on_segment_direction(p: &Coord, direction: &[Rat; 2], a: &Coord, b: &Coord) -> bool {
    on_segment_direction_inner(p, direction, a, b, None)
        .expect("pure directional incidence has no admission refusal")
}
pub(crate) fn on_segment_direction_admitted(
    p: &Coord,
    direction: &[Rat; 2],
    a: &Coord,
    b: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    on_segment_direction_inner(p, direction, a, b, Some(admission))
}
fn on_segment_direction_inner(
    p: &Coord,
    direction: &[Rat; 2],
    a: &Coord,
    b: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    let evaluate = || {
        let between = |point: &Rat, step: &Rat, one: &Rat, other: &Rat| {
            let (lo, hi) = if one <= other {
                (one, other)
            } else {
                (other, one)
            };
            let compare = |endpoint: &Rat| point.cmp(endpoint).then_with(|| step.signum().cmp(&0));
            !compare(lo).is_lt() && !compare(hi).is_gt()
        };
        between(p.x(), &direction[0], a.x(), b.x()) && between(p.y(), &direction[1], a.y(), b.y())
    };
    let in_box = if let Some(admission) = admission.as_deref_mut() {
        admission.rational(
            RationalCompare,
            &[
                p.x(),
                p.y(),
                &direction[0],
                &direction[1],
                a.x(),
                a.y(),
                b.x(),
                b.y(),
            ],
            6,
            || Ok(evaluate()),
        )?
    } else {
        evaluate()
    };
    Ok(in_box && directional_orientation_inner(a, b, p, direction, admission)? == 0)
}
pub(crate) fn ray_crosses_direction(
    point: &Coord,
    direction: &[Rat; 2],
    a: &Coord,
    b: &Coord,
) -> bool {
    ray_crosses_direction_inner(point, direction, a, b, None)
        .expect("pure directional crossing has no admission refusal")
}
pub(crate) fn ray_crosses_direction_admitted(
    point: &Coord,
    direction: &[Rat; 2],
    a: &Coord,
    b: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    ray_crosses_direction_inner(point, direction, a, b, Some(admission))
}
fn ray_crosses_direction_inner(
    point: &Coord,
    direction: &[Rat; 2],
    a: &Coord,
    b: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    let dy_sign = if let Some(admission) = admission.as_deref_mut() {
        admission.rational(RationalCompare, &[b.y(), a.y()], 1, || Ok(b.y().cmp(a.y())))?
    } else {
        b.y().cmp(a.y())
    };
    let sign = directional_orientation_inner(a, b, point, direction, admission)?;
    Ok(sign.cmp(&0) == dy_sign && sign != 0)
}
/// The sign of edge×(point−a)+ε(edge×direction), for sufficiently small ε>0.
fn directional_orientation_inner(
    a: &Coord,
    b: &Coord,
    point: &Coord,
    direction: &[Rat; 2],
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<i32, GeoError> {
    let constant = orientation_inner(a, b, point, admission.as_deref_mut())?;
    if constant != 0 {
        return Ok(constant);
    }
    if let Some((ordinate, factor)) = axis_direction_inner(a, b, admission.as_deref_mut())? {
        if factor == 0 {
            return Ok(0);
        }
        let sign = exact_rational(admission, Linear, &[&direction[ordinate]], || {
            direction[ordinate].signum()
        })?;
        return Ok(factor * sign);
    }
    let dx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b.x(), a.x()],
        || b.x().sub(a.x()),
    )?;
    let dy = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b.y(), a.y()],
        || b.y().sub(a.y()),
    )?;
    Ok(determinant2_inner(&dx, &dy, &direction[0], &direction[1], admission)?.signum())
}
fn on_segment_inner(
    p: &Coord,
    a: &Coord,
    b: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    // Closed-box exclusion is exact and avoids constructing a determinant for
    // a point outside either original coordinate interval.
    Ok(
        between_inner(p.x(), a.x(), b.x(), admission.as_deref_mut())?
            && between_inner(p.y(), a.y(), b.y(), admission.as_deref_mut())?
            && orientation_inner(a, b, p, admission)? == 0,
    )
}

/// The exact midpoint of `a`-`b` in the plane.
///
/// Exact because it is used as a *representative* of a segment fragment: the
/// classification of the whole fragment is read off this one point, so a midpoint
/// that had drifted off the fragment would be classified against the wrong
/// geometry. The result is [`crate::geom::CoordDim::Xy`] — `z` and `m` are
/// dropped per Clause 10.2.
pub fn midpoint(a: &Coord, b: &Coord) -> Coord {
    midpoint_inner(a, b, None).expect("pure exact midpoint has no admission refusal")
}
pub(crate) fn midpoint_admitted(
    a: &Coord,
    b: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Coord, GeoError> {
    midpoint_inner(a, b, Some(admission))
}
fn midpoint_inner(
    a: &Coord,
    b: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Coord, GeoError> {
    let h = half();
    let mut ordinate = |a: &Rat, b: &Rat| {
        let sum = exact_rational(admission.as_deref_mut(), RationalAdd, &[a, b], || a.add(b))?;
        exact_rational(
            admission.as_deref_mut(),
            RationalMultiply,
            &[&sum, &h],
            || sum.mul(&h),
        )
    };
    Ok(Coord::xy(ordinate(a.x(), b.x())?, ordinate(a.y(), b.y())?))
}

/// How two closed segments meet.
// `Collinear` is twice the size of `Point` because a `Coord` is four exact
// rationals, and clippy's `large_enum_variant` would have the two extremes boxed.
// That trade is the wrong way round here: this value is returned by move from the
// noder's inner loop, which runs once per segment pair, so boxing would convert a
// stack move into a heap allocation on every collinear overlap in exchange for
// shrinking a leaf function's stack frame. The allow is scoped to this one type.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentIntersection {
    /// They do not meet at all.
    None,
    /// They meet in exactly one point.
    Point(Coord),
    /// They are collinear and share a sub-segment of positive length, whose two
    /// extremes are reported in lexicographic `(x, y)` order.
    Collinear {
        /// The lexicographically smaller extreme of the overlap.
        from: Coord,
        /// The lexicographically larger extreme of the overlap.
        to: Coord,
    },
}

/// How the two closed segments `a1`-`a2` and `b1`-`b2` meet, computed exactly.
///
/// The crossing point of two non-parallel segments is
/// `a1 + t·(a2 - a1)` with
/// `t = ((b1 - a1) × (b2 - b1)) / ((a2 - a1) × (b2 - b1))`, evaluated entirely in
/// [`Rat`]: no float, no rounding, and therefore a point that satisfies
/// [`on_segment`] against both inputs by construction.
///
/// Every degenerate shape is handled rather than assumed away:
///
/// * a segment whose endpoints are equal in the plane **is a point**, and is
///   tested for incidence rather than being fed to a division that would have a
///   zero denominator;
/// * parallel-but-not-collinear segments answer [`SegmentIntersection::None`];
/// * collinear segments that overlap answer [`SegmentIntersection::Collinear`]
///   with the overlap's extremes, and collinear segments that meet at a single
///   shared endpoint answer [`SegmentIntersection::Point`] — the distinction
///   matters because the first contributes a one-dimensional witness to a DE-9IM
///   entry and the second only a zero-dimensional one.
pub fn intersect(a1: &Coord, a2: &Coord, b1: &Coord, b2: &Coord) -> SegmentIntersection {
    intersect_inner(a1, a2, b1, b2, None).expect("pure exact intersection has no admission refusal")
}
/// The same original exact segment law with stepwise operand admission.
pub(crate) fn intersect_admitted(
    a1: &Coord,
    a2: &Coord,
    b1: &Coord,
    b2: &Coord,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<SegmentIntersection, GeoError> {
    intersect_inner(a1, a2, b1, b2, Some(admission))
}
pub(crate) fn same_planar_inner(
    a: &Coord,
    b: &Coord,
    admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    let evaluate = || a.same_planar(b);
    if let Some(admission) = admission {
        // Canonical Rat equality compares limbs, never cross-multiplies.
        admission.rational(Linear, &[a.x(), a.y(), b.x(), b.y()], 4, || Ok(evaluate()))
    } else {
        Ok(evaluate())
    }
}
fn plane_inner(
    coord: &Coord,
    admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Coord, GeoError> {
    if let Some(admission) = admission {
        admission.rational(Linear, &[coord.x(), coord.y()], 2, || Ok(plane(coord)))
    } else {
        Ok(plane(coord))
    }
}
pub(crate) fn compare_inner(
    a: &Coord,
    b: &Coord,
    admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Ordering, GeoError> {
    if let Some(admission) = admission {
        admission.rational(RationalCompare, &[a.x(), a.y(), b.x(), b.y()], 2, || {
            Ok(cmp_xy(a, b))
        })
    } else {
        Ok(cmp_xy(a, b))
    }
}
fn intersect_inner(
    a1: &Coord,
    a2: &Coord,
    b1: &Coord,
    b2: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<SegmentIntersection, GeoError> {
    for ((point, other_a), (other_point, other_b)) in [
        ((a1, a2), (b1, b2)),
        ((a1, a2), (b2, b1)),
        ((a2, a1), (b1, b2)),
        ((a2, a1), (b2, b1)),
    ] {
        if same_planar_inner(point, other_point, admission.as_deref_mut())?
            && orientation_inner(point, other_a, other_b, admission.as_deref_mut())? != 0
        {
            // Two distinct supporting lines meet at exactly the already
            // proved original shared endpoint. Collinear cases retain the
            // complete overlap law below.
            return Ok(SegmentIntersection::Point(plane_inner(point, admission)?));
        }
    }
    let operands = [
        a1.x(),
        a1.y(),
        a2.x(),
        a2.y(),
        b1.x(),
        b1.y(),
        b2.x(),
        b2.y(),
    ];
    let separated = if let Some(admission) = admission.as_deref_mut() {
        admission.rational(RationalCompare, &operands, 12, || {
            Ok(segment_boxes_disjoint(a1, a2, b1, b2))
        })?
    } else {
        segment_boxes_disjoint(a1, a2, b1, b2)
    };
    if separated {
        return Ok(SegmentIntersection::None);
    }
    let a_point = same_planar_inner(a1, a2, admission.as_deref_mut())?;
    let b_point = same_planar_inner(b1, b2, admission.as_deref_mut())?;
    match (a_point, b_point) {
        (true, true) => Ok(if same_planar_inner(a1, b1, admission.as_deref_mut())? {
            SegmentIntersection::Point(plane_inner(a1, admission)?)
        } else {
            SegmentIntersection::None
        }),
        (true, false) => Ok(if on_segment_inner(a1, b1, b2, admission.as_deref_mut())? {
            SegmentIntersection::Point(plane_inner(a1, admission)?)
        } else {
            SegmentIntersection::None
        }),
        (false, true) => Ok(if on_segment_inner(b1, a1, a2, admission.as_deref_mut())? {
            SegmentIntersection::Point(plane_inner(b1, admission)?)
        } else {
            SegmentIntersection::None
        }),
        (false, false) => intersect_nondegenerate(a1, a2, b1, b2, admission),
    }
}
fn segment_boxes_disjoint(a1: &Coord, a2: &Coord, b1: &Coord, b2: &Coord) -> bool {
    a1.x().min(a2.x()) > b1.x().max(b2.x())
        || b1.x().min(b2.x()) > a1.x().max(a2.x())
        || a1.y().min(a2.y()) > b1.y().max(b2.y())
        || b1.y().min(b2.y()) > a1.y().max(a2.y())
}
/// The original nondegenerate law, charging each reduced operand before use.
fn intersect_nondegenerate(
    a1: &Coord,
    a2: &Coord,
    b1: &Coord,
    b2: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<SegmentIntersection, GeoError> {
    let rx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[a2.x(), a1.x()],
        || a2.x().sub(a1.x()),
    )?;
    let ry = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[a2.y(), a1.y()],
        || a2.y().sub(a1.y()),
    )?;
    let sx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b2.x(), b1.x()],
        || b2.x().sub(b1.x()),
    )?;
    let sy = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b2.y(), b1.y()],
        || b2.y().sub(b1.y()),
    )?;
    let qpx = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b1.x(), a1.x()],
        || b1.x().sub(a1.x()),
    )?;
    let qpy = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[b1.y(), a1.y()],
        || b1.y().sub(a1.y()),
    )?;
    let denominator = determinant2_inner(&rx, &ry, &sx, &sy, admission.as_deref_mut())?;
    if denominator.is_zero() {
        return intersect_parallel(a1, a2, b1, b2, admission);
    }
    let t = determinant_ratio(&qpx, &qpy, &sx, &sy, &denominator, admission.as_deref_mut())?;
    if !between_inner(&t, &Rat::zero(), &Rat::one(), admission.as_deref_mut())? {
        return Ok(SegmentIntersection::None);
    }
    let u = determinant_ratio(&qpx, &qpy, &rx, &ry, &denominator, admission.as_deref_mut())?;
    if !between_inner(&u, &Rat::zero(), &Rat::one(), admission.as_deref_mut())? {
        return Ok(SegmentIntersection::None);
    }
    let xpart = exact_rational(
        admission.as_deref_mut(),
        RationalMultiply,
        &[&t, &rx],
        || t.mul(&rx),
    )?;
    let ypart = exact_rational(
        admission.as_deref_mut(),
        RationalMultiply,
        &[&t, &ry],
        || t.mul(&ry),
    )?;
    let x = exact_rational(
        admission.as_deref_mut(),
        RationalAdd,
        &[a1.x(), &xpart],
        || a1.x().add(&xpart),
    )?;
    let y = exact_rational(admission, RationalAdd, &[a1.y(), &ypart], || {
        a1.y().add(&ypart)
    })?;
    Ok(SegmentIntersection::Point(Coord::xy(x, y)))
}
fn determinant_ratio(
    qx: &Rat,
    qy: &Rat,
    x: &Rat,
    y: &Rat,
    denominator: &Rat,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<Rat, GeoError> {
    let numerator = determinant2_inner(qx, qy, x, y, admission.as_deref_mut())?;
    exact_rational(
        admission,
        RationalDivide,
        &[&numerator, denominator],
        || {
            numerator
                .div(denominator)
                .expect("proved nonzero denominator")
        },
    )
}
fn intersect_parallel(
    a1: &Coord,
    a2: &Coord,
    b1: &Coord,
    b2: &Coord,
    mut admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<SegmentIntersection, GeoError> {
    if orientation_inner(a1, a2, b1, admission.as_deref_mut())? != 0 {
        return Ok(SegmentIntersection::None);
    }
    let (a_lo, a_hi) = ordered(a1, a2, admission.as_deref_mut())?;
    let (b_lo, b_hi) = ordered(b1, b2, admission.as_deref_mut())?;
    let lo = if compare_inner(a_lo, b_lo, admission.as_deref_mut())? == Ordering::Greater {
        a_lo
    } else {
        b_lo
    };
    let hi = if compare_inner(a_hi, b_hi, admission.as_deref_mut())? == Ordering::Less {
        a_hi
    } else {
        b_hi
    };
    Ok(match compare_inner(lo, hi, admission.as_deref_mut())? {
        Ordering::Greater => SegmentIntersection::None,
        Ordering::Equal => SegmentIntersection::Point(plane_inner(lo, admission)?),
        Ordering::Less => SegmentIntersection::Collinear {
            from: plane_inner(lo, admission.as_deref_mut())?,
            to: plane_inner(hi, admission)?,
        },
    })
}
fn ordered<'a>(
    one: &'a Coord,
    other: &'a Coord,
    admission: Option<&mut ExactAdmission<'_, '_>>,
) -> Result<(&'a Coord, &'a Coord), GeoError> {
    Ok(
        if compare_inner(one, other, admission)? == Ordering::Greater {
            (other, one)
        } else {
            (one, other)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        SegmentIntersection, cmp_xy, half, intersect, midpoint, on_segment, orientation, plane,
    };
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim};

    fn r(value: i64) -> Rat {
        Rat::from_i64(value)
    }

    fn c(x: i64, y: i64) -> Coord {
        Coord::xy(r(x), r(y))
    }

    /// A rational from a numerator and a denominator, for the exact expected
    /// crossing points.
    fn q(numerator: i64, denominator: i64) -> Rat {
        Rat::new(Int::from_i64(numerator), Int::from_i64(denominator))
            .expect("a non-zero denominator")
    }

    /// `orientation` reports the three signs, and reverses when two arguments
    /// swap.
    #[test]
    fn orientation_is_the_exact_sign_of_the_cross_product() {
        assert_eq!(
            orientation(&c(0, 0), &c(1, 0), &c(0, 1)),
            1,
            "a left turn is counter-clockwise"
        );
        assert_eq!(
            orientation(&c(0, 0), &c(1, 0), &c(0, -1)),
            -1,
            "a right turn is clockwise"
        );
        assert_eq!(
            orientation(&c(0, 0), &c(1, 1), &c(2, 2)),
            0,
            "three collinear points have a zero cross product"
        );
        assert_eq!(
            orientation(&c(1, 0), &c(0, 0), &c(0, 1)),
            -1,
            "swapping two arguments reverses the sign"
        );
    }

    #[test]
    fn axis_orientation_matches_the_original_determinant_for_exact_rationals() {
        let values = [q(-19, 7), q(-1, 11), r(0), q(13, 17), q(53, 19)];
        let mut context = crate::MetricContext::wgs84().unwrap();
        for fixed in &values {
            for from in &values {
                for to in &values {
                    let horizontal = (
                        Coord::xy(from.clone(), fixed.clone()),
                        Coord::xy(to.clone(), fixed.clone()),
                    );
                    let vertical = (
                        Coord::xy(fixed.clone(), from.clone()),
                        Coord::xy(fixed.clone(), to.clone()),
                    );
                    for x in &values {
                        for y in &values {
                            let point = Coord::xy(x.clone(), y.clone());
                            for (a, b) in [&horizontal, &vertical] {
                                let determinant = super::cross_inner(a, b, &point, None).unwrap();
                                assert_eq!(orientation(a, b, &point), determinant.signum());
                                context.begin(1).unwrap();
                                assert_eq!(
                                    super::orientation_admitted(
                                        a,
                                        b,
                                        &point,
                                        &mut crate::numerical::ExactAdmission::new(
                                            &mut context,
                                            &mut crate::context::WorkProgress::new(None),
                                        ),
                                    )
                                    .unwrap(),
                                    determinant.signum(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn axial_formal_direction_uses_the_exact_first_nonzero_coefficient() {
        let values = [q(-11, 7), r(0), q(17, 13)];
        let horizontal = (Coord::xy(q(-3, 11), q(5, 7)), Coord::xy(q(19, 3), q(5, 7)));
        let vertical = (Coord::xy(q(5, 7), q(-3, 11)), Coord::xy(q(5, 7), q(19, 3)));
        let mut context = crate::MetricContext::wgs84().unwrap();
        for (a, b) in [&horizontal, &vertical] {
            let point = midpoint(a, b);
            for dx in &values {
                for dy in &values {
                    let direction = [dx.clone(), dy.clone()];
                    let probe = Coord::xy(point.x().add(dx), point.y().add(dy));
                    // The constant coefficient vanishes exactly, so any
                    // positive displacement gives the same remaining sign.
                    let expected = super::cross_inner(a, b, &probe, None).unwrap().signum();
                    assert_eq!(
                        super::directional_orientation_inner(a, b, &point, &direction, None)
                            .unwrap(),
                        expected
                    );
                    context.begin(1).unwrap();
                    assert_eq!(
                        super::directional_orientation_inner(
                            a,
                            b,
                            &point,
                            &direction,
                            Some(&mut crate::numerical::ExactAdmission::new(
                                &mut context,
                                &mut crate::context::WorkProgress::new(None),
                            )),
                        )
                        .unwrap(),
                        expected,
                    );
                }
            }
        }
    }

    #[test]
    fn shared_noncollinear_endpoints_retain_exact_planar_witnesses_in_every_order() {
        let shared = Coord::new(q(1, 3), q(2, 7), Some(r(17)), Some(r(23)));
        let a = Coord::xy(q(9, 11), q(-3, 5));
        let b = Coord::xy(q(5, 13), q(17, 19));
        let expected =
            SegmentIntersection::Point(Coord::xy(shared.x().clone(), shared.y().clone()));
        for (a1, a2) in [(&shared, &a), (&a, &shared)] {
            for (b1, b2) in [(&shared, &b), (&b, &shared)] {
                for (a1, a2, b1, b2) in [(a1, a2, b1, b2), (b1, b2, a1, a2)] {
                    assert_eq!(intersect(a1, a2, b1, b2), expected);
                    let mut context = crate::MetricContext::wgs84().unwrap();
                    context.begin(1).unwrap();
                    assert_eq!(
                        super::intersect_admitted(
                            a1,
                            a2,
                            b1,
                            b2,
                            &mut crate::numerical::ExactAdmission::new(
                                &mut context,
                                &mut crate::context::WorkProgress::new(None)
                            )
                        )
                        .unwrap(),
                        expected
                    );
                }
            }
        }
    }

    /// A point that is collinear with a segment but beyond its end is NOT on it;
    /// the neighbouring point that is within the span is.
    #[test]
    fn on_segment_is_collinearity_and_containment_not_collinearity_alone() {
        let a = c(0, 0);
        let b = c(2, 2);
        assert!(
            !on_segment(&c(3, 3), &a, &b),
            "collinear but past the end is off the segment"
        );
        assert!(
            !on_segment(&c(-1, -1), &a, &b),
            "collinear but before the start is off the segment"
        );
        assert!(on_segment(&c(1, 1), &a, &b), "the interior point is on it");
        assert!(on_segment(&a, &a, &b), "a segment is closed at its start");
        assert!(on_segment(&b, &a, &b), "a segment is closed at its end");
        assert!(
            !on_segment(&c(1, 2), &a, &b),
            "a point off the line is off the segment"
        );
    }

    /// A zero-length segment is a point, and incidence with it is equality.
    #[test]
    fn a_zero_length_segment_behaves_as_the_point_it_is() {
        let a = c(3, 4);
        assert!(on_segment(&a, &a, &a), "the point lies on itself");
        assert!(
            !on_segment(&c(3, 5), &a, &a),
            "nothing else lies on a zero-length segment"
        );
    }

    /// The midpoint is exact, and it drops elevation and measure per Clause 10.2.
    #[test]
    fn midpoint_is_exact_and_planar() {
        let mid = midpoint(&c(0, 0), &c(1, 3));
        assert_eq!(*mid.x(), q(1, 2), "an exact half, not a rounded one");
        assert_eq!(*mid.y(), q(3, 2), "an exact three halves");
        assert_eq!(mid.dim(), CoordDim::Xy, "z and m are dropped");

        let with_z = Coord::new(r(0), r(0), Some(r(100)), Some(r(200)));
        let other_z = Coord::new(r(2), r(2), Some(r(900)), Some(r(900)));
        let projected = midpoint(&with_z, &other_z);
        assert_eq!(projected, c(1, 1), "the midpoint ignores z and m entirely");
        assert_eq!(
            projected.dim(),
            CoordDim::Xy,
            "the projection happens on the way out"
        );
    }

    #[test]
    fn admitted_midpoint_preserves_original_xy_and_drops_metadata() {
        let a = Coord::new(
            Rat::from_i64(-3),
            Rat::from_i64(1),
            Some(Rat::from_i64(40)),
            Some(Rat::from_i64(7)),
        );
        let b = Coord::new(
            Rat::from_i64(2),
            Rat::from_i64(8),
            Some(Rat::from_i64(80)),
            Some(Rat::from_i64(9)),
        );
        let mut context = crate::MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let result = super::midpoint_admitted(
            &a,
            &b,
            &mut crate::numerical::ExactAdmission::new(
                &mut context,
                &mut crate::context::WorkProgress::new(None),
            ),
        )
        .unwrap();
        assert_eq!(result, midpoint(&a, &b));
        assert_eq!(result.z(), None);
        assert_eq!(result.m(), None);
    }

    /// Two segments that cross at a non-lattice point produce that point
    /// EXACTLY, and the point round-trips through `on_segment` on both inputs.
    #[test]
    fn a_proper_crossing_is_an_exact_rational_that_lies_on_both_segments() {
        let (a1, a2) = (c(0, 0), c(3, 3));
        let (b1, b2) = (c(0, 1), c(1, 0));
        let SegmentIntersection::Point(p) = intersect(&a1, &a2, &b1, &b2) else {
            panic!("two crossing segments meet in a point");
        };
        assert_eq!(*p.x(), q(1, 2), "the exact crossing abscissa");
        assert_eq!(*p.y(), q(1, 2), "the exact crossing ordinate");
        assert!(
            on_segment(&p, &a1, &a2),
            "the crossing point must lie on the first segment by construction"
        );
        assert!(
            on_segment(&p, &b1, &b2),
            "the crossing point must lie on the second segment by construction"
        );
    }

    /// A crossing whose coordinates are not representable in binary floating
    /// point is still exact here.
    #[test]
    fn a_crossing_at_a_third_is_exact_rather_than_rounded() {
        let (a1, a2) = (c(0, 0), c(3, 0));
        let (b1, b2) = (c(1, -1), c(1, 2));
        let SegmentIntersection::Point(p) = intersect(&a1, &a2, &b1, &b2) else {
            panic!("the segments cross");
        };
        assert_eq!(p, c(1, 0), "the crossing is the lattice point (1, 0)");

        let (c1, c2) = (c(0, 0), c(1, 1));
        let (d1, d2) = (c(0, 1), c(2, 0));
        let SegmentIntersection::Point(qp) = intersect(&c1, &c2, &d1, &d2) else {
            panic!("the segments cross");
        };
        assert_eq!(*qp.x(), q(2, 3), "two thirds exactly");
        assert_eq!(*qp.y(), q(2, 3), "two thirds exactly");
    }

    /// Segments whose infinite lines cross but whose spans do not are a miss —
    /// and the neighbouring case where the spans do reach is a hit, so the
    /// refusal is not merely a blanket rejection.
    #[test]
    fn a_crossing_outside_either_span_is_a_miss_but_the_neighbouring_hit_is_not() {
        let (a1, a2) = (c(0, 0), c(1, 0));
        let (b1, b2) = (c(5, -1), c(5, 1));
        assert_eq!(
            intersect(&a1, &a2, &b1, &b2),
            SegmentIntersection::None,
            "the lines cross at x = 5, which is off the first segment"
        );
        let (long1, long2) = (c(0, 0), c(6, 0));
        assert_eq!(
            intersect(&long1, &long2, &b1, &b2),
            SegmentIntersection::Point(c(5, 0)),
            "extending the first segment to reach x = 5 makes it a hit"
        );
    }

    /// Parallel-but-distinct segments miss; the neighbouring collinear pair does
    /// not.
    #[test]
    fn parallel_disjoint_misses_and_collinear_overlap_reports_its_two_extremes() {
        assert_eq!(
            intersect(&c(0, 0), &c(2, 0), &c(0, 1), &c(2, 1)),
            SegmentIntersection::None,
            "two distinct parallel lines never meet"
        );
        assert_eq!(
            intersect(&c(0, 0), &c(3, 0), &c(1, 0), &c(5, 0)),
            SegmentIntersection::Collinear {
                from: c(1, 0),
                to: c(3, 0)
            },
            "the overlap is [1, 3] on the shared line"
        );
    }

    /// The collinear overlap is canonical: the same pair of segments in any
    /// argument or endpoint order yields the identical `from`/`to`.
    #[test]
    fn the_collinear_overlap_is_canonical_under_every_argument_order() {
        let expected = SegmentIntersection::Collinear {
            from: c(1, 1),
            to: c(3, 3),
        };
        let orders = [
            (c(0, 0), c(3, 3), c(1, 1), c(5, 5)),
            (c(3, 3), c(0, 0), c(1, 1), c(5, 5)),
            (c(0, 0), c(3, 3), c(5, 5), c(1, 1)),
            (c(1, 1), c(5, 5), c(0, 0), c(3, 3)),
            (c(5, 5), c(1, 1), c(3, 3), c(0, 0)),
        ];
        for (a1, a2, b1, b2) in orders {
            assert_eq!(
                intersect(&a1, &a2, &b1, &b2),
                expected,
                "the overlap extremes are ordered lexicographically, not by argument order"
            );
        }
    }

    /// A vertical shared line ties on `x`, so `y` has to break it — otherwise the
    /// "lexicographic order is linear along the line" argument fails exactly
    /// where it is needed.
    #[test]
    fn a_vertical_collinear_overlap_is_ordered_by_y() {
        assert_eq!(
            intersect(&c(1, 0), &c(1, 4), &c(1, 2), &c(1, 9)),
            SegmentIntersection::Collinear {
                from: c(1, 2),
                to: c(1, 4)
            },
            "on a vertical line every x ties and y is the linear order"
        );
    }

    /// Collinear segments meeting at exactly one shared endpoint are a `Point`,
    /// not a degenerate `Collinear` — the DE-9IM entry they witness differs.
    #[test]
    fn collinear_segments_touching_at_one_point_report_a_point() {
        assert_eq!(
            intersect(&c(0, 0), &c(2, 0), &c(2, 0), &c(5, 0)),
            SegmentIntersection::Point(c(2, 0)),
            "a shared endpoint is a zero-dimensional meeting"
        );
        assert_eq!(
            intersect(&c(0, 0), &c(2, 0), &c(3, 0), &c(5, 0)),
            SegmentIntersection::None,
            "collinear with a gap is still a miss"
        );
    }

    /// Every combination of degenerate operands is answered, and none of them
    /// divides by zero.
    #[test]
    fn degenerate_segments_are_handled_in_every_combination() {
        assert_eq!(
            intersect(&c(1, 1), &c(1, 1), &c(1, 1), &c(1, 1)),
            SegmentIntersection::Point(c(1, 1)),
            "two coincident points meet at themselves"
        );
        assert_eq!(
            intersect(&c(1, 1), &c(1, 1), &c(2, 2), &c(2, 2)),
            SegmentIntersection::None,
            "two distinct points do not meet"
        );
        assert_eq!(
            intersect(&c(1, 1), &c(1, 1), &c(0, 0), &c(3, 3)),
            SegmentIntersection::Point(c(1, 1)),
            "a point on a segment meets it"
        );
        assert_eq!(
            intersect(&c(1, 2), &c(1, 2), &c(0, 0), &c(3, 3)),
            SegmentIntersection::None,
            "a point off a segment does not"
        );
        assert_eq!(
            intersect(&c(0, 0), &c(3, 3), &c(2, 2), &c(2, 2)),
            SegmentIntersection::Point(c(2, 2)),
            "the mirrored degenerate case answers the same way"
        );
    }

    /// `intersect` ignores elevation and measure, per Clause 10.2, and returns a
    /// planar coordinate.
    #[test]
    fn intersection_ignores_z_and_m_and_returns_a_planar_coordinate() {
        let a1 = Coord::new(r(0), r(0), Some(r(10)), None);
        let a2 = Coord::new(r(2), r(2), Some(r(20)), None);
        let b1 = Coord::new(r(0), r(2), Some(r(-5)), None);
        let b2 = Coord::new(r(2), r(0), Some(r(-9)), None);
        let SegmentIntersection::Point(p) = intersect(&a1, &a2, &b1, &b2) else {
            panic!("the projections cross");
        };
        assert_eq!(p, c(1, 1), "the crossing is decided in the plane alone");
        assert_eq!(p.dim(), CoordDim::Xy, "no elevation survives");
    }

    /// The internal helpers hold up their end: `plane` projects, `cmp_xy` is
    /// lexicographic, and `half` is one half.
    #[test]
    fn the_internal_helpers_do_exactly_what_they_claim() {
        let raised = Coord::new(r(4), r(5), Some(r(6)), Some(r(7)));
        assert_eq!(plane(&raised), c(4, 5), "plane keeps x and y only");
        assert_eq!(
            cmp_xy(&c(1, 9), &c(2, 0)),
            core::cmp::Ordering::Less,
            "x dominates"
        );
        assert_eq!(
            cmp_xy(&c(2, 0), &c(2, 1)),
            core::cmp::Ordering::Less,
            "y breaks an x tie"
        );
        assert_eq!(
            cmp_xy(&c(2, 1), &c(2, 1)),
            core::cmp::Ordering::Equal,
            "equal coordinates tie"
        );
        assert_eq!(half().add(&half()), Rat::one(), "two halves are one");
    }
}
