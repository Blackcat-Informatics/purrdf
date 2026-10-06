// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original physical area in the smooth normal atlas. With z=sin(latitude),
//! J(z)=a²(1-e²)/(1-e²z²)². The north-regular potential is
//! A_N=(Q(1)-Q(z))/(1-z²)*(nx dny-ny dnx). Its exterior derivative is the
//! ellipsoid's physical surface measure. R_x(pi) supplies its south-regular
//! partner A_S=-(Q(1)+Q(z))dλ. Certified hemisphere panels use their regular
//! gauge; possible equator contacts retain both gauge fluxes and complete
//! continuous longitude images. The cut contributes 2Q(1) times the eastward
//! Interior measure. No smaller-side convention or guessed tangent is used.
//!
//! Integrating J's geometric series derives every coefficient here. For r=e²,
//! (Q(1)-Q(z))/(1-z²)=a²(1-r)/(1+z) * sum_k
//! [(k+1)r^k/(2k+1) * (1+z+...+z^(2k))]. Since |z|≤1, after N terms its
//! numerator tail is at most a²(1-r)r^N*((N+1)-Nr)/(1-r)². General native
//! completion refines this tail, quadrature and cut uncertainty together until
//! true-area half-even rounding is decisive for Left and its complement.

mod hemisphere;

use super::arcs::{self, ChartLine};
use crate::context::WorkProgress;
use crate::geodesic::normal::{FixedArithmetic, NormalArithmetic, TaylorArithmetic};
use crate::numerical::fixed_from_rat;
use crate::{GeoError, MetricContext, PreparedCurve, Rat};
use purrdf_xsd::{
    BigInt,
    math::{
        CoordinateMath, FixedInterval, MathError, SymmetricTaylorPanel,
        integrate_taylor_panel_observed,
    },
};

struct AreaPotential {
    polynomial: Vec<FixedInterval>,
    tail: FixedInterval,
    pole: FixedInterval,
    gauge: PoleGauge,
}

/// R_x(pi) is an orientation-preserving isometry of the physical ellipsoid.
/// Applying it to the north potential produces the south-regular potential
/// without deriving a second density or losing the original edge law.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PoleGauge {
    North,
    South,
}

impl PoleGauge {
    fn latitude<A: NormalArithmetic>(
        self,
        value: &A::Value,
        arithmetic: &mut A,
    ) -> Result<A::Value, MathError> {
        if self == Self::South {
            arithmetic.neg(value)
        } else {
            Ok(value.clone())
        }
    }
}
impl AreaPotential {
    fn surface_in(&self, math: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
        self.pole
            .mul(&FixedInterval::pi(math)?, math)?
            .mul(&FixedInterval::from_i64(4, math)?, math)
    }
    fn prepare(
        ellipsoid: &crate::PreparedEllipsoid,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, MathError> {
        let one = FixedInterval::from_i64(1, math)?;
        let r = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
        let complement = one.sub(&r, math)?;
        let scale = fixed_from_rat(ellipsoid.semimajor(), math)?
            .square(math)?
            .mul(&complement, math)?;
        let target = FixedInterval::from_ratio(
            &BigInt::from_i128(1),
            &BigInt::from_i128(1).mul_pow2(math.limits().precision_bits / 3 + 8),
            math,
        )?;
        let mut terms = Vec::new();
        let mut power = one.clone();
        let mut pole = FixedInterval::from_i64(0, math)?;
        let mut retained = 0usize;
        let result = (|| {
            let mut n = 0i64;
            let tail = loop {
                progress.math_poll(math)?;
                let term = scale
                    .mul(&power, math)?
                    .mul(&FixedInterval::from_i64(n + 1, math)?, math)?
                    .div(&FixedInterval::from_i64(2 * n + 1, math)?, math)?;
                let bytes = term.workspace_bytes().saturating_mul(8).saturating_add(128);
                math.reserve_workspace(bytes)?;
                retained = retained
                    .checked_add(bytes)
                    .ok_or(MathError::WorkspaceExhausted)?;
                pole = pole.add(&term, math)?;
                terms.push(term);
                n = n.checked_add(1).ok_or(MathError::WorkExhausted)?;
                power = power.mul(&r, math)?;
                let tail = scale
                    .mul(&power, math)?
                    .mul(
                        &FixedInterval::from_i64(n + 1, math)?
                            .sub(&r.mul(&FixedInterval::from_i64(n, math)?, math)?, math)?,
                        math,
                    )?
                    .div(&complement.square(math)?, math)?;
                if tail.upper() <= target.lower() {
                    break tail;
                }
                // Once a positive power is enclosed by one fixed-grid unit,
                // further multiplication cannot reduce its outward upper
                // endpoint. An absolute density target below this arithmetic
                // floor requires higher precision, not an unbounded series.
                if power.lower().is_zero() && power.upper() <= &BigInt::from_i128(1) {
                    return Err(MathError::PrecisionExhausted);
                }
            };
            let mut accumulated = FixedInterval::from_i64(0, math)?;
            let mut polynomial = Vec::with_capacity(terms.len().saturating_mul(2));
            for (k, term) in terms.iter().enumerate().rev() {
                accumulated = accumulated.add(term, math)?;
                polynomial.push(accumulated.clone());
                if k != 0 {
                    polynomial.push(accumulated.clone());
                }
            }
            polynomial.reverse();
            let pole = FixedInterval::from_bounds(
                pole.lower().clone(),
                pole.upper().add(tail.upper()),
                math,
            )?;
            Ok(Self {
                polynomial,
                tail,
                pole,
                gauge: PoleGauge::North,
            })
        })();
        if result.is_err() {
            math.release_workspace(retained)?;
        }
        result
    }
}

pub(crate) fn surface_area_in(
    ellipsoid: &crate::PreparedEllipsoid,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    AreaPotential::prepare(ellipsoid, math, progress)?.surface_in(math)
}

fn potential_polynomial<A: NormalArithmetic>(
    latitude: &A::Value,
    coefficients: &[A::Value],
    arithmetic: &mut A,
) -> Result<A::Value, MathError> {
    let mut polynomial = coefficients
        .last()
        .expect("nonempty original series")
        .clone();
    for coefficient in coefficients[..coefficients.len() - 1].iter().rev() {
        let product = arithmetic.mul(&polynomial, latitude)?;
        polynomial = arithmetic.add(&product, coefficient)?;
    }
    Ok(polynomial)
}

fn density<A: NormalArithmetic>(
    normal: &[A::Value; 3],
    derivative: &[A::Value; 3],
    coefficients: &[A::Value],
    gauge: PoleGauge,
    arithmetic: &mut A,
) -> Result<A::Value, MathError> {
    let latitude = gauge.latitude(&normal[2], arithmetic)?;
    let polynomial = potential_polynomial(&latitude, coefficients, arithmetic)?;
    let one = arithmetic.one()?;
    let denominator = arithmetic.add(&one, &latitude)?;
    let ratio = arithmetic.div(&polynomial, &denominator)?;
    let first = arithmetic.mul(&normal[0], &derivative[1])?;
    let second = arithmetic.mul(&normal[1], &derivative[0])?;
    let cross = arithmetic.sub(&first, &second)?;
    let cross = gauge.latitude(&cross, arithmetic)?;
    arithmetic.mul(&ratio, &cross)
}

/// For an original parallel, nx*dny-ny*dnx=(1-z²)*dλ identically.
/// Cancelling before interval evaluation gives (1-z)*P(z)*dλ, including
/// the regular pole. This retains the same generated potential and tail;
/// no independent area formula or sampled trigonometric correlation is used.
fn parallel_integral(
    line: &ChartLine,
    potential: &AreaPotential,
    parameter: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<(FixedInterval, FixedInterval)>, MathError> {
    if !matches!(line, ChartLine::Linear(_)) {
        return Ok(None);
    }
    let (image, derivative) = line.image(parameter, iterations, math, progress)?;
    let derivative = derivative.ok_or(MathError::PrecisionExhausted)?;
    if derivative[0].is_exact_zero() {
        let zero = FixedInterval::from_i64(0, math)?;
        return Ok(Some((zero.clone(), zero)));
    }
    if !derivative[1].is_exact_zero() {
        return Ok(None);
    }
    let (latitude, _) = image[1].sin_cos(math)?;
    let latitude = potential
        .gauge
        .latitude(&latitude, &mut FixedArithmetic(math))?;
    let polynomial =
        potential_polynomial(&latitude, &potential.polynomial, &mut FixedArithmetic(math))?;
    let complement = FixedInterval::from_i64(1, math)?.sub(&latitude, math)?;
    let longitude = potential
        .gauge
        .latitude(&derivative[0], &mut FixedArithmetic(math))?;
    let measure = complement
        .mul(&longitude, math)?
        .mul(&parameter.width(math)?, math)?;
    let integral = polynomial.mul(&measure, math)?;
    let remainder = potential.tail.mul(&measure.abs(math)?, math)?;
    Ok(Some((integral, remainder)))
}

fn panel_integral(
    line: &ChartLine,
    potential: &AreaPotential,
    parameter: &FixedInterval,
    allowed: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(FixedInterval, FixedInterval), MathError> {
    if let Some(value) = parallel_integral(line, potential, parameter, iterations, math, progress)?
    {
        return Ok(value);
    }
    let (normal, derivative) = if let ChartLine::Geodesic(line) = line {
        if line.is_meridian() {
            // Decide from the exact conserved source law before a coordinate
            // box that straddles a pole loses its trigonometric correlation.
            let zero = FixedInterval::from_i64(0, math)?;
            return Ok((zero.clone(), zero));
        }
        let image = line.continuation_enclosure_in(parameter, iterations, math, progress)?;
        if image.sin_alpha0.is_exact_zero() {
            // A selected meridian has zero ordinary north-potential integral.
            // Its south-pole transitions belong to the explicit atlas sectors.
            let zero = FixedInterval::from_i64(0, math)?;
            return Ok((zero.clone(), zero));
        }
        if line.is_equatorial() {
            // The complete selected equator has Q=0 and constant longitude
            // speed. Its north-potential integral is Q(1)*delta-longitude;
            // complete revolutions retain the original length and direction.
            let derivative = image.derivative.ok_or(MathError::PrecisionExhausted)?;
            let integral = potential
                .pole
                .mul(&derivative[0], math)?
                .mul(&parameter.width(math)?, math)?;
            let integral = if potential.gauge == PoleGauge::South {
                integral.neg(math)?
            } else {
                integral
            };
            return Ok((integral, FixedInterval::from_i64(0, math)?));
        }
        (image.normal, Some(image.normal_derivative))
    } else {
        line.normal(parameter, iterations, math, progress)?
    };
    let derivative = derivative.ok_or(MathError::PrecisionExhausted)?;
    let one = FixedInterval::from_i64(1, math)?;
    let cross = normal[0]
        .mul(&derivative[1], math)?
        .sub(&normal[1].mul(&derivative[0], math)?, math)?;
    let series_error = potential
        .tail
        .div(
            &one.add(
                &potential
                    .gauge
                    .latitude(&normal[2], &mut FixedArithmetic(math))?,
                math,
            )?,
            math,
        )?
        .mul(&cross.abs(math)?, math)?
        .mul(&parameter.width(math)?, math)?;
    if let ChartLine::Transformed { curve, .. } = line {
        // The first-order panel enclosure is cheap and decides coarse or
        // nearly uniform panels; the Taylor model refines the others.
        let integral = density(
            &normal,
            &derivative,
            &potential.polynomial,
            potential.gauge,
            &mut FixedArithmetic(math),
        )?
        .mul(&parameter.width(math)?, math)?;
        let error = integral
            .width(math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?
            .add(&series_error, math)?;
        if error.upper() > allowed.lower()
            && let Some(value) =
                transformed_taylor_integral(curve, potential, parameter, math, progress)?
        {
            return Ok((value.0, value.1.add(&series_error, math)?));
        }
        return Ok((integral.midpoint(math)?, error));
    }
    let middle = parameter.midpoint(math)?;
    let half = parameter
        .width(math)?
        .div(&FixedInterval::from_i64(2, math)?, math)?;
    let panel = SymmetricTaylorPanel::new(&middle, parameter, &half, math)?;
    let (integral, remainder) = integrate_taylor_panel_observed(
        8,
        512,
        panel,
        math,
        progress,
        |math, progress| progress.math_poll(math),
        |t, workspace, math, progress| {
            let (normal, derivative) = line
                .normal_taylor_in(t, iterations, workspace, math, progress)?
                .ok_or(MathError::PrecisionExhausted)?;
            let coefficients = potential
                .polynomial
                .iter()
                .map(|value| workspace.constant(value.clone(), math))
                .collect::<Result<Vec<_>, _>>()?;
            density(
                &normal,
                &derivative,
                &coefficients,
                potential.gauge,
                &mut TaylorArithmetic { workspace, math },
            )
        },
    )?;
    Ok((integral, remainder.add(&series_error, math)?))
}

/// The panel integral of a transformed image with a Taylor model of its
/// normal, in the panel's normalized variable `σ` in `[-1,1]`. `None` when
/// the image has no such model or the model cannot certify this panel; the
/// caller's first-order panel enclosure then decides it.
fn transformed_taylor_integral(
    curve: &crate::operation::OperationImageCurve,
    potential: &AreaPotential,
    parameter: &FixedInterval,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<(FixedInterval, FixedInterval)>, MathError> {
    use crate::ellipsoidal::jet::{MercatorPolynomial, ORDER};
    let zero = Rat::zero();
    let one = Rat::one();
    let Some(model) = MercatorPolynomial::new(curve, [&zero, &one], math, progress)? else {
        return Ok(None);
    };
    let centre = parameter.midpoint(math)?;
    let half = parameter
        .width(math)?
        .div(&FixedInterval::from_i64(2, math)?, math)?;
    let constants = model.constants_fixed(&centre, &half, math)?;
    let origin = FixedInterval::from_i64(0, math)?;
    let unit_half = FixedInterval::from_i64(1, math)?;
    let unit = FixedInterval::from_bounds(
        unit_half.neg(math)?.lower().clone(),
        unit_half.upper().clone(),
        math,
    )?;
    let panel = SymmetricTaylorPanel::new(&origin, &unit, &unit_half, math)?;
    let result = integrate_taylor_panel_observed(
        ORDER,
        model.live_jets() + potential.polynomial.len(),
        panel,
        math,
        progress,
        |math, progress| progress.math_poll(math),
        |sigma, workspace, math, progress| {
            progress.math_poll(math)?;
            let (normal, derivative) = model.normal(sigma, &constants, workspace, math)?;
            let mut coefficients = purrdf_core::SmallVec::<[_; 16]>::new();
            for value in &potential.polynomial {
                coefficients.push(workspace.constant(value.clone(), math)?);
            }
            density(
                &normal,
                &derivative,
                &coefficients,
                potential.gauge,
                &mut TaylorArithmetic { workspace, math },
            )
        },
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(MathError::PrecisionExhausted) => Ok(None),
        Err(error) => Err(error),
    }
}

fn integrate_range(
    line: &ChartLine,
    potential: &AreaPotential,
    parameter: FixedInterval,
    (iterations, depth_limit): (u32, u32),
    budget: &FixedInterval,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let bytes = (depth_limit as usize + 2).saturating_mul(4096);
    math.reserve_workspace(bytes)?;
    let result = (|| {
        let mut total = FixedInterval::from_i64(0, math)?;
        let mut pending =
            purrdf_lex::walk::WorkList::<(FixedInterval, u32), 8>::with((parameter, 0));
        while let Some((parameter, depth)) = pending.pop() {
            progress.math_poll(math)?;
            let allowed = budget.mul(&parameter.width(math)?, math)?;
            let value = panel_integral(
                line, potential, &parameter, &allowed, iterations, math, progress,
            );
            let accepted = match &value {
                Ok((_, error)) => error.upper() <= allowed.lower(),
                Err(MathError::PrecisionExhausted) => false,
                Err(error) => return Err(error.clone()),
            };
            if accepted {
                // Keep the complete original integral's Taylor remainder
                // in the enclosure. Native area completion must prove one
                // true rounding bin, so this error refines with the budget.
                let (integral, error) = value?;
                let integral = integral.add(&error.neg(math)?.hull(&error, math)?, math)?;
                total = total.add(&integral, math)?;
            } else {
                if depth >= depth_limit {
                    return Err(MathError::PrecisionExhausted);
                }
                let [left, right] = super::oriented::split_parameter(&parameter, math)?;
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
        }
        Ok(total)
    })();
    math.release_workspace(bytes)?;
    result
}

/// Enclosed true physical Left-side area and the complete ellipsoid surface.
/// Native completion retains every quadrature, contact and endpoint remainder.
pub(crate) struct RingArea {
    pub(crate) left: FixedInterval,
    pub(crate) surface: FixedInterval,
    /// Already admitted detached endpoints; the consuming owner releases these
    /// only after dropping this enclosure.
    pub(crate) storage: u64,
}

/// Selected original fragments share both regular potentials and the complete
/// equatorial cut construction with rings. Parameter grids retain their full
/// implicit contact tails, so they do not replace the true selected boundary.
pub(crate) fn selected_area(
    boundary: &arcs::arrangement::NativeBoundaryArrangement,
    parameters: &[crate::ellipsoidal::native::Parameters],
    budget: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<RingArea, GeoError> {
    let bytes = (boundary.fragments().len() as u64).saturating_mul(4096);
    context.admit_workspace(bytes)?;
    progress.context_poll(context)?;
    let fragments = boundary
        .fragments()
        .iter()
        .zip(parameters)
        .filter(|(fragment, _)| {
            fragment.stratum() == arcs::arrangement::SelectedFragmentStratum::ArealBoundary
        })
        .map(|(fragment, parameters)| hemisphere::Fragment {
            edge: fragment.original_edge(),
            endpoints: parameters.endpoints.each_ref(),
            errors: parameters.endpoint_errors(),
            reversed: fragment.reversed(),
        })
        .collect::<Vec<_>>();
    let result = hemisphere::area(
        &fragments,
        budget,
        context,
        progress,
        |point, context, progress| {
            if matches!(
                arcs::arrangement::selected_contact_stratum(point, boundary, context, progress,)?,
                Some(
                    arcs::arrangement::SelectedContactStratum::CurveInterior
                        | arcs::arrangement::SelectedContactStratum::IsolatedPoint
                )
            ) {
                return Ok(crate::Set::Exterior);
            }
            super::locate_selected_boundary(point, boundary, context, progress)
        },
    );
    drop(fragments);
    context.release_workspace(bytes)?;
    result
}

fn tail_bound(
    line: &ChartLine,
    potential: &AreaPotential,
    parameter: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    if let Some((integral, remainder)) =
        parallel_integral(line, potential, parameter, iterations, math, progress)?
    {
        return integral.abs(math)?.add(&remainder, math);
    }
    if matches!(line, ChartLine::Geodesic(line) if line.is_meridian()) {
        return FixedInterval::from_i64(0, math);
    }
    let (_, derivative) = line.image(parameter, iterations, math, progress)?;
    if derivative
        .as_ref()
        .is_some_and(|derivative| derivative[0].is_exact_zero())
    {
        return FixedInterval::from_i64(0, math);
    }
    let (normal, derivative) = line.normal(parameter, iterations, math, progress)?;
    let derivative = derivative.ok_or(MathError::PrecisionExhausted)?;
    let density = density(
        &normal,
        &derivative,
        &potential.polynomial,
        potential.gauge,
        &mut FixedArithmetic(math),
    )?
    .abs(math)?;
    let cross = normal[0]
        .mul(&derivative[1], math)?
        .sub(&normal[1].mul(&derivative[0], math)?, math)?
        .abs(math)?;
    let remainder = potential
        .tail
        .div(
            &FixedInterval::from_i64(1, math)?.add(
                &potential
                    .gauge
                    .latitude(&normal[2], &mut FixedArithmetic(math))?,
                math,
            )?,
            math,
        )?
        .mul(&cross, math)?;
    density
        .add(&remainder, math)?
        .mul(&parameter.width(math)?, math)
}

pub(crate) fn ring_area(
    curve: &PreparedCurve,
    budget: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<RingArea, GeoError> {
    let zero = Rat::zero();
    let one = Rat::one();
    let bytes = (curve.edges().len() as u64).saturating_mul(4096);
    context.admit_workspace(bytes)?;
    progress.context_poll(context)?;
    let fragments = curve
        .edges()
        .iter()
        .map(|edge| hemisphere::Fragment {
            edge,
            endpoints: [&zero, &one],
            errors: [&zero, &zero],
            reversed: false,
        })
        .collect::<Vec<_>>();
    let result = hemisphere::area(
        &fragments,
        budget,
        context,
        progress,
        |point, context, progress| {
            super::oriented::locate_validated(curve, point, context, progress)
        },
    );
    drop(fragments);
    context.release_workspace(bytes)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Coord, ExecutionLimits, ExecutionPolicy, GeographicReference, LonLat, OrientedInterior,
        PreparedCoordinate, PreparedEdge, PreparedGeometry, PreparedPolygon, PreparedRegion,
        ShortestGeodesicArc,
    };

    fn coordinate(
        longitude: i64,
        latitude: i64,
        reference: &GeographicReference,
    ) -> PreparedCoordinate {
        PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
            reference,
        )
        .unwrap()
    }
    fn polygon_geometry(
        ring: PreparedCurve,
        side: OrientedInterior,
        context: &mut MetricContext,
    ) -> PreparedGeometry {
        let polygon = PreparedPolygon::from_curves(vec![ring], side, context).unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap()
    }

    #[test]
    fn normal_area_preserves_native_parallel_cap_and_its_large_complement() {
        let reference = GeographicReference::wgs84();
        let ring = PreparedCurve::from_source(
            &[
                Coord::xy(Rat::from_i64(-180), Rat::from_i64(30)),
                Coord::xy(Rat::from_i64(180), Rat::from_i64(30)),
            ],
            &reference,
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let left = polygon_geometry(ring.clone(), OrientedInterior::Left, &mut context);
        let right = polygon_geometry(ring, OrientedInterior::Right, &mut context);
        let left_area = crate::ellipsoidal::area(&left, &mut context).unwrap();
        let right_area = crate::ellipsoidal::area(&right, &mut context).unwrap();
        // Independent analytic Q(1), Q(1/2) expressions evaluated with 90
        // decimal digits and a bounded Machin pi series, frozen at 8 places.
        let cap = Rat::parse_decimal("127944540878342.71812180").unwrap();
        let whole = Rat::parse_decimal("510065621724088.50929491").unwrap();
        let tolerance = Rat::parse_decimal("0.1").unwrap();
        assert!(left_area.exact().sub(&cap).abs() < tolerance);
        assert!(left_area.exact().add(right_area.exact()).sub(&whole).abs() < tolerance);
        assert!(right_area.exact() > left_area.exact());
        let perimeter = crate::ellipsoidal::perimeter(&left, &mut context).unwrap();
        let expected = Rat::parse_decimal("34735060.89032274").unwrap();
        assert!(perimeter.exact().sub(&expected).abs() < Rat::parse_decimal("0.001").unwrap());
    }

    #[test]
    fn original_affine_lune_includes_both_pole_aliases_and_its_complement() {
        let reference = GeographicReference::wgs84();
        // The source frames differ at each physical pole. Join the two genuine
        // meridians there; an explicit pole-to-pole-alias edge would be constant.
        let ring = PreparedCurve::new(
            [((60, -90), (60, 90)), ((0, 90), (0, -90))]
                .map(|(start, end)| {
                    PreparedEdge::SourceLinear(Box::new(
                        crate::SourceLinearEdge::new(
                            coordinate(start.0, start.1, &reference),
                            coordinate(end.0, end.1, &reference),
                        )
                        .unwrap(),
                    ))
                })
                .to_vec(),
        );
        let mut context = MetricContext::wgs84().unwrap();
        let left = polygon_geometry(ring.clone(), OrientedInterior::Left, &mut context);
        let right = polygon_geometry(ring, OrientedInterior::Right, &mut context);
        let left_area = crate::ellipsoidal::area(&left, &mut context).unwrap();
        let right_area = crate::ellipsoidal::area(&right, &mut context).unwrap();
        let surface = Rat::parse_decimal("510065621724088.50929491").unwrap();
        let tolerance = Rat::parse_decimal("0.1").unwrap();
        assert!(
            left_area
                .exact()
                .sub(&surface.div(&Rat::from_i64(6)).unwrap())
                .abs()
                < tolerance
        );
        assert!(
            left_area
                .exact()
                .add(right_area.exact())
                .sub(&surface)
                .abs()
                < tolerance
        );
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items *= 2;
        let mut raised =
            MetricContext::new(reference, ExecutionPolicy::new(limits).unwrap()).unwrap();
        assert_eq!(
            crate::ellipsoidal::area(&left, &mut raised).unwrap(),
            left_area
        );
    }

    #[test]
    fn geodesic_polar_wedges_include_the_declared_endpoint_sector() {
        let reference = GeographicReference::wgs84();
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 2_000_000;
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::new(limits).unwrap()).unwrap();
        for vertices in [
            [(0, 90), (0, 0), (60, 0), (0, 90)],
            [(0, -90), (60, 0), (0, 0), (0, -90)],
        ] {
            let mut edges = Vec::new();
            for pair in vertices.windows(2) {
                edges.push(PreparedEdge::ShortestGeodesic(Box::new(
                    ShortestGeodesicArc::new(
                        coordinate(pair[0].0, pair[0].1, &reference),
                        coordinate(pair[1].0, pair[1].1, &reference),
                        &mut context,
                    )
                    .unwrap(),
                )));
            }
            let geometry = polygon_geometry(
                PreparedCurve::new(edges),
                OrientedInterior::Left,
                &mut context,
            );
            let area = crate::ellipsoidal::area(&geometry, &mut context).unwrap_or_else(|error| {
                panic!(
                    "polar sector {vertices:?}: {error:?}; work={}",
                    context.work_items()
                )
            });
            let expected = Rat::parse_decimal("42505468477007.37577458").unwrap();
            assert!(area.exact().sub(&expected).abs() < Rat::parse_decimal("0.1").unwrap());
        }
    }

    fn geodesic_ring(
        vertices: &[(i64, i64)],
        reference: &GeographicReference,
        context: &mut MetricContext,
    ) -> PreparedCurve {
        PreparedCurve::new(
            vertices
                .windows(2)
                .map(|pair| {
                    PreparedEdge::ShortestGeodesic(Box::new(
                        ShortestGeodesicArc::new(
                            coordinate(pair[0].0, pair[0].1, reference),
                            coordinate(pair[1].0, pair[1].1, reference),
                            context,
                        )
                        .unwrap(),
                    ))
                })
                .collect(),
        )
    }

    #[test]
    fn both_pole_geodesic_lune_and_selected_union_preserve_the_full_complement() {
        let reference = GeographicReference::wgs84();
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 2_000_000;
        let policy = ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let vertices = [(0, 90), (0, 0), (0, -90), (60, 0), (0, 90)];
        let lune = polygon_geometry(
            geodesic_ring(&vertices, &reference, &mut context),
            OrientedInterior::Left,
            &mut context,
        );
        let whole = Rat::parse_decimal("510065621724088.50929491").unwrap();
        let expected = whole.div(&Rat::from_i64(6)).unwrap();
        let tolerance = Rat::parse_decimal("0.1").unwrap();
        let actual = crate::ellipsoidal::area(&lune, &mut context).unwrap();
        assert!(actual.exact().sub(&expected).abs() < tolerance);
        let mut polygons = Vec::new();
        for vertices in [
            [(0, 90), (0, 0), (60, 0), (0, 90)],
            [(0, -90), (60, 0), (0, 0), (0, -90)],
        ] {
            let ring = geodesic_ring(&vertices, &reference, &mut context);
            polygons.push(
                PreparedPolygon::from_curves(vec![ring], OrientedInterior::Left, &mut context)
                    .unwrap(),
            );
        }
        let region = PreparedRegion::polygons(polygons);
        let joined = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            region.clone(),
            policy,
        )
        .unwrap();
        let complement = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            Vec::new(),
            region.complement(),
            policy,
        )
        .unwrap();
        assert!(matches!(
            crate::ellipsoidal::area(&joined, &mut context),
            Err(GeoError::WorkExhausted { limit: 2_000_000 })
        ));
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
        // This newly introduced multi-ring fixture retains its measured 2M
        // refusal and separately admits the complete union/hemisphere proof.
        // Production limits and all pre-existing fixtures remain unchanged.
        limits.max_work_items = 10_000_000;
        let mut adequate = MetricContext::new(
            context.reference().clone(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let joined = crate::ellipsoidal::area(&joined, &mut adequate).unwrap();
        let complement = crate::ellipsoidal::area(&complement, &mut adequate).unwrap();
        assert_eq!(joined, actual);
        assert!(joined.exact().add(complement.exact()).sub(&whole).abs() < tolerance);
        assert!(complement.exact() > joined.exact());
    }

    #[test]
    fn transformed_equator_tangency_keeps_the_original_curve_and_both_sides() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference, Polynomial2d, PolynomialTerm,
        };
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let reference = GeographicReference::new(
            crate::PreparedEllipsoid::new(Rat::one(), Rat::parse_decimal("298.257223563").unwrap())
                .unwrap(),
            Digest32::new([91; 32]),
            crate::AxisOrder::LonLat,
        );
        let op_ref = |id, unit| OperationReference {
            realization: Digest32::new([id; 32]),
            unit,
            swapped_axes: false,
        };
        // The source map (x,y)->(x,y+x²) has determinant one everywhere.
        // Its bottom edge is tangent to the equator at one exact interior
        // parameter; no transverse root certificate can discard that contact.
        let polynomial = Polynomial2d::new(
            2,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 0,
                    y_power: 1,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
            ],
        )
        .unwrap();
        let polynomial = CoordinateOperation::compile(
            op_ref(1, CoordinateUnit::Metres),
            op_ref(2, CoordinateUnit::Metres),
            OperationModel::Polynomial2d(polynomial),
        )
        .unwrap();
        let geographic = CoordinateOperation::compile(
            op_ref(2, CoordinateUnit::Metres),
            OperationReference {
                realization: reference.id().digest(),
                unit: CoordinateUnit::Degrees,
                swapped_axes: false,
            },
            OperationModel::MercatorToGeographic {
                radius: Rat::one(),
                eccentricity_squared: Rat::zero(),
                square_domain: false,
            },
        )
        .unwrap();
        let chain = Arc::new(OperationChain::compile(vec![polynomial, geographic]).unwrap());
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 2_000_000;
        let policy = ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let vertices = [(-1, 0), (1, 0), (1, 1), (-1, 1), (-1, 0)].map(|(x, y)| {
            Coord::xy(
                Rat::from_i64(x).div(&Rat::from_i64(100)).unwrap(),
                Rat::from_i64(y).div(&Rat::from_i64(100)).unwrap(),
            )
        });
        let ring = PreparedCurve::new(
            vertices
                .windows(2)
                .map(|pair| {
                    PreparedEdge::Transformed(Box::new(
                        OperationImageCurve::new(
                            pair[0].clone(),
                            pair[1].clone(),
                            Arc::clone(&chain),
                            reference.clone(),
                            None,
                            policy,
                        )
                        .unwrap(),
                    ))
                })
                .collect(),
        );
        let left = polygon_geometry(ring.clone(), OrientedInterior::Left, &mut context);
        let right = polygon_geometry(ring, OrientedInterior::Right, &mut context);
        let tangent = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        assert_eq!(
            crate::atlas::locate(&tangent, left.region(), &mut context).unwrap(),
            crate::Set::Boundary
        );
        let a = crate::ellipsoidal::area(&left, &mut context).unwrap();
        let b = crate::ellipsoidal::area(&right, &mut context).unwrap();
        assert!(a.exact().is_zero());
        assert_eq!(b.exact(), &Rat::parse_decimal("12.54").unwrap());
    }

    #[test]
    fn transformed_polar_boundary_uses_its_smooth_original_normal_and_complement() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference,
        };
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let reference = GeographicReference::new(
            crate::PreparedEllipsoid::new(Rat::one(), Rat::parse_decimal("298.257223563").unwrap())
                .unwrap(),
            Digest32::new([96; 32]),
            crate::AxisOrder::LonLat,
        );
        let operation = CoordinateOperation::compile(
            OperationReference {
                realization: Digest32::new([97; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            OperationReference {
                realization: reference.id().digest(),
                unit: CoordinateUnit::Degrees,
                swapped_axes: false,
            },
            OperationModel::GeocentricToGeographic {
                ellipsoid: reference.ellipsoid().clone(),
            },
        )
        .unwrap();
        let chain = Arc::new(OperationChain::compile(vec![operation]).unwrap());
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 10_000_000;
        let policy = ExecutionPolicy::new(limits).unwrap();
        for sign in [-1, 1] {
            let mut context = MetricContext::new(reference.clone(), policy).unwrap();
            let z = reference
                .ellipsoid()
                .semiminor()
                .add(&Rat::from_i64(12))
                .mul(&Rat::from_i64(sign));
            let mut vertices = [(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)].map(|(x, y)| {
                Coord::new(
                    Rat::from_i64(x).div(&Rat::from_i64(100)).unwrap(),
                    Rat::from_i64(y).div(&Rat::from_i64(100)).unwrap(),
                    Some(z.clone()),
                    None,
                )
            });
            if sign < 0 {
                vertices.reverse();
            }
            let ring = PreparedCurve::new(
                vertices
                    .windows(2)
                    .map(|pair| {
                        PreparedEdge::Transformed(Box::new(
                            OperationImageCurve::new(
                                pair[0].clone(),
                                pair[1].clone(),
                                Arc::clone(&chain),
                                reference.clone(),
                                None,
                                policy,
                            )
                            .unwrap(),
                        ))
                    })
                    .collect(),
            );
            let left = polygon_geometry(ring.clone(), OrientedInterior::Left, &mut context);
            let right = polygon_geometry(ring, OrientedInterior::Right, &mut context);
            let pole = LonLat::new(Rat::zero(), Rat::from_i64(sign * 90)).unwrap();
            assert_eq!(
                crate::atlas::locate(&pole, left.region(), &mut context).unwrap(),
                crate::Set::Boundary
            );
            let a = crate::ellipsoidal::area(&left, &mut context).unwrap();
            let b = crate::ellipsoidal::area(&right, &mut context).unwrap();
            assert!(a.exact().is_zero());
            assert_eq!(b.exact(), &Rat::parse_decimal("12.54").unwrap());
        }
    }

    #[test]
    fn returned_large_area_stays_charged_and_cancelled_attempts_release_owners() {
        use purrdf_hash::hex::Digest32;
        let reference = GeographicReference::new(
            crate::PreparedEllipsoid::new(
                Rat::from_i64(1_000_000_000_000_000),
                Rat::parse_decimal("298.257223563").unwrap(),
            )
            .unwrap(),
            Digest32::new([93; 32]),
            crate::AxisOrder::LonLat,
        );
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 10_000_000;
        let policy = ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let ring = geodesic_ring(
            &[(0, 90), (0, 0), (60, 0), (0, 90)],
            &reference,
            &mut context,
        );
        context.begin(0).unwrap();
        let mut progress = WorkProgress::new(None);
        let result = ring_area(
            &ring,
            &Rat::parse_decimal("0.025").unwrap(),
            &mut context,
            &mut progress,
        )
        .unwrap();
        assert!(result.storage > 0);
        assert_eq!(
            context.current_workspace_bytes() - context.retained_workspace_bytes(),
            result.storage
        );
        let retained = result.storage;
        drop(result);
        context.release_workspace(retained).unwrap();
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
        struct Refuse {
            calls: usize,
        }
        impl crate::MetricWorkObserver for Refuse {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                self.calls += 1;
                if self.calls == 20 {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        context.begin(0).unwrap();
        let mut observer = Refuse { calls: 0 };
        let mut progress = WorkProgress::new(Some(&mut observer));
        let result = ring_area(
            &ring,
            &Rat::parse_decimal("0.025").unwrap(),
            &mut context,
            &mut progress,
        );
        assert!(matches!(result, Err(GeoError::Cancelled)));
        assert_eq!(observer.calls, 20);
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
    }
    #[test]
    fn selected_south_pole_union_uses_the_regular_gauge_and_preserves_complement() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let mut polygons = Vec::new();
        for [west, east] in [[0, 60], [30, 90]] {
            // The different written longitudes at the two ends are the same
            // physical south pole. Every nonpolar edge is an exact meridian
            // or equator; the union is independently one eighth of the surface.
            let ring = PreparedCurve::from_source(
                &[(east, -90), (east, 0), (west, 0), (west, -90)]
                    .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y))),
                &reference,
            )
            .unwrap();
            polygons.push(
                PreparedPolygon::from_curves(vec![ring], OrientedInterior::Left, &mut context)
                    .unwrap(),
            );
        }
        let region = PreparedRegion::polygons(polygons);
        let geometry = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            region.clone(),
            context.policy(),
        )
        .unwrap();
        let inverse = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            Vec::new(),
            region.complement(),
            context.policy(),
        )
        .unwrap();
        let area = crate::ellipsoidal::area(&geometry, &mut context).unwrap();
        let other = crate::ellipsoidal::area(&inverse, &mut context).unwrap();
        let surface = Rat::parse_decimal("510065621724088.50929491").unwrap();
        let tolerance = Rat::parse_decimal("0.1").unwrap();
        assert!(area.exact().mul(&Rat::from_i64(8)).sub(&surface).abs() < tolerance);
        assert!(area.exact().add(other.exact()).sub(&surface).abs() < tolerance);
        assert!(other.exact() > area.exact());
    }
}
