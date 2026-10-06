// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical certificates, exact quantities and typed refusals.

use purrdf_core::TermValue;
use purrdf_geo_kernel::{
    GeoError, MetricEstimate, MetricProofReceipt, Rat, binding::GeoQueryIdentity, cells::CellId,
};
use purrdf_hash::hex;
use purrdf_lex::json::{Object, Value};

use super::GeoCallError;

/// The complete version-one identity record, shared with output admission.
pub(super) const IDENTITY_MEMBER_COUNT: u64 = 16;

pub(super) fn identity(identity: &GeoQueryIdentity) -> Value {
    Object::new()
        .with("distance_law", identity.distance_law.digest().to_string())
        .with("inverse_law", identity.inverse_law.digest().to_string())
        .with("direct_law", identity.direct_law.digest().to_string())
        .with(
            "geometry_distance_law",
            identity.geometry_distance_law.digest().to_string(),
        )
        .with("length_law", identity.length_law.digest().to_string())
        .with("perimeter_law", identity.perimeter_law.digest().to_string())
        .with("area_law", identity.area_law.digest().to_string())
        .with(
            "geodesic_area_integral_law",
            identity.geodesic_area_integral_law.digest().to_string(),
        )
        .with("topology_law", identity.topology_law.digest().to_string())
        .with(
            "image_materialization_law",
            identity.image_materialization_law.digest().to_string(),
        )
        .with(
            "point_buffer_materialization_law",
            identity
                .point_buffer_materialization_law
                .digest()
                .to_string(),
        )
        .with("binding", identity.binding.digest().to_string())
        .with(
            "buffer_materialization_law",
            identity.buffer_materialization_law.digest().to_string(),
        )
        .with(
            "global_point_buffer_materialization_law",
            identity
                .global_point_buffer_materialization_law
                .digest()
                .to_string(),
        )
        .with(
            "global_buffer_materialization_law",
            identity
                .global_buffer_materialization_law
                .digest()
                .to_string(),
        )
        .with("policy", identity.policy.digest().to_string())
        .into()
}

pub(super) fn geometry_metric(
    estimate: &purrdf_geo_kernel::ellipsoidal::GeometryMetricEstimate,
) -> Value {
    let area = estimate.is_area();
    Object::new()
        .with(
            "value",
            estimate.exact().to_decimal_string(if area { 2 } else { 6 }),
        )
        .with("unit", if area { "square-metres" } else { "metres" })
        .with(
            "reported_double_bits",
            key(estimate.reported_double().to_bits()),
        )
        .with("error_bound", upper_decimal(&estimate.error_bound()))
        .with("law", estimate.law_id().digest().to_string())
        .with("binding", estimate.binding_id().digest().to_string())
        .with("certificate", hex::encode(&estimate.certificate_bytes()))
        .into()
}

pub(super) fn metric(estimate: &MetricEstimate) -> Value {
    Object::new()
        .with("metres", estimate.value().exact().to_decimal_string(6))
        .with("micrometres", estimate.quantized_micrometres().to_string())
        .with(
            "reported_double_bits",
            key(estimate.reported_double().to_bits()),
        )
        .with("rounding_bound_metres", "0.0000005")
        .with(
            "host_conversion_bound_metres",
            upper_decimal(estimate.host_conversion_bound().exact()),
        )
        .with("law", estimate.law_id().digest().to_string())
        .with("binding", estimate.binding_id().digest().to_string())
        .with("certificate", hex::encode(&estimate.certificate_bytes()))
        .into()
}

pub(super) fn inverse(answer: &purrdf_geo_kernel::geodesic::InverseResult) -> Value {
    Object::new()
        .with(
            "branch_multiplicity",
            match answer.branch_multiplicity() {
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Unique => "unique",
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Two => "two",
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Continuum => "continuum",
            },
        )
        .with("distance", metric(answer.distance()))
        .with(
            "forward_azimuth_degrees",
            answer
                .forward_azimuth()
                .map(|value| value.to_decimal_string(15)),
        )
        .with(
            "final_azimuth_degrees",
            answer
                .final_azimuth()
                .map(|value| value.to_decimal_string(15)),
        )
        .with("arc_degrees", answer.arc_degrees().to_decimal_string(15))
        .with(
            "reduced_length_metres",
            answer.reduced_length().exact().to_decimal_string(6),
        )
        .with("scale12", answer.scale12().to_decimal_string(15))
        .with("scale21", answer.scale21().to_decimal_string(15))
        .with(
            "geodesic_quadrilateral_area_square_metres",
            answer
                .geodesic_quadrilateral_area()
                .exact()
                .to_decimal_string(2),
        )
        .with(
            "pole_cuts",
            Value::Array(
                answer
                    .pole_cuts()
                    .iter()
                    .map(|(latitude, from, to)| {
                        Object::new()
                            .with("latitude", latitude.to_decimal_string(15))
                            .with("from_longitude", from.to_decimal_string(15))
                            .with("to_longitude", to.to_decimal_string(15))
                            .into()
                    })
                    .collect(),
            ),
        )
        .with("law", answer.law_id().digest().to_string())
        .with("binding", answer.binding_id().digest().to_string())
        .with("certificate", hex::encode(&answer.certificate_bytes()))
        .into()
}

pub(super) fn direct(answer: &purrdf_geo_kernel::geodesic::DirectResult) -> Value {
    Object::new()
        .with(
            "longitude",
            answer
                .endpoint()
                .longitude()
                .to_decimal_string(answer.output_grid().decimal_places()),
        )
        .with(
            "latitude",
            answer
                .endpoint()
                .latitude()
                .to_decimal_string(answer.output_grid().decimal_places()),
        )
        .with(
            "final_azimuth_degrees",
            answer
                .final_azimuth()
                .to_decimal_string(answer.output_grid().decimal_places()),
        )
        .with(
            "surface_error_bound_metres",
            upper_decimal(answer.surface_error_bound().exact()),
        )
        .with("law", answer.law_id().digest().to_string())
        .with("binding", answer.binding_id().digest().to_string())
        .with("certificate", hex::encode(&answer.certificate_bytes()))
        .into()
}

fn enclosure(lower: &Rat, upper: &Rat) -> Value {
    Object::new()
        .with("lower", lower_decimal(lower))
        .with("upper", upper_decimal(upper))
        .into()
}

pub(super) fn inverse_proof(receipt: &purrdf_geo_kernel::geodesic::InverseProofReceipt) -> Value {
    Object::new()
        .with("distance", proof(&receipt.distance))
        .with(
            "forward_azimuth_degrees",
            receipt
                .forward_azimuth
                .as_ref()
                .map(|(a, b)| enclosure(a, b)),
        )
        .with(
            "final_azimuth_degrees",
            receipt.final_azimuth.as_ref().map(|(a, b)| enclosure(a, b)),
        )
        .with(
            "arc_degrees",
            enclosure(&receipt.arc_degrees.0, &receipt.arc_degrees.1),
        )
        .with(
            "reduced_length_metres",
            enclosure(
                receipt.reduced_length.0.exact(),
                receipt.reduced_length.1.exact(),
            ),
        )
        .with("scale12", enclosure(&receipt.scale12.0, &receipt.scale12.1))
        .with("scale21", enclosure(&receipt.scale21.0, &receipt.scale21.1))
        .with(
            "geodesic_quadrilateral_area_square_metres",
            enclosure(
                receipt.geodesic_quadrilateral_area.0.exact(),
                receipt.geodesic_quadrilateral_area.1.exact(),
            ),
        )
        .into()
}

pub(super) fn transform(
    answer: &purrdf_geo_kernel::TransformResult,
    unit: purrdf_geo_kernel::operation::CoordinateUnit,
) -> Result<Value, GeoCallError> {
    let point = answer.point();
    let mut result = Object::new()
        .with("x", super::profile::exact_decimal(&point.x)?)
        .with("y", super::profile::exact_decimal(&point.y)?)
        .with(
            "z_metres",
            point
                .z
                .as_ref()
                .map(super::profile::exact_decimal)
                .transpose()?,
        )
        .with(
            "epoch_decimal_year",
            point
                .epoch
                .as_ref()
                .map(super::profile::exact_decimal)
                .transpose()?,
        )
        .with(
            "unit",
            match unit {
                purrdf_geo_kernel::operation::CoordinateUnit::Degrees => "degrees",
                purrdf_geo_kernel::operation::CoordinateUnit::Metres => "metres",
            },
        )
        .with("operation", answer.operation_id().to_string())
        .with("law", answer.law_id().digest().to_string())
        .with("certificate", hex::encode(&answer.certificate_bytes()));
    let grid = answer.output_grid();
    if grid != purrdf_geo_kernel::TransformOutputGrid::DEFAULT {
        result = result
            .with("angular_decimal_places", grid.angular_decimal_places())
            .with("metric_decimal_places", grid.metric_decimal_places());
    }
    Ok(result.into())
}

pub(super) fn proof(receipt: &MetricProofReceipt) -> Value {
    Object::new()
        .with("lower_metres", lower_decimal(receipt.lower.exact()))
        .with("upper_metres", upper_decimal(receipt.upper.exact()))
        .with("precision_bits", receipt.precision_bits)
        .with("work_items", receipt.work_items)
        .into()
}

/// Only proof-display bounds round outward; completed values keep their law's grid.
pub(super) fn upper_decimal(value: &Rat) -> String {
    bound_decimal(value, true)
}
pub(super) fn lower_decimal(value: &Rat) -> String {
    bound_decimal(value, false)
}

fn bound_decimal(value: &Rat, upward: bool) -> String {
    const SCALE: u32 = 36;
    let rounding = if upward {
        purrdf_xsd::ieee::ratio::Rounding::Up
    } else {
        purrdf_xsd::ieee::ratio::Rounding::Down
    };
    value.to_decimal_string_with(SCALE, rounding)
}

pub(super) fn key(key: u64) -> String {
    hex::encode(&key.to_be_bytes())
}

pub(super) fn range(range: purrdf_geo_kernel::cells::CellRange) -> Value {
    Object::new()
        .with("profile", range.profile().digest().to_string())
        .with("min", key(range.min()))
        .with("max", key(range.max()))
        .with("stride", key(range.stride()))
        .with("stored_level", range.stored_level())
        .with("logical_count", range.count().to_string())
        .into()
}

pub(super) fn cell(cell: CellId) -> Value {
    Object::new()
        .with("profile", cell.profile().digest().to_string())
        .with("key", key(cell.key()))
        .with("face", cell.face())
        .with("level", cell.level())
        .with("big_endian", hex::encode(&cell.to_be_bytes()))
        .with("range_min", key(cell.range_min()))
        .with("range_max", key(cell.range_max()))
        .into()
}

pub(super) fn term(term: &TermValue) -> Result<Value, GeoCallError> {
    Ok(match term {
        TermValue::Iri(iri) => Object::new()
            .with("kind", "iri")
            .with("value", iri.as_str())
            .into(),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Object::new()
            .with("kind", "literal")
            .with("value", lexical_form.as_str())
            .with("datatype", datatype.as_str())
            .with("language", language.clone())
            .with(
                "direction",
                direction.map(purrdf_core::RdfTextDirection::as_str),
            )
            .into(),
        _ => {
            return Err(GeoError::domain(
                "geometry scalar returned a term outside the declared IRI/literal result carrier",
            )
            .into());
        }
    })
}

fn exact_rational(value: &Rat) -> Value {
    super::profile::exact_decimal(value).map_or_else(
        |_| {
            Object::new()
                .with("numerator", value.numerator().to_string())
                .with("denominator", value.denominator().to_string())
                .into()
        },
        Value::from,
    )
}

fn environment_evidence(
    evidence: &purrdf_xsd::ieee::environment::FloatEnvironmentEvidence,
) -> Value {
    use purrdf_xsd::ieee::environment::FloatEnvironmentEvidence;
    match evidence {
        FloatEnvironmentEvidence::Register { name, bits } => Object::new()
            .with("kind", "register")
            .with("name", *name)
            .with("bits", key(*bits))
            .into(),
        FloatEnvironmentEvidence::Probe {
            operation,
            expected,
            observed,
        } => Object::new()
            .with("kind", "probe")
            .with("operation", *operation)
            .with("expected_bits", key(*expected))
            .with("observed_bits", key(*observed))
            .into(),
        _ => Object::new().with("kind", "unknown").into(),
    }
}

pub(super) fn refusal_admitted(
    error: &GeoCallError,
    invocation: &super::invocation::Invocation,
) -> Value {
    match invocation.admit_refusal(error) {
        Ok(()) => refusal(error),
        // Fixed operational refusals require no original-operand formatting.
        // They remain available even when the detailed record cannot be admitted.
        Err(error) => refusal(&GeoCallError::Engine(error)),
    }
}

pub(super) fn refusal(error: &GeoCallError) -> Value {
    let mut fields = Object::new().with("message", error.to_string());
    match error {
        GeoCallError::Decode(error) => {
            fields = fields
                .with("code", "invalid-record")
                .with("pointer", error.pointer());
        }
        GeoCallError::Engine(error) => {
            let code = match error {
                GeoError::SourceRead(_) => "source-read",
                GeoError::Arity(_) => "arity",
                GeoError::Config(_) => "configuration",
                GeoError::Literal(_) => "literal",
                GeoError::Unsupported(_) => "unsupported",
                GeoError::Domain(_) => "domain",
                GeoError::CoordinateOutOfRange { axis, value } => {
                    fields = fields
                        .with("axis", *axis)
                        .with("value", exact_rational(value));
                    "coordinate-range"
                }
                GeoError::NonFiniteCoordinate { axis, bits } => {
                    fields = fields.with("axis", *axis).with("bits", key(*bits));
                    "nonfinite-coordinate"
                }
                GeoError::NonFiniteThreshold { bits } => {
                    fields = fields.with("bits", key(*bits));
                    "nonfinite-threshold"
                }
                GeoError::UnregisteredCrs(crs) => {
                    fields = fields.with("crs", crs.as_str());
                    "unregistered-crs"
                }
                GeoError::MissingOperation { source, target } => {
                    fields = fields
                        .with("source", source.as_str())
                        .with("target", target.as_str());
                    "missing-operation"
                }
                GeoError::MissingHeight => "missing-height",
                GeoError::AmbiguousGeodesic => "ambiguous-geodesic",
                GeoError::AmbiguousTransform { roots } => {
                    fields = fields.with("roots", *roots);
                    "ambiguous-transform"
                }
                GeoError::InvalidEllipsoid(reason) => {
                    fields = fields.with("reason", *reason);
                    "invalid-ellipsoid"
                }
                GeoError::InvalidExecutionPolicy(reason) => {
                    fields = fields.with("reason", *reason);
                    "invalid-policy"
                }
                GeoError::PrecisionExhausted { bits } => {
                    fields = fields.with("bits", *bits);
                    "precision-exhausted"
                }
                GeoError::WorkExhausted { limit } => {
                    fields = fields.with("limit", *limit);
                    "work-exhausted"
                }
                GeoError::MemoryExhausted { limit } => {
                    fields = fields.with("limit", *limit);
                    "memory-exhausted"
                }
                GeoError::NumericalScratch(error) => {
                    use purrdf_xsd::integer::LimbScratchError;
                    match error {
                        LimbScratchError::Capacity {
                            required_limbs,
                            admitted_limbs,
                        } => {
                            fields = fields
                                .with("reason", "capacity")
                                .with("required_limbs", *required_limbs as u64)
                                .with("admitted_limbs", *admitted_limbs as u64);
                        }
                        LimbScratchError::Exhausted { destinations } => {
                            fields = fields
                                .with("reason", "destinations")
                                .with("destinations", *destinations as u64);
                        }
                        LimbScratchError::Retained {
                            held_destinations,
                            destinations,
                        } => {
                            fields = fields
                                .with("reason", "retained-destinations")
                                .with("held_destinations", *held_destinations as u64)
                                .with("destinations", *destinations as u64);
                        }
                        LimbScratchError::SizeOverflow => {
                            fields = fields.with("reason", "size-overflow");
                        }
                    }
                    "numerical-scratch"
                }
                GeoError::NumericalTaylorScratch(error) => {
                    match error {
                        purrdf_xsd::math::TaylorScratchError::InUse => {
                            fields = fields.with("reason", "in-use");
                        }
                    }
                    "numerical-taylor-scratch"
                }
                GeoError::OutputExhausted { limit } => {
                    fields = fields.with("limit", *limit);
                    "output-exhausted"
                }
                GeoError::Cancelled => "cancelled",
                GeoError::ConvergenceExhausted { iterations } => {
                    fields = fields.with("iterations", *iterations);
                    "convergence-exhausted"
                }
                GeoError::FloatEnvironment(error) => {
                    use purrdf_xsd::ieee::environment::FloatEnvironmentError;
                    let (reason, evidence) = match error {
                        FloatEnvironmentError::TrapsEnabled { evidence } => {
                            ("traps-enabled", Some(evidence))
                        }
                        FloatEnvironmentError::FlushToZero { evidence } => {
                            ("flush-to-zero", Some(evidence))
                        }
                        FloatEnvironmentError::RoundingMode { evidence } => {
                            ("rounding-mode", Some(evidence))
                        }
                        FloatEnvironmentError::DoubleRounding { evidence } => {
                            ("double-rounding", Some(evidence))
                        }
                        _ => ("unknown", None),
                    };
                    fields = fields
                        .with("reason", reason)
                        .with("evidence", evidence.map(environment_evidence));
                    "floating-environment"
                }
                GeoError::ArithmeticOverflow(operation) => {
                    fields = fields.with("operation", *operation);
                    "arithmetic-overflow"
                }
                GeoError::NonPositiveEdgeLength(target) => {
                    fields = fields.with("target_metres", exact_rational(target));
                    "nonpositive-edge-length"
                }
                GeoError::UnattainableEdgeLength { target, minimum } => {
                    fields = fields
                        .with("target_metres", exact_rational(target))
                        .with("minimum_metres", exact_rational(minimum));
                    "unattainable-edge-length"
                }
                GeoError::InvalidResolution(level) => {
                    fields = fields.with("level", *level);
                    "invalid-resolution"
                }
                GeoError::InvalidCellId(value) => {
                    fields = fields.with("key", key(*value));
                    "invalid-cell"
                }
                GeoError::GridProfileMismatch => "grid-profile-mismatch",
                GeoError::InvalidStoredLevel { cell, stored } => {
                    fields = fields
                        .with("cell_level", *cell)
                        .with("stored_level", *stored);
                    "invalid-stored-level"
                }
                GeoError::InvalidAncestorLevel { cell, ancestor } => {
                    fields = fields
                        .with("cell_level", *cell)
                        .with("ancestor_level", *ancestor);
                    "invalid-ancestor-level"
                }
                GeoError::RootHasNoParent => "root-has-no-parent",
                GeoError::LeafHasNoChildren => "leaf-has-no-children",
                GeoError::InvalidOutputLength { expected, actual } => {
                    fields = fields.with("expected", *expected).with("actual", *actual);
                    "invalid-output-length"
                }
                GeoError::NegativePhysicalRadius(value) => {
                    fields = fields.with("radius_metres", exact_rational(value));
                    "negative-physical-radius"
                }
                GeoError::InvalidCoverLevels { min, max } => {
                    fields = fields.with("min", *min).with("max", *max);
                    "invalid-cover-levels"
                }
                GeoError::CoverCellsExhausted { limit } => {
                    fields = fields.with("limit", *limit);
                    "cover-cells-exhausted"
                }
                GeoError::DuplicatePointKey(key) => {
                    fields = fields.with("key", key.to_string());
                    "duplicate-point-key"
                }
                GeoError::PointIndexReferenceMismatch => "point-index-reference-mismatch",
                _ => "engine-refusal",
            };
            fields = fields
                .with("code", code)
                .with("expression_error", error.is_expression_error());
        }
    }
    fields.into()
}
