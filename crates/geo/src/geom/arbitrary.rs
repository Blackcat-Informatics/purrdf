// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A deterministic generator of nested geometries, and the deep-nesting fixtures,
//! for the tests that compare an iterative walk over a tree with its recursive
//! reference and then run the walk a hundred thousand levels deep on a small stack.
//!
//! The generator recurses, on the shallow depths it is asked for; it is the reference
//! side of every such comparison, never the code under test.

use super::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, Rings};
use crate::exact::Rat;

/// The nesting every deep test uses.
pub(crate) const DEEP: usize = 100_000;

/// The stack every deep test runs on: small enough that a walk with one frame per
/// level aborts within the first thousand.
pub(crate) const SMALL_STACK: usize = 128 * 1024;

/// A 64-bit linear congruential generator: one seed yields one sequence on every
/// host, so a failing round is reproducible from its seed and index.
pub(crate) struct Lcg(u64);

impl Lcg {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next thirty-two bits, taken from the well-mixed high half of the state.
    pub(crate) fn next_u32(&mut self) -> u32 {
        let state =
            purrdf_testkit::rng::lcg64_next(&mut self.0, purrdf_testkit::rng::LCG64_MMIX_INCREMENT);
        (state >> 32) as u32
    }

    /// A value in `0..bound`, for a `bound` of at least one.
    pub(crate) fn below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }

    /// One chance in `out_of`.
    pub(crate) fn chance(&mut self, out_of: u32) -> bool {
        self.below(out_of) == 0
    }

    /// An integer in `-4..=4`: small, so generated positions coincide often enough
    /// to exercise the degenerate cases.
    pub(crate) fn small(&mut self) -> i64 {
        i64::from(self.below(9)) - 4
    }
}

/// A position of dimension `dim` with small integer ordinates.
pub(crate) fn coord(rng: &mut Lcg, dim: CoordDim) -> Coord {
    let x = Rat::from_i64(rng.small());
    let y = Rat::from_i64(rng.small());
    let z = dim.has_z().then(|| Rat::from_i64(rng.small()));
    let m = dim.has_m().then(|| Rat::from_i64(rng.small()));
    Coord::new(x, y, z, m)
}

fn coords(rng: &mut Lcg, dim: CoordDim, count: u32) -> CoordSeq {
    (0..count).map(|_| coord(rng, dim)).collect()
}

/// A line's positions: none, or two to four.
fn line(rng: &mut Lcg, dim: CoordDim) -> CoordSeq {
    if rng.chance(5) {
        CoordSeq::new()
    } else {
        let count = 2 + rng.below(3);
        coords(rng, dim, count)
    }
}

/// A closed ring of four to six positions.
fn ring(rng: &mut Lcg, dim: CoordDim) -> CoordSeq {
    let corners = 3 + rng.below(3);
    let mut ring = coords(rng, dim, corners);
    let first = ring[0].clone();
    ring.push(first);
    ring
}

/// A polygon's rings: none, or an exterior with up to one hole.
fn rings(rng: &mut Lcg, dim: CoordDim) -> Rings {
    if rng.chance(5) {
        Rings::new()
    } else {
        let holes = rng.below(2);
        (0..=holes).map(|_| ring(rng, dim)).collect()
    }
}

/// A point member: empty one time in six.
fn point(rng: &mut Lcg, dim: CoordDim) -> Option<Coord> {
    if rng.chance(6) {
        None
    } else {
        Some(coord(rng, dim))
    }
}

/// A geometry of dimension `dim` whose collections nest at most `depth` levels,
/// drawing every kind, empty and filled, with collections of up to three members.
pub(crate) fn geometry(rng: &mut Lcg, dim: CoordDim, depth: usize) -> Geometry {
    let choice = if depth > 0 && rng.chance(2) {
        6
    } else {
        rng.below(6)
    };
    let body = match choice {
        0 => GeometryBody::Point(point(rng, dim)),
        1 => GeometryBody::LineString(line(rng, dim)),
        2 => GeometryBody::Polygon(rings(rng, dim)),
        3 => GeometryBody::MultiPoint((0..rng.below(4)).map(|_| point(rng, dim)).collect()),
        4 => GeometryBody::MultiLineString((0..rng.below(3)).map(|_| line(rng, dim)).collect()),
        5 => GeometryBody::MultiPolygon((0..rng.below(3)).map(|_| rings(rng, dim)).collect()),
        _ => GeometryBody::GeometryCollection(
            (0..rng.below(4))
                .map(|_| geometry(rng, dim, depth - 1))
                .collect(),
        ),
    };
    Geometry::new(dim, body).expect("the generator writes well-formed bodies")
}

/// `geometry` wrapped in `levels` single-member collections of its own dimension,
/// built one level at a time.
pub(crate) fn nest(mut geometry: Geometry, levels: usize) -> Geometry {
    for _ in 0..levels {
        geometry = Geometry::new(
            geometry.dim(),
            GeometryBody::GeometryCollection(vec![geometry]),
        )
        .expect("a collection of one member shares its dimension");
    }
    geometry
}
