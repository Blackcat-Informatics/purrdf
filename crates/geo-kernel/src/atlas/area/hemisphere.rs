// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Two regular gauges with a complete, enclosed equatorial cut. A possible
//! contact is retained as its whole continuous longitude image; a tangency is
//! never discarded because a transverse root test cannot certify it. Unknown
//! strips carry both possible gauge fluxes and every possible cut contribution.

use super::{AreaPotential, PoleGauge, RingArea, integrate_range, tail_bound};
use crate::atlas::arcs::{self, ChartLine};
use crate::context::WorkProgress;
use crate::numerical::{
    ExactAdmission, exact_bounds, fixed_from_bounds, fixed_from_rat, geo_math_error,
};
use crate::{GeoError, LonLat, MetricContext, PreparedEdge, Rat, Set};
use purrdf_xsd::{
    BigInt,
    integer::ExactOperation,
    math::{CoordinateMath, FixedInterval, MathError},
};

/// Frozen source-parameter approximation, with complete original endpoint tails.
pub(super) struct Fragment<'a> {
    pub(super) edge: &'a PreparedEdge,
    pub(super) endpoints: [&'a Rat; 2],
    pub(super) errors: [&'a Rat; 2],
    pub(super) reversed: bool,
}

struct LongitudeBand {
    outer: FixedInterval,
    /// Only this inner interval is proved to be an actual equatorial run.
    boundary: Option<FixedInterval>,
}

struct BoundaryIntegral {
    value: FixedInterval,
    pole: FixedInterval,
    surface: FixedInterval,
    bands: Vec<LongitudeBand>,
    storage: u64,
}

impl BoundaryIntegral {
    fn owned_bytes(&self) -> u64 {
        let mut bytes = size_of::<Self>() + self.bands.capacity() * size_of::<LongitudeBand>();
        bytes += self.value.detached_heap_bytes()
            + self.pole.detached_heap_bytes()
            + self.surface.detached_heap_bytes();
        for band in &self.bands {
            bytes += band.outer.detached_heap_bytes();
            bytes += band
                .boundary
                .as_ref()
                .map_or(0, FixedInterval::detached_heap_bytes);
        }
        bytes as u64
    }
}

fn add_band(
    bands: &mut Vec<LongitudeBand>,
    outer: &FixedInterval,
    boundary: Option<&FixedInterval>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(), MathError> {
    let bytes = size_of::<LongitudeBand>()
        .saturating_mul(2)
        .saturating_add(outer.detached_heap_bytes())
        .saturating_add(boundary.map_or(0, FixedInterval::detached_heap_bytes));
    math.reserve_workspace(bytes)?;
    progress.math_poll(math)?;
    bands.push(LongitudeBand {
        outer: outer.detached(),
        boundary: boundary.map(FixedInterval::detached),
    });
    Ok(())
}

fn symmetric_error(
    value: &FixedInterval,
    error: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    value.add(&error.neg(math)?.hull(error, math)?, math)
}

fn endpoint_tail(
    line: &ChartLine,
    potential: &mut AreaPotential,
    parameter: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let (normal, _) = line.normal(parameter, iterations, math, progress)?;
    if normal[2].lower() >= &BigInt::zero() {
        potential.gauge = PoleGauge::North;
        tail_bound(line, potential, parameter, iterations, math, progress)
    } else if normal[2].upper() <= &BigInt::zero() {
        potential.gauge = PoleGauge::South;
        tail_bound(line, potential, parameter, iterations, math, progress)
    } else {
        // A cut band can be on either side. Both regular potentials must be
        // included; selecting only the currently preferred gauge loses a tail.
        potential.gauge = PoleGauge::North;
        let north = tail_bound(line, potential, parameter, iterations, math, progress)?;
        potential.gauge = PoleGauge::South;
        north.add(
            &tail_bound(line, potential, parameter, iterations, math, progress)?,
            math,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one complete original line partition has explicit proof and admission inputs"
)]
fn integrate_fragment(
    fragment: &Fragment<'_>,
    line: &ChartLine,
    potential: &mut AreaPotential,
    core_budget: &FixedInterval,
    band_budget: &FixedInterval,
    limits: (u32, u32),
    bands: &mut Vec<LongitudeBand>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let (iterations, depth_limit) = limits;
    if let ChartLine::Geodesic(line) = line
        && line.is_meridian()
    {
        // Retain the line home's exact endpoint exclusions and lifted pole
        // enumeration before partitioning a meridian in the smooth atlas.
        // Neither a chart alias nor an RN15 endpoint invents a pole event.
        line.interior_meridian_poles_in(iterations, math, progress)?;
    }
    let zero = FixedInterval::from_i64(0, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let unit = zero.hull(&one, math)?;
    let [start, end] = fragment.endpoints.map(|point| fixed_from_rat(point, math));
    let (start, end) = (start?, end?);
    let core = FixedInterval::from_bounds(start.lower().clone(), end.upper().clone(), math)?;
    let [first_error, last_error] = fragment.errors.map(|error| fixed_from_rat(error, math));
    let (first_error, last_error) = (first_error?, last_error?);
    // A Boundary label may only cover parameters present in every original
    // endpoint enclosure. The expanded inventory proves contact exclusion,
    // but its extra tails must remain uncertain rather than become Boundary.
    let guaranteed_start = start.add(&first_error, math)?;
    let guaranteed_end = end.sub(&last_error, math)?;
    let guaranteed = if guaranteed_start.upper() <= guaranteed_end.lower() {
        Some(FixedInterval::from_bounds(
            guaranteed_start.upper().clone(),
            guaranteed_end.lower().clone(),
            math,
        )?)
    } else {
        None
    };
    let expanded = start
        .sub(&first_error, math)?
        .hull(&end.add(&last_error, math)?, math)?;
    let expanded = expanded
        .intersection(&unit, math)?
        .ok_or(MathError::PrecisionExhausted)?;
    let scratch = (depth_limit as usize + 2).saturating_mul(4096);
    math.reserve_workspace(scratch)?;
    let result = (|| {
        let mut total = zero.clone();
        let mut unknown = zero.clone();
        let mut pending =
            purrdf_lex::walk::WorkList::<(FixedInterval, u32), 8>::with((expanded, 0));
        while let Some((parameter, depth)) = pending.pop() {
            progress.math_poll(math)?;
            let (normal, derivative) = line.normal(&parameter, iterations, math, progress)?;
            let north = normal[2].lower() > &BigInt::zero();
            let south = normal[2].upper() < &BigInt::zero();
            let mut accepted = north || south;
            potential.gauge = if south {
                PoleGauge::South
            } else {
                PoleGauge::North
            };
            let mut flux_error = zero.clone();
            if !accepted {
                let derivative = derivative.ok_or(MathError::PrecisionExhausted)?;
                let cross = normal[0]
                    .mul(&derivative[1], math)?
                    .sub(&normal[1].mul(&derivative[0], math)?, math)?;
                let equatorial = normal[2].is_exact_zero() && derivative[2].is_exact_zero();
                let angular = if equatorial {
                    Some(line.image(&parameter, iterations, math, progress)?)
                } else {
                    None
                };
                let direction = angular
                    .as_ref()
                    .and_then(|(_, derivative)| derivative.as_ref())
                    .map_or(&cross, |derivative| &derivative[0]);
                if equatorial
                    && (direction.lower() > &BigInt::zero()
                        || direction.upper() < &BigInt::zero()
                        || direction.is_exact_zero())
                {
                    let westward = direction.upper() < &BigInt::zero();
                    potential.gauge = if westward ^ fragment.reversed {
                        PoleGauge::South
                    } else {
                        PoleGauge::North
                    };
                    // A derivative sign over the WHOLE panel proves that its
                    // continuous lift is monotone (or constant). Endpoint
                    // enclosures therefore bound every longitude. Evaluating
                    // sigma-f*sigma as independent wide intervals would keep
                    // an artificial finite cut band even at infinite precision.
                    // Unproved directions and folded images never take this path.
                    let first_parameter = FixedInterval::from_bounds(
                        parameter.lower().clone(),
                        parameter.lower().clone(),
                        math,
                    )?;
                    let last_parameter = FixedInterval::from_bounds(
                        parameter.upper().clone(),
                        parameter.upper().clone(),
                        math,
                    )?;
                    let (first_image, _) =
                        line.image(&first_parameter, iterations, math, progress)?;
                    let (last_image, _) =
                        line.image(&last_parameter, iterations, math, progress)?;
                    let outer = first_image[0].hull(&last_image[0], math)?;
                    let inner_parameter = if let Some(guaranteed) = &guaranteed {
                        guaranteed.intersection(&parameter, math)?
                    } else {
                        None
                    };
                    let inner = if let Some(parameter) = inner_parameter {
                        let first = FixedInterval::from_bounds(
                            parameter.lower().clone(),
                            parameter.lower().clone(),
                            math,
                        )?;
                        let last = FixedInterval::from_bounds(
                            parameter.upper().clone(),
                            parameter.upper().clone(),
                            math,
                        )?;
                        let first = if first.lower() == first_parameter.lower() {
                            first_image
                        } else {
                            line.image(&first, iterations, math, progress)?.0
                        };
                        let last = if last.upper() == last_parameter.upper() {
                            last_image
                        } else {
                            line.image(&last, iterations, math, progress)?.0
                        };
                        let (low, high) = if westward {
                            (&last[0], &first[0])
                        } else {
                            (&first[0], &last[0])
                        };
                        if low.upper() <= high.lower() {
                            Some(FixedInterval::from_bounds(
                                low.upper().clone(),
                                high.lower().clone(),
                                math,
                            )?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    add_band(bands, &outer, inner.as_ref(), math, progress)?;
                    accepted = true;
                } else {
                    let half = one.div(&FixedInterval::from_i64(2, math)?, math)?;
                    if normal[2].abs(math)?.upper() < half.lower() {
                        let speed = cross
                            .abs(math)?
                            .div(&one.sub(&normal[2].square(math)?, math)?, math)?;
                        flux_error = potential
                            .pole
                            .mul(&FixedInterval::from_i64(2, math)?, math)?
                            .mul(&speed, math)?
                            .mul(&parameter.width(math)?, math)?;
                        let (image, _) = line.image(&parameter, iterations, math, progress)?;
                        let cut_error = potential
                            .pole
                            .mul(&FixedInterval::from_i64(2, math)?, math)?
                            .mul(&image[0].width(math)?, math)?;
                        let complete = flux_error.add(&cut_error, math)?;
                        if complete.upper() <= band_budget.lower() {
                            add_band(bands, &image[0], None, math, progress)?;
                            unknown = unknown.add(&complete, math)?;
                            accepted = true;
                        }
                    }
                }
            }
            if !accepted {
                if depth >= depth_limit {
                    return Err(MathError::PrecisionExhausted);
                }
                let [left, right] = crate::atlas::oriented::split_parameter(&parameter, math)?;
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
                continue;
            }
            if let Some(parameter) = parameter.intersection(&core, math)? {
                let integral = integrate_range(
                    line,
                    potential,
                    parameter,
                    limits,
                    core_budget,
                    math,
                    progress,
                )?;
                total = total.add(&symmetric_error(&integral, &flux_error, math)?, math)?;
            }
        }
        // The finite collection, rather than an assumed root count, bounds the
        // complete simultaneous uncertain strips. Too many bands are refined
        // by the outer accuracy loop; no successful list is truncated.
        if unknown.upper() > core_budget.lower() {
            return Err(MathError::PrecisionExhausted);
        }
        for (point, error) in [&start, &end].into_iter().zip([&first_error, &last_error]) {
            if error.is_exact_zero() {
                continue;
            }
            let tail = point
                .sub(error, math)?
                .hull(&point.add(error, math)?, math)?;
            if let Some(tail) = tail.intersection(&unit, math)? {
                let bound = endpoint_tail(line, potential, &tail, iterations, math, progress)?;
                total = symmetric_error(&total, &bound, math)?;
            }
        }
        if fragment.reversed {
            total.neg(math)
        } else {
            Ok(total)
        }
    })();
    math.release_workspace(scratch)?;
    result
}

fn boundary_integral(
    fragments: &[Fragment<'_>],
    bits: u32,
    budget: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<BoundaryIntegral, GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    let bytes = fragments.len().saturating_mul(size_of::<&PreparedEdge>()) as u64;
    context.admit_workspace(bytes)?;
    let edges = fragments
        .iter()
        .map(|fragment| fragment.edge)
        .collect::<Vec<_>>();
    let result = arcs::with_line_refs_retained(
        &edges,
        bits,
        true,
        context,
        progress,
        |lines, math, progress| {
            let result = (|| {
                let mut potential = AreaPotential::prepare(reference.ellipsoid(), math, progress)?;
                let divisor = FixedInterval::from_ratio(
                    &BigInt::from_i128(fragments.len().max(1) as i128),
                    &BigInt::from_i128(1),
                    math,
                )?;
                let budget = fixed_from_rat(budget, math)?.div(&divisor, math)?;
                let core_budget = budget.div(&FixedInterval::from_i64(8, math)?, math)?;
                let band_budget = core_budget.div(
                    &FixedInterval::from_i64(
                        i64::from(policy.limits().max_subdivision_levels) * 8,
                        math,
                    )?,
                    math,
                )?;
                let mut value = FixedInterval::from_i64(0, math)?;
                let mut bands = Vec::new();
                for (fragment, line) in fragments.iter().zip(lines) {
                    let contribution = integrate_fragment(
                        fragment,
                        line,
                        &mut potential,
                        &core_budget,
                        &band_budget,
                        (
                            policy.limits().max_iterations,
                            policy.limits().max_subdivision_levels,
                        ),
                        &mut bands,
                        math,
                        progress,
                    )?;
                    value = value.add(&contribution, math)?;
                }
                let surface = potential.surface_in(math)?;
                let copies = size_of::<BoundaryIntegral>()
                    .saturating_add(value.detached_heap_bytes())
                    .saturating_add(potential.pole.detached_heap_bytes())
                    .saturating_add(surface.detached_heap_bytes());
                math.reserve_workspace(copies)?;
                progress.math_poll(math)?;
                let mut result = BoundaryIntegral {
                    value: value.detached(),
                    pole: potential.pole.detached(),
                    surface: surface.detached(),
                    bands,
                    storage: 0,
                };
                result.storage = result.owned_bytes();
                let storage = result.storage;
                Ok((result, storage))
            })();
            result.map_err(|error| geo_math_error(&error, policy))
        },
    );
    drop(edges);
    context.release_workspace(bytes)?;
    let (result, _) = result?;
    Ok(result)
}

struct CutInterval {
    lower: Rat,
    upper: Rat,
    boundary: bool,
}

fn cut_intervals(
    integral: &BoundaryIntegral,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Vec<CutInterval>, u64), GeoError> {
    let policy = context.policy();
    let mut intervals = Vec::new();
    let bits = integral.value.precision_bits();
    // Internal walls enclose the cut; they are not a reported coordinate grid.
    // Increasing arithmetic precision must also reduce their uncertainty.
    let places = (u64::from(bits) * 3 / 10).max(24) as u32;
    let mut retained = 0;
    let result = (|| {
        for band in &integral.bands {
            for (longitude, boundary) in
                [(Some(&band.outer), false), (band.boundary.as_ref(), true)]
            {
                let Some(longitude) = longitude else {
                    continue;
                };
                let ((lower, upper), copied) = crate::numerical::with_math_for_sources_retained(
                    context,
                    progress,
                    bits,
                    4096,
                    &[],
                    |math, progress| {
                        let result = (|| {
                            let conversion = FixedInterval::from_i64(180, math)?
                                .div(&FixedInterval::pi(math)?, math)?;
                            let (lower, upper) = if boundary {
                                // An actual Boundary interval must contract at both ends.
                                // Outward conversion here would label a thin unknown strip
                                // outside the original equatorial image as Boundary.
                                let first = FixedInterval::from_bounds(
                                    longitude.lower().clone(),
                                    longitude.lower().clone(),
                                    math,
                                )?
                                .mul(&conversion, math)?;
                                let last = FixedInterval::from_bounds(
                                    longitude.upper().clone(),
                                    longitude.upper().clone(),
                                    math,
                                )?
                                .mul(&conversion, math)?;
                                let lower = first
                                    .round_decimal_with(
                                        places,
                                        purrdf_xsd::ieee::ratio::Rounding::Up,
                                        math,
                                    )?
                                    .1;
                                let upper = last
                                    .round_decimal_with(
                                        places,
                                        purrdf_xsd::ieee::ratio::Rounding::Down,
                                        math,
                                    )?
                                    .0;
                                (lower, upper)
                            } else {
                                let degrees = longitude.mul(&conversion, math)?;
                                let lower = degrees
                                    .round_decimal_with(
                                        places,
                                        purrdf_xsd::ieee::ratio::Rounding::Down,
                                        math,
                                    )?
                                    .0;
                                let upper = degrees
                                    .round_decimal_with(
                                        places,
                                        purrdf_xsd::ieee::ratio::Rounding::Up,
                                        math,
                                    )?
                                    .1;
                                (lower, upper)
                            };
                            let ([lower, upper], bytes) = crate::numerical::math_share_rationals(
                                [
                                    crate::numerical::math_quantized_decimal(
                                        &lower, places, math, progress,
                                    )?,
                                    crate::numerical::math_quantized_decimal(
                                        &upper, places, math, progress,
                                    )?,
                                ],
                                math,
                                progress,
                            )?;
                            Ok(([lower, upper].into(), bytes))
                        })();
                        result.map_err(|error| geo_math_error(&error, policy))
                    },
                )?;
                let wrapped = (|| {
                    if ExactAdmission::new(context, progress).rational(
                        ExactOperation::RationalCompare,
                        &[&lower, &upper],
                        1,
                        || Ok(lower > upper),
                    )? {
                        return Ok(());
                    }
                    let cost = crate::numerical::rational_cost(
                        ExactOperation::Linear,
                        &[&lower, &upper],
                        16,
                    )
                    .ok_or(GeoError::ArithmeticOverflow("cut interval owner storage"))?;
                    context.admit_workspace(cost.workspace_bytes)?;
                    let wrapped = (|| {
                        progress.context_poll(context)?;
                        let aliases =
                            crate::atlas::longitude_intervals(&lower, &upper, context, progress)?;
                        for (lower, upper) in aliases {
                            if intervals.len() as u64 >= policy.limits().max_output_elements {
                                return Err(GeoError::OutputExhausted {
                                    limit: policy.limits().max_output_elements,
                                });
                            }
                            // Geometric Vec growth uses at most two inline slots
                            // per retained interval; immutable limbs/headers are
                            // measured before sharing, even at raised precision.
                            let bytes = (2 * size_of::<CutInterval>()) as u64
                                + lower.shared_owned_bytes() as u64
                                + upper.shared_owned_bytes() as u64;
                            context.retain_workspace(bytes, &mut retained)?;
                            progress.context_poll(context)?;
                            intervals.push(CutInterval {
                                lower: lower.into_shared(),
                                upper: upper.into_shared(),
                                boundary,
                            });
                        }
                        Ok(())
                    })();
                    context.release_workspace(cost.workspace_bytes)?;
                    wrapped
                })();
                drop((lower, upper));
                context.release_workspace(copied)?;
                wrapped?;
            }
        }
        Ok((intervals, retained))
    })();
    if result.is_err() {
        context.release_workspace(retained)?;
    }
    result
}

fn equator_measure(
    intervals: &[CutInterval],
    small_width: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    locate: &mut impl FnMut(&LonLat, &mut MetricContext, &mut WorkProgress<'_>) -> Result<Set, GeoError>,
) -> Result<((Rat, Rat), u64), GeoError> {
    context.charge_work(intervals.len() as u64)?;
    progress.context_poll(context)?;
    let mut limbs = 0u64;
    let mut maximum_bits = 0u64;
    for interval in intervals {
        for value in [&interval.lower, &interval.upper] {
            limbs = limbs
                .checked_add(value.allocated_bytes() as u64)
                .ok_or(GeoError::ArithmeticOverflow("equator node limb storage"))?;
            maximum_bits = maximum_bits.max(crate::numerical::rational_operand_bits(value));
        }
    }
    // Every cut coordinate has a denominator dividing the same decimal grid.
    // Midpoints add one binary factor, and total disjoint measure is <=360°.
    // Reduced measure numerators therefore need at most ten additional bits;
    // ordinary cross-product scratch is separately admitted at each operation.
    let bytes = (intervals.len() as u64)
        .checked_mul(2)
        .and_then(|count| count.checked_add(2))
        .and_then(|count| count.checked_mul(size_of::<Rat>() as u64))
        .and_then(|bytes| bytes.checked_add(limbs))
        .and_then(|bytes| {
            bytes.checked_add(
                maximum_bits
                    .saturating_add(10)
                    .div_ceil(8)
                    .saturating_mul(8),
            )
        })
        .and_then(|bytes| bytes.checked_add(8192))
        .ok_or(GeoError::ArithmeticOverflow("equator node storage"))?;
    context.admit_workspace(bytes)?;
    let result = (|| {
        progress.context_poll(context)?;
        let mut nodes = Vec::with_capacity(intervals.len().saturating_mul(2).saturating_add(2));
        nodes.push(Rat::from_i64(-180));
        nodes.push(Rat::from_i64(180));
        for interval in intervals {
            nodes.push(crate::numerical::copy_rat(
                &interval.lower,
                context,
                progress,
            )?);
            nodes.push(crate::numerical::copy_rat(
                &interval.upper,
                context,
                progress,
            )?);
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut nodes, |left, right| {
            ExactAdmission::new(context, progress).rational(
                ExactOperation::RationalCompare,
                &[left, right],
                1,
                || Ok(left.cmp(right)),
            )
        })?;
        let mut lower = Rat::zero();
        let mut upper = Rat::zero();
        for pair in nodes.windows(2) {
            let mut admission = ExactAdmission::new(context, progress);
            if admission.rational(
                ExactOperation::RationalCompare,
                &[&pair[0], &pair[1]],
                1,
                || Ok(pair[0] == pair[1]),
            )? {
                continue;
            }
            let width = admission.rational(
                ExactOperation::RationalAdd,
                &[&pair[0], &pair[1]],
                1,
                || Ok(pair[1].sub(&pair[0])),
            )?;
            let midpoint = admission.rational(
                ExactOperation::RationalAdd,
                &[&pair[0], &pair[1]],
                2,
                || {
                    Ok(pair[0]
                        .add(&pair[1])
                        .div(&Rat::from_i64(2))
                        .expect("positive divisor"))
                },
            )?;
            let mut unknown = false;
            let mut boundary = false;
            for interval in intervals {
                if admission.rational(
                    ExactOperation::RationalCompare,
                    &[&midpoint, &interval.lower, &interval.upper],
                    2,
                    || Ok(midpoint > interval.lower && midpoint < interval.upper),
                )? {
                    boundary |= interval.boundary;
                    unknown |= !interval.boundary;
                }
            }
            // A small boundary-free component can be enclosed as unknown
            // instead of spending a topology proof on a point arbitrarily
            // close to a cut. Its complete positive measure is included below;
            // the outer true-area rounding loop decides whether it is adequate.
            if !boundary && !unknown {
                unknown = admission.rational(
                    ExactOperation::RationalCompare,
                    &[&width, small_width],
                    1,
                    || Ok(width <= *small_width),
                )?;
            }
            let class = if boundary || unknown {
                Set::Boundary
            } else {
                let point = admission.rational(ExactOperation::Linear, &[&midpoint], 1, || {
                    LonLat::new(midpoint.clone(), Rat::zero())
                })?;
                locate(&point, context, progress)?
            };
            if class == Set::Interior {
                let mut admission = ExactAdmission::new(context, progress);
                lower = admission.rational(
                    ExactOperation::RationalAdd,
                    &[&lower, &width],
                    1,
                    || Ok(lower.add(&width)),
                )?;
                upper = admission.rational(
                    ExactOperation::RationalAdd,
                    &[&upper, &width],
                    1,
                    || Ok(upper.add(&width)),
                )?;
            } else if unknown && !boundary {
                upper = ExactAdmission::new(context, progress).rational(
                    ExactOperation::RationalAdd,
                    &[&upper, &width],
                    1,
                    || Ok(upper.add(&width)),
                )?;
            } else if class == Set::Boundary && !boundary {
                // A boundary point outside every complete contact image contradicts
                // this partition's proof; refine rather than assume an interior.
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            }
        }
        let retained = (2 * size_of::<Rat>()) as u64
            + lower.allocated_bytes() as u64
            + upper.allocated_bytes() as u64;
        if retained > bytes {
            return Err(GeoError::ArithmeticOverflow(
                "equator measure result storage",
            ));
        }
        Ok(((lower, upper), retained))
    })();
    let retained = result.as_ref().map_or(0, |(_, bytes)| *bytes);
    context.release_workspace(bytes - retained)?;
    result
}

/// The completed general law is true-area half-even rounding, not the midpoint
/// of a heuristic quadrature. Truncation, endpoint tails, tangencies, continuous
/// longitude images and the whole cut measure refine together until decisive.
pub(super) fn area(
    fragments: &[Fragment<'_>],
    initial_budget: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    mut locate: impl FnMut(&LonLat, &mut MetricContext, &mut WorkProgress<'_>) -> Result<Set, GeoError>,
) -> Result<RingArea, GeoError> {
    let policy = context.policy();
    let mut budget = crate::numerical::copy_rat(initial_budget, context, progress)?;
    let mut bits = 96;
    loop {
        // Partition, quadrature, both gauge tails and the whole cut measure
        // belong to this accuracy attempt. A refusal at any of those sites
        // must reach the same refinement decision, rather than bypass it.
        let result = (|| {
            let integral = boundary_integral(fragments, bits, &budget, context, progress)?;
            let result = (|| {
                let (intervals, interval_storage) = cut_intervals(&integral, context, progress)?;
                let small_width = crate::numerical::with_math_for_sources_retained(
                    context,
                    progress,
                    bits,
                    4096,
                    &[&budget],
                    |math, progress| {
                        let result = (|| {
                            let divisor = integral
                                .pole
                                .mul(&FixedInterval::from_i64(2, math)?, math)?
                                .mul(&FixedInterval::pi(math)?, math)?
                                .div(&FixedInterval::from_i64(180, math)?, math)?
                                .mul(
                                    &FixedInterval::from_ratio(
                                        &BigInt::from_i128(
                                            (intervals.len().saturating_mul(2).saturating_add(2))
                                                as i128,
                                        ),
                                        &BigInt::from_i128(1),
                                        math,
                                    )?,
                                    math,
                                )?
                                .mul(&FixedInterval::from_i64(8, math)?, math)?;
                            let value = fixed_from_rat(&budget, math)?.div(&divisor, math)?;
                            let ([lower], bytes) = crate::numerical::math_share_rationals(
                                [exact_bounds(&value).0],
                                math,
                                progress,
                            )?;
                            Ok((lower, bytes))
                        })();
                        result.map_err(|error| geo_math_error(&error, policy))
                    },
                );
                let measured = match small_width {
                    Ok((width, bytes)) => {
                        let result =
                            equator_measure(&intervals, &width, context, progress, &mut locate);
                        drop(width);
                        context.release_workspace(bytes)?;
                        result
                    }
                    Err(error) => Err(error),
                };
                drop(intervals);
                context.release_workspace(interval_storage)?;
                let (measured, measure_storage) = measured?;
                let result = crate::numerical::with_math_for_sources_retained(
                    context,
                    progress,
                    bits,
                    8192,
                    &[&measured.0, &measured.1],
                    |math, progress| {
                        let result = (|| {
                            let measure = fixed_from_bounds(&measured.0, &measured.1, math)?
                                .mul(&FixedInterval::pi(math)?, math)?
                                .div(&FixedInterval::from_i64(180, math)?, math)?;
                            let left = integral.value.add(
                                &integral
                                    .pole
                                    .mul(&FixedInterval::from_i64(2, math)?, math)?
                                    .mul(&measure, math)?,
                                math,
                            )?;
                            let opposite = integral.surface.sub(&left, math)?;
                            let (a, b) = left.round_decimal(2, math)?;
                            let (c, d) = opposite.round_decimal(2, math)?;
                            if a != b || c != d {
                                return Err(MathError::PrecisionExhausted);
                            }
                            let copies =
                                left.detached_heap_bytes() + integral.surface.detached_heap_bytes();
                            math.reserve_workspace(copies)?;
                            progress.math_poll(math)?;
                            Ok((
                                (
                                    RingArea {
                                        left: left.detached(),
                                        surface: integral.surface.detached(),
                                        storage: copies as u64,
                                    },
                                    [a, c],
                                ),
                                copies as u64,
                            ))
                        })();
                        result.map_err(|error| geo_math_error(&error, policy))
                    },
                );
                drop(measured);
                context.release_workspace(measure_storage)?;
                let ((value, expected), output_storage) = result?;
                if bits == 96 {
                    return Ok(value);
                }
                let narrowed = crate::numerical::with_math_for_sources_retained(
                    context,
                    progress,
                    96,
                    8192,
                    &[],
                    |math, progress| {
                        let result = (|| {
                            let (a, b) = exact_bounds(&value.left);
                            let (c, d) = exact_bounds(&value.surface);
                            let left = fixed_from_bounds(&a, &b, math)?;
                            let surface = fixed_from_bounds(&c, &d, math)?;
                            let (a, b) = left.round_decimal(2, math)?;
                            let (c, d) = surface.sub(&left, math)?.round_decimal(2, math)?;
                            if a != b || c != d || a != expected[0] || c != expected[1] {
                                return Err(MathError::PrecisionExhausted);
                            }
                            let copies = left.detached_heap_bytes() + surface.detached_heap_bytes();
                            math.reserve_workspace(copies)?;
                            progress.math_poll(math)?;
                            Ok((
                                RingArea {
                                    left: left.detached(),
                                    surface: surface.detached(),
                                    storage: copies as u64,
                                },
                                copies as u64,
                            ))
                        })();
                        result.map_err(|error| geo_math_error(&error, policy))
                    },
                );
                drop(value);
                context.release_workspace(output_storage)?;
                narrowed.map(|(value, _)| value)
            })();
            let retained = integral.storage;
            drop(integral);
            context.release_workspace(retained)?;
            result
        })();
        match result {
            Ok(value) => return Ok(value),
            Err(GeoError::PrecisionExhausted { .. }) => {
                if bits >= policy.limits().max_precision_bits {
                    return Err(GeoError::PrecisionExhausted { bits });
                }
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
                budget = ExactAdmission::new(context, progress).rational(
                    ExactOperation::RationalDivide,
                    &[&budget],
                    1,
                    || Ok(budget.div(&Rat::from_i64(16)).expect("positive divisor")),
                )?;
            }
            Err(error) => return Err(error),
        }
    }
}
