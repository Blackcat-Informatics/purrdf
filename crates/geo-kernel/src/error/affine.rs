// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original affine declarations and rejected observation metadata.

/// The first rejected requirement of an original Helmert declaration or point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HelmertInputReason {
    /// The declared ppm scale does not give a positive scale factor.
    DeclaredScale,
    /// A rate-bearing model requires an actual observation epoch.
    ObservationEpoch,
    /// The original epoch-adjusted ppm scale does not give a positive factor.
    AdjustedScale,
    /// Epoch adjustment itself was entered without the required epoch.
    AdjustmentEpoch,
}
impl purrdf_hash::error::StructuredError for HelmertInputReason {
    purrdf_hash::error_visit! {
        DeclaredScale => "helmert-input-declared-scale" {},
        ObservationEpoch => "helmert-input-observation-epoch" {},
        AdjustedScale => "helmert-input-adjusted-scale" {},
        AdjustmentEpoch => "helmert-input-adjustment-epoch" {},
    }
}
impl core::fmt::Display for HelmertInputReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::DeclaredScale => "Helmert scale factor must be positive",
            Self::ObservationEpoch => "a rate-bearing Helmert requires an actual observation epoch",
            Self::AdjustedScale => "epoch-adjusted Helmert scale factor must be positive",
            Self::AdjustmentEpoch => "a Helmert observation epoch is required",
        })
    }
}

/// Native affine input data at the original refusal computation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AffineInputError {
    /// An original Helmert declaration or its observation metadata is invalid.
    Helmert {
        /// First rejected requirement.
        reason: HelmertInputReason,
        /// Complete original model, including rates, pivot and conventions.
        parameters: Box<crate::operation::HelmertParameters>,
        /// Complete original point when it has already been constructed.
        point: Option<crate::operation::OperationPoint>,
        /// Original height at metadata ingress, before point construction.
        height: Option<crate::context::Retained<crate::Rat>>,
        /// Original computed ppm scale for an adjusted-scale failure.
        adjusted_scale: Option<crate::context::Retained<crate::Rat>>,
    },
    /// The declared similarity scale is not positive.
    SimilarityScale {
        /// Complete original similarity declaration.
        parameters: crate::operation::Similarity2d,
    },
}
impl purrdf_hash::error::StructuredError for AffineInputError {
    purrdf_hash::error_visit! {
        Helmert { reason, parameters, point, height, adjusted_scale } => "affine-helmert-input" {
            "reason" => Record(reason), "parameters" => Record(&**parameters),
            "point" => Value(match point {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "height" => Value(match height {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "adjusted_scale" => Value(match adjusted_scale {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
        SimilarityScale { parameters } => "affine-similarity-scale" { "parameters" => Record(parameters) },
    }
}
impl AffineInputError {
    pub(crate) fn helmert(
        parameters: &crate::operation::HelmertParameters,
        point: Option<&crate::operation::OperationPoint>,
        height: Option<&crate::Rat>,
        adjusted_scale: Option<&crate::Rat>,
        reason: HelmertInputReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            parameters,
            point,
            adjusted_scale,
            progress,
            |parameters, point, adjusted_scale, progress| {
                let height = match height
                    .map(|value| super::GeoError::diagnostic_number(value, progress))
                    .transpose()
                {
                    Ok(height) => height,
                    Err(error) => return error,
                };
                crate::context::Retained::produce(
                    progress,
                    |memory| {
                        let parameters =
                            memory.boxed(parameters).map_err(super::GeoError::Storage)?;
                        Ok(Self::Helmert {
                            reason,
                            parameters,
                            point,
                            height,
                            adjusted_scale,
                        })
                    },
                    |_, _| Ok(()),
                )
                .map_or_else(|error| error, super::GeoError::AffineInput)
            },
        )
    }

    pub(crate) fn similarity(
        parameters: &crate::operation::Similarity2d,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::diagnostic_inputs(
            parameters,
            None,
            None,
            progress,
            |parameters, _, _, progress| {
                super::GeoError::retained_failure_with_progress(
                    Self::SimilarityScale { parameters },
                    super::GeoError::AffineInput,
                    progress,
                )
            },
        )
    }

    /// Whether this is an observation/point failure rather than a model declaration failure.
    #[must_use]
    pub const fn is_domain_error(&self) -> bool {
        matches!(
            self,
            Self::Helmert {
                reason: HelmertInputReason::ObservationEpoch
                    | HelmertInputReason::AdjustedScale
                    | HelmertInputReason::AdjustmentEpoch,
                ..
            }
        )
    }
}
impl core::fmt::Display for AffineInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Helmert { reason, .. } => core::fmt::Display::fmt(reason, f),
            Self::SimilarityScale { .. } => f.write_str("similarity scale must be positive"),
        }
    }
}
impl std::error::Error for AffineInputError {}

/// The original human context of a typed validation failure crossing a math seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationValidationContext {
    /// Observation adjustment inside an affine matrix computation.
    AffineEpoch,
    /// Presence or validation of actual source-panel metadata.
    SourceMetadata,
    /// Original source-panel coordinate validation.
    SourceDomain,
    /// Validation of the rounded inverse in its original declared source zone.
    InverseSourceZone,
    /// Exact grid location at a numerical boundary.
    GridLocation,
    /// Original missing patch corner during image evaluation.
    GridHole,
}
impl purrdf_hash::error::StructuredError for OperationValidationContext {
    purrdf_hash::error_visit! {
        AffineEpoch => "operation-validation-affine-epoch" {},
        SourceMetadata => "operation-validation-source-metadata" {},
        SourceDomain => "operation-validation-source-domain" {},
        InverseSourceZone => "operation-validation-inverse-source-zone" {},
        GridLocation => "operation-validation-grid-location" {},
        GridHole => "operation-validation-grid-hole" {},
    }
}
impl OperationValidationContext {
    pub(crate) const fn detail(self) -> &'static str {
        match self {
            Self::AffineEpoch => "missing epoch",
            Self::SourceMetadata => "source image panel lacks actual model metadata",
            Self::SourceDomain => "source image panel outside declared model domain",
            Self::InverseSourceZone => "rounded inverse outside its declared source zone",
            Self::GridLocation => "bilinear grid point is outside complete patches",
            Self::GridHole => "bilinear grid hole",
        }
    }
}
