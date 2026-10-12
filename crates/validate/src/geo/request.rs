// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed version-one requests; exact input survives every host boundary.

use purrdf_core::TermValue;
use purrdf_geo_kernel::{
    Crs, LonLat, Metres, XsdDoubleMetres,
    cells::{CellId, GridProfileId, NativeGridProfile},
};
use purrdf_hash::hex::Digest32;
use purrdf_iri::vocab::ogc;
use purrdf_lex::json::{
    Value,
    record::{DecodeError, Record},
};
use purrdf_sparql_eval::geo::functions::GeofFunction;

use super::{GeoCallError, decode::Decode, profile::fixed_hex};

/// An exact validated point, with IEEE dyadics retained when explicitly supplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointInput(pub LonLat);

impl PointInput {
    /// Decode a strict point while retaining typed coordinate refusals.
    ///
    /// # Errors
    ///
    /// Refuses invalid records and nonfinite or out-of-range original values.
    pub fn from_value(value: &Value) -> Result<Self, GeoCallError> {
        super::decode::value(value, Self::from_value_native)
    }

    pub(super) fn from_value_native(
        value: &Value,
        decode: &Decode<'_, '_, '_, '_>,
    ) -> Result<Self, GeoCallError> {
        let mut fields = decode.record(value, "exact geographic point")?;
        let encoding: Option<String> = fields.optional_native("encoding")?;
        let point = match encoding.as_deref().unwrap_or("decimal") {
            "decimal" => decode.location(
                fields.required_with("longitude", |value| decode.decimal(value))?,
                fields.required_with("latitude", |value| decode.decimal(value))?,
            ),
            "ieee64" => {
                let longitude =
                    fields.required_with("longitude", |value| fixed_hex::<8>(value, decode))?;
                let latitude =
                    fields.required_with("latitude", |value| fixed_hex::<8>(value, decode))?;
                decode.binary_location(
                    f64::from_bits(u64::from_be_bytes(longitude)),
                    f64::from_bits(u64::from_be_bytes(latitude)),
                )
            }
            other => return Err(decode.unknown_variant(other, &["decimal", "ieee64"]).into()),
        }?;
        fields.deny_unknown()?;
        Ok(Self(point))
    }
}

/// One scalar or batch operation of the shared engine.
#[derive(Debug)]
pub enum GeoRequest {
    /// Certified membership in a physical offset of the complete prepared source.
    OffsetContains {
        /// Explicit reference for the geographic point.
        crs: Crs,
        /// Original source, retaining its selected carrier or explicit edge laws.
        geometry: super::GeometryInput,
        /// Exact physical radius; negative radii define the empty offset.
        radius: Metres,
        /// Original exact geographic point in the declared reference.
        point: PointInput,
    },
    /// Certified outward polygon materialization of a complete point offset.
    BufferPoints {
        /// Explicit target geographic carrier reference.
        crs: Crs,
        /// Original point collection, including continuous symbolic images.
        geometry: super::GeometryInput,
        /// Exact physical radius; negative gives empty, zero source closure.
        radius: Metres,
    },
    /// Certified outward polygon materialization of the complete source offset.
    Buffer {
        /// Explicit target geographic carrier reference.
        crs: Crs,
        /// Complete original prepared source set and selected curve laws.
        geometry: super::GeometryInput,
        /// Exact physical radius; negative gives empty, zero source closure.
        radius: Metres,
    },
    /// Complete fixed or mixed conservative closed-shape cover.
    Cover(super::CoverRequest),
    /// Inspect a compiled named operation's exact chain identity.
    Operation {
        /// Exact caller-supplied operation dispatch IRI.
        name: Crs,
    },
    /// Transform original source ordinates with an explicit named chain.
    Transform {
        /// Exact caller-supplied operation dispatch IRI.
        name: Crs,
        /// Original exact geographic or operation point.
        point: purrdf_geo_kernel::OperationPoint,
        /// Declared angular and metre output quanta, admitted by the Rust engine.
        grid: purrdf_geo_kernel::TransformOutputGrid,
    },
    /// Materialize a complete continuous original carrier image through a named chain.
    TransformGeometry {
        /// Exact registered operation dispatch name, which fixes the target carrier.
        name: Crs,
        /// Original typed WKT or GeoJSON carrier, preserving source coordinates.
        geometry: TermValue,
        /// Exact supplied observation epoch when the model requires rates.
        epoch: Option<purrdf_geo_kernel::Rat>,
    },
    /// Transform a complete original-coordinate batch through a shared chain.
    TransformBatch {
        /// Exact caller-supplied operation dispatch IRI.
        name: Crs,
        /// Complete original-coordinate batch in caller order.
        points: Vec<purrdf_geo_kernel::OperationPoint>,
        /// Shared declared output quanta for the complete original batch.
        grid: purrdf_geo_kernel::TransformOutputGrid,
    },
    /// Inspect the effective immutable reference, law and policy identities.
    Profile,
    /// Correctly rounded shortest point distance in a declared reference.
    Distance {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// First original exact point.
        a: PointInput,
        /// Second original exact point.
        b: PointInput,
        /// Include the separate invocation enclosure receipt.
        proof: bool,
    },
    /// A complete distance batch with one context and caller-buffer computation.
    DistanceBatch {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// Complete input pairs in caller order.
        pairs: Vec<(PointInput, PointInput)>,
    },
    /// Canonical shortest-branch inverse metadata and optional separate proof.
    Inverse {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// First original exact point.
        a: PointInput,
        /// Second original exact point.
        b: PointInput,
        /// Include invocation enclosures separately from completed output.
        proof: bool,
    },
    /// Complete canonical inverse metadata batch in caller order.
    InverseBatch {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// Complete original exact point pairs.
        pairs: Vec<(PointInput, PointInput)>,
    },
    /// Compare the public reported binary64 distance and promoted threshold.
    Within {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// First original exact point.
        a: PointInput,
        /// Second original exact point.
        b: PointInput,
        /// Threshold under the explicitly selected comparison law.
        threshold: XsdDoubleMetres,
    },
    /// Compare the unrounded physical distance and exact threshold.
    WithinPhysical {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// First original exact point.
        a: PointInput,
        /// Second original exact point.
        b: PointInput,
        /// Threshold under the explicitly selected comparison law.
        threshold: Metres,
    },
    /// Direct propagation using an explicit or the frozen default angular grid.
    Direct {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// Original exact starting point.
        start: PointInput,
        /// Exact clockwise azimuth in degrees.
        azimuth: purrdf_geo_kernel::Rat,
        /// Exact nonnegative propagation length in metres.
        length: Metres,
        /// Explicit half-even angular quantum; fifteen places by default.
        grid: purrdf_geo_kernel::geodesic::DirectOutputGrid,
    },
    /// Complete caller-buffer direct propagation batch.
    DirectBatch {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// Exact original start, clockwise azimuth and length tuples.
        inputs: Vec<(PointInput, purrdf_geo_kernel::Rat, Metres)>,
        /// One explicit angular grid shared by the complete batch.
        grid: purrdf_geo_kernel::geodesic::DirectOutputGrid,
    },
    /// Fixed-grid materialization of a certified unique original-endpoint arc.
    ShortestArcAt {
        /// Exact declared geographic carrier reference IRI.
        crs: Crs,
        /// Original first endpoint.
        a: PointInput,
        /// Original second endpoint.
        b: PointInput,
        /// Exact closed unit-interval arc parameter.
        parameter: purrdf_geo_kernel::Rat,
    },
    /// Shared standard scalar geometry functions, with original carrier terms.
    Geometry {
        /// The shared standard scalar function inventory member.
        function: GeofFunction,
        /// Original RDF argument terms in source order.
        arguments: Vec<TermValue>,
    },
    /// A certified SI metric over the complete source-linear carrier geometry.
    GeometryMetric {
        /// The shared geographic approximation law (length, perimeter or area).
        metric: purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw,
        /// Original WKT or GeoJSON RDF literal, preserving all source ordinates.
        geometry: super::GeometryInput,
    },
    /// Certified minimum surface distance between two complete carrier geometries.
    GeometryDistance {
        /// Complete original source-linear first carrier geometry.
        a: super::GeometryInput,
        /// Complete original source-linear second carrier geometry.
        b: super::GeometryInput,
    },
    /// Exact physical DE-9IM matrix between two complete prepared sources.
    GeometryRelate {
        /// Complete original carrier, explicit prepared or continuous-image source.
        a: super::GeometryInput,
        /// Complete original carrier, explicit prepared or continuous-image source.
        b: super::GeometryInput,
    },
    /// Deterministic integer-only geographic cell assignment.
    Cell {
        /// Explicit native geographic grid profile.
        grid: NativeGridProfile,
        /// Original exact geographic or operation point.
        point: PointInput,
        /// Hierarchy resolution from zero through thirty.
        level: u8,
    },
    /// Deterministic integer-only complete assignment batch.
    CellBatch {
        /// Explicit native geographic grid profile.
        grid: NativeGridProfile,
        /// Complete original-coordinate batch in caller order.
        points: Vec<PointInput>,
        /// Hierarchy resolution from zero through thirty.
        level: u8,
    },
    /// A cell's exact hierarchy, byte order and stored-level range.
    CellHierarchy {
        /// Validated profile-qualified cell identifier.
        cell: CellId,
        /// Optional requested ancestor level.
        ancestor: Option<u8>,
        /// External-store resolution of the descendant range.
        stored_level: u8,
    },
    /// Certified physical cell bounds and optional maximum-edge level selection.
    CellScale {
        /// Explicit native geographic grid profile.
        grid: NativeGridProfile,
        /// Hierarchy resolution from zero through thirty.
        level: u8,
        /// Optional strictly positive maximum edge target in metres.
        maximum_edge: Option<Metres>,
    },
}

impl GeoRequest {
    /// The borrowed dispatch/reference identifier stays live throughout the call.
    /// Capacity is ownership; logical byte visits use its actual spelling length.
    pub(super) fn reference_admission(
        &self,
    ) -> Result<Option<(u64, u64)>, purrdf_geo_kernel::GeoError> {
        let reference = match self {
            Self::Operation { name }
            | Self::Transform { name, .. }
            | Self::TransformGeometry { name, .. }
            | Self::TransformBatch { name, .. } => name,
            Self::OffsetContains { crs, .. }
            | Self::BufferPoints { crs, .. }
            | Self::Buffer { crs, .. }
            | Self::Distance { crs, .. }
            | Self::DistanceBatch { crs, .. }
            | Self::Inverse { crs, .. }
            | Self::InverseBatch { crs, .. }
            | Self::Within { crs, .. }
            | Self::WithinPhysical { crs, .. }
            | Self::Direct { crs, .. }
            | Self::DirectBatch { crs, .. }
            | Self::ShortestArcAt { crs, .. } => crs,
            _ => return Ok(None),
        };
        reference_allowance(reference).map(Some)
    }

    /// Typed source collections remain live alongside their native output.
    /// Read only their length/capacity metadata before any allocation or walk.
    pub(super) fn collection_admission(
        &self,
    ) -> Result<Option<(u64, u64)>, purrdf_geo_kernel::GeoError> {
        let (count, capacity, width) = match self {
            Self::TransformBatch { points, .. } => (
                points.len(),
                points.capacity(),
                size_of::<purrdf_geo_kernel::OperationPoint>(),
            ),
            Self::DistanceBatch { pairs, .. } | Self::InverseBatch { pairs, .. } => (
                pairs.len(),
                pairs.capacity(),
                size_of::<(PointInput, PointInput)>(),
            ),
            Self::DirectBatch { inputs, .. } => (
                inputs.len(),
                inputs.capacity(),
                size_of::<(PointInput, purrdf_geo_kernel::Rat, Metres)>(),
            ),
            Self::Geometry { arguments, .. } => (
                arguments.len(),
                arguments.capacity(),
                size_of::<TermValue>(),
            ),
            Self::CellBatch { points, .. } => {
                (points.len(), points.capacity(), size_of::<PointInput>())
            }
            _ => return Ok(None),
        };
        let bytes = (capacity as u64).checked_mul(width as u64).ok_or(
            purrdf_geo_kernel::GeoError::ArithmeticOverflow("typed geographic source collection"),
        )?;
        Ok(Some((count as u64, bytes)))
    }

    /// Parse a strict version-one request, preserving decimal and IEEE inputs.
    ///
    /// # Errors
    ///
    /// Refuses malformed JSON, duplicate or unknown members and invalid inputs.
    pub fn parse(text: &str) -> Result<Self, GeoCallError> {
        text.parse()
    }

    pub(super) fn from_value_native(
        value: &Value,
        decode: &Decode<'_, '_, '_, '_>,
    ) -> Result<Self, GeoCallError> {
        let mut fields = decode.record(value, "geographic request version one")?;
        let version: u32 = fields.required_native("version")?;
        if version != 1 {
            return Err(decode.custom("geographic request version must be 1").into());
        }
        let operation = fields.tag(
            "operation",
            &[
                "cover-disk",
                "cover-reported",
                "cover-box",
                "cover-region",
                "profile",
                "distance",
                "distance-batch",
                "inverse",
                "inverse-batch",
                "within",
                "within-physical",
                "direct",
                "direct-batch",
                "shortest-arc-at",
                "geometry",
                "geometry-metric",
                "geometry-distance",
                "geometry-relate",
                "offset-contains",
                "buffer-points",
                "buffer",
                "cell",
                "cell-batch",
                "cell-hierarchy",
                "cell-scale",
                "operation",
                "transform",
                "transform-batch",
                "transform-geometry",
            ],
        )?;
        let request = match operation {
            "offset-contains" => Self::OffsetContains {
                crs: crs(&mut fields, decode)?,
                geometry: fields.required_with("geometry", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
                radius: Metres::new(
                    fields.required_with("radius_metres", |value| decode.decimal(value))?,
                ),
                point: fields.required_with("point", |value| {
                    PointInput::from_value_native(value, decode)
                })?,
            },
            "buffer-points" => Self::BufferPoints {
                crs: crs(&mut fields, decode)?,
                geometry: fields.required_with("geometry", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
                radius: Metres::new(
                    fields.required_with("radius_metres", |value| decode.decimal(value))?,
                ),
            },
            "buffer" => Self::Buffer {
                crs: crs(&mut fields, decode)?,
                geometry: fields.required_with("geometry", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
                radius: Metres::new(
                    fields.required_with("radius_metres", |value| decode.decimal(value))?,
                ),
            },
            "cover-disk" | "cover-reported" | "cover-box" | "cover-region" => {
                Self::Cover(super::cover::decode(&mut fields, operation, decode)?)
            }
            "operation" => Self::Operation {
                name: name(&mut fields, decode)?,
            },
            "transform" => Self::Transform {
                name: name(&mut fields, decode)?,
                point: fields
                    .required_with("point", |value| super::operation::point(value, decode))?,
                grid: transform_grid(&mut fields, decode)?,
            },
            "transform-geometry" => Self::TransformGeometry {
                name: name(&mut fields, decode)?,
                geometry: fields.required_with("geometry", |value| term(value, decode))?,
                epoch: fields.optional_with("epoch_decimal_year", |value| decode.decimal(value))?,
            },
            "transform-batch" => Self::TransformBatch {
                name: name(&mut fields, decode)?,
                points: fields.required_with("points", |value| {
                    decode.items(value, |value| super::operation::point(value, decode))
                })?,
                grid: transform_grid(&mut fields, decode)?,
            },
            "profile" => Self::Profile,
            "distance" => Self::Distance {
                crs: crs(&mut fields, decode)?,
                a: fields
                    .required_with("a", |value| PointInput::from_value_native(value, decode))?,
                b: fields
                    .required_with("b", |value| PointInput::from_value_native(value, decode))?,
                proof: fields.optional_native("proof")?.unwrap_or(false),
            },
            "distance-batch" => Self::DistanceBatch {
                crs: crs(&mut fields, decode)?,
                pairs: fields.required_with("pairs", |value| {
                    decode.items(value, |value| pair(value, decode))
                })?,
            },
            "inverse" => Self::Inverse {
                crs: crs(&mut fields, decode)?,
                a: fields
                    .required_with("a", |value| PointInput::from_value_native(value, decode))?,
                b: fields
                    .required_with("b", |value| PointInput::from_value_native(value, decode))?,
                proof: fields.optional_native("proof")?.unwrap_or(false),
            },
            "inverse-batch" => Self::InverseBatch {
                crs: crs(&mut fields, decode)?,
                pairs: fields.required_with("pairs", |value| {
                    decode.items(value, |value| pair(value, decode))
                })?,
            },
            "within" => Self::Within {
                crs: crs(&mut fields, decode)?,
                a: fields
                    .required_with("a", |value| PointInput::from_value_native(value, decode))?,
                b: fields
                    .required_with("b", |value| PointInput::from_value_native(value, decode))?,
                threshold: fields
                    .required_with("threshold_metres", |value| threshold(value, decode))?,
            },
            "within-physical" => Self::WithinPhysical {
                crs: crs(&mut fields, decode)?,
                a: fields
                    .required_with("a", |value| PointInput::from_value_native(value, decode))?,
                b: fields
                    .required_with("b", |value| PointInput::from_value_native(value, decode))?,
                threshold: Metres::new(
                    fields.required_with("threshold_metres", |value| decode.decimal(value))?,
                ),
            },
            "direct" => Self::Direct {
                crs: crs(&mut fields, decode)?,
                start: fields.required_with("start", |value| {
                    PointInput::from_value_native(value, decode)
                })?,
                azimuth: fields.required_with("azimuth_degrees", |value| decode.decimal(value))?,
                length: Metres::new(
                    fields.required_with("length_metres", |value| decode.decimal(value))?,
                ),
                grid: purrdf_geo_kernel::geodesic::DirectOutputGrid::new(
                    fields
                        .optional_native("angular_decimal_places")?
                        .unwrap_or(15),
                ),
            },
            "direct-batch" => Self::DirectBatch {
                crs: crs(&mut fields, decode)?,
                inputs: fields.required_with("inputs", |value| {
                    decode.items(value, |value| direct_input(value, decode))
                })?,
                grid: purrdf_geo_kernel::geodesic::DirectOutputGrid::new(
                    fields
                        .optional_native("angular_decimal_places")?
                        .unwrap_or(15),
                ),
            },
            "shortest-arc-at" => Self::ShortestArcAt {
                crs: crs(&mut fields, decode)?,
                a: fields
                    .required_with("a", |value| PointInput::from_value_native(value, decode))?,
                b: fields
                    .required_with("b", |value| PointInput::from_value_native(value, decode))?,
                parameter: fields.required_with("parameter", |value| decode.decimal(value))?,
            },
            "geometry" => {
                let name: String = fields.required_native("function")?;
                let function = GeofFunction::from_local_name(&name).ok_or_else(|| {
                    decode.custom(format_args!("unknown standard geometry function `{name}`"))
                })?;
                Self::Geometry {
                    function,
                    arguments: fields.required_with("arguments", |value| {
                        decode.items(value, |value| term(value, decode))
                    })?,
                }
            }
            "geometry-metric" => Self::GeometryMetric {
                metric: match fields.tag(
                    "metric",
                    &["length", "perimeter", "area", "geodesic-area-integral"],
                )? {
                    "length" => purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Length,
                    "perimeter" => purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Perimeter,
                    "geodesic-area-integral" => {
                        purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::GeodesicAreaIntegral
                    }
                    _ => purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Area,
                },
                geometry: fields.required_with("geometry", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
            },
            "geometry-distance" => Self::GeometryDistance {
                a: fields.required_with("a", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
                b: fields.required_with("b", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
            },
            "geometry-relate" => Self::GeometryRelate {
                a: fields.required_with("a", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
                b: fields.required_with("b", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?,
            },
            "cell" => Self::Cell {
                grid: grid(&mut fields)?,
                point: fields.required_with("point", |value| {
                    PointInput::from_value_native(value, decode)
                })?,
                level: fields.required_native("level")?,
            },
            "cell-batch" => Self::CellBatch {
                grid: grid(&mut fields)?,
                points: fields.required_with("points", |value| {
                    decode.items(value, |value| PointInput::from_value_native(value, decode))
                })?,
                level: fields.required_native("level")?,
            },
            "cell-hierarchy" => Self::CellHierarchy {
                cell: fields.required_with("cell", |value| cell(value, decode))?,
                ancestor: fields.optional_native("ancestor")?,
                stored_level: fields.optional_native("stored_level")?.unwrap_or(30),
            },
            _ => Self::CellScale {
                grid: grid(&mut fields)?,
                level: fields.required_native("level")?,
                maximum_edge: fields
                    .optional_with("maximum_edge_metres", |value| decode.decimal(value))?
                    .map(Metres::new),
            },
        };
        fields.deny_unknown()?;
        Ok(request)
    }
}

/// Source identifier metadata uses the carrier's one byte/copy allowance home.
pub(super) fn reference_allowance(
    reference: &Crs,
) -> Result<(u64, u64), purrdf_geo_kernel::GeoError> {
    let (work, storage) = purrdf_geo_kernel::carrier::InputAdmission::text_allowance(
        reference.as_str().len() as u64,
        reference.retained_text_bytes() as u64,
    )?;
    Ok((
        work.checked_add(1)
            .ok_or(purrdf_geo_kernel::GeoError::ArithmeticOverflow(
                "source identifier work",
            ))?,
        storage,
    ))
}

fn name(fields: &mut Record<'_>, decode: &Decode<'_, '_, '_, '_>) -> Result<Crs, GeoCallError> {
    let name = fields.required_with("name", |value| decode.crs(value))?;
    Ok(name)
}

pub(super) fn crs(
    fields: &mut Record<'_>,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Crs, GeoCallError> {
    let crs = fields.optional_with("crs", |value| decode.crs(value))?;
    crs.map_or_else(|| decode.crs_text(ogc::CRS84), Ok)
}

pub(super) fn grid(fields: &mut Record<'_>) -> Result<NativeGridProfile, DecodeError> {
    Ok(match fields.tag("grid", &["wgs84", "cgcs2000"])? {
        "wgs84" => NativeGridProfile::Wgs84,
        _ => NativeGridProfile::Cgcs2000,
    })
}

fn pair(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<(PointInput, PointInput), GeoCallError> {
    let mut fields = decode.record(value, "point-distance pair")?;
    let pair = (
        fields.required_with("a", |value| PointInput::from_value_native(value, decode))?,
        fields.required_with("b", |value| PointInput::from_value_native(value, decode))?,
    );
    fields.deny_unknown()?;
    Ok(pair)
}

fn direct_input(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<(PointInput, purrdf_geo_kernel::Rat, Metres), GeoCallError> {
    let mut fields = decode.record(value, "direct geodesic input")?;
    let input = (
        fields.required_with("start", |value| {
            PointInput::from_value_native(value, decode)
        })?,
        fields.required_with("azimuth_degrees", |value| decode.decimal(value))?,
        Metres::new(fields.required_with("length_metres", |value| decode.decimal(value))?),
    );
    fields.deny_unknown()?;
    Ok(input)
}

pub(super) fn threshold(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<XsdDoubleMetres, GeoCallError> {
    let mut fields = decode.record(value, "promoted distance threshold")?;
    let kind = fields.tag("kind", &["double-bits", "decimal", "integer"])?;
    let threshold = match kind {
        "double-bits" => XsdDoubleMetres::new(f64::from_bits(u64::from_be_bytes(
            fields.required_with("value", |value| fixed_hex::<8>(value, decode))?,
        )))
        .map_err(GeoCallError::from),
        _ => fields.required_with("value", |value| {
            let text = value
                .as_str()
                .ok_or_else(|| decode.invalid_type(value, "numeric threshold string"))?;
            decode.threshold(
                text,
                if kind == "decimal" {
                    purrdf_xsd::XsdDatatype::Decimal
                } else {
                    purrdf_xsd::XsdDatatype::Integer
                },
            )
        }),
    }?;
    fields.deny_unknown()?;
    Ok(threshold)
}

fn cell(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<CellId, GeoCallError> {
    let mut fields = decode.record(value, "profile-qualified cell identifier")?;
    let profile = fields.required_with("profile", |value| fixed_hex::<32>(value, decode))?;
    let key = fields.required_with("key", |value| fixed_hex::<8>(value, decode))?;
    let cell = CellId::from_key(
        GridProfileId::from_digest(Digest32::new(profile)),
        u64::from_be_bytes(key),
    )?;
    let face: Option<u8> = fields.optional_native("face")?;
    let level: Option<u8> = fields.optional_native("level")?;
    let bytes = fields.optional_with("big_endian", |value| fixed_hex::<8>(value, decode))?;
    let min = fields.optional_with("range_min", |value| fixed_hex::<8>(value, decode))?;
    let max = fields.optional_with("range_max", |value| fixed_hex::<8>(value, decode))?;
    if face.is_some_and(|face| face != cell.face())
        || level.is_some_and(|level| level != cell.level())
        || bytes.is_some_and(|bytes| bytes != cell.to_be_bytes())
        || min.is_some_and(|min| u64::from_be_bytes(min) != cell.range_min())
        || max.is_some_and(|max| u64::from_be_bytes(max) != cell.range_max())
    {
        return Err(decode
            .custom("cell metadata differs from its validated identifier")
            .into());
    }
    fields.deny_unknown()?;
    Ok(cell)
}

pub(super) fn term(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<TermValue, GeoCallError> {
    let mut fields = decode.record(value, "geometry function argument term")?;
    let kind = fields.tag("kind", &["iri", "literal"])?;
    let lexical: String = fields.required_native("value")?;
    let term = if kind == "iri" {
        TermValue::Iri(lexical)
    } else {
        let datatype: Option<String> = fields.optional_native("datatype")?;
        let language: Option<String> = fields.optional_native("language")?;
        let direction = fields.optional_with("direction", |value| {
            value
                .as_str()
                .and_then(purrdf_core::RdfTextDirection::from_str_token)
                .ok_or_else(|| decode.invalid_value(value, "RDF base direction ltr or rtl"))
        })?;
        let literal = purrdf_core::RdfLiteral {
            lexical_form: lexical,
            datatype,
            language,
            direction,
        };
        purrdf_core::RdfLiteral::validate_components(
            literal.datatype_iri(),
            literal.language.as_deref(),
            direction,
        )
        .map_err(|error| decode.custom(error))?;
        decode.literal(literal)?
    };
    fields.deny_unknown()?;
    Ok(term)
}

impl std::str::FromStr for GeoRequest {
    type Err = GeoCallError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        super::decode::record(
            text,
            purrdf_geo_kernel::ExecutionPolicy::geometry(),
            None,
            Self::from_value_native,
        )
        .map(|(request, _)| request)
    }
}

fn transform_grid(
    fields: &mut Record<'_>,
    _decode: &Decode<'_, '_, '_, '_>,
) -> Result<purrdf_geo_kernel::TransformOutputGrid, GeoCallError> {
    Ok(purrdf_geo_kernel::TransformOutputGrid::new(
        fields
            .optional_native("angular_decimal_places")?
            .unwrap_or(15),
        fields
            .optional_native("metric_decimal_places")?
            .unwrap_or(6),
    ))
}
