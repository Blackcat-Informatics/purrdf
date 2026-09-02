// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GeoSPARQL 1.1 for PurRDF: exact, float-free geometry, reached from SPARQL
//! through the evaluator's two extension seams.
//!
//! This crate is a **sibling**, not a kernel change. It parses `geo:wktLiteral`
//! and `geo:geoJSONLiteral` lexical forms into an exact geometry model, decides
//! the OGC topological relations over that model, computes the non-topological
//! measures and constructors, and hands the whole `geof:` family to a host as
//! registrations against
//! [`UserFunctionRegistry`](purrdf_sparql_eval::UserFunctionRegistry) (scalar
//! functions) and
//! [`PropertyFunctionRegistry`](purrdf_sparql_eval::PropertyFunctionRegistry)
//! (the Query Rewrite relations). Nothing in `purrdf-core` or the evaluator
//! changes to make that work; both seams already existed.
//!
//! # It mints no vocabulary
//!
//! GeoSPARQL's IRIs are OGC's, not PurRDF's. Every IRI this crate reads or writes
//! — the two literal datatypes, the `geof:` function names, the `geo:` spatial
//! relations rewritten by the Query Rewrite relations, and the coordinate
//! reference system a WKT
//! literal omits — is supplied by the caller through [`GeoVocab`], which has no
//! `Default` and never will. A vocabulary term that is absent makes the feature
//! that needs it a hard error, never a fabricated fallback. Every fixture and
//! doctest in this crate uses `example.org`.
//!
//! # Every answer is exact, and identical on every target
//!
//! Geometry is where floating point normally destroys reproducibility: `f64`
//! addition is not associative, so a different traversal order gives a different
//! answer, and a native build and a `wasm32-unknown-unknown` build can disagree
//! about a predicate that sits near a boundary. This crate closes that channel
//! rather than mitigating it.
//!
//! * **Coordinates are read as exact rationals.** A lexical decimal
//!   (`-83.4`, `1.5e3`) is parsed digit by digit into an exact numerator and
//!   denominator by [`exact::Rat::parse_decimal`]. `str::parse::<f64>()` appears
//!   nowhere on the ingest path, so nothing is rounded on the way in.
//! * **Every geometric decision is integer arithmetic.** Orientation, segment
//!   intersection, point-in-ring, ring winding, and the DE-9IM matrix are
//!   comparisons of exact rationals over the arbitrary-precision integers in
//!   [`exact`]. Rust specifies integer arithmetic completely and identically on
//!   every target, so two targets cannot disagree.
//! * **Irrational measures are integer square roots.** A length is a sum of
//!   `sqrt` terms, and a sum of rounded terms depends on the rounding. So each
//!   segment's length is computed as an exact integer square root at a fixed
//!   internal scale and the terms are summed as integers — one rounding, at the
//!   end, of a value that was exact until then.
//! * **The single float boundary is the result literal.** An `xsd:double` result
//!   is produced by [`exact::Rat::to_f64`], which computes the correctly rounded
//!   nearest double with integer arithmetic and assembles it with
//!   [`f64::from_bits`]. The crate root denies `clippy::float_arithmetic`, so
//!   there is no second float path to find.
//!
//! # What is here, and what is not
//!
//! See [`docs/design/purrdf-geo-exactness.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-geo-exactness.md)
//! for the full accounting. In outline: the literal codecs, every topological
//! relation of the Simple Features, Egenhofer and RCC8 families, the accessors
//! and the exactly-computable measures and constructors are implemented. The
//! operations that would require facilities this crate deliberately does not have
//! — a coordinate-reference-system database for `geof:transform`, an ellipsoidal
//! geodesic for the `metric*` family — are registered and **hard-error by name**.
//! They are never silently absent and never answer a default: a topological
//! predicate that returned `false` because it was unimplemented would be
//! indistinguishable from one that returned `false` because the geometries
//! genuinely do not relate.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

// The surface is split on one line: the crate's *value types* are re-exported flat
// at the root, while its *operations* stay under the module that names them.
// `wkt::parse` and `geojson::parse` are two different functions with one obvious
// name, and `measure::area` says which kind of area it means where a bare `area`
// would not — so flattening those would either collide outright or drop the
// qualifier that makes the call site readable.
mod de9im;
mod error;

pub mod construct;
pub mod determinism;
pub mod exact;
pub mod functions;
pub mod geojson;
pub mod geom;
pub mod json;
pub mod measure;
pub mod relation;
pub mod relations;
pub mod topology;
pub mod vocab;
pub mod wkt;

pub use de9im::{Dim, IntersectionMatrix, Pattern, Set, Slot};
pub use error::GeoError;
pub use exact::{Int, Rat};
pub use geom::{
    Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral, Rings,
};
pub use json::JsonValue;
pub use relations::{RelationFamily, SpatialRelation, transpose};
pub use topology::{
    SegmentIntersection, curve_boundary_points, has_area, intersect, locate, midpoint, on_segment,
    orientation, relate, relate_pattern, topological_dimension,
};
pub use vocab::{CrsUnit, DEFAULT_COORDINATE_SCALE, GeoTerm, GeoVocab, GeoVocabBuilder};
