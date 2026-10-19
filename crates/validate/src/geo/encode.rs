// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical certificates, exact quantities and typed refusals.

use super::{GeoCallError, decode::Decode};
use purrdf_core::TermValue;
use purrdf_geo_kernel::{
    GeoError, MetricEstimate, MetricProofReceipt, Rat, binding::GeoQueryIdentity, cells::CellId,
};
use purrdf_lex::json::{Object, Value};
use purrdf_xsd::ieee::ratio::Rounding;

/// The complete version-one identity record, shared with output admission.
pub(super) const IDENTITY_MEMBER_COUNT: u64 = 16;

/// One envelope body for sessions and retained indexes. Bounded operational
/// failures move unchanged to the caller before any envelope allocation; the
/// explicit resident String adapter may render that same cause separately.
pub(super) fn response(
    identity: Option<&GeoQueryIdentity>,
    index: Option<purrdf_hash::hex::Digest32>,
    outcome: Result<Value, GeoCallError>,
    invocation: Option<&super::invocation::Invocation>,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<(String, bool), GeoCallError> {
    if invocation.is_some()
        && let Err(GeoCallError::Engine(error)) = &outcome
        && !error.is_expression_error()
    {
        return outcome.map(|_| unreachable!("checked original failure"));
    }
    let success = outcome.is_ok();
    let mut fields = Object::new()
        .with_memory_cell("version", 1_u32, decode.storage.memory())?
        .with_memory_cell(
            "identity",
            identity
                .map(|value| self::identity(value, decode))
                .transpose()?,
            decode.storage.memory(),
        )?;
    if let Some(index) = index {
        fields =
            fields.with_memory_cell("index", decode.format(&index)?, decode.storage.memory())?;
    }
    let fields = match outcome {
        Ok(value) => fields.with_memory_cell("result", value, decode.storage.memory())?,
        Err(error) => {
            let value = match invocation {
                Some(invocation) => refusal_admitted(&error, invocation, decode)?,
                None => refusal(&error, decode)?,
            };
            fields.with_memory_cell("error", value, decode.storage.memory())?
        }
    };
    decode.write(&fields.into()).map(|text| (text, success))
}

pub(super) fn identity(
    identity: &GeoQueryIdentity,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "distance_law",
            decode.format(&identity.distance_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "inverse_law",
            decode.format(&identity.inverse_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "direct_law",
            decode.format(&identity.direct_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "geometry_distance_law",
            decode.format(&identity.geometry_distance_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "length_law",
            decode.format(&identity.length_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "perimeter_law",
            decode.format(&identity.perimeter_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "area_law",
            decode.format(&identity.area_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "geodesic_area_integral_law",
            decode.format(&identity.geodesic_area_integral_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "topology_law",
            decode.format(&identity.topology_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "image_materialization_law",
            decode.format(&identity.image_materialization_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "point_buffer_materialization_law",
            decode.format(&identity.point_buffer_materialization_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "binding",
            decode.format(&identity.binding.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "buffer_materialization_law",
            decode.format(&identity.buffer_materialization_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "global_point_buffer_materialization_law",
            decode.format(&identity.global_point_buffer_materialization_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "global_buffer_materialization_law",
            decode.format(&identity.global_buffer_materialization_law.digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "policy",
            decode.format(&identity.policy.digest())?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn geometry_metric(
    estimate: &purrdf_geo_kernel::ellipsoidal::GeometryMetricEstimate,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let area = estimate.is_area();
    Ok(Object::new()
        .with_memory_cell(
            "value",
            decode.decimal_text(
                estimate.exact(),
                Some(if area { 2 } else { 6 }),
                Rounding::NearestEven,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "unit",
            if area { "square-metres" } else { "metres" },
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "reported_double_bits",
            key(estimate.reported_double().to_bits(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "error_bound",
            upper_decimal(&estimate.error_bound(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "law",
            decode.format(&estimate.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "binding",
            decode.format(&estimate.binding_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "certificate",
            decode.certificate(|admission| estimate.certificate_bytes_admitted(admission))?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn metric(
    estimate: &MetricEstimate,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "metres",
            decode.decimal_text(estimate.value().exact(), Some(6), Rounding::NearestEven)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "micrometres",
            decode.integer_text(estimate.quantized_micrometres())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "reported_double_bits",
            key(estimate.reported_double().to_bits(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "rounding_bound_metres",
            "0.0000005",
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "host_conversion_bound_metres",
            upper_decimal(estimate.host_conversion_bound().exact(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "law",
            decode.format(&estimate.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "binding",
            decode.format(&estimate.binding_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "certificate",
            decode.certificate(|admission| estimate.certificate_bytes_admitted(admission))?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn inverse(
    answer: &purrdf_geo_kernel::geodesic::InverseResult,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let cuts = decode.array(answer.pole_cuts().iter(), |(latitude, from, to)| {
        Ok(Object::new()
            .with_memory_cell(
                "latitude",
                decode.decimal_text(latitude, Some(15), Rounding::NearestEven)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "from_longitude",
                decode.decimal_text(from, Some(15), Rounding::NearestEven)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "to_longitude",
                decode.decimal_text(to, Some(15), Rounding::NearestEven)?,
                decode.storage.memory(),
            )?
            .into())
    })?;
    Ok(Object::new()
        .with_memory_cell(
            "branch_multiplicity",
            match answer.branch_multiplicity() {
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Unique => "unique",
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Two => "two",
                purrdf_geo_kernel::geodesic::ShortestBranchMultiplicity::Continuum => "continuum",
            },
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "distance",
            metric(answer.distance(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "forward_azimuth_degrees",
            answer
                .forward_azimuth()
                .map(|value| decode.decimal_text(value, Some(15), Rounding::NearestEven))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "final_azimuth_degrees",
            answer
                .final_azimuth()
                .map(|value| decode.decimal_text(value, Some(15), Rounding::NearestEven))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "arc_degrees",
            decode.decimal_text(answer.arc_degrees(), Some(15), Rounding::NearestEven)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "reduced_length_metres",
            decode.decimal_text(
                answer.reduced_length().exact(),
                Some(6),
                Rounding::NearestEven,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "scale12",
            decode.decimal_text(answer.scale12(), Some(15), Rounding::NearestEven)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "scale21",
            decode.decimal_text(answer.scale21(), Some(15), Rounding::NearestEven)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "geodesic_quadrilateral_area_square_metres",
            decode.decimal_text(
                answer.geodesic_quadrilateral_area().exact(),
                Some(2),
                Rounding::NearestEven,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell("pole_cuts", cuts, decode.storage.memory())?
        .with_memory_cell(
            "law",
            decode.format(&answer.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "binding",
            decode.format(&answer.binding_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "certificate",
            decode.certificate(|admission| answer.certificate_bytes_admitted(admission))?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn direct(
    answer: &purrdf_geo_kernel::geodesic::DirectResult,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let places = answer.output_grid().decimal_places();
    Ok(Object::new()
        .with_memory_cell(
            "longitude",
            decode.decimal_text(
                answer.endpoint().longitude(),
                Some(places),
                Rounding::NearestEven,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "latitude",
            decode.decimal_text(
                answer.endpoint().latitude(),
                Some(places),
                Rounding::NearestEven,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "final_azimuth_degrees",
            decode.decimal_text(answer.final_azimuth(), Some(places), Rounding::NearestEven)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "surface_error_bound_metres",
            upper_decimal(answer.surface_error_bound().exact(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "law",
            decode.format(&answer.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "binding",
            decode.format(&answer.binding_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "certificate",
            decode.certificate(|admission| answer.certificate_bytes_admitted(admission))?,
            decode.storage.memory(),
        )?
        .into())
}

fn enclosure(
    lower: &Rat,
    upper: &Rat,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "lower",
            lower_decimal(lower, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "upper",
            upper_decimal(upper, decode)?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn inverse_proof(
    receipt: &purrdf_geo_kernel::geodesic::InverseProofReceipt,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "distance",
            proof(&receipt.distance, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "forward_azimuth_degrees",
            receipt
                .forward_azimuth
                .as_ref()
                .map(|(a, b)| enclosure(a, b, decode))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "final_azimuth_degrees",
            receipt
                .final_azimuth
                .as_ref()
                .map(|(a, b)| enclosure(a, b, decode))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "arc_degrees",
            enclosure(&receipt.arc_degrees.0, &receipt.arc_degrees.1, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "reduced_length_metres",
            enclosure(
                receipt.reduced_length.0.exact(),
                receipt.reduced_length.1.exact(),
                decode,
            )?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "scale12",
            enclosure(&receipt.scale12.0, &receipt.scale12.1, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "scale21",
            enclosure(&receipt.scale21.0, &receipt.scale21.1, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "geodesic_quadrilateral_area_square_metres",
            enclosure(
                receipt.geodesic_quadrilateral_area.0.exact(),
                receipt.geodesic_quadrilateral_area.1.exact(),
                decode,
            )?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn transform(
    answer: &purrdf_geo_kernel::TransformResult,
    unit: purrdf_geo_kernel::operation::CoordinateUnit,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let point = answer.point();
    let mut result = Object::new()
        .with_memory_cell(
            "x",
            super::profile::exact_decimal(&point.x, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "y",
            super::profile::exact_decimal(&point.y, decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "z_metres",
            point
                .z
                .as_ref()
                .map(|value| super::profile::exact_decimal(value, decode))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "epoch_decimal_year",
            point
                .epoch
                .as_ref()
                .map(|value| super::profile::exact_decimal(value, decode))
                .transpose()?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "unit",
            match unit {
                purrdf_geo_kernel::operation::CoordinateUnit::Degrees => "degrees",
                purrdf_geo_kernel::operation::CoordinateUnit::Metres => "metres",
            },
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "operation",
            decode.format(&answer.operation_id())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "law",
            decode.format(&answer.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "certificate",
            decode.certificate(|admission| answer.certificate_bytes_admitted(admission))?,
            decode.storage.memory(),
        )?;
    let grid = answer.output_grid();
    if grid != purrdf_geo_kernel::TransformOutputGrid::DEFAULT {
        result = result
            .with_memory_cell(
                "angular_decimal_places",
                grid.angular_decimal_places(),
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "metric_decimal_places",
                grid.metric_decimal_places(),
                decode.storage.memory(),
            )?;
    }
    Ok(result.into())
}
pub(super) fn proof(
    receipt: &MetricProofReceipt,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "lower_metres",
            lower_decimal(receipt.lower.exact(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "upper_metres",
            upper_decimal(receipt.upper.exact(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "precision_bits",
            receipt.precision_bits,
            decode.storage.memory(),
        )?
        .with_memory_cell("work_items", receipt.work_items, decode.storage.memory())?
        .into())
}
/// Only proof-display bounds round outward; completed values keep their law's grid.
pub(super) fn upper_decimal(
    value: &Rat,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<String, GeoCallError> {
    decode.decimal_text(value, Some(36), Rounding::Up)
}
pub(super) fn lower_decimal(
    value: &Rat,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<String, GeoCallError> {
    decode.decimal_text(value, Some(36), Rounding::Down)
}
pub(super) fn key(key: u64, decode: &Decode<'_, '_, '_, '_>) -> Result<String, GeoCallError> {
    decode.hex(&key.to_be_bytes())
}
pub(super) fn range(
    range: purrdf_geo_kernel::cells::CellRange,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "profile",
            decode.format(&range.profile().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell("min", key(range.min(), decode)?, decode.storage.memory())?
        .with_memory_cell("max", key(range.max(), decode)?, decode.storage.memory())?
        .with_memory_cell(
            "stride",
            key(range.stride(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "stored_level",
            range.stored_level(),
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "logical_count",
            decode.format(&range.count())?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn cell(cell: CellId, decode: &Decode<'_, '_, '_, '_>) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "profile",
            decode.format(&cell.profile().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell("key", key(cell.key(), decode)?, decode.storage.memory())?
        .with_memory_cell("face", cell.face(), decode.storage.memory())?
        .with_memory_cell("level", cell.level(), decode.storage.memory())?
        .with_memory_cell(
            "big_endian",
            decode.hex(&cell.to_be_bytes())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "range_min",
            key(cell.range_min(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "range_max",
            key(cell.range_max(), decode)?,
            decode.storage.memory(),
        )?
        .into())
}
pub(super) fn term(
    term: &TermValue,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(match term {
        TermValue::Iri(iri) => Object::new()
            .with_memory_cell("kind", "iri", decode.storage.memory())?
            .with_memory_cell("value", iri.as_str(), decode.storage.memory())?
            .into(),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Object::new()
            .with_memory_cell("kind", "literal", decode.storage.memory())?
            .with_memory_cell("value", lexical_form.as_str(), decode.storage.memory())?
            .with_memory_cell("datatype", datatype.as_str(), decode.storage.memory())?
            .with_memory_cell("language", language.as_deref(), decode.storage.memory())?
            .with_memory_cell(
                "direction",
                direction.map(purrdf_core::RdfTextDirection::as_str),
                decode.storage.memory(),
            )?
            .into(),
        _ => {
            return Err(GeoError::domain(
                "geometry scalar returned a term outside the declared IRI/literal result carrier",
            )
            .into());
        }
    })
}
fn exact_rational(value: &Rat, decode: &Decode<'_, '_, '_, '_>) -> Result<Value, GeoCallError> {
    match super::profile::exact_decimal(value, decode) {
        Ok(text) => Ok(Value::String(text)),
        Err(GeoCallError::Engine(error))
            if matches!(error.original(), GeoError::GeometryDomain(cause)
                if matches!(&**cause, purrdf_geo_kernel::GeometryDomainError::NonDecimalParameter { .. })) =>
        {
            Ok(Object::new()
                .with_memory_cell(
                    "numerator",
                    decode.integer_text(&value.numerator())?,
                    decode.storage.memory(),
                )?
                .with_memory_cell(
                    "denominator",
                    decode.integer_text(&value.denominator())?,
                    decode.storage.memory(),
                )?
                .into())
        }
        Err(original) => Err(original),
    }
}
fn environment_evidence(
    evidence: &purrdf_xsd::ieee::environment::FloatEnvironmentEvidence,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    use purrdf_xsd::ieee::environment::FloatEnvironmentEvidence;
    Ok(match evidence {
        FloatEnvironmentEvidence::Register { name, bits } => Object::new()
            .with_memory_cell("kind", "register", decode.storage.memory())?
            .with_memory_cell("name", *name, decode.storage.memory())?
            .with_memory_cell("bits", key(*bits, decode)?, decode.storage.memory())?
            .into(),
        FloatEnvironmentEvidence::Probe {
            operation,
            expected,
            observed,
        } => Object::new()
            .with_memory_cell("kind", "probe", decode.storage.memory())?
            .with_memory_cell("operation", *operation, decode.storage.memory())?
            .with_memory_cell(
                "expected_bits",
                key(*expected, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "observed_bits",
                key(*observed, decode)?,
                decode.storage.memory(),
            )?
            .into(),
        _ => Object::new()
            .with_memory_cell("kind", "unknown", decode.storage.memory())?
            .into(),
    })
}
fn vocabulary_cause(
    error: &purrdf_geo_kernel::VocabularyError,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    use purrdf_geo_kernel::{VocabularyError, VocabularyNamespace};
    let code = match error {
        VocabularyError::UnregisteredOutputUnit { .. } => "vocabulary-unregistered-output-unit",
        VocabularyError::UndeclaredUnit { .. } => "vocabulary-undeclared-unit",
        VocabularyError::UnitMismatch { .. } => "vocabulary-unit-mismatch",
        VocabularyError::MissingMetre { .. } => "vocabulary-missing-metre",
        VocabularyError::MissingSimpleFeatures { .. } => "vocabulary-missing-simple-features",
        VocabularyError::EmptyNamespace(_) => "vocabulary-empty-namespace",
        VocabularyError::EmptyUnit { .. } => "vocabulary-empty-unit",
        VocabularyError::ConflictingUnit { .. } => "vocabulary-conflicting-unit",
        VocabularyError::EmptyMetre => "vocabulary-empty-metre",
    };
    let fields = Object::new().with_memory_cell("code", code, decode.storage.memory())?;
    Ok(match error {
        VocabularyError::UnregisteredOutputUnit { unit } => {
            fields.with_memory_cell("unit", unit.as_str(), decode.storage.memory())?
        }
        VocabularyError::UndeclaredUnit { crs }
        | VocabularyError::MissingMetre { crs }
        | VocabularyError::EmptyUnit { crs } => {
            fields.with_memory_cell("crs", crs.as_str(), decode.storage.memory())?
        }
        VocabularyError::UnitMismatch {
            crs,
            declared,
            requested,
        } => fields
            .with_memory_cell("crs", crs.as_str(), decode.storage.memory())?
            .with_memory_cell("declared", declared.as_str(), decode.storage.memory())?
            .with_memory_cell("requested", requested.as_str(), decode.storage.memory())?,
        VocabularyError::MissingSimpleFeatures { kind } => {
            fields.with_memory_cell("kind", kind.geojson_type(), decode.storage.memory())?
        }
        VocabularyError::EmptyNamespace(namespace) => {
            let code = match namespace {
                VocabularyNamespace::Core => "vocabulary-namespace-core",
                VocabularyNamespace::Function => "vocabulary-namespace-function",
                VocabularyNamespace::SimpleFeatures => "vocabulary-namespace-simple-features",
            };
            let namespace =
                Object::new().with_memory_cell("code", code, decode.storage.memory())?;
            fields.with_memory_cell("namespace", Value::from(namespace), decode.storage.memory())?
        }
        VocabularyError::ConflictingUnit { crs, first, second } => fields
            .with_memory_cell("crs", crs.as_str(), decode.storage.memory())?
            .with_memory_cell("first", first.as_str(), decode.storage.memory())?
            .with_memory_cell("second", second.as_str(), decode.storage.memory())?,
        VocabularyError::EmptyMetre => fields,
    }
    .into())
}

pub(super) fn refusal_admitted(
    error: &GeoCallError,
    invocation: &super::invocation::Invocation,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    invocation.admit_refusal(error)?;
    refusal(error, decode)
}
pub(super) fn refusal(
    error: &GeoCallError,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let mut fields = Object::new().with_memory_cell(
        "message",
        decode.message(error)?,
        decode.storage.memory(),
    )?;
    match error {
        GeoCallError::Decode(error) => {
            fields = fields
                .with_memory_cell("code", "invalid-record", decode.storage.memory())?
                .with_memory_cell("pointer", error.pointer(), decode.storage.memory())?;
        }
        GeoCallError::Engine(error) => {
            let error = error.original();
            let code = match error {
                GeoError::SourceRead(_) => "source-read",
                GeoError::Arity(_) => "arity",
                GeoError::Config(_) | GeoError::NoRelationFamilies => "configuration",
                GeoError::Literal(_) | GeoError::ArgumentInput(_) => "literal",
                GeoError::Unsupported(_) => "unsupported",
                GeoError::Domain(_) => "domain",
                GeoError::NativeMath(purrdf_xsd::math::MathError::Domain(reason)) => {
                    fields = fields.with_memory_cell("reason", *reason, decode.storage.memory())?;
                    "domain"
                }
                GeoError::Vocabulary(cause) => {
                    fields = fields.with_memory_cell(
                        "cause",
                        vocabulary_cause(cause, decode)?,
                        decode.storage.memory(),
                    )?;
                    if error.is_expression_error() {
                        "domain"
                    } else {
                        "configuration"
                    }
                }
                GeoError::CoordinateOutOfRange { axis, value } => {
                    fields = fields
                        .with_memory_cell("axis", *axis, decode.storage.memory())?
                        .with_memory_cell(
                            "value",
                            exact_rational(value, decode)?,
                            decode.storage.memory(),
                        )?;
                    "coordinate-range"
                }
                GeoError::NonFiniteCoordinate { axis, bits } => {
                    fields = fields
                        .with_memory_cell("axis", *axis, decode.storage.memory())?
                        .with_memory_cell("bits", key(*bits, decode)?, decode.storage.memory())?;
                    "nonfinite-coordinate"
                }
                GeoError::NonFiniteThreshold { bits } => {
                    fields = fields.with_memory_cell(
                        "bits",
                        key(*bits, decode)?,
                        decode.storage.memory(),
                    )?;
                    "nonfinite-threshold"
                }
                GeoError::UnregisteredCrs(crs) => {
                    fields =
                        fields.with_memory_cell("crs", crs.as_str(), decode.storage.memory())?;
                    "unregistered-crs"
                }
                GeoError::MissingOperation { source, target } => {
                    fields = fields
                        .with_memory_cell("source", source.as_str(), decode.storage.memory())?
                        .with_memory_cell("target", target.as_str(), decode.storage.memory())?;
                    "missing-operation"
                }
                GeoError::MissingHeight => "missing-height",
                GeoError::AmbiguousGeodesic => "ambiguous-geodesic",
                GeoError::AmbiguousTransform { roots } => {
                    fields = fields.with_memory_cell("roots", *roots, decode.storage.memory())?;
                    "ambiguous-transform"
                }
                GeoError::InvalidEllipsoid(cause) => {
                    fields = fields
                        .with_memory_cell(
                            "reason",
                            cause.requirement.message(),
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "semimajor",
                            exact_rational(&cause.semimajor, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "inverse_flattening",
                            exact_rational(&cause.inverse_flattening, decode)?,
                            decode.storage.memory(),
                        )?;
                    "invalid-ellipsoid"
                }
                GeoError::InvalidExecutionPolicy(limits) => {
                    let limits = Object::new()
                        .with_memory_cell(
                            "max_output_elements",
                            limits.max_output_elements,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_work_items",
                            limits.max_work_items,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_workspace_bytes",
                            limits.max_workspace_bytes,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_iterations",
                            limits.max_iterations,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_subdivision_levels",
                            limits.max_subdivision_levels,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_precision_bits",
                            limits.max_precision_bits,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "max_scratch_destinations",
                            limits.max_scratch_destinations,
                            decode.storage.memory(),
                        )?;
                    fields = fields
                        .with_memory_cell(
                            "reason",
                            error.detail().as_ref(),
                            decode.storage.memory(),
                        )?
                        .with_memory_cell("limits", Value::from(limits), decode.storage.memory())?;
                    "invalid-policy"
                }
                GeoError::PrecisionExhausted { bits } => {
                    fields = fields.with_memory_cell("bits", *bits, decode.storage.memory())?;
                    "precision-exhausted"
                }
                GeoError::WorkExhausted { limit } => {
                    fields = fields.with_memory_cell("limit", *limit, decode.storage.memory())?;
                    "work-exhausted"
                }
                GeoError::MemoryExhausted { limit } => {
                    fields = fields.with_memory_cell("limit", *limit, decode.storage.memory())?;
                    "memory-exhausted"
                }
                GeoError::NumericalScratch(error) => {
                    use purrdf_xsd::integer::LimbScratchError;
                    match error {
                        LimbScratchError::Destinations { required, admitted } => {
                            fields = fields
                                .with_memory_cell(
                                    "reason",
                                    "required-destinations",
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "required_destinations",
                                    *required as u64,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "admitted_destinations",
                                    *admitted as u64,
                                    decode.storage.memory(),
                                )?;
                        }
                        LimbScratchError::Capacity {
                            required_limbs,
                            admitted_limbs,
                        } => {
                            fields = fields
                                .with_memory_cell("reason", "capacity", decode.storage.memory())?
                                .with_memory_cell(
                                    "required_limbs",
                                    *required_limbs as u64,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "admitted_limbs",
                                    *admitted_limbs as u64,
                                    decode.storage.memory(),
                                )?;
                        }
                        LimbScratchError::Exhausted { destinations } => {
                            fields = fields
                                .with_memory_cell(
                                    "reason",
                                    "destinations",
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "destinations",
                                    *destinations as u64,
                                    decode.storage.memory(),
                                )?;
                        }
                        LimbScratchError::Retained {
                            held_destinations,
                            destinations,
                        } => {
                            fields = fields
                                .with_memory_cell(
                                    "reason",
                                    "retained-destinations",
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "held_destinations",
                                    *held_destinations as u64,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "destinations",
                                    *destinations as u64,
                                    decode.storage.memory(),
                                )?;
                        }
                        LimbScratchError::Storage(error) => {
                            fields = fields.with_memory_cell(
                                "reason",
                                decode.format(error)?,
                                decode.storage.memory(),
                            )?;
                        }
                        LimbScratchError::SizeOverflow => {
                            fields = fields.with_memory_cell(
                                "reason",
                                "size-overflow",
                                decode.storage.memory(),
                            )?;
                        }
                        LimbScratchError::AllocationFailed => {
                            fields = fields.with_memory_cell(
                                "reason",
                                "allocation-failed",
                                decode.storage.memory(),
                            )?;
                        }
                    }
                    "numerical-scratch"
                }
                GeoError::NumericalTaylorScratch(error) => {
                    match error {
                        purrdf_xsd::math::TaylorScratchError::InUse => {
                            fields = fields.with_memory_cell(
                                "reason",
                                "in-use",
                                decode.storage.memory(),
                            )?;
                        }
                        purrdf_xsd::math::TaylorScratchError::InvalidInitializer => {
                            fields = fields.with_memory_cell(
                                "reason",
                                "invalid-initializer",
                                decode.storage.memory(),
                            )?;
                        }
                        purrdf_xsd::math::TaylorScratchError::Storage(error) => {
                            fields = fields
                                .with_memory_cell(
                                    "reason",
                                    "native-storage",
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "native_storage",
                                    decode.format(error)?,
                                    decode.storage.memory(),
                                )?;
                        }
                    }
                    "numerical-taylor-scratch"
                }
                GeoError::OutputExhausted { limit } => {
                    fields = fields.with_memory_cell("limit", *limit, decode.storage.memory())?;
                    "output-exhausted"
                }
                GeoError::Cancelled => "cancelled",
                GeoError::ConvergenceExhausted { iterations } => {
                    fields = fields.with_memory_cell(
                        "iterations",
                        *iterations,
                        decode.storage.memory(),
                    )?;
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
                        .with_memory_cell("reason", reason, decode.storage.memory())?
                        .with_memory_cell(
                            "evidence",
                            evidence
                                .map(|evidence| environment_evidence(evidence, decode))
                                .transpose()?,
                            decode.storage.memory(),
                        )?;
                    "floating-environment"
                }
                GeoError::ArithmeticOverflow(operation) => {
                    fields = fields.with_memory_cell(
                        "operation",
                        *operation,
                        decode.storage.memory(),
                    )?;
                    "arithmetic-overflow"
                }
                GeoError::NonPositiveEdgeLength(target) => {
                    fields = fields.with_memory_cell(
                        "target_metres",
                        exact_rational(target, decode)?,
                        decode.storage.memory(),
                    )?;
                    "nonpositive-edge-length"
                }
                GeoError::UnattainableEdgeLength { target, minimum } => {
                    fields = fields
                        .with_memory_cell(
                            "target_metres",
                            exact_rational(target, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "minimum_metres",
                            exact_rational(minimum, decode)?,
                            decode.storage.memory(),
                        )?;
                    "unattainable-edge-length"
                }
                GeoError::InvalidResolution(level) => {
                    fields = fields.with_memory_cell("level", *level, decode.storage.memory())?;
                    "invalid-resolution"
                }
                GeoError::InvalidCellId(value) => {
                    fields = fields.with_memory_cell(
                        "key",
                        key(*value, decode)?,
                        decode.storage.memory(),
                    )?;
                    "invalid-cell"
                }
                GeoError::GridProfileMismatch => "grid-profile-mismatch",
                GeoError::InvalidStoredLevel { cell, stored } => {
                    fields = fields
                        .with_memory_cell("cell_level", *cell, decode.storage.memory())?
                        .with_memory_cell("stored_level", *stored, decode.storage.memory())?;
                    "invalid-stored-level"
                }
                GeoError::InvalidAncestorLevel { cell, ancestor } => {
                    fields = fields
                        .with_memory_cell("cell_level", *cell, decode.storage.memory())?
                        .with_memory_cell("ancestor_level", *ancestor, decode.storage.memory())?;
                    "invalid-ancestor-level"
                }
                GeoError::RootHasNoParent => "root-has-no-parent",
                GeoError::LeafHasNoChildren => "leaf-has-no-children",
                GeoError::InvalidOutputLength { expected, actual } => {
                    fields = fields
                        .with_memory_cell("expected", *expected, decode.storage.memory())?
                        .with_memory_cell("actual", *actual, decode.storage.memory())?;
                    "invalid-output-length"
                }
                GeoError::NegativePhysicalRadius(value) => {
                    fields = fields.with_memory_cell(
                        "radius_metres",
                        exact_rational(value, decode)?,
                        decode.storage.memory(),
                    )?;
                    "negative-physical-radius"
                }
                GeoError::InvalidCoverLevels { min, max } => {
                    fields = fields
                        .with_memory_cell("min", *min, decode.storage.memory())?
                        .with_memory_cell("max", *max, decode.storage.memory())?;
                    "invalid-cover-levels"
                }
                GeoError::CoverCellsExhausted { limit } => {
                    fields = fields.with_memory_cell("limit", *limit, decode.storage.memory())?;
                    "cover-cells-exhausted"
                }
                GeoError::DuplicatePointKey(key) => {
                    fields = fields.with_memory_cell(
                        "key",
                        decode.format(key)?,
                        decode.storage.memory(),
                    )?;
                    "duplicate-point-key"
                }
                GeoError::PointIndexReferenceMismatch => "point-index-reference-mismatch",
                _ => "engine-refusal",
            };
            fields = fields
                .with_memory_cell("code", code, decode.storage.memory())?
                .with_memory_cell(
                    "expression_error",
                    error.is_expression_error(),
                    decode.storage.memory(),
                )?;
        }
    }
    Ok(fields.into())
}
