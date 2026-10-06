// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Taylor-model arc length of an original polynomial image under the
//! spherical inverse Mercator.
//!
//! For a chain `Polynomial2d` then `MercatorToGeographic` with zero
//! eccentricity, the image of an affine original edge has the closed form
//!
//! ```text
//! (X, Y) = P(u(τ)),  λ = X/R,  E = exp(Y/R),
//! cos φ = 2E/(E²+1),  sin φ = (E²−1)/(E²+1),  φ' = cos φ · Y'/R,
//! s = sqrt((M φ')² + (N cos φ λ')²)
//! ```
//!
//! where `N` and `M` are the target ellipsoid's prime-vertical and meridian
//! radii at `φ`. Every quantity is an interval Taylor jet in the fragment
//! parameter `τ`, so the complete-panel remainder of the even-order panel rule
//! certifies the integral with high-order convergence. The first-order panel
//! enclosure needs a number of panels proportional to the speed's total
//! variation divided by the budget, which is unbounded in practice for an image
//! whose speed changes by a constant factor. A jet whose speed vanishes in the
//! panel is not analytic there; that panel reports `PrecisionExhausted` and the
//! caller's original first-order enclosure decides it.

use crate::Rat;
use crate::context::WorkProgress;
use crate::numerical::{fixed_from_rat, math_exact_rationals};
use crate::operation::{OperationImageCurve, OperationModel, Polynomial2d};
use purrdf_xsd::integer::ExactOperation;
use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, MathError, SymmetricTaylorPanel, TaylorJet, TaylorWorkspace,
};

/// The even Taylor quadrature order of one panel.
const ORDER: usize = 8;

/// One polynomial-Mercator image restricted to an exact parameter range,
/// rewritten as normalized polynomial arguments `u_a(τ) = offset_a + slope_a τ`
/// for `τ` in `[0,1]`.
pub(super) struct MercatorPolynomial<'a> {
    polynomial: &'a Polynomial2d,
    radius: &'a Rat,
    offset: [Rat; 2],
    slope: [Rat; 2],
    semimajor: &'a Rat,
    eccentricity_squared: Rat,
}

impl<'a> MercatorPolynomial<'a> {
    /// The exact affine normalized arguments of the original edge on
    /// `[start, end]`, or `None` for any other chain.
    pub(super) fn new(
        image: &'a OperationImageCurve,
        [start, end]: [&Rat; 2],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<Self>, MathError> {
        let operations = image.chain().operations();
        let [first, second] = operations else {
            return Ok(None);
        };
        let OperationModel::Polynomial2d(polynomial) = first.model() else {
            return Ok(None);
        };
        let OperationModel::MercatorToGeographic {
            radius,
            eccentricity_squared,
            ..
        } = second.model()
        else {
            return Ok(None);
        };
        if !eccentricity_squared.is_zero()
            || usize::from(polynomial.degree()) > ORDER
            || second.source().swapped_axes
        {
            return Ok(None);
        }
        let (a, b) = image.source_endpoints();
        if a.z().is_some() || b.z().is_some() {
            return Ok(None);
        }
        let mut from = [a.x(), a.y()];
        let mut to = [b.x(), b.y()];
        if first.source().swapped_axes {
            from.reverse();
            to.reverse();
        }
        let mut offset = [Rat::zero(), Rat::zero()];
        let mut slope = [Rat::zero(), Rat::zero()];
        for axis in 0..2 {
            let origin = &polynomial.origin()[axis];
            let scale = &polynomial.scale()[axis];
            let operands = [from[axis], to[axis], start, end, origin, scale];
            let (value, rate) = math_exact_rationals(
                ExactOperation::RationalDivide,
                &operands,
                8,
                math,
                progress,
                || {
                    let delta = to[axis].sub(from[axis]);
                    let value = from[axis]
                        .add(&delta.mul(start))
                        .sub(origin)
                        .div(scale)
                        .expect("positive polynomial scale");
                    let rate = delta
                        .mul(&end.sub(start))
                        .div(scale)
                        .expect("positive polynomial scale");
                    (value, rate)
                },
            )?;
            offset[axis] = value;
            slope[axis] = rate;
        }
        let ellipsoid = image.reference().ellipsoid();
        Ok(Some(Self {
            polynomial,
            radius,
            offset,
            slope,
            semimajor: ellipsoid.semimajor(),
            eccentricity_squared: ellipsoid.eccentricity_squared(),
        }))
    }

    /// The certified integral of the speed over the exact dyadic `τ` panel
    /// `[index, index+1]/2^level`, as a complete enclosure.
    ///
    /// The panel is integrated in its normalized variable `σ` in `[-1,1]`,
    /// `τ = centre + half·σ`, so the jet is `ds/dσ` and its integral is the
    /// panel length. Fixed-point coefficients and remainder powers stay of
    /// unit scale; a remainder in `τ` would need `half^(ORDER+1)`, which is
    /// below the fixed grid on deep panels.
    pub(super) fn panel(
        &self,
        level: u32,
        index: u64,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<FixedInterval, MathError> {
        let denominator = crate::Int::one().shl(level + 1);
        let centre = Rat::new(crate::Int::from_u64(2 * index + 1), denominator.clone())
            .expect("positive dyadic");
        let half = Rat::new(crate::Int::one(), denominator).expect("positive dyadic");
        let constants = self.constants(&centre, &half, math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let one = FixedInterval::from_i64(1, math)?;
        let unit =
            FixedInterval::from_bounds(one.neg(math)?.lower().clone(), one.upper().clone(), math)?;
        let panel = SymmetricTaylorPanel::new(&zero, &unit, &one, math)?;
        let live = 2 * (ORDER + 1) + 6 * self.polynomial.terms().len() + 48;
        let (integral, remainder) = purrdf_xsd::math::integrate_taylor_panel_observed(
            ORDER,
            live,
            panel,
            math,
            progress,
            |math, progress| progress.math_poll(math),
            |parameter, workspace, math, progress| {
                progress.math_poll(math)?;
                self.speed(parameter, &constants, workspace, math)
            },
        )?;
        integral.add(&remainder.neg(math)?.hull(&remainder, math)?, math)
    }

    /// Panel constants; the normalized arguments become
    /// `(offset + slope·centre) + (slope·half)·σ`.
    fn constants(
        &self,
        centre: &Rat,
        half: &Rat,
        math: &mut CoordinateMath,
    ) -> Result<Constants, MathError> {
        let one = FixedInterval::from_i64(1, math)?;
        let centre = fixed_from_rat(centre, math)?;
        let half = fixed_from_rat(half, math)?;
        let mut offset = [
            FixedInterval::from_i64(0, math)?,
            FixedInterval::from_i64(0, math)?,
        ];
        let mut slope = [
            FixedInterval::from_i64(0, math)?,
            FixedInterval::from_i64(0, math)?,
        ];
        for axis in 0..2 {
            let rate = fixed_from_rat(&self.slope[axis], math)?;
            offset[axis] =
                fixed_from_rat(&self.offset[axis], math)?.add(&rate.mul(&centre, math)?, math)?;
            slope[axis] = rate.mul(&half, math)?;
        }
        let e2 = fixed_from_rat(&self.eccentricity_squared, math)?;
        Ok(Constants {
            inverse_radius: one.div(&fixed_from_rat(self.radius, math)?, math)?,
            semimajor: fixed_from_rat(self.semimajor, math)?,
            complement: one.sub(&e2, math)?,
            e2,
            one,
            two: FixedInterval::from_i64(2, math)?,
            offset,
            slope,
        })
    }

    /// The speed jet `ds/dσ` at one exact point or over one complete panel.
    fn speed<'scope>(
        &self,
        parameter: &FixedInterval,
        constants: &Constants,
        workspace: &mut TaylorWorkspace<'scope>,
        math: &mut CoordinateMath,
    ) -> Result<TaylorJet<'scope>, MathError> {
        let degree = usize::from(self.polynomial.degree());
        let mut powers: [[Option<TaylorJet<'scope>>; ORDER + 1]; 2] = [[None; ORDER + 1]; 2];
        for (axis, table) in powers.iter_mut().enumerate() {
            let value =
                constants.offset[axis].add(&constants.slope[axis].mul(parameter, math)?, math)?;
            let argument = workspace.argument(value, constants.slope[axis].clone(), math)?;
            let mut current = workspace.constant(constants.one.clone(), math)?;
            table[0] = Some(current);
            for slot in table.iter_mut().take(degree + 1).skip(1) {
                current = workspace.mul(current, argument, math)?;
                *slot = Some(current);
            }
        }
        let zero = FixedInterval::from_i64(0, math)?;
        let mut image = [
            workspace.constant(zero.clone(), math)?,
            workspace.constant(zero, math)?,
        ];
        for term in self.polynomial.terms() {
            let power = |axis: usize, exponent: u16| {
                powers[axis][usize::from(exponent)]
                    .ok_or(MathError::Domain("polynomial term above declared degree"))
            };
            let monomial = workspace.mul(power(0, term.x_power)?, power(1, term.y_power)?, math)?;
            for (row, coefficient) in [&term.x_coefficient, &term.y_coefficient]
                .into_iter()
                .enumerate()
            {
                let coefficient = fixed_from_rat(coefficient, math)?;
                let scaled = workspace.scale(monomial, &coefficient, math)?;
                image[row] = workspace.add(image[row], scaled, math)?;
            }
        }
        // The image ordinates are polynomials of degree at most ORDER in τ,
        // so their derivative jets are exact coefficient shifts.
        let mut rates = [image[0], image[1]];
        for (row, rate) in rates.iter_mut().enumerate() {
            let derivative = workspace.constant(FixedInterval::from_i64(0, math)?, math)?;
            for order in 0..ORDER {
                let factor = FixedInterval::from_i64(
                    i64::try_from(order + 1).map_err(|_| MathError::WorkExhausted)?,
                    math,
                )?;
                let value = workspace
                    .coefficient(image[row], order + 1)?
                    .mul(&factor, math)?;
                workspace.set_coefficient(derivative, order, value, math)?;
            }
            *rate = derivative;
        }
        let y = workspace.scale(image[1], &constants.inverse_radius, math)?;
        let exponential = workspace.exp(y, math)?;
        let square = workspace.mul(exponential, exponential, math)?;
        let one = workspace.constant(constants.one.clone(), math)?;
        let denominator = workspace.add(square, one, math)?;
        let doubled = workspace.scale(exponential, &constants.two, math)?;
        let cosine = workspace.div(doubled, denominator, math)?;
        let numerator = workspace.sub(square, one, math)?;
        let sine = workspace.div(numerator, denominator, math)?;
        let longitude_rate = workspace.scale(rates[0], &constants.inverse_radius, math)?;
        let isometric_rate = workspace.scale(rates[1], &constants.inverse_radius, math)?;
        let latitude_rate = workspace.mul(cosine, isometric_rate, math)?;
        let sine_square = workspace.mul(sine, sine, math)?;
        let flattened = workspace.scale(sine_square, &constants.e2, math)?;
        let w2 = workspace.sub(one, flattened, math)?;
        let w = workspace.sqrt(w2, math)?;
        let semimajor = workspace.constant(constants.semimajor.clone(), math)?;
        let normal = workspace.div(semimajor, w, math)?;
        let meridian_numerator = workspace.scale(normal, &constants.complement, math)?;
        let meridian = workspace.div(meridian_numerator, w2, math)?;
        let north = workspace.mul(meridian, latitude_rate, math)?;
        let parallel = workspace.mul(normal, cosine, math)?;
        let east = workspace.mul(parallel, longitude_rate, math)?;
        let north_square = workspace.mul(north, north, math)?;
        let east_square = workspace.mul(east, east, math)?;
        let speed_square = workspace.add(north_square, east_square, math)?;
        if !speed_square_positive(workspace.coefficient(speed_square, 0)?) {
            return Err(MathError::PrecisionExhausted);
        }
        workspace.sqrt(speed_square, math)
    }
}

fn speed_square_positive(value: &FixedInterval) -> bool {
    !value.lower().is_negative() && !value.lower().is_zero()
}

struct Constants {
    inverse_radius: FixedInterval,
    semimajor: FixedInterval,
    complement: FixedInterval,
    e2: FixedInterval,
    one: FixedInterval,
    two: FixedInterval,
    offset: [FixedInterval; 2],
    slope: [FixedInterval; 2],
}
