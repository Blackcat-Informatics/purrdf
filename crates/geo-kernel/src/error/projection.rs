// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original applicability and Mercator input refusals.

/// The original GCJ applicability requirement that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GcjInputReason {
    /// The declared applicability includes a pole.
    Poles,
    /// The actual source point lies outside the explicit applicability.
    Outside,
}
impl purrdf_hash::error::StructuredError for GcjInputReason {
    purrdf_hash::error_visit! {
        Poles => "gcj-input-poles" {},
        Outside => "gcj-input-outside" {},
    }
}
impl core::fmt::Display for GcjInputReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Poles => "GCJ applicability must exclude both poles",
            Self::Outside => "GCJ point is outside the explicitly declared applicability",
        })
    }
}

/// The first rejected requirement for explicit Mercator parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MercatorRequirement {
    /// The original radius/semimajor axis is not positive.
    Radius,
    /// The original squared eccentricity is below zero.
    EccentricityLower,
    /// The original squared eccentricity is at least one.
    EccentricityUpper,
}
impl purrdf_hash::error::StructuredError for MercatorRequirement {
    purrdf_hash::error_visit! {
        Radius => "mercator-requirement-radius" {},
        EccentricityLower => "mercator-requirement-eccentricity-lower" {},
        EccentricityUpper => "mercator-requirement-eccentricity-upper" {},
    }
}

/// Native projection inputs at their original validation failure.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProjectionInputError {
    /// Original GCJ applicability and the rejected point, when present.
    Gcj {
        /// First rejected requirement.
        reason: GcjInputReason,
        /// Complete original applicability declaration.
        domain: Box<crate::operation::Applicability>,
        /// Original point for an outside-applicability refusal.
        point: Option<crate::operation::OperationPoint>,
    },
    /// Invalid explicit radius or squared eccentricity.
    MercatorConfiguration {
        /// First rejected requirement in the original comparison order.
        requirement: MercatorRequirement,
        /// Original semantic model descriptor.
        law: &'static [u8],
        /// Original inverse square-domain declaration, absent on other models.
        square_domain: Option<bool>,
        /// Original radius or semimajor axis.
        radius: crate::context::Retained<crate::Rat>,
        /// Original squared eccentricity, absent on spherical Web Mercator.
        eccentricity_squared: Option<crate::context::Retained<crate::Rat>>,
    },
    /// A Mercator source point is at a pole.
    MercatorPole {
        /// Original semantic model descriptor.
        law: &'static [u8],
        /// Complete original point, including any height and epoch.
        point: crate::operation::OperationPoint,
        /// Original explicitly supplied radius/semimajor axis, when supplied.
        radius: Option<crate::context::Retained<crate::Rat>>,
        /// Original explicitly supplied squared eccentricity, when supplied.
        eccentricity_squared: Option<crate::context::Retained<crate::Rat>>,
    },
}
impl purrdf_hash::error::StructuredError for ProjectionInputError {
    purrdf_hash::error_visit! {
        Gcj { reason, domain, point } => "projection-gcj-input" {
            "reason" => Record(reason), "domain" => Record(&**domain),
            "point" => Value(match point {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
        MercatorConfiguration { requirement, law, square_domain, radius, eccentricity_squared } => "projection-mercator-configuration" {
            "requirement" => Record(requirement), "law" => Bytes(law),
            "square_domain" => Value(match square_domain {
                Some(value) => purrdf_hash::error::ErrorValue::Boolean(*value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "radius" => Record(&**radius),
            "eccentricity_squared" => Value(match eccentricity_squared {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
        MercatorPole { law, point, radius, eccentricity_squared } => "projection-mercator-pole" {
            "law" => Bytes(law), "point" => Record(point),
            "radius" => Value(match radius {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "eccentricity_squared" => Value(match eccentricity_squared {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
    }
}
impl ProjectionInputError {
    pub(crate) fn gcj(
        domain: &crate::operation::Applicability,
        point: Option<&crate::operation::OperationPoint>,
        reason: GcjInputReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            domain,
            point,
            None,
            progress,
            |domain, point, _, progress| {
                crate::context::Retained::produce(
                    progress,
                    |memory| {
                        let domain = memory.boxed(domain).map_err(super::GeoError::Storage)?;
                        Ok(Self::Gcj {
                            reason,
                            domain,
                            point,
                        })
                    },
                    |_, _| Ok(()),
                )
                .map_or_else(|error| error, super::GeoError::ProjectionInput)
            },
        )
    }

    pub(crate) fn mercator_configuration(
        radius: &crate::Rat,
        eccentricity_squared: Option<&crate::Rat>,
        law: &'static [u8],
        square_domain: Option<bool>,
        requirement: MercatorRequirement,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let radius = super::GeoError::diagnostic_number(radius, progress)?;
            let eccentricity_squared = eccentricity_squared
                .map(|value| super::GeoError::diagnostic_number(value, progress))
                .transpose()?;
            Ok(Self::MercatorConfiguration {
                requirement,
                law,
                square_domain,
                radius,
                eccentricity_squared,
            })
        })();
        match captured {
            Ok(cause) => super::GeoError::retained_failure_with_progress(
                cause,
                super::GeoError::ProjectionInput,
                progress,
            ),
            Err(error) => error,
        }
    }

    pub(crate) fn mercator_pole(
        point: &crate::operation::OperationPoint,
        law: &'static [u8],
        radius: Option<&crate::Rat>,
        eccentricity_squared: Option<&crate::Rat>,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(point, None, None, progress, |point, _, _, progress| {
            let captured = (|| {
                let radius = radius
                    .map(|value| super::GeoError::diagnostic_number(value, progress))
                    .transpose()?;
                let eccentricity_squared = eccentricity_squared
                    .map(|value| super::GeoError::diagnostic_number(value, progress))
                    .transpose()?;
                Ok(Self::MercatorPole {
                    law,
                    point,
                    radius,
                    eccentricity_squared,
                })
            })();
            match captured {
                Ok(cause) => super::GeoError::retained_failure_with_progress(
                    cause,
                    super::GeoError::ProjectionInput,
                    progress,
                ),
                Err(error) => error,
            }
        })
    }

    /// Whether the original source point is outside the model's domain.
    #[must_use]
    pub const fn is_domain_error(&self) -> bool {
        matches!(
            self,
            Self::Gcj {
                reason: GcjInputReason::Outside,
                ..
            } | Self::MercatorPole { .. }
        )
    }
}
impl core::fmt::Display for ProjectionInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Gcj { reason, .. } => core::fmt::Display::fmt(reason, f),
            Self::MercatorConfiguration {
                eccentricity_squared: None,
                ..
            } => f.write_str("Web Mercator radius must be positive"),
            Self::MercatorConfiguration {
                eccentricity_squared: Some(_),
                ..
            } => f.write_str(
                "inverse Mercator requires positive radius and squared eccentricity in [0,1)",
            ),
            Self::MercatorPole { .. } => f.write_str("Mercator is undefined at a pole"),
        }
    }
}
impl std::error::Error for ProjectionInputError {}
