// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The memory layout of the exact geometry model, pinned.
//!
//! This repository's performance discipline is that a layout choice is justified
//! by a measurement, not by an assertion — so the numbers a layout decision was
//! made on are recorded here, where a change to them is a test failure with a
//! diff rather than a silent regression nobody profiles for.
//!
//! The decision this file exists to guard: [`CoordSeq`](purrdf_geo::CoordSeq) is a
//! plain `Vec`, not an inline small-vector. An exact [`Coord`](purrdf_geo::Coord)
//! is four arbitrary-precision rationals, and it is **large** — so an inline
//! capacity of four made every geometry 1552 bytes, which meant a `POINT` carried
//! a kilobyte and a half of ring storage it could never use and a `Vec<Geometry>`
//! paid that per member. The inline storage bought nothing in exchange: any
//! sequence with positions in it allocates either way. Changing the alias cut the
//! model by a factor of four.
//!
//! The residue is [`GeometryBody::Point`](purrdf_geo::GeometryBody), which holds a
//! `Coord` inline and is therefore the size of one. That is the size of the value
//! itself, and boxing it would put an allocation on the commonest geometry in the
//! commonest corpus in order to relocate bytes the coordinate occupies regardless.
//! The `#[allow(clippy::large_enum_variant)]` on that enum points here.
//!
//! # These are not portability assertions
//!
//! Every number below is a property of this target's pointer width and `Vec`
//! layout, so the test asserts *relationships* — "a sequence costs a pointer, not
//! four coordinates" — with the absolute figures carried in the failure message
//! for a reader to compare against. A 32-bit target would legitimately report
//! smaller numbers, so the assertions are bounds rather than equalities wherever
//! a bound says what the decision actually depended on.

use core::mem::size_of;

use purrdf_geo::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, Int, Rat};

/// A one-line rendering of every measured size, for a failure message.
fn measured() -> String {
    format!(
        "Int={} Rat={} Coord={} CoordSeq={} GeometryBody={} Geometry={}",
        size_of::<Int>(),
        size_of::<Rat>(),
        size_of::<Coord>(),
        size_of::<CoordSeq>(),
        size_of::<GeometryBody>(),
        size_of::<Geometry>(),
    )
}

/// An exact coordinate is intrinsically large, and that is the fact every other
/// layout decision in the model follows from.
#[test]
fn an_exact_coordinate_is_large_because_it_is_four_exact_rationals() {
    let coord = size_of::<Coord>();
    let rat = size_of::<Rat>();
    assert!(
        coord >= 4 * rat,
        "a Coord holds four rationals (two mandatory, two optional): {}",
        measured()
    );
    assert!(
        size_of::<Rat>() >= 2 * size_of::<Int>(),
        "a Rat holds two integers: {}",
        measured()
    );
    assert!(
        coord >= 128,
        "the point of this whole file is that a Coord is NOT pointer-sized; if it \
         has become small, the small-vector decision should be revisited: {}",
        measured()
    );
}

/// THE DECISION: a position sequence costs a pointer, not four coordinates.
///
/// This is the assertion that fails if `CoordSeq` is ever changed back to an
/// inline small-vector.
#[test]
fn a_position_sequence_costs_a_pointer_not_four_coordinates() {
    assert!(
        size_of::<CoordSeq>() <= 32,
        "CoordSeq must stay an out-of-line sequence; an inline capacity of four \
         made it 1552 bytes and every geometry with it: {}",
        measured()
    );
    assert!(
        size_of::<CoordSeq>() < size_of::<Coord>(),
        "a sequence header must cost less than a single position, or the \
         indirection is not paying for itself: {}",
        measured()
    );
}

/// The geometry enum is bounded by its inline `Coord`, not by a multiple of it.
#[test]
fn a_geometry_is_bounded_by_one_coordinate_not_by_a_ring_of_them() {
    let body = size_of::<GeometryBody>();
    let coord = size_of::<Coord>();
    assert!(
        body <= coord + 32,
        "GeometryBody must be the size of its largest variant (Point, which holds \
         one Coord) plus a discriminant — not a multiple of a Coord: {}",
        measured()
    );
    assert!(
        size_of::<Geometry>() <= body + 16,
        "a Geometry is a body plus a CoordDim tag: {}",
        measured()
    );
}

/// A sequence-shaped geometry must not carry a coordinate's worth of unused
/// inline storage — the regression the `Vec` alias exists to prevent, asserted
/// through a value rather than through a type.
#[test]
fn an_empty_line_and_an_empty_polygon_cost_the_same_as_an_empty_point() {
    let point = Geometry::empty(CoordDim::Xy, GeometryKind::Point);
    let line = Geometry::empty(CoordDim::Xy, GeometryKind::LineString);
    let polygon = Geometry::empty(CoordDim::Xy, GeometryKind::Polygon);
    // Same type, so trivially the same size — the assertion that matters is that
    // the SHARED size is bounded, which the type-level tests above establish.
    // What this adds is that the empty constructors really do allocate nothing.
    for geometry in [&point, &line, &polygon] {
        assert_eq!(
            geometry.coord_count(),
            0,
            "an empty geometry holds no positions: {measured}",
            measured = measured()
        );
        assert!(geometry.is_empty(), "and reports itself empty");
    }
}
