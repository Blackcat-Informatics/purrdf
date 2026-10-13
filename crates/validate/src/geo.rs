// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The version-one geographic string boundary shared by every language host.
//!
//! Records are strict: duplicate and unknown members refuse. Exact coordinates
//! and quantities are decimal strings; IEEE inputs use fixed-width bit strings.
//! Reference, mathematical-law and execution-policy identities remain separate.
//! No host implements coordinate mathematics or changes numerical output grids.

mod cover;
mod decode;
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

/// An immutable native profile snapshot. Clones retain the same admitted
/// source tables and exact shared control without copying the profile graph.
pub type GeoProfileSnapshot = purrdf_lex::allocation::Shared<GeoProfile>;

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

purrdf_lex::variant_from!(GeoCallError { Engine(GeoError) });

impl From<purrdf_lex::allocation::StorageError> for GeoCallError {
    fn from(error: purrdf_lex::allocation::StorageError) -> Self {
        Self::Engine(GeoError::Storage(error))
    }
}

impl From<DecodeError> for GeoCallError {
    fn from(error: DecodeError) -> Self {
        match error.storage_error() {
            Some(storage) => Self::Engine(GeoError::Storage(storage)),
            None => Self::Decode(error),
        }
    }
}

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

    fn within_native(
        self,
        token: &str,
        allocation: Option<&dyn purrdf_lex::json::record::RecordAllocation>,
    ) -> Self {
        match self {
            Self::Decode(error) => Self::Decode(error.within_native(token, allocation)),
            Self::Engine(error) => Self::Engine(error),
        }
    }

    fn within_index_native(
        self,
        index: usize,
        allocation: Option<&dyn purrdf_lex::json::record::RecordAllocation>,
    ) -> Self {
        match self {
            Self::Decode(error) => Self::Decode(error.within_index_native(index, allocation)),
            Self::Engine(error) => Self::Engine(error),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
fn read_record(text: &str) -> Result<purrdf_lex::json::Value, GeoCallError> {
    decode::record(
        text,
        purrdf_geo_kernel::ExecutionPolicy::geometry(),
        None,
        |value, decode| {
            decode
                .storage
                .native(|memory| value.clone_with_memory(memory))
                .map_err(GeoError::Storage)
                .map_err(Into::into)
        },
    )
    .map(|(value, _)| value)
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
