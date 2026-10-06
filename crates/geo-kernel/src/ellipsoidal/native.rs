// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original selected boundary fragments. Contacts remain implicit in topology;
//! this metric's separate law freezes their half-even parameter grid at 24
//! decimal places and certifies both omitted/added endpoint tails. A tighter
//! contact proof cannot select a different completed approximation.

use super::{GeometryMetricLaw, integrate_image_on, integrate_linear};
use crate::atlas::arcs::arrangement::{NativeBoundaryArrangement, SourceParameter};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, fixed_from_rat, math_exact_rational, math_exact_rationals};
use crate::{GeoError, LonLat, MetricContext, PreparedEdge, Rat, SourceLinearEdge};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

pub(crate) struct Parameters {
    pub(crate) endpoints: [Rat; 2],
    error: [Rat; 2],
}
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
) -> Result<Vec<Parameters>, GeoError> {
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
) -> Result<Vec<Parameters>, GeoError> {
    let bytes = (parameters.len() as u64).saturating_mul(4096);
    context.admit_workspace(bytes)?;
    *retained = retained
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow("native metric parameters"))?;
    let mut output = Vec::with_capacity(parameters.len());
    for pair in parameters {
        let mut endpoints = Vec::with_capacity(2);
        let mut errors = Vec::with_capacity(2);
        for original in pair {
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
                        progress.exact_context(context, cost, |context| {
                            context.integer_scratch().map_or_else(
                                || Ok(value.round_to_scale(24)),
                                |scratch| {
                                    value
                                        .round_to_scale_in(24, scratch)
                                        .map_err(GeoError::NumericalScratch)
                                },
                            )
                        })
                    };
                let low = round(lower, context, progress)?;
                let high = round(upper, context, progress)?;
                if low == high {
                    let cost = ExactArithmeticCost::decimal_rational(low.bit_len(), 24).ok_or(
                        GeoError::ArithmeticOverflow("native parameter decimal construction"),
                    )?;
                    let value = progress.exact_context(context, cost, |context| {
                        context.integer_scratch().map_or_else(
                            || Ok(Rat::from_decimal(low.clone(), 24)),
                            |scratch| {
                                Rat::from_decimal_in(&low, 24, scratch)
                                    .map_err(GeoError::NumericalScratch)
                            },
                        )
                    })?;
                    let mut admission = ExactAdmission::new(context, progress);
                    let left = admission.rational(
                        ExactOperation::RationalAdd,
                        &[&value, lower],
                        1,
                        || Ok(value.sub(lower).abs()),
                    )?;
                    let right = admission.rational(
                        ExactOperation::RationalAdd,
                        &[upper, &value],
                        1,
                        || Ok(upper.sub(&value).abs()),
                    )?;
                    let order = admission.rational(
                        ExactOperation::RationalCompare,
                        &[&left, &right],
                        1,
                        || Ok(left.cmp(&right)),
                    )?;
                    let error = if order.is_lt() { right } else { left };
                    endpoints.push(value);
                    errors.push(error);
                    break;
                }
                let maximum = context.policy().limits().max_precision_bits;
                if bits > maximum {
                    return Err(GeoError::PrecisionExhausted { bits: maximum });
                }
                let reservation = original.refinement_storage_bound(bits)?;
                context.admit_workspace(reservation)?;
                let result = original.refined_in(bits, context, progress);
                let old = refined
                    .as_ref()
                    .map_or(0, SourceParameter::retained_workspace_bytes);
                refined = match result {
                    Ok(value) => {
                        let owned = value.retained_workspace_bytes();
                        if owned > reservation {
                            context.release_workspace(reservation)?;
                            return Err(GeoError::ArithmeticOverflow(
                                "refined source-cut storage bound",
                            ));
                        }
                        context.release_workspace(reservation.saturating_sub(owned))?;
                        *retained =
                            retained
                                .checked_add(owned)
                                .ok_or(GeoError::ArithmeticOverflow(
                                    "native parameter proof storage",
                                ))?;
                        context.release_workspace(old)?;
                        *retained =
                            retained
                                .checked_sub(old)
                                .ok_or(GeoError::ArithmeticOverflow(
                                    "native parameter proof release",
                                ))?;
                        Some(value)
                    }
                    Err(error) => {
                        context.release_workspace(reservation)?;
                        return Err(error);
                    }
                };
                bits = bits.saturating_mul(2);
                if bits > maximum && bits / 2 < maximum {
                    bits = maximum;
                }
            }
        }
        output.push(Parameters {
            endpoints: endpoints.try_into().expect("two fragment endpoints"),
            error: errors.try_into().expect("two fragment endpoint enclosures"),
        });
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
                || {
                    line.end()
                        .point()
                        .longitude()
                        .sub(line.start().point().longitude())
                        .abs()
                        .add(
                            &line
                                .end()
                                .point()
                                .latitude()
                                .sub(line.start().point().latitude())
                                .abs(),
                        )
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
        || {
            budget
                .mul(&Rat::from_i64(15))
                .div(&Rat::from_i64(16))
                .expect("positive denominator")
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
                    || LonLat::new(longitude.clone(), latitude.clone()),
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
