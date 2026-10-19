// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original transverse-projection declarations and rejected source points.

/// The first rejected transverse-projection requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransverseInputReason {
    /// The original scale is not positive.
    Scale,
    /// The original squared eccentricity exceeds the Taylor certificate bound.
    Eccentricity,
    /// UTM requires a zone in 1..=60 without a zone prefix.
    UtmZone,
    /// The declared meridian differs from the zone's meridian or its range.
    Meridian,
    /// The source latitude contradicts the declared hemisphere.
    Hemisphere,
    /// The original source latitude lies outside UTM's admitted domain.
    Latitude,
    /// The original source longitude lies outside the declared zone.
    Zone,
}

impl purrdf_hash::error::StructuredError for TransverseInputReason {
    purrdf_hash::error_visit! {
        Scale => "transverse-input-scale" {},
        Eccentricity => "transverse-input-eccentricity" {},
        UtmZone => "transverse-input-utm-zone" {},
        Meridian => "transverse-input-meridian" {},
        Hemisphere => "transverse-input-hemisphere" {},
        Latitude => "transverse-input-latitude" {},
        Zone => "transverse-input-zone" {},
    }
}

impl core::fmt::Display for TransverseInputReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Scale => "transverse Mercator scale must be positive",
            Self::Eccentricity => {
                "native transverse Mercator Taylor certificate requires squared eccentricity <=0.01"
            }
            Self::UtmZone => "UTM requires zone 1..=60 and no zone prefix",
            Self::Meridian => "declared transverse Mercator zone and meridian disagree",
            Self::Hemisphere => "point contradicts the explicitly declared projection hemisphere",
            Self::Latitude => "point is outside the UTM latitude domain",
            Self::Zone => "point is outside the explicitly declared transverse Mercator zone",
        })
    }
}

/// Original projection inputs at their native validation failure.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransverseInputError {
    /// First requirement that failed, before later validation was attempted.
    pub reason: TransverseInputReason,
    /// Complete original projection declaration.
    pub projection: crate::operation::TransverseMercator,
    /// Complete original source point for point-domain failures.
    pub point: Option<crate::operation::OperationPoint>,
    /// Original computed comparison value, when the check computed one.
    pub comparison: Option<crate::context::Retained<crate::Rat>>,
}

impl purrdf_hash::error::StructuredError for TransverseInputError {
    purrdf_hash::error_visit! {
        @record self { reason, projection, point, comparison } => "transverse-input" {
            "reason" => Record(&self.reason), "projection" => Record(&self.projection),
            "point" => Value(match &self.point {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "comparison" => Value(match &self.comparison {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        }
    }
}

impl TransverseInputError {
    pub(crate) fn capture(
        projection: &crate::operation::TransverseMercator,
        point: Option<&crate::operation::OperationPoint>,
        comparison: Option<&crate::Rat>,
        reason: TransverseInputReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            projection,
            point,
            comparison,
            progress,
            |projection, point, comparison, progress| {
                super::GeoError::retained_failure_with_progress(
                    Self {
                        reason,
                        projection,
                        point,
                        comparison,
                    },
                    super::GeoError::TransverseInput,
                    progress,
                )
            },
        )
    }

    /// Whether the refusal is a source-point domain failure.
    #[must_use]
    pub const fn is_domain_error(&self) -> bool {
        matches!(
            self.reason,
            TransverseInputReason::Hemisphere
                | TransverseInputReason::Latitude
                | TransverseInputReason::Zone
        )
    }
}

impl core::fmt::Display for TransverseInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.reason, f)
    }
}
impl std::error::Error for TransverseInputError {}
