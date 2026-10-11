// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! * Let `ys` be the sorted, deduplicated `y` of every event point. Outside
//!   their range there is no edge and only the already-established `E/E` area.
//! * For each consecutive pair take `ym = (y_i + y_{i+1}) / 2` — exact, and
//!   strictly between, so **no vertex and no intersection point lies on the line
//!   `y = ym`**.
//! * Compute the exact abscissa at which each segment of A or B crosses that
//!   line. Sort and deduplicate them. Sample the midpoint of each consecutive
//!   pair. Outside all crossings both geometries are exterior, and E/E is
//!   already dimension two, so no outside sample is constructed.
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
//! two consecutive crossing abscissae, and the sampled midpoint of that interval
//! is in the face. An unbounded outside interval is E/E for finite geometries.
//! Every face lying in `I/I`, `I/E` or `E/I` therefore has a witness raising its
//! cell, unless that cell has already reached the absorbing dimension two.
//!
//! The scan is skipped entirely when neither geometry has area, because then no
//! entry other than `E/E` can be `2`. It also stops as soon as every cell it could
//! still decide has reached `2`.
//!
//! **Singleton reduction.** A nonempty point operand has one interior position
//! and no boundary. Locate that position once to fix its whole interior row or
//! column, then node only the other operand. Removing one point from an open
//! fragment or face leaves its positive dimension unchanged, so those witnesses
//! contribute to the singleton's exterior even when their sample equals it.
//! Event witnesses still use both exact locations. Area and fragment witnesses
//! run first, and traversal stops only when the two remaining cells actually
//! attain conservative dimension ceilings. Degenerate and invalid inputs whose
//! cells do not attain those ceilings retain the complete witness traversal.

use super::locate::{horizontal_crossing, horizontal_crossing_admitted, locate, locate_admitted};
use super::noding::{AdmittedWork, LegacyWork, NodingWork, events_inner, fragments_inner};
use super::segment::{mean_admitted, mean_inner};
use crate::context::{RetainedBuilder, WorkProgress};
use crate::de9im::{Dim, IntersectionMatrix, Pattern, Set};
use crate::exact::Rat;
use crate::geom::{Coord, CoordSeq, Geometry, GeometryBody, LeafCoords};
use crate::numerical::ExactAdmission;
use crate::{GeoError, MetricContext, MetricWorkObserver};
use purrdf_xsd::integer::ExactOperation;
use std::borrow::Cow;

/// The DE-9IM intersection matrix of the ordered pair `(a, b)`, computed exactly.
///
/// Every well-formed pair has a matrix. This resident door retains its existing
/// infallible allocation contract; [`relate_metered`] runs the same matrix body
/// with caller-selected admission and the original native allocation owners.
/// Only `x` and `y` are read, per GeoSPARQL 1.1 Clause 10.2.
pub fn relate(a: &Geometry, b: &Geometry) -> IntersectionMatrix {
    relate_inner(
        a,
        b,
        &mut LegacyWork(&mut |_, _: Option<&Coord>| Ok::<_, GeoError>(())),
    )
    .expect("resident exact topology has its infallible allocation contract")
}

/// Compute the same complete matrix with the caller's checked execution policy.
/// Native arrays and arithmetic destinations are admitted before allocation,
/// and every original payload dies before its associated working allowance.
///
/// # Errors
/// Returns the first caller refusal, cancellation, physical allocation failure,
/// checked layout overflow, or exhausted explicitly selected execution policy.
pub fn relate_metered(
    a: &Geometry,
    b: &Geometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<IntersectionMatrix, GeoError> {
    relate_admitted(a, b, context, &mut WorkProgress::integer(Some(observer)))
}

pub(crate) fn relate_admitted(
    a: &Geometry,
    b: &Geometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<IntersectionMatrix, GeoError> {
    context.with_metadata(progress, |context, progress, retained| {
        relate_inner(
            a,
            b,
            &mut AdmittedWork {
                context,
                progress,
                retained,
            },
        )
    })
}

/// Whether the ordered pair `(a, b)` satisfies `pattern`.
pub fn relate_pattern(a: &Geometry, b: &Geometry, pattern: &Pattern) -> bool {
    relate(a, b).matches(pattern)
}

/// The original noder supplies the working arrays and exact-expression seam;
/// topology adds only its existing location and scan primitives.
trait RelateWork: NodingWork<Error = GeoError> {
    fn mean(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError>;
    fn leaves<'a>(&mut self, root: &'a Geometry) -> Result<Self::Array<&'a Geometry>, GeoError>;
    fn locate(&mut self, point: &Coord, geometry: &Geometry) -> Result<Set, GeoError>;
    fn crossing(&mut self, a: &Coord, b: &Coord, height: &Rat) -> Result<Option<Rat>, GeoError>;
    fn persistent_rat(&mut self, value: Rat) -> Result<Rat, GeoError>;
}

impl<F: FnMut(u64, Option<&Coord>) -> Result<(), GeoError>> RelateWork for LegacyWork<'_, F> {
    fn mean(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        mean_inner(a, b, None)
    }
    fn leaves<'a>(&mut self, root: &'a Geometry) -> Result<Vec<&'a Geometry>, GeoError> {
        let mut values = Vec::new();
        let mut resident = purrdf_lex::allocation::Resident;
        let _: core::ops::ControlFlow<()> = crate::geom::visit_leaves_with_memory(
            root,
            &mut purrdf_lex::allocation::Memory::new(&mut resident),
            |leaf| {
                values.push(leaf);
                core::ops::ControlFlow::Continue(())
            },
        )
        .expect("resident topology tree storage");
        Ok(values)
    }
    fn locate(&mut self, point: &Coord, geometry: &Geometry) -> Result<Set, GeoError> {
        Ok(locate(point, geometry))
    }
    fn crossing(&mut self, a: &Coord, b: &Coord, height: &Rat) -> Result<Option<Rat>, GeoError> {
        Ok(horizontal_crossing(a, b, height))
    }
    fn persistent_rat(&mut self, value: Rat) -> Result<Rat, GeoError> {
        Ok(value)
    }
}

impl RelateWork for AdmittedWork<'_, '_> {
    fn mean(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        mean_admitted(a, b, self.context, self.progress)
    }
    fn leaves<'a>(
        &mut self,
        root: &'a Geometry,
    ) -> Result<RetainedBuilder<&'a Geometry>, GeoError> {
        let mut values = RetainedBuilder::default();
        let context = &mut *self.context;
        let retained = &mut *self.retained;
        crate::geom::visit_leaves_with_progress(root, self.progress, |leaf, progress| {
            values.push_admitted(leaf, context, progress, retained)
        })?;
        Ok(values)
    }
    fn locate(&mut self, point: &Coord, geometry: &Geometry) -> Result<Set, GeoError> {
        locate_admitted(point, geometry, self.context, self.progress)
    }
    fn crossing(&mut self, a: &Coord, b: &Coord, height: &Rat) -> Result<Option<Rat>, GeoError> {
        horizontal_crossing_admitted(
            a,
            b,
            height,
            &mut ExactAdmission::new(self.context, self.progress),
        )
    }
    fn persistent_rat(&mut self, value: Rat) -> Result<Rat, GeoError> {
        crate::context::persistent_numeric(value, self.context, self.progress, self.retained)
    }
}

fn relate_inner<W: RelateWork>(
    a: &Geometry,
    b: &Geometry,
    work: &mut W,
) -> Result<IntersectionMatrix, GeoError> {
    let mut matrix = IntersectionMatrix::new();
    matrix.set(Set::Exterior, Set::Exterior, Dim::Two);
    let a_leaves = work.leaves(a)?;
    let b_leaves = work.leaves(b)?;
    let targets = AreaTargets::new(area_present(&a_leaves), area_present(&b_leaves));
    let point_targets = match (a.body(), b.body()) {
        (GeometryBody::Point(Some(point)), _) => {
            matrix.set(Set::Interior, work.locate(point, b)?, Dim::Zero);
            Some(PointTargets::new(false, &b_leaves))
        }
        (_, GeometryBody::Point(Some(point))) => {
            matrix.set(work.locate(point, a)?, Set::Interior, Dim::Zero);
            Some(PointTargets::new(true, &a_leaves))
        }
        _ => None,
    };
    let mut edges = W::Array::default();
    let mut seeds = W::Array::default();
    for (leaves, singleton) in [
        (
            &a_leaves[..],
            point_targets.is_some_and(|targets| !targets.transpose),
        ),
        (
            &b_leaves[..],
            point_targets.is_some_and(|targets| targets.transpose),
        ),
    ] {
        if singleton {
            continue;
        }
        for &member in leaves {
            collect_edges(member, &mut edges, work)?;
            for coordinate in LeafCoords::of(member) {
                work.push(&mut seeds, coordinate)?;
            }
        }
    }
    work.release(a_leaves)?;
    work.release(b_leaves)?;
    unique_edge_supports(&mut edges, work)?;
    let events = events_inner(&edges, seeds.iter().copied().map(Cow::Borrowed), work)?;
    work.release(seeds)?;
    // A singleton fixes its complete interior and boundary rows (or columns).
    // Establish area witnesses first so the two remaining cells can reach their
    // dimension ceilings without classifying every duplicate witness. Syntax
    // supplies only ceilings: degenerate polygons still exhaust the witnesses.
    if point_targets.is_some() && (targets.interior_exterior || targets.exterior_interior) {
        scan_for_areas(
            a,
            b,
            &events,
            &edges,
            &mut matrix,
            targets,
            point_targets,
            work,
        )?;
    }
    if point_targets.is_none() {
        classify_events(a, b, &events, &mut matrix, point_targets, work)?;
    }
    for &edge in edges.iter() {
        if point_targets.is_some_and(|targets| targets.settled(&matrix)) {
            break;
        }
        let fragments = fragments_inner(edge, &events, work)?;
        for &(from, to) in fragments.iter() {
            if point_targets.is_some_and(|targets| targets.settled(&matrix)) {
                break;
            }
            let mid = Coord::xy(work.mean(from.x(), to.x())?, work.mean(from.y(), to.y())?);
            let (in_a, in_b) = locate_positive(&mid, a, b, point_targets, work)?;
            matrix.raise(in_a, in_b, Dim::One);
        }
        work.release(fragments)?;
    }
    // A fragment can attain a curve ceiling that no event can attain. For a
    // singleton, visit those higher-dimensional witnesses first; events remain
    // necessary if either ceiling is unattained (including degenerate inputs).
    if point_targets.is_some() {
        classify_events(a, b, &events, &mut matrix, point_targets, work)?;
    }
    if point_targets.is_none() && (targets.interior_exterior || targets.exterior_interior) {
        scan_for_areas(
            a,
            b,
            &events,
            &edges,
            &mut matrix,
            targets,
            point_targets,
            work,
        )?;
    }
    work.release(events)?;
    work.release(edges)?;
    Ok(matrix)
}

fn classify_events<W: RelateWork>(
    a: &Geometry,
    b: &Geometry,
    events: &[Coord],
    matrix: &mut IntersectionMatrix,
    point_targets: Option<PointTargets>,
    work: &mut W,
) -> Result<(), GeoError> {
    for point in events {
        if point_targets.is_some_and(|targets| targets.settled(matrix)) {
            break;
        }
        let in_a = work.locate(point, a)?;
        let in_b = work.locate(point, b)?;
        matrix.raise(in_a, in_b, Dim::Zero);
    }
    Ok(())
}

/// Removing one point cannot lower the dimension of a nonempty open fragment
/// or face. The singleton's row/column is already exact, so a positive-
/// dimensional witness contributes to its exterior even if its sample happens
/// to equal that point. Only the other operand therefore needs noding; all of
/// its vertices and isolated points remain seeds. Zero-dimensional events still
/// use both actual locations and cannot include the singleton in its exterior.
fn locate_positive<W: RelateWork>(
    sample: &Coord,
    a: &Geometry,
    b: &Geometry,
    point_targets: Option<PointTargets>,
    work: &mut W,
) -> Result<(Set, Set), GeoError> {
    match point_targets {
        Some(targets) if targets.transpose => Ok((work.locate(sample, a)?, Set::Exterior)),
        Some(_) => Ok((Set::Exterior, work.locate(sample, b)?)),
        None => Ok((work.locate(sample, a)?, work.locate(sample, b)?)),
    }
}

/// Only the witness supports lose multiplicity. The original operands retain
/// every ring and curve for location and boundary parity, and every original
/// endpoint remains a noding seed. Reversed and coincident supports therefore
/// introduce no new event or fragment witness.
fn unique_edge_supports<'a, W: RelateWork>(
    edges: &mut W::Array<(&'a Coord, &'a Coord)>,
    work: &mut W,
) -> Result<(), GeoError> {
    for edge in edges.iter_mut() {
        if work.compare_coordinate(edge.0, edge.1)?.is_gt() {
            core::mem::swap(&mut edge.0, &mut edge.1);
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(edges, |a, b| {
        work.visit(1, None)?;
        let first = work.compare_coordinate(a.0, b.0)?;
        if first.is_eq() {
            work.compare_coordinate(a.1, b.1)
        } else {
            Ok(first)
        }
    })?;
    let unique = purrdf_lex::walk::try_partition_unique_by(edges, |a, b| {
        Ok::<_, GeoError>(
            work.compare_coordinate(a.0, b.0)?.is_eq()
                && work.compare_coordinate(a.1, b.1)?.is_eq(),
        )
    })?;
    work.truncate(edges, unique);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct PointTargets {
    transpose: bool,
    interior: Dim,
    boundary: Dim,
}

impl PointTargets {
    fn new(transpose: bool, leaves: &[&Geometry]) -> Self {
        let mut interior = Dim::Empty;
        let mut boundary = Dim::Empty;
        for geometry in leaves {
            let (i, b) = match geometry.body() {
                GeometryBody::Point(_) | GeometryBody::MultiPoint(_) => (Dim::Zero, Dim::Empty),
                GeometryBody::LineString(_) | GeometryBody::MultiLineString(_) => {
                    (Dim::One, Dim::Zero)
                }
                GeometryBody::Polygon(_) | GeometryBody::MultiPolygon(_) => (Dim::Two, Dim::One),
                GeometryBody::GeometryCollection(_) => unreachable!("only leaves are retained"),
            };
            interior = interior.max(i);
            boundary = boundary.max(b);
        }
        Self {
            transpose,
            interior,
            boundary,
        }
    }

    fn settled(self, matrix: &IntersectionMatrix) -> bool {
        let dimension = |set| {
            if self.transpose {
                matrix.get(set, Set::Exterior)
            } else {
                matrix.get(Set::Exterior, set)
            }
        };
        dimension(Set::Interior) == self.interior && dimension(Set::Boundary) == self.boundary
    }
}

fn area_present(leaves: &[&Geometry]) -> bool {
    leaves.iter().any(|geometry| match geometry.body() {
        GeometryBody::Polygon(rings) => !rings.is_empty(),
        GeometryBody::MultiPolygon(members) => members.iter().any(|rings| !rings.is_empty()),
        GeometryBody::GeometryCollection(_) => unreachable!("only leaves are retained"),
        GeometryBody::Point(_)
        | GeometryBody::LineString(_)
        | GeometryBody::MultiPoint(_)
        | GeometryBody::MultiLineString(_) => false,
    })
}

/// Borrow source coordinates: every predicate still projects to XY at its
/// original arithmetic home, so Z/M limbs need not be copied into edge records.
fn collect_edges<'a, W: RelateWork>(
    geometry: &'a Geometry,
    out: &mut W::Array<(&'a Coord, &'a Coord)>,
    work: &mut W,
) -> Result<(), GeoError> {
    match geometry.body() {
        GeometryBody::LineString(coords) => push_chain(coords, out, work)?,
        GeometryBody::Polygon(rings) | GeometryBody::MultiLineString(rings) => {
            for ring in rings {
                push_chain(ring, out, work)?;
            }
        }
        GeometryBody::MultiPolygon(members) => {
            for rings in members {
                for ring in rings {
                    push_chain(ring, out, work)?;
                }
            }
        }
        GeometryBody::GeometryCollection(_) => unreachable!("only leaves are visited"),
        GeometryBody::Point(_) | GeometryBody::MultiPoint(_) => {}
    }
    Ok(())
}

fn push_chain<'a, W: RelateWork>(
    coords: &'a CoordSeq,
    out: &mut W::Array<(&'a Coord, &'a Coord)>,
    work: &mut W,
) -> Result<(), GeoError> {
    for pair in coords.windows(2) {
        work.push(out, (&pair[0], &pair[1]))?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct AreaTargets {
    interior_interior: bool,
    interior_exterior: bool,
    exterior_interior: bool,
}
impl AreaTargets {
    fn new(area_a: bool, area_b: bool) -> Self {
        Self {
            interior_interior: area_a && area_b,
            interior_exterior: area_a,
            exterior_interior: area_b,
        }
    }
    fn settled(self, matrix: &IntersectionMatrix) -> bool {
        let done =
            |wanted: bool, row: Set, column: Set| !wanted || matrix.get(row, column) == Dim::Two;
        done(self.interior_interior, Set::Interior, Set::Interior)
            && done(self.interior_exterior, Set::Interior, Set::Exterior)
            && done(self.exterior_interior, Set::Exterior, Set::Interior)
    }
}

fn scan_for_areas<W: RelateWork>(
    a: &Geometry,
    b: &Geometry,
    events: &[Coord],
    edges: &[(&Coord, &Coord)],
    matrix: &mut IntersectionMatrix,
    targets: AreaTargets,
    point_targets: Option<PointTargets>,
    work: &mut W,
) -> Result<(), GeoError> {
    let Some(bands) = band_boundaries(events, work)? else {
        return Ok(());
    };
    for pair in bands.windows(2) {
        if targets.settled(matrix) {
            break;
        }
        let height = work.mean(&pair[0], &pair[1])?;
        let crossings = sorted_crossings(edges, &height, work)?;
        // Outside every original crossing, both finite geometries are exterior;
        // E/E is already dimension two. Open bands exclude every input vertex
        // and isolated point, so only intervals between crossings can supply a
        // missing area witness. Stream those witnesses: once their target cells
        // settle, no unread midpoint or temporary abscissa table is produced.
        for window in crossings.windows(2) {
            if targets.settled(matrix) {
                break;
            }
            let abscissa = work.mean(&window[0], &window[1])?;
            let abscissa = work.persistent_rat(abscissa)?;
            let ordinate = work.exact(ExactOperation::Linear, &[&height], 1, |values| {
                values.original(&height)
            })?;
            let sample = Coord::xy(abscissa, ordinate);
            let (in_a, in_b) = locate_positive(&sample, a, b, point_targets, work)?;
            debug_assert!(
                in_a != Set::Boundary && in_b != Set::Boundary,
                "a scan-line sample cannot lie on a fully noded boundary"
            );
            match (in_a, in_b) {
                (Set::Interior, Set::Interior | Set::Exterior) | (Set::Exterior, Set::Interior) => {
                    matrix.raise(in_a, in_b, Dim::Two);
                }
                _ => {}
            }
        }
        work.release(crossings)?;
    }
    work.release(bands)
}

fn sort_rationals<W: RelateWork>(values: &mut W::Array<Rat>, work: &mut W) -> Result<(), GeoError> {
    purrdf_lex::walk::try_sort_unstable_by(values, |a, b| {
        work.visit(1, None)?;
        work.compare(a, b)
    })?;
    let unique =
        purrdf_lex::walk::try_partition_unique_by(values, |a, b| -> Result<_, GeoError> {
            Ok(work.compare(a, b)?.is_eq())
        })?;
    work.truncate(values, unique);
    Ok(())
}

fn band_boundaries<W: RelateWork>(
    events: &[Coord],
    work: &mut W,
) -> Result<Option<W::Array<Rat>>, GeoError> {
    let mut ordinates = W::Array::default();
    for point in events {
        let value = work.exact(ExactOperation::Linear, &[point.y()], 1, |values| {
            values.original(point.y())
        })?;
        let value = work.persistent_rat(value)?;
        work.push(&mut ordinates, value)?;
    }
    sort_rationals(&mut ordinates, work)?;
    if ordinates.len() < 2 {
        work.release(ordinates)?;
        return Ok(None);
    }
    // Every planar segment stays between its endpoint ordinates. Bands outside
    // this range can witness only E/E, which is unconditionally dimension two.
    Ok(Some(ordinates))
}

fn sorted_crossings<W: RelateWork>(
    edges: &[(&Coord, &Coord)],
    height: &Rat,
    work: &mut W,
) -> Result<W::Array<Rat>, GeoError> {
    let mut crossings = W::Array::default();
    for &(from, to) in edges {
        if let Some(value) = work.crossing(from, to, height)? {
            let value = work.persistent_rat(value)?;
            work.push(&mut crossings, value)?;
        }
    }
    sort_rationals(&mut crossings, work)?;
    Ok(crossings)
}

#[cfg(test)]
mod tests {
    use super::super::locate::{locate, topological_dimension};
    use super::{relate, relate_pattern};
    use crate::de9im::{Dim, IntersectionMatrix, Pattern, Set};
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, Rings};
    use crate::relations::{SpatialRelation, transpose};

    struct Observe;
    impl crate::MetricWorkObserver for Observe {
        fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), crate::GeoError> {
            Ok(())
        }
    }

    #[test]
    fn native_matrix_owner_preserves_every_original_golden_and_relation_family() {
        for (name, a, b) in corpus() {
            let original = relate(&a, &b);
            let mut context = crate::MetricContext::wgs84().unwrap();
            let native = super::relate_metered(&a, &b, &mut context, &mut Observe).unwrap();
            assert_eq!(native, original, "{name}");
            for relation in SpatialRelation::ALL {
                let dimensions = [topological_dimension(&a), topological_dimension(&b)];
                assert_eq!(
                    relation.holds(&native, dimensions[0], dimensions[1]),
                    relation.holds(&original, dimensions[0], dimensions[1]),
                    "{name}: {relation:?}",
                );
            }
        }
    }

    #[test]
    fn all_small_map_pairs_complete_in_one_default_work_budget() {
        // The seven spatial objects in the query-rewrite fixture: each feature
        // and its geometry are distinct entries, plus the bare interior point.
        let geometries = [point(1, 1), point(9, 9), boxed(0, 0, 4, 4), point(2, 2)];
        let entries = [0, 0, 1, 1, 2, 2, 3];
        let mut context = crate::MetricContext::wgs84().unwrap();
        for a in entries {
            for b in entries {
                let expected = match (a, b) {
                    (2, 2) => "2FFF1FFF2",
                    (2, 1) => "FF2FF10F2",
                    (1, 2) => "FF0FFF212",
                    (2, _) => "0F2FF1FF2",
                    (_, 2) => "0FFFFF212",
                    _ if a == b => "0FFFFFFF2",
                    _ => "FF0FFF0F2",
                };
                let matrix = super::relate_metered(
                    &geometries[a],
                    &geometries[b],
                    &mut context,
                    &mut Observe,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "pair {a}/{b}, cumulative work {}: {error}",
                        context.work_items()
                    )
                });
                assert_eq!(matrix.to_string(), expected, "pair {a}/{b}");
            }
        }
    }

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

    #[test]
    fn point_witness_settlement_preserves_degenerate_and_repeated_supports() {
        let doubled_curve = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(vec![seq(&[(0, 0), (4, 0)]), seq(&[(4, 0), (0, 0)])]),
        )
        .unwrap();
        let cases = [
            (point(2, 0), line(&[(0, 0), (4, 0)]), "0FFFFF102"),
            (point(0, 0), doubled_curve, "0FFFFF1F2"),
            (
                point(2, 0),
                polygon(&[&[(0, 0), (4, 0), (2, 0), (0, 0)]]),
                "F0FFFFF12",
            ),
            (
                point(0, 0),
                polygon(&[&[(0, 0), (0, 0), (0, 0), (0, 0)]]),
                "F0FFFFFF2",
            ),
            (
                point(2, 1),
                polygon(&[&[(0, 0), (4, 4), (0, 4), (4, 0), (0, 0)]]),
                "0FFFFF212",
            ),
            (
                point(2, 2),
                polygon(&[&[(0, 0), (4, 4), (0, 4), (4, 0), (0, 0)]]),
                "F0FFFF212",
            ),
        ];
        for (point, geometry, expected) in cases {
            let expected = IntersectionMatrix::parse(expected).unwrap();
            assert_eq!(relate(&point, &geometry), expected);
            assert_eq!(relate(&geometry, &point), transpose(&expected));
            for (a, b, expected) in [
                (&point, &geometry, expected),
                (&geometry, &point, transpose(&expected)),
            ] {
                let mut context = crate::MetricContext::wgs84().unwrap();
                assert_eq!(
                    super::relate_metered(a, b, &mut context, &mut Observe).unwrap(),
                    expected,
                );
            }
        }
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

/// The edge collector's walk over nested collections, against a recursive
/// reference on generated trees, and the whole matrix a hundred thousand levels
/// deep.
#[cfg(test)]
mod nesting_tests {
    use super::{RelateWork, collect_edges, relate};
    use crate::exact::Rat;
    use crate::geom::arbitrary::{self, Lcg};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody};
    use crate::topology::noding::LegacyWork;

    /// The recursive collection: a collection's edges are its members' in written
    /// order, and a geometry that is not a collection is read by the one-node walk.
    fn reference_collect_edges<'a>(geometry: &'a Geometry, out: &mut Vec<(&'a Coord, &'a Coord)>) {
        match geometry.body() {
            GeometryBody::GeometryCollection(members) => {
                for member in members {
                    reference_collect_edges(member, out);
                }
            }
            _ => collect_edges(
                geometry,
                out,
                &mut LegacyWork(&mut |_, _: Option<&Coord>| Ok::<_, crate::GeoError>(())),
            )
            .unwrap(),
        }
    }

    fn c(x: i64, y: i64) -> Coord {
        Coord::xy(Rat::from_i64(x), Rat::from_i64(y))
    }

    fn seq(points: &[(i64, i64)]) -> CoordSeq {
        points.iter().map(|&(x, y)| c(x, y)).collect()
    }

    fn point(x: i64, y: i64) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::Point(Some(c(x, y))))
            .expect("a well-formed point")
    }

    /// The axis-aligned box `[x0, x1] × [y0, y1]`, written counter-clockwise.
    fn boxed(x0: i64, y0: i64, x1: i64, y1: i64) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::Polygon(vec![seq(&[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, y0),
            ])]),
        )
        .expect("a closed ring")
    }

    /// Over generated planar trees the work-list collector gathers the same edges,
    /// in the same order, as the recursive reference.
    #[test]
    fn edge_collection_agrees_with_the_recursive_reference_on_generated_trees() {
        let mut rng = Lcg::new(0x5eed_8001);
        for round in 0..300 {
            let tree = arbitrary::geometry(&mut rng, CoordDim::Xy, 4);
            let (mut edges, mut reference) = (Vec::new(), Vec::new());
            let mut callback = |_, _: Option<&Coord>| Ok::<_, crate::GeoError>(());
            let mut work = LegacyWork(&mut callback);
            for leaf in work.leaves(&tree).unwrap() {
                collect_edges(leaf, &mut edges, &mut work).unwrap();
            }
            reference_collect_edges(&tree, &mut reference);
            assert_eq!(edges, reference, "round {round}: {tree:?}");
        }
    }

    /// A single-member collection has its member's edges, positions, area and
    /// locations, so its DE-9IM matrix against anything is its member's: the closed
    /// form, pinned at depths one and two and then computed a hundred thousand
    /// levels deep on a 128 KiB stack, on both sides of the relation.
    #[test]
    fn a_hundred_thousand_deep_collection_relates_like_its_member() {
        let square = boxed(0, 0, 2, 2);
        let overlapping = boxed(1, 1, 3, 3);
        let inside = point(1, 1);
        let square_point = relate(&square, &inside);
        let square_box = relate(&square, &overlapping);
        assert_eq!(square_point.to_string(), "0F2FF1FF2");
        assert_eq!(square_box.to_string(), "212101212");
        for levels in 1..=2 {
            let nested = arbitrary::nest(square.clone(), levels);
            assert_eq!(relate(&nested, &inside), square_point, "{levels} levels");
            assert_eq!(relate(&nested, &overlapping), square_box, "{levels} levels");
            assert_eq!(
                relate(&nested, &arbitrary::nest(overlapping.clone(), levels)),
                square_box,
                "{levels} levels on both sides"
            );
        }

        purrdf_stack::on_stack(arbitrary::SMALL_STACK, move || {
            let deep = arbitrary::nest(square, arbitrary::DEEP);
            assert_eq!(relate(&deep, &inside), square_point);
            assert_eq!(relate(&deep, &overlapping), square_box);
            let deep_other = arbitrary::nest(overlapping, arbitrary::DEEP);
            assert_eq!(relate(&deep, &deep_other), square_box);
        })
        .expect("the thread starts");
    }
}
