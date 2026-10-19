// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original exact operands refused by metric-unit conversion.

/// The requirement that failed at the original conversion computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricUnitReason {
    /// The declared linear factor is zero or negative.
    NonPositiveFactor,
    /// A checked positive effective divisor unexpectedly had no quotient.
    MissingQuotient,
}
impl purrdf_hash::error::StructuredError for MetricUnitReason {
    purrdf_hash::error_visit! {
        NonPositiveFactor => "metric-unit-nonpositive-factor" {},
        MissingQuotient => "metric-unit-missing-quotient" {},
    }
}

/// The original conversion request and its exact refused factor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetricUnitError {
    /// Original conversion requirement.
    pub reason: MetricUnitReason,
    /// Original exact input quantity.
    pub value: crate::context::Retained<crate::Rat>,
    /// Original declared linear metre factor.
    pub factor: crate::context::Retained<crate::Rat>,
    /// Original effective divisor, when division itself refused.
    pub divisor: Option<crate::context::Retained<crate::Rat>>,
    /// Original requested physical dimension.
    pub dimension: crate::metric::MetricDimension,
    /// Original conversion direction.
    pub direction: crate::metric::MetricUnitDirection,
}
impl purrdf_hash::error::StructuredError for MetricUnitError {
    purrdf_hash::error_visit! {
        @record self { reason, value, factor, divisor, dimension, direction } => "metric-unit" {
            "reason" => Record(&self.reason), "value" => Record(&*self.value), "factor" => Record(&*self.factor),
            "divisor" => Value(match &self.divisor {
                Some(value) => purrdf_hash::error::ErrorValue::Record(&**value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "dimension" => Record(&self.dimension), "direction" => Record(&self.direction)
        }
    }
}
impl MetricUnitError {
    pub(crate) fn capture(
        reason: MetricUnitReason,
        value: &crate::Rat,
        factor: &crate::Rat,
        divisor: Option<&crate::Rat>,
        dimension: crate::metric::MetricDimension,
        direction: crate::metric::MetricUnitDirection,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let value = super::GeoError::diagnostic_number(value, progress)?;
            let factor = super::GeoError::diagnostic_number(factor, progress)?;
            let divisor = divisor
                .map(|value| super::GeoError::diagnostic_number(value, progress))
                .transpose()?;
            crate::context::Retained::produce(
                progress,
                |_| {
                    Ok(Self {
                        reason,
                        value,
                        factor,
                        divisor,
                        dimension,
                        direction,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::MetricUnit)
    }
}
impl core::fmt::Display for MetricUnitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self.reason {
            MetricUnitReason::NonPositiveFactor => {
                "metric output units require a positive linear metre factor"
            }
            MetricUnitReason::MissingQuotient => "positive metric unit factor required",
        })
    }
}
impl std::error::Error for MetricUnitError {}
