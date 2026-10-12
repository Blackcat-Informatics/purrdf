// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit geographic references and resource admission, decoded once.

use purrdf_geo_kernel::{
    AxisOrder, Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference,
    PreparationBudget, PreparedEllipsoid, Rat,
};
use purrdf_hash::hex::Digest32;
use purrdf_lex::json::{Object, Value, record::DecodeError};

use super::{GeoCallError, decode::Decode};

pub(super) fn fixed_hex<const N: usize>(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<[u8; N], DecodeError> {
    let text = value
        .as_str()
        .ok_or_else(|| decode.invalid_type(value, "fixed-width lowercase hexadecimal string"))?;
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(decode.invalid_value(value, "fixed-width lowercase hexadecimal string"));
    }
    let mut output = [0; N];
    purrdf_hash::hex::decode_to_slice(text, &mut output).map_err(|error| decode.custom(error))?;
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
    let ((profile, budget), _) =
        super::decode::record(text, compile_policy, None, |value, decode| {
            let mut budget = decode.receipt()?;
            let mut fields = decode.record(value, "geographic profile version one")?;
            let version: u32 = fields.required_native("version")?;
            if version != 1 {
                return Err(decode.custom("geographic profile version must be 1").into());
            }
            let references = fields
                .optional_with("references", |value| {
                    decode.items(value, |value| reference(value, &mut budget, decode))
                })?
                .unwrap_or_default();
            let limits = fields.optional_with("limits", |value| limits(value, decode))?;
            let operations = fields
                .optional_with("operations", |value| {
                    decode.items(value, |value| operation_binding(value, &mut budget, decode))
                })?
                .unwrap_or_default();
            let units = fields
                .optional_with("units", |value| {
                    decode.items(value, |value| linear_unit(value, decode))
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
        })?;
    Ok((profile, budget))
}

fn linear_unit(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<(Crs, Rat), GeoCallError> {
    let mut fields = decode.record(value, "explicit linear output unit")?;
    let iri = fields.required_with("iri", |value| decode.crs(value))?;
    let factor = fields.required_with("metres_per_unit", |value| decode.decimal(value))?;
    fields.deny_unknown()?;
    Ok((iri, factor))
}

fn reference(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<(Crs, GeographicReference), GeoCallError> {
    let mut fields = decode.record(value, "explicit geographic reference")?;
    let crs = fields.required_with("crs", |value| decode.crs(value))?;
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
            let semimajor =
                fields.required_with("semimajor_metres", |value| decode.decimal(value))?;
            let inverse_flattening =
                fields.required_with("inverse_flattening", |value| decode.decimal(value))?;
            let datum = fields.required_with("datum", |value| fixed_hex::<32>(value, decode))?;
            GeographicReference::new(
                PreparedEllipsoid::new_in_budget(semimajor, inverse_flattening, budget)?,
                Digest32::new(datum),
                axes,
            )
        }
    };
    fields.deny_unknown()?;
    Ok((crs, reference))
}

fn limits(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<ExecutionLimits, DecodeError> {
    let mut fields = decode.record(value, "geographic execution limits")?;
    let defaults = ExecutionLimits::GEOMETRY;
    let limits = ExecutionLimits {
        max_output_elements: fields
            .optional_native("max_output_elements")?
            .unwrap_or(defaults.max_output_elements),
        max_work_items: fields
            .optional_native("max_work_items")?
            .unwrap_or(defaults.max_work_items),
        max_workspace_bytes: fields
            .optional_native("max_workspace_bytes")?
            .unwrap_or(defaults.max_workspace_bytes),
        max_iterations: fields
            .optional_native("max_iterations")?
            .unwrap_or(defaults.max_iterations),
        max_subdivision_levels: fields
            .optional_native("max_subdivision_levels")?
            .unwrap_or(defaults.max_subdivision_levels),
        max_precision_bits: fields
            .optional_native("max_precision_bits")?
            .unwrap_or(defaults.max_precision_bits),
        max_scratch_destinations: fields
            .optional_native("max_scratch_destinations")?
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
    super::decode::produce(policy, None, |decode| profile_string(profile, decode))
}

fn profile_string(
    profile: &GeoProfile,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<String, GeoCallError> {
    let references = decode.array(profile.references().iter(), |binding| {
        let reference = binding.reference();
        let mut fields = Object::new()
            .with_memory_cell("crs", binding.crs().as_str(), decode.storage.memory())?
            .with_memory_cell(
                "axes",
                match reference.axes() {
                    AxisOrder::LonLat => "lon-lat",
                    AxisOrder::LatLon => "lat-lon",
                },
                decode.storage.memory(),
            )?;
        if reference == &GeographicReference::wgs84().with_axes(reference.axes()) {
            fields = fields.with_memory_cell("ellipsoid", "wgs84", decode.storage.memory())?;
        } else if reference == &GeographicReference::cgcs2000().with_axes(reference.axes()) {
            fields = fields.with_memory_cell("ellipsoid", "cgcs2000", decode.storage.memory())?;
        } else {
            fields = fields
                .with_memory_cell("ellipsoid", "custom", decode.storage.memory())?
                .with_memory_cell(
                    "semimajor_metres",
                    exact_decimal(reference.ellipsoid().semimajor(), decode)?,
                    decode.storage.memory(),
                )?
                .with_memory_cell(
                    "inverse_flattening",
                    exact_decimal(reference.ellipsoid().inverse_flattening(), decode)?,
                    decode.storage.memory(),
                )?
                .with_memory_cell(
                    "datum",
                    decode.format(&reference.datum())?,
                    decode.storage.memory(),
                )?;
        }
        Ok(fields.into())
    })?;
    let policy = profile.policy();
    let limits = policy.limits();
    let operations = decode.array(profile.operations().iter(), |binding| {
        Ok(Object::new()
            .with_memory_cell("name", binding.name().as_str(), decode.storage.memory())?
            .with_memory_cell(
                "source_crs",
                binding.source().as_str(),
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "target_crs",
                binding.target().as_str(),
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "chain",
                super::operation::chain_json(binding.chain(), decode)?,
                decode.storage.memory(),
            )?
            .into())
    })?;
    let units = decode.array(profile.linear_units().iter(), |binding| {
        Ok(Object::new()
            .with_memory_cell("iri", binding.unit().as_str(), decode.storage.memory())?
            .with_memory_cell(
                "metres_per_unit",
                exact_decimal(binding.metres_per_unit(), decode)?,
                decode.storage.memory(),
            )?
            .into())
    })?;
    let value = Object::new()
        .with_memory_cell("version", 1_u32, decode.storage.memory())?
        .with_memory_cell("references", references, decode.storage.memory())?
        .with_memory_cell("operations", operations, decode.storage.memory())?
        .with_memory_cell("units", units, decode.storage.memory())?
        .with_memory_cell(
            "limits",
            Object::new()
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
                )?,
            decode.storage.memory(),
        )?
        .into();
    decode.write(&value)
}

pub(super) fn exact_decimal(
    value: &Rat,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<String, GeoCallError> {
    decode.decimal_text(value, None, purrdf_xsd::ieee::ratio::Rounding::NearestEven)
}

fn operation_binding(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<(Crs, Crs, Crs, purrdf_geo_kernel::OperationChain), GeoCallError> {
    let mut fields = decode.record(value, "named explicit operation registration")?;
    let name = fields.required_with("name", |value| decode.crs(value))?;
    let source = fields.required_with("source_crs", |value| decode.crs(value))?;
    let target = fields.required_with("target_crs", |value| decode.crs(value))?;
    let chain = fields.required_with("chain", |value| {
        super::operation::chain(value, budget, decode)
    })?;
    fields.deny_unknown()?;
    Ok((name, source, target, chain))
}
