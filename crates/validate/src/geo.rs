// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The version-one geographic string boundary shared by every language host.
//!
//! Records are strict: duplicate and unknown members refuse. Exact coordinates
//! and quantities are decimal strings; IEEE inputs use fixed-width bit strings.
//! Reference, mathematical-law and execution-policy identities remain separate.
//! No host implements coordinate mathematics or changes numerical output grids.

mod cover;
mod encode;
mod geometry;
mod invocation;
mod operation;
mod output;
mod point_index;
mod profile;
mod request;
mod session;

pub use cover::{CoverRequest, CoverResolution, CoverShape};
pub use geometry::{EdgeInput, GeometryInput, GeometryParts, PolygonInput, RegionInput};
pub use point_index::GeoPointIndex;
pub use profile::{
    profile_from_str, profile_from_str_with_policy, profile_to_string,
    profile_to_string_with_policy,
};
pub use purrdf_geo_kernel::GeoProfile;
pub use purrdf_geo_kernel::binding::STANDARD_PROFILE;
pub use request::{GeoRequest, PointInput};
pub use session::GeoSession;

use std::fmt;

use purrdf_geo_kernel::GeoError;
use purrdf_lex::json::record::DecodeError;

/// A strict boundary-shape refusal or an unchanged typed engine refusal.
#[derive(Debug)]
pub enum GeoCallError {
    /// A JSON grammar or strict-record refusal, with its source pointer.
    Decode(DecodeError),
    /// A numerical, reference, geometry or resource refusal from the engine.
    Engine(GeoError),
}

purrdf_lex::variant_from!(GeoCallError { Decode(DecodeError), Engine(GeoError) });

impl fmt::Display for GeoCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::Engine(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GeoCallError {}

impl purrdf_lex::json::record::Within for GeoCallError {
    fn within(self, token: &str) -> Self {
        match self {
            Self::Decode(error) => Self::Decode(error.within(token)),
            Self::Engine(error) => Self::Engine(error),
        }
    }
}

#[cfg(test)]
mod tests;

fn read_record(text: &str) -> Result<purrdf_lex::json::Value, GeoCallError> {
    read_record_with_policy(text, purrdf_geo_kernel::ExecutionPolicy::geometry())
        .map(|(value, _)| value)
}

fn read_record_with_policy(
    text: &str,
    policy: purrdf_geo_kernel::ExecutionPolicy,
) -> Result<
    (
        purrdf_lex::json::Value,
        purrdf_geo_kernel::PreparationBudget,
    ),
    GeoCallError,
> {
    read_record_observed(text, policy, None)
}

fn read_record_for_invocation(
    text: &str,
    policy: purrdf_geo_kernel::ExecutionPolicy,
    invocation: &invocation::Invocation,
) -> Result<
    (
        purrdf_lex::json::Value,
        purrdf_geo_kernel::PreparationBudget,
    ),
    GeoCallError,
> {
    let mut phase = invocation.phase(purrdf_geo_kernel::PreparationBudget::new(policy));
    read_record_observed(text, policy, Some(&mut phase))
}

fn read_record_observed(
    text: &str,
    policy: purrdf_geo_kernel::ExecutionPolicy,
    observer: Option<&mut dyn purrdf_geo_kernel::MetricWorkObserver>,
) -> Result<
    (
        purrdf_lex::json::Value,
        purrdf_geo_kernel::PreparationBudget,
    ),
    GeoCallError,
> {
    use purrdf_lex::{
        json::{self, Value},
        walk::WorkList,
    };
    let limit = policy.limits().max_workspace_bytes;
    let mut admission =
        purrdf_geo_kernel::carrier::InputAdmission::new(text.len(), policy, observer)?;
    let decoded = {
        let mut reader = json::Reader::new_observed(
            text,
            json::Limits {
                max_depth: 32,
                max_values: limit / 128,
                max_string_bytes: usize::try_from(limit / 64).unwrap_or(usize::MAX),
                ..json::Limits::DEFAULT
            },
            &mut admission,
        );
        reader
            .read_value()
            .and_then(|value| reader.finish().map(|()| value))
    };
    if let Some(error) = admission.take_error() {
        return Err(error.into());
    }
    let value = decoded.map_err(DecodeError::from)?;
    // Typed decimal construction is admitted from the original Rat grammar
    // before any integer, power, or normalized rational is constructed.
    let mut pending: WorkList<(&Value, bool), 32> = WorkList::with((&value, false));
    while let Some((node, numeric)) = pending.pop() {
        admission.admit_items(1)?;
        match node {
            Value::Array(items) => pending.extend(items.iter().map(|value| (value, numeric))),
            Value::Object(fields) => pending.extend(fields.iter().map(|(field, value)| {
                (
                    value,
                    decimal_field(
                        field,
                        fields.get("kind").and_then(Value::as_str),
                        fields.get("encoding").and_then(Value::as_str),
                    ),
                )
            })),
            Value::String(text) if numeric => admission.admit_decimal_lexeme(text)?,
            _ => {}
        }
    }
    let receipt = admission.receipt();
    let mut budget = purrdf_geo_kernel::PreparationBudget::new(policy);
    budget.retain(receipt.work_items(), receipt.workspace_bytes())?;
    Ok((value, budget))
}

/// Numeric payload roles of the strict version-one records. Names, identifiers,
/// carrier lexical strings and binary bit strings are never parsed as decimals.
fn decimal_field(field: &str, kind: Option<&str>, encoding: Option<&str>) -> bool {
    if field == "value" {
        return matches!(kind, Some("decimal" | "integer"));
    }
    if matches!(field, "longitude" | "latitude") && encoding == Some("ieee64") {
        return false;
    }
    matches!(
        field,
        "longitude"
            | "latitude"
            | "x"
            | "y"
            | "z"
            | "m"
            | "z_metres"
            | "epoch_decimal_year"
            | "radius_metres"
            | "threshold_metres"
            | "azimuth_degrees"
            | "length_metres"
            | "parameter"
            | "maximum_edge_metres"
            | "metres_per_unit"
            | "semimajor_metres"
            | "inverse_flattening"
            | "eccentricity_squared"
            | "central_meridian_degrees"
            | "scale"
            | "false_easting_metres"
            | "false_northing_metres"
            | "scale_ppm"
            | "scale_ppm_per_year"
            | "rotation_degrees"
            | "translation_metres"
            | "rotation_arcseconds"
            | "pivot_metres"
            | "translation_metres_per_year"
            | "rotation_arcseconds_per_year"
            | "origin_metres"
            | "normalization_metres"
            | "spacing_metres"
            | "min_x_metres"
            | "max_x_metres"
            | "min_y_metres"
            | "max_y_metres"
            | "west"
            | "east"
            | "south"
            | "north"
            | "x_coefficient_metres"
            | "y_coefficient_metres"
            | "x_metres"
            | "y_metres"
    )
}
