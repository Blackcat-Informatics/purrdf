// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit geographic references and resource admission, decoded once.

use purrdf_geo_kernel::{
    AxisOrder, Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference,
    PreparationBudget, PreparedEllipsoid, Rat,
};
use purrdf_hash::hex::Digest32;
use purrdf_lex::json::{
    self, Object, Value,
    record::{DecodeError, Record},
};

use super::GeoCallError;

pub(super) fn decimal(value: &Value) -> Result<Rat, DecodeError> {
    let text = value
        .as_str()
        .ok_or_else(|| DecodeError::invalid_type(value, "exact decimal string"))?;
    Rat::parse_decimal(text)
        .ok_or_else(|| DecodeError::invalid_value(value, "finite exact decimal string"))
}

pub(super) fn fixed_hex<const N: usize>(value: &Value) -> Result<[u8; N], DecodeError> {
    let text = value.as_str().ok_or_else(|| {
        DecodeError::invalid_type(value, "fixed-width lowercase hexadecimal string")
    })?;
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DecodeError::invalid_value(
            value,
            "fixed-width lowercase hexadecimal string",
        ));
    }
    let mut output = [0; N];
    output.copy_from_slice(&purrdf_hash::hex::decode_canonical(text).map_err(DecodeError::custom)?);
    Ok(output)
}

/// Read a version-one profile without inferring a datum, axes or operation.
///
/// # Errors
///
/// Refuses malformed, duplicate or unknown fields and invalid registrations.
pub fn profile_from_str(text: &str) -> Result<GeoProfile, GeoCallError> {
    profile_from_str_with_policy(text, ExecutionPolicy::geometry())
}

/// Decode and compile a strict profile with explicit cumulative configuration limits.
/// Invocation limits inside the profile remain a separate admission identity.
/// # Errors
/// Refuses malformed records and limits before any unadmitted exact/configuration body.
pub fn profile_from_str_with_policy(
    text: &str,
    compile_policy: ExecutionPolicy,
) -> Result<GeoProfile, GeoCallError> {
    let (profile, mut budget) = profile_with_budget(text, compile_policy)?;
    profile.query_identity_in_budget(&mut budget)?;
    Ok(profile)
}

pub(super) fn profile_with_budget(
    text: &str,
    compile_policy: ExecutionPolicy,
) -> Result<(GeoProfile, PreparationBudget), GeoCallError> {
    let (value, mut budget) = super::read_record_with_policy(text, compile_policy)?;
    let mut fields = Record::new(&value, "geographic profile version one")?;
    let version: u32 = fields.required("version")?;
    if version != 1 {
        return Err(DecodeError::custom("geographic profile version must be 1").into());
    }
    let references = fields
        .optional_with("references", |value| {
            json::record::items_with(value, |value| reference(value, &mut budget))
        })?
        .unwrap_or_default();
    let limits = fields.optional_with("limits", limits)?;
    let operations = fields
        .optional_with("operations", |value| {
            json::record::items_with(value, |value| operation_binding(value, &mut budget))
        })?
        .unwrap_or_default();
    let units = fields
        .optional_with("units", |value| {
            json::record::items_with(value, linear_unit)
        })?
        .unwrap_or_default();
    fields.deny_unknown()?;
    let mut profile = GeoProfile::standard();
    for (crs, reference) in references {
        profile.register_reference_in_budget(crs, reference, &mut budget)?;
    }
    for (name, source, target, chain) in operations {
        profile.register_operation_in_budget(name, source, target, chain, &mut budget)?;
    }
    for (unit, factor) in units {
        profile.register_linear_unit_in_budget(unit, factor, &mut budget)?;
    }
    if let Some(limits) = limits {
        profile = profile.with_limits(limits)?;
    }
    Ok((profile, budget))
}

fn linear_unit(value: &Value) -> Result<(Crs, Rat), GeoCallError> {
    let mut fields = Record::new(value, "explicit linear output unit")?;
    let iri: String = fields.required("iri")?;
    let factor = fields.required_with("metres_per_unit", decimal)?;
    fields.deny_unknown()?;
    Ok((Crs::new(iri)?, factor))
}

fn reference(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<(Crs, GeographicReference), GeoCallError> {
    let mut fields = Record::new(value, "explicit geographic reference")?;
    let crs: String = fields.required("crs")?;
    let axes = fields.tag("axes", &["lon-lat", "lat-lon"])?;
    let axes = if axes == "lon-lat" {
        AxisOrder::LonLat
    } else {
        AxisOrder::LatLon
    };
    let ellipsoid = fields.tag("ellipsoid", &["wgs84", "cgcs2000", "custom"])?;
    let reference = match ellipsoid {
        "wgs84" => GeographicReference::wgs84().with_axes(axes),
        "cgcs2000" => GeographicReference::cgcs2000().with_axes(axes),
        _ => {
            let semimajor = fields.required_with("semimajor_metres", decimal)?;
            let inverse_flattening = fields.required_with("inverse_flattening", decimal)?;
            let datum = fields.required_with("datum", fixed_hex::<32>)?;
            GeographicReference::new(
                PreparedEllipsoid::new_in_budget(semimajor, inverse_flattening, budget)?,
                Digest32::new(datum),
                axes,
            )
        }
    };
    fields.deny_unknown()?;
    Ok((Crs::new(crs)?, reference))
}

fn limits(value: &Value) -> Result<ExecutionLimits, DecodeError> {
    let mut fields = Record::new(value, "geographic execution limits")?;
    let defaults = ExecutionLimits::GEOMETRY;
    let limits = ExecutionLimits {
        max_output_elements: fields
            .optional("max_output_elements")?
            .unwrap_or(defaults.max_output_elements),
        max_work_items: fields
            .optional("max_work_items")?
            .unwrap_or(defaults.max_work_items),
        max_workspace_bytes: fields
            .optional("max_workspace_bytes")?
            .unwrap_or(defaults.max_workspace_bytes),
        max_iterations: fields
            .optional("max_iterations")?
            .unwrap_or(defaults.max_iterations),
        max_subdivision_levels: fields
            .optional("max_subdivision_levels")?
            .unwrap_or(defaults.max_subdivision_levels),
        max_precision_bits: fields
            .optional("max_precision_bits")?
            .unwrap_or(defaults.max_precision_bits),
        max_scratch_destinations: fields
            .optional("max_scratch_destinations")?
            .unwrap_or(defaults.max_scratch_destinations),
    };
    fields.deny_unknown()?;
    Ok(limits)
}

/// Canonical strict profile bytes, including explicit defaults and reference order.
/// # Errors
///
/// A native custom rational that has no exact admitted decimal representation
/// refuses; serializing a profile never silently changes its binding identity.
pub fn profile_to_string(profile: &GeoProfile) -> Result<String, GeoCallError> {
    profile_to_string_with_policy(profile, ExecutionPolicy::geometry())
}

/// Serialize a complete original profile under explicit configuration admission.
/// Invocation limits in the profile remain unchanged and do not replace this
/// export allowance. No JSON/result or numerical text is built before admission.
///
/// # Errors
/// Refuses original graph, exact formatting or complete JSON output limits.
pub fn profile_to_string_with_policy(
    profile: &GeoProfile,
    policy: ExecutionPolicy,
) -> Result<String, GeoCallError> {
    let mut budget = PreparationBudget::new(policy);
    profile.retain_configuration_in_budget(&mut budget)?;
    super::output::admit(budget, super::output::profile(profile)?)?;
    profile_string(profile)
}

fn profile_string(profile: &GeoProfile) -> Result<String, GeoCallError> {
    let references: Vec<_> = profile
        .references()
        .iter()
        .map(|binding| {
            let reference = binding.reference();
            let mut fields = Object::new().with("crs", binding.crs().as_str()).with(
                "axes",
                match reference.axes() {
                    AxisOrder::LonLat => "lon-lat",
                    AxisOrder::LatLon => "lat-lon",
                },
            );
            if reference == &GeographicReference::wgs84().with_axes(reference.axes()) {
                fields = fields.with("ellipsoid", "wgs84");
            } else if reference == &GeographicReference::cgcs2000().with_axes(reference.axes()) {
                fields = fields.with("ellipsoid", "cgcs2000");
            } else {
                fields = fields
                    .with("ellipsoid", "custom")
                    .with(
                        "semimajor_metres",
                        exact_decimal(reference.ellipsoid().semimajor())?,
                    )
                    .with(
                        "inverse_flattening",
                        exact_decimal(reference.ellipsoid().inverse_flattening())?,
                    )
                    .with("datum", reference.datum().to_string());
            }
            Ok::<_, GeoCallError>(Value::from(fields))
        })
        .collect::<Result<_, _>>()?;
    let policy = profile.policy();
    let limits = policy.limits();
    let operations = profile
        .operations()
        .iter()
        .map(|binding| {
            Ok::<_, GeoCallError>(
                Object::new()
                    .with("name", binding.name().as_str())
                    .with("source_crs", binding.source().as_str())
                    .with("target_crs", binding.target().as_str())
                    .with("chain", super::operation::chain_json(binding.chain())?)
                    .into(),
            )
        })
        .collect::<Result<Vec<Value>, _>>()?;
    let units = profile
        .linear_units()
        .iter()
        .map(|binding| {
            Ok::<_, GeoCallError>(
                Object::new()
                    .with("iri", binding.unit().as_str())
                    .with("metres_per_unit", exact_decimal(binding.metres_per_unit())?)
                    .into(),
            )
        })
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(json::write_compact(
        &Object::new()
            .with("version", 1_u32)
            .with("references", references)
            .with("operations", operations)
            .with("units", units)
            .with(
                "limits",
                Object::new()
                    .with("max_output_elements", limits.max_output_elements)
                    .with("max_work_items", limits.max_work_items)
                    .with("max_workspace_bytes", limits.max_workspace_bytes)
                    .with("max_iterations", limits.max_iterations)
                    .with("max_subdivision_levels", limits.max_subdivision_levels)
                    .with("max_precision_bits", limits.max_precision_bits)
                    .with("max_scratch_destinations", limits.max_scratch_destinations),
            )
            .into(),
    ))
}

pub(super) fn exact_decimal(value: &Rat) -> Result<String, GeoCallError> {
    let scale = value
        .finite_decimal_scale()
        .and_then(|scale| u32::try_from(scale).ok())
        .filter(|scale| *scale <= 100_000)
        .ok_or_else(|| {
            purrdf_geo_kernel::GeoError::domain(
                "custom profile parameter has no exact decimal representation",
            )
        })?;
    Ok(value.to_decimal_string(scale))
}

fn operation_binding(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<(Crs, Crs, Crs, purrdf_geo_kernel::OperationChain), GeoCallError> {
    let mut fields = Record::new(value, "named explicit operation registration")?;
    let name: String = fields.required("name")?;
    let source: String = fields.required("source_crs")?;
    let target: String = fields.required("target_crs")?;
    let chain = fields.required_with("chain", |value| super::operation::chain(value, budget))?;
    fields.deny_unknown()?;
    Ok((Crs::new(name)?, Crs::new(source)?, Crs::new(target)?, chain))
}
