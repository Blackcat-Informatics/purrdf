// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original distance-certificate operands and failed requirements.

/// The first failed requirement for a nonnegative distance enclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DistanceEnclosureReason {
    /// The original lower bound exceeds the upper bound.
    Reversed,
    /// The original lower bound is negative.
    Negative,
}
impl purrdf_hash::error::StructuredError for DistanceEnclosureReason {
    purrdf_hash::error_visit! {
        Reversed => "distance-enclosure-reversed" {},
        Negative => "distance-enclosure-negative" {},
    }
}

/// The geodesic computation that requires nonnegative travel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TravelOperation {
    /// Direct geodesic solution.
    Direct,
    /// Azimuth/length arc preparation.
    Arc,
}
impl purrdf_hash::error::StructuredError for TravelOperation {
    purrdf_hash::error_visit! {
        Direct => "geodesic-travel-direct" {},
        Arc => "geodesic-travel-arc" {},
    }
}

/// The original operation consuming a closed-unit parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitParameterOperation {
    /// An interval of an original geodesic arc.
    ArcInterval,
    /// A point on a shortest geodesic arc.
    ShortestArc,
    /// A point on a written coordinate-linear edge.
    LinearEdge,
    /// A point on an azimuth/length arc.
    AzimuthArc,
}
impl purrdf_hash::error::StructuredError for UnitParameterOperation {
    purrdf_hash::error_visit! {
        ArcInterval => "unit-parameter-arc-interval" {},
        ShortestArc => "unit-parameter-shortest-arc" {},
        LinearEdge => "unit-parameter-linear-edge" {},
        AzimuthArc => "unit-parameter-azimuth-arc" {},
    }
}

/// The first failed closed-unit parameter requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitParameterRequirement {
    /// The first parameter is below zero.
    Lower,
    /// The last parameter is above one.
    Upper,
    /// The first parameter exceeds the last parameter.
    Order,
}
impl purrdf_hash::error::StructuredError for UnitParameterRequirement {
    purrdf_hash::error_visit! {
        Lower => "unit-parameter-lower" {},
        Upper => "unit-parameter-upper" {},
        Order => "unit-parameter-order" {},
    }
}

/// Original operands that cannot form a distance or geodesic result.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MetricInputError {
    /// The selected-branch overlap cannot form its exact endpoint parameter.
    BranchLength {
        /// Actual selected minimum travel length.
        minimum: crate::context::Retained<crate::Rat>,
        /// Actual branch length used as the parameter denominator.
        length: crate::context::Retained<crate::Rat>,
    },
    /// A direct solution or original arc has negative travel.
    NegativeTravel {
        /// Original consuming computation.
        operation: TravelOperation,
        /// Original exact travel quantity in metres.
        distance: crate::context::Retained<crate::Rat>,
    },
    /// An original parameter or interval is outside the closed unit domain.
    Parameter {
        /// Original consuming operation.
        operation: UnitParameterOperation,
        /// First original requirement that failed.
        requirement: UnitParameterRequirement,
        /// Original parameter or lower endpoint.
        lower: crate::context::Retained<crate::Rat>,
        /// Original upper endpoint for an interval, absent for a single parameter.
        upper: Option<crate::context::Retained<crate::Rat>>,
    },
    /// A quantized shortest distance must be nonnegative.
    NegativeDistance {
        /// Original integer micrometre quantity, moved without copying its limbs.
        micrometres: crate::Int,
        /// Original certificate binding.
        binding: crate::GeoBindingId,
    },
    /// The original closed interval is not a nonnegative distance enclosure.
    Enclosure {
        /// First original requirement that failed.
        reason: DistanceEnclosureReason,
        /// Original exact lower bound.
        lower: crate::context::Retained<crate::Rat>,
        /// Original exact upper bound.
        upper: crate::context::Retained<crate::Rat>,
        /// Original certificate binding.
        binding: crate::GeoBindingId,
        /// Original admitted precision.
        precision_bits: u32,
    },
}
impl purrdf_hash::error::StructuredError for MetricInputError {
    purrdf_hash::error_visit! {
        BranchLength { minimum, length } => "metric-selected-branch-length" {
            "minimum" => Record(&**minimum), "length" => Record(&**length)
        },
        NegativeTravel { operation, distance } => "metric-negative-travel" {
            "operation" => Record(operation), "distance" => Record(&**distance)
        },
        Parameter { operation, requirement, lower, upper } => "metric-unit-parameter" {
            "operation" => Record(operation), "requirement" => Record(requirement), "lower" => Record(&**lower),
            "upper" => Value(match upper {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "minimum" => Unsigned(0), "maximum" => Unsigned(1)
        },
        NegativeDistance { micrometres, binding } => "metric-negative-distance" {
            "micrometres" => Record(micrometres), "binding" => Record(binding)
        },
        Enclosure { reason, lower, upper, binding, precision_bits } => "metric-invalid-enclosure" {
            "reason" => Record(reason), "lower" => Record(&**lower), "upper" => Record(&**upper),
            "binding" => Record(binding), "precision_bits" => Unsigned(u128::from(*precision_bits))
        },
    }
}
impl MetricInputError {
    pub(crate) fn branch_length(
        minimum: &crate::Rat,
        length: &crate::Rat,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let minimum = super::GeoError::diagnostic_number(minimum, progress)?;
            let length = super::GeoError::diagnostic_number(length, progress)?;
            crate::context::Retained::produce(
                progress,
                |_| Ok(Self::BranchLength { minimum, length }),
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::MetricInput)
    }

    pub(crate) fn negative_travel(
        distance: &crate::Rat,
        operation: TravelOperation,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured =
            super::GeoError::diagnostic_number(distance, progress).and_then(|distance| {
                crate::context::Retained::produce(
                    progress,
                    |_| {
                        Ok(Self::NegativeTravel {
                            operation,
                            distance,
                        })
                    },
                    |_, _| Ok(()),
                )
            });
        captured.map_or_else(|error| error, super::GeoError::MetricInput)
    }

    pub(crate) fn parameter(
        lower: &crate::Rat,
        upper: Option<&crate::Rat>,
        operation: UnitParameterOperation,
        requirement: UnitParameterRequirement,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let lower = super::GeoError::diagnostic_number(lower, progress)?;
            let upper = upper
                .map(|value| super::GeoError::diagnostic_number(value, progress))
                .transpose()?;
            crate::context::Retained::produce(
                progress,
                |_| {
                    Ok(Self::Parameter {
                        operation,
                        requirement,
                        lower,
                        upper,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::MetricInput)
    }

    pub(crate) fn negative_distance(
        micrometres: crate::Int,
        binding: crate::GeoBindingId,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        crate::context::Retained::produce(
            progress,
            |_| {
                Ok(Self::NegativeDistance {
                    micrometres,
                    binding,
                })
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::MetricInput)
    }

    pub(crate) fn enclosure(
        reason: DistanceEnclosureReason,
        lower: &crate::Rat,
        upper: &crate::Rat,
        binding: crate::GeoBindingId,
        precision_bits: u32,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let lower = super::GeoError::diagnostic_number(lower, progress)?;
            let upper = super::GeoError::diagnostic_number(upper, progress)?;
            crate::context::Retained::produce(
                progress,
                |_| {
                    Ok(Self::Enclosure {
                        reason,
                        lower,
                        upper,
                        binding,
                        precision_bits,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::MetricInput)
    }
}
impl core::fmt::Display for MetricInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::BranchLength { .. } => "positive selected branch length",
            Self::NegativeTravel { operation, .. } => match operation {
                TravelOperation::Direct => "direct distance must be nonnegative",
                TravelOperation::Arc => "arc travel distance must be nonnegative",
            },
            Self::Parameter { operation, .. } => match operation {
                UnitParameterOperation::ArcInterval => "arc parameter interval must be in [0,1]",
                UnitParameterOperation::ShortestArc => "geodesic arc parameter must be in [0,1]",
                UnitParameterOperation::LinearEdge => "edge parameter outside [0,1]",
                UnitParameterOperation::AzimuthArc => "arc parameter outside [0,1]",
            },
            Self::NegativeDistance { .. } => "a shortest distance cannot be negative",
            Self::Enclosure { .. } => "invalid nonnegative distance enclosure",
        })
    }
}
impl std::error::Error for MetricInputError {}
