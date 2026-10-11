// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original selected boundary fragments. Contacts remain implicit in topology;
//! this metric's separate law freezes their half-even parameter grid at 24
//! decimal places and certifies both omitted/added endpoint tails. A tighter
//! contact proof cannot select a different completed approximation.

use super::{GeometryMetricLaw, integrate_image_on, integrate_linear};
use crate::atlas::arcs::arrangement::{NativeBoundaryArrangement, SourceParameter};
use crate::context::{RetainedBuilder, WorkProgress};
use crate::numerical::{ExactAdmission, fixed_from_rat, math_exact_rational, math_exact_rationals};
use crate::{GeoError, MetricContext, PreparedEdge, Rat, SourceLinearEdge};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

pub(crate) struct Parameters {
    pub(crate) endpoints: [Rat; 2],
    error: [Rat; 2],
}
crate::context::retained::native_detached_fields!(Parameters {
    native: [endpoints, error],
    copy: []
});
impl Parameters {
    pub(crate) fn endpoint_errors(&self) -> [&Rat; 2] {
        self.error.each_ref()
    }
}

pub(super) fn prepare(
    boundary: &NativeBoundaryArrangement,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<RetainedBuilder<Parameters>, GeoError> {
    prepare_parameters(
        boundary
            .fragments()
            .iter()
            .map(|fragment| fragment.parameters().each_ref()),
        context,
        progress,
        retained,
    )
}

/// One frozen contact-grid completion for metric and distance samples. These
/// samples carry original-cut tail bounds; they never replace the source set.
pub(super) fn prepare_parameters<'a>(
    parameters: impl ExactSizeIterator<Item = [&'a SourceParameter; 2]>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<RetainedBuilder<Parameters>, GeoError> {
    let mut output = RetainedBuilder::default();
    output.reserve_admitted(parameters.len(), context, progress, retained)?;
    for pair in parameters {
        let mut endpoints = [Rat::zero(), Rat::zero()];
        let mut errors = [Rat::zero(), Rat::zero()];
        for (index, original) in pair.into_iter().enumerate() {
            let mut refined = None;
            let mut bits = 80;
            loop {
                let cut = refined.as_ref().unwrap_or(original);
                let (lower, upper) = cut.bounds();
                let round =
                    |value: &Rat, context: &mut MetricContext, progress: &mut WorkProgress<'_>| {
                        let cost = ExactArithmeticCost::rational_round(
                            value.numerator().bit_len(),
                            value.denominator().bit_len(),
                            24,
                        )
                        .ok_or(GeoError::ArithmeticOverflow(
                            "native contact parameter rounding",
                        ))?;
                        ExactAdmission::new(context, progress)
                            .expression_cost(cost, |values| Ok(values.round_to_scale(value, 24)?))
                    };
                let low = round(lower, context, progress)?;
                let high = round(upper, context, progress)?;
                if low == high {
                    let cost = ExactArithmeticCost::decimal_rational(low.bit_len(), 24).ok_or(
                        GeoError::ArithmeticOverflow("native parameter decimal construction"),
                    )?;
                    let value = ExactAdmission::new(context, progress)
                        .expression_cost(cost, |values| Ok(values.decimal(&low, 24)?))?;
                    let mut admission = ExactAdmission::new(context, progress);
                    let left = admission.expression_result(
                        ExactOperation::RationalAdd,
                        &[&value, lower],
                        1,
                        |values| Ok(values.absolute(&values.subtract(&value, lower)?)?),
                    )?;
                    let right = admission.expression_result(
                        ExactOperation::RationalAdd,
                        &[upper, &value],
                        1,
                        |values| Ok(values.absolute(&values.subtract(upper, &value)?)?),
                    )?;
                    let order = admission.expression_result(
                        ExactOperation::RationalCompare,
                        &[&left, &right],
                        1,
                        |values| Ok(values.compare(&(left), &right)?),
                    )?;
                    let error = if order.is_lt() { right } else { left };
                    endpoints[index] = value;
                    errors[index] = error;
                    break;
                }
                let maximum = context.policy().limits().max_precision_bits;
                if bits > maximum {
                    return Err(GeoError::PrecisionExhausted { bits: maximum });
                }
                refined = Some(original.refined_in(bits, context, progress)?);
                bits = bits.saturating_mul(2);
                if bits > maximum && bits / 2 < maximum {
                    bits = maximum;
                }
            }
        }
        ExactAdmission::new(context, progress).push_detached(
            &mut output,
            Parameters {
                endpoints,
                error: errors,
            },
            retained,
        )?;
    }
    Ok(output)
}

pub(super) fn length(
    edge: &PreparedEdge,
    parameters: &Parameters,
    ellipsoid: &crate::PreparedEllipsoid,
    limits: (u32, u32),
    budget: &Rat,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let width = fixed_from_rat(&parameters.endpoints[1], math)?
        .sub(&fixed_from_rat(&parameters.endpoints[0], math)?, math)?;
    if width.lower().is_negative() {
        return Err(MathError::PrecisionExhausted);
    }
    let speed = match edge {
        PreparedEdge::SourceLinear(line) => {
            let variation = math_exact_rationals(
                ExactOperation::RationalAdd,
                &[
                    line.start().point().longitude(),
                    line.start().point().latitude(),
                    line.end().point().longitude(),
                    line.end().point().latitude(),
                ],
                3,
                math,
                progress,
                |values| {
                    let longitude = values.subtract(
                        line.end().point().longitude(),
                        line.start().point().longitude(),
                    )?;
                    let longitude = values.absolute(&longitude)?;
                    let latitude = values.subtract(
                        line.end().point().latitude(),
                        line.start().point().latitude(),
                    )?;
                    let latitude = values.absolute(&latitude)?;
                    values.add(&longitude, &latitude)
                },
            )?;
            fixed_from_rat(ellipsoid.normal_metric_bounds_ref().1.exact(), math)?
                .mul(&fixed_from_rat(&variation, math)?, math)?
                .mul(&FixedInterval::pi(math)?, math)?
                .div(&FixedInterval::from_i64(180, math)?, math)?
        }
        PreparedEdge::AzimuthLength(arc) => fixed_from_rat(arc.length().exact(), math)?,
        PreparedEdge::ShortestGeodesic(arc) => crate::numerical::fixed_from_bounds(
            arc.proof().distance.lower.exact(),
            arc.proof().distance.upper.exact(),
            math,
        )?,
        PreparedEdge::Transformed(image) => image.speed_in(
            &Rat::zero(),
            &Rat::one(),
            crate::operation::OperationSolverLimits {
                iterations: limits.1,
                subdivisions: limits.0,
                quantize_inverse: false,
            },
            math,
            progress,
        )?,
    };
    let error = fixed_from_rat(&parameters.error[0], math)?
        .add(&fixed_from_rat(&parameters.error[1], math)?, math)?
        .mul(&speed, math)?;
    let allowed = fixed_from_rat(budget, math)?.div(&FixedInterval::from_i64(16, math)?, math)?;
    if error.upper() > allowed.lower() {
        return Err(MathError::PrecisionExhausted);
    }
    let core_budget = math_exact_rationals(
        ExactOperation::RationalDivide,
        &[budget],
        2,
        math,
        progress,
        |values| {
            let product = values.multiply(budget, &Rat::from_i64(15))?;
            Ok(values
                .divide(&product, &Rat::from_i64(16))?
                .expect("positive denominator"))
        },
    )?;
    match edge {
        PreparedEdge::SourceLinear(line) => {
            let mut endpoint = |parameter: &Rat| {
                let longitude = SourceLinearEdge::interpolate_ordinate_math(
                    line.start().point().longitude(),
                    line.end().point().longitude(),
                    parameter,
                    math,
                    progress,
                )?;
                let latitude = SourceLinearEdge::interpolate_ordinate_math(
                    line.start().point().latitude(),
                    line.end().point().latitude(),
                    parameter,
                    math,
                    progress,
                )?;
                math_exact_rational(
                    ExactOperation::RationalCompare,
                    &[&longitude, &latitude],
                    math,
                    progress,
                    |values| values.location_result(&longitude, &latitude),
                )?
                .map_err(|_| MathError::Domain("native source fragment left geographic bounds"))
            };
            let start = endpoint(&parameters.endpoints[0])?;
            let end = endpoint(&parameters.endpoints[1])?;
            integrate_linear(
                (&start, &end),
                GeometryMetricLaw::Length,
                ellipsoid,
                limits.0,
                &core_budget,
                math,
                progress,
            )
        }
        PreparedEdge::AzimuthLength(_) | PreparedEdge::ShortestGeodesic(_) => {
            speed.mul(&width, math)
        }
        PreparedEdge::Transformed(image) => integrate_image_on(
            image,
            limits,
            &core_budget,
            Some(parameters.endpoints.each_ref()),
            math,
            progress,
        ),
    }
}
