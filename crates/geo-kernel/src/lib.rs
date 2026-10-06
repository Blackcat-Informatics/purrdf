// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic geometry, topology, metrics and indexes shared by every PurRDF host.
//! Coordinates retain their exact rational value; evaluator adapters live in
//! `purrdf-sparql-eval`.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

mod de9im;
mod error;
mod numerical;

pub mod atlas;
pub mod binding;
pub mod buffer;
pub mod carrier;
pub mod cells;
pub mod construct;
pub mod context;
pub mod determinism;
pub mod ellipsoidal;
pub mod exact;
pub mod geodesic;
pub mod geographic;
pub mod geojson;
pub mod geom;
pub mod index;
pub mod json;
pub mod measure;
pub mod metric;
pub mod operation;
pub mod prepared;
pub mod profile;
pub mod relations;
pub mod topology;
pub mod vocab;
pub mod wkt;

pub use binding::{
    GeoProfile, GeoQueryIdentity, LinearUnitBinding, OperationBinding, ReferenceBinding,
    standard_vocabulary,
};
pub use buffer::OffsetRegion;
pub use context::{
    MaterializedOutputReceipt, MetricContext, MetricWorkContinuation, MetricWorkObserver,
    PreparationBudget, PreparedSourceReceipt,
};
pub use de9im::{Dim, IntersectionMatrix, Pattern, Set, Slot};
pub use ellipsoidal::{GeometryMetricEstimate, GeometryMetricLaw};
pub use error::GeoError;
pub use exact::{Int, Rat};
pub use geodesic::PreparedGeodesic;
pub use geographic::LonLat;
pub use geom::{
    Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral, Rings,
};
pub use json::JsonValue;
pub use metric::{Metres, MetricEstimate, MetricProofReceipt, SquareMetres, XsdDoubleMetres};
pub use operation::{
    CoordinateOperation, OperationChain, OperationImagePoint, OperationPoint, TransformOutputGrid,
    TransformResult,
};
pub use prepared::{
    AzimuthLengthArc, OrientedInterior, PreparedCoordinate, PreparedCurve, PreparedEdge,
    PreparedGeometry, PreparedPolygon, PreparedRegion, RegionInterior, ShortestGeodesicArc,
    SourceLinearEdge,
};
pub use profile::{
    AxisOrder, ExecutionLimits, ExecutionPolicy, ExecutionPolicyId, GeoBindingId,
    GeographicReference, ImplementationReceipt, PointLaw, PreparedEllipsoid, SemanticLawId,
};
pub use relations::{RelationFamily, SpatialRelation, transpose};
pub use topology::{
    SegmentIntersection, curve_boundary_points, has_area, intersect, locate, midpoint, on_segment,
    orientation, relate, relate_pattern, topological_dimension,
};
pub use vocab::{CrsUnit, DEFAULT_COORDINATE_SCALE, GeoTerm, GeoVocab, GeoVocabBuilder};
