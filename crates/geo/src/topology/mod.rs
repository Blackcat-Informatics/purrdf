// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The topology engine: exact point location and the DE-9IM intersection matrix.
//!
//! Everything GeoSPARQL calls a *topological relation* is a pattern read off one
//! matrix, so this module tree has exactly one job — produce that matrix — and the
//! twenty-four relations in [`crate::relations`] are lookups against it. The
//! alternative, twenty-four hand-written predicates, would be twenty-four
//! independent chances to answer `false` for the wrong reason, and a wrong `false`
//! from a topological predicate is indistinguishable from a right one. There is
//! nothing downstream that can catch it, so it has to be impossible upstream.
//!
//! # Two contracts govern every line in here
//!
//! **Exactness.** Not "high precision" — exactness. Every question this module
//! answers is ultimately the *sign* of an arithmetic expression: which side of a
//! line a point falls on, whether a cross product is zero, whether a crossing
//! parameter lies in `[0, 1]`. A sign is a discrete answer read out of a
//! continuous computation, and that is exactly where floating point stops being an
//! approximation and becomes a different answer: a determinant that should be zero
//! evaluates to `-1.2e-17`, the sign flips, and the engine reports that a vertex
//! is not on its own edge. No tolerance repairs that in general — one large enough
//! to absorb the error merges genuinely distinct points, and one small enough to
//! keep them apart does not absorb it. So there is no floating point here at all:
//! the crate root denies `clippy::float_arithmetic`, every ordinate is an exact
//! [`crate::exact::Rat`], and integer arithmetic is fully specified by Rust on
//! every target — which is what makes a native answer and a
//! `wasm32-unknown-unknown` answer equal by construction rather than by luck.
//!
//! **Planarity.** OGC GeoSPARQL 1.1 Clause 10.2, verbatim: *"Geometric functions
//! working with Geometries that have Z values will ignore Z values in calculations
//! and first project geometry onto the Z=0 level. … Like Z values in coordinates,
//! M values are to be ignored."* So all topology here is planar. No function in
//! this module tree reads [`crate::geom::Coord::z`] or
//! [`crate::geom::Coord::m`], every coordinate is projected onto its `(x, y)` pair
//! on the way in, and every coordinate any of these functions *returns* is
//! [`crate::geom::CoordDim::Xy`]. That is a simplification the specification hands
//! us, and taking it is what keeps the engine from having to define a
//! three-dimensional interior nobody asked for.
//!
//! # Determinism
//!
//! No hash iteration reaches a result. Every collection that feeds an answer is
//! sorted with a total order derived from [`crate::exact::Rat`]'s [`Ord`] — the
//! lexicographic `(x, y)` order for positions, the natural order for ordinates —
//! and [`crate::de9im::IntersectionMatrix::raise`] is a maximum, so the
//! accumulation of witnesses cannot depend on the order they are visited in. The
//! matrix is a pure function of the two geometries.
//!
//! # The three files
//!
//! * `segment` — the exact primitives: orientation, incidence, midpoint, and the
//!   intersection of two closed segments including every degenerate shape.
//! * [`locate()`] — which of a geometry's three point-sets a position belongs to,
//!   kind by kind, with the OGC boundary definitions spelled out.
//! * [`relate()`] — the noder, the witness passes and the exact horizontal scan line
//!   that together decide all nine entries.

mod locate;
mod relate;
mod segment;

pub use locate::{curve_boundary_points, has_area, locate, topological_dimension};
pub use relate::{relate, relate_pattern};
pub use segment::{SegmentIntersection, intersect, midpoint, on_segment, orientation};
