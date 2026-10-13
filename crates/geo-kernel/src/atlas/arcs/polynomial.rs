// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact contacts of two transformed edges sharing one polynomial-led chain.
//!
//! When the chain is a caller polynomial `P` followed by an operation that is
//! injective on the edges' images, two image points coincide exactly when
//! their polynomial images do. For original affine edges `L(t)` and `R(u)`,
//! `t, u` in `[0,1]`, the contacts are therefore the solutions of the exact
//! rational system
//!
//! ```text
//! X(L(t)) = X(R(u)),   Y(L(t)) = Y(R(u)).
//! ```
//!
//! Eliminating `u` gives the resultant `Res_u(t)`. Its real roots in `[0,1]`
//! are isolated with an exact Sturm sequence, and each root is recovered as a
//! rational only after the exact polynomial vanishes there; for each rational
//! `t` the matching `u` are the rational roots of the gcd of the two
//! specialized polynomials. Multiple roots, such as a fold point where one
//! image has zero parameter speed, are exact here, unlike a numerical
//! inclusion. Two affine images on one shared axis line meet in the exact
//! overlap of their coordinate ranges. Any other shared component, or any
//! irrational root in `[0,1]`, leaves this certificate inapplicable and the
//! caller's general solver runs unchanged.
//!
//! Every rational operation is admitted through the shared exact admission at
//! its actual operand widths before it executes, so the charge is the work
//! performed and governor cancellation is honoured between operations.

use super::{ContactList, CurveIntersection};
use crate::context::{RetainedBuilder, WorkProgress};
use crate::operation::{OperationImageCurve, OperationModel, Polynomial2d};
use crate::{GeoError, Int, MetricContext, PreparedEdge, Rat};
use core::cmp::Ordering;
use purrdf_xsd::integer::{ExactOperation, ExactOperation::Linear};

const ADMISSION: &str = "polynomial contact admission";

/// Admits each exact rational operation before it runs.
struct Meter<'a, 'b, 'c> {
    context: &'a mut MetricContext,
    progress: &'b mut WorkProgress<'c>,
    retained: &'a mut u64,
}

impl Meter<'_, '_, '_> {
    fn run<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
        ) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress)
            .expression(operation, operands, 1, evaluate)
    }
    fn add(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress).add(a, b)
    }
    fn sub(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress).subtract(a, b)
    }
    fn mul(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress).multiply(a, b)
    }
    fn div(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        if b.is_zero() {
            return Err(GeoError::domain("polynomial contact division by zero"));
        }
        Ok(
            crate::numerical::ExactAdmission::new(self.context, self.progress)
                .divide(a, b)?
                .expect("checked nonzero divisor"),
        )
    }
    fn compare(&mut self, a: &Rat, b: &Rat) -> Result<Ordering, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress).compare(a, b)
    }

    fn negate(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::ExactAdmission::new(self.context, self.progress).negate(value)
    }
    fn absolute(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        self.run(Linear, &[value], |values| values.absolute(value))
    }
    fn buffer<T>(&mut self, length: usize) -> Result<RetainedBuilder<T>, GeoError> {
        let mut values = RetainedBuilder::default();
        values.reserve_admitted(length, self.context, self.progress, self.retained)?;
        Ok(values)
    }
    fn push<T>(&mut self, values: &mut RetainedBuilder<T>, value: T) -> Result<(), GeoError> {
        values.push_admitted(value, self.context, self.progress, self.retained)
    }
    fn zeros(&mut self, length: usize) -> Result<RetainedBuilder<Rat>, GeoError> {
        let mut values = self.buffer(length)?;
        for _ in 0..length {
            values.push_reserved(Rat::zero());
        }
        Ok(values)
    }
    fn canonical_roots(&mut self, values: &mut RetainedBuilder<Rat>) -> Result<(), GeoError> {
        purrdf_lex::walk::try_sort_unstable_by(values, |a, b| self.compare(a, b))?;
        values.dedup_by(|a, b| self.compare(a, b).map(Ordering::is_eq))
    }

    fn contacts<const N: usize>(
        &mut self,
        contacts: [CurveIntersection; N],
    ) -> Result<ContactList, GeoError> {
        super::contact_list(contacts, self.context, self.progress)
    }
}

/// A univariate polynomial over the rationals, lowest degree first, without
/// trailing zero coefficients; the zero polynomial is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Poly(crate::PreparedValues<Rat>);

impl Poly {
    fn constant(value: Rat, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        Self::from_values([value].into_iter(), meter)
    }
    fn linear(constant: Rat, slope: Rat, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        Self::from_values([constant, slope].into_iter(), meter)
    }
    fn from_values(
        values: impl ExactSizeIterator<Item = Rat>,
        meter: &mut Meter<'_, '_, '_>,
    ) -> Result<Self, GeoError> {
        let mut buffer = meter.buffer(values.len())?;
        for value in values {
            buffer.push_reserved(value);
        }
        Ok(Self::from_builder(buffer))
    }
    fn from_builder(mut values: RetainedBuilder<Rat>) -> Self {
        while values.last().is_some_and(Rat::is_zero) {
            values.pop();
        }
        Self(crate::PreparedValues::from_admitted(values))
    }
    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    fn degree(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }
    fn leading(&self) -> Rat {
        self.0.last().cloned().unwrap_or_else(Rat::zero)
    }
    fn coefficient(&self, power: usize) -> Rat {
        self.0.get(power).cloned().unwrap_or_else(Rat::zero)
    }
    fn neg(&self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let mut values = meter.buffer(self.0.len())?;
        for value in &self.0 {
            values.push_reserved(meter.negate(value)?);
        }
        Ok(Self::from_builder(values))
    }
    fn add(&self, rhs: &Self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let length = self.0.len().max(rhs.0.len());
        let mut sum = meter.buffer(length)?;
        for index in 0..length {
            sum.push_reserved(meter.add(&self.coefficient(index), &rhs.coefficient(index))?);
        }
        Ok(Self::from_builder(sum))
    }
    fn sub(&self, rhs: &Self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        self.add(&rhs.neg(meter)?, meter)
    }
    fn scale(&self, factor: &Rat, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let mut scaled = meter.buffer(self.0.len())?;
        for value in &self.0 {
            scaled.push_reserved(meter.mul(value, factor)?);
        }
        Ok(Self::from_builder(scaled))
    }
    fn mul(&self, rhs: &Self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::default());
        }
        let length = self
            .0
            .len()
            .checked_add(rhs.0.len())
            .and_then(|length| length.checked_sub(1))
            .ok_or(GeoError::ArithmeticOverflow(ADMISSION))?;
        let mut product = meter.zeros(length)?;
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in rhs.0.iter().enumerate() {
                let term = meter.mul(a, b)?;
                product[i + j] = meter.add(&product[i + j], &term)?;
            }
        }
        Ok(Self::from_builder(product))
    }
    fn power(&self, exponent: u16, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let mut result = Self::constant(Rat::one(), meter)?;
        for _ in 0..exponent {
            result = result.mul(self, meter)?;
        }
        Ok(result)
    }
    fn evaluate(&self, at: &Rat, meter: &mut Meter<'_, '_, '_>) -> Result<Rat, GeoError> {
        let mut sum = Rat::zero();
        for coefficient in self.0.iter().rev() {
            let product = meter.mul(&sum, at)?;
            sum = meter.add(&product, coefficient)?;
        }
        Ok(sum)
    }
    fn derivative(&self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let mut derivative = meter.buffer(self.0.len().saturating_sub(1))?;
        for (power, value) in self.0.iter().enumerate().skip(1) {
            let power =
                i64::try_from(power).map_err(|_| GeoError::ArithmeticOverflow(ADMISSION))?;
            derivative.push_reserved(meter.mul(value, &Rat::from_i64(power))?);
        }
        Ok(Self::from_builder(derivative))
    }
    /// Euclidean division over the rationals: the quotient and the remainder.
    fn divide(
        &self,
        divisor: &Self,
        meter: &mut Meter<'_, '_, '_>,
    ) -> Result<(Self, Self), GeoError> {
        let degree = divisor
            .degree()
            .ok_or_else(|| GeoError::domain("polynomial contact division by zero"))?;
        let lead = divisor.leading();
        let mut remainder = self.clone();
        let mut quotient = meter.zeros(self.0.len().saturating_sub(degree))?;
        while let Some(current) = remainder.degree()
            && current >= degree
        {
            let factor = meter.div(&remainder.leading(), &lead)?;
            let shift = current - degree;
            let length = shift
                .checked_add(divisor.0.len())
                .ok_or(GeoError::ArithmeticOverflow(ADMISSION))?;
            let mut subtrahend = meter.buffer(length)?;
            for _ in 0..shift {
                subtrahend.push_reserved(Rat::zero());
            }
            for value in &divisor.0 {
                subtrahend.push_reserved(meter.mul(value, &factor)?);
            }
            quotient[shift] = factor;
            // Exact arithmetic cancels the leading term, so the degree drops.
            remainder = remainder.sub(&Self::from_builder(subtrahend), meter)?;
        }
        Ok((Self::from_builder(quotient), remainder))
    }
    fn monic(&self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let lead = self.leading();
        if lead.is_zero() {
            return Ok(self.clone());
        }
        let mut monic = meter.buffer(self.0.len())?;
        for value in &self.0 {
            monic.push_reserved(meter.div(value, &lead)?);
        }
        Ok(Self::from_builder(monic))
    }
    fn gcd(&self, other: &Self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let (mut a, mut b) = (self.clone(), other.clone());
        while !b.is_zero() {
            let remainder = a.divide(&b, meter)?.1;
            a = b;
            b = remainder;
        }
        a.monic(meter)
    }
    /// The squarefree part: the same distinct roots, each simple.
    fn squarefree(&self, meter: &mut Meter<'_, '_, '_>) -> Result<Self, GeoError> {
        let common = self.gcd(&self.derivative(meter)?, meter)?;
        if common.degree().unwrap_or(0) == 0 {
            self.monic(meter)
        } else {
            self.divide(&common, meter)?.0.monic(meter)
        }
    }
}

/// Sign changes of a Sturm sequence at one exact point; zeros are skipped.
fn sign_changes(
    sequence: &[Poly],
    at: &Rat,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<usize, GeoError> {
    let mut changes = 0;
    let mut previous = 0;
    for polynomial in sequence {
        let sign = polynomial.evaluate(at, meter)?.signum();
        if sign != 0 {
            if previous != 0 && sign != previous {
                changes += 1;
            }
            previous = sign;
        }
    }
    Ok(changes)
}

fn sturm(
    squarefree: &Poly,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<RetainedBuilder<Poly>, GeoError> {
    let mut sequence = meter.buffer(2)?;
    sequence.push_reserved(squarefree.clone());
    sequence.push_reserved(squarefree.derivative(meter)?);
    while let Some(last) = sequence.last()
        && !last.is_zero()
        && last.degree() != Some(0)
    {
        let length = sequence.len();
        let next = sequence[length - 2]
            .divide(&sequence[length - 1], meter)?
            .1
            .neg(meter)?;
        if next.is_zero() {
            break;
        }
        meter.push(&mut sequence, next)?;
    }
    Ok(sequence)
}

/// The rational with the smallest denominator in a closed interval of
/// nonnegative rationals with `lower < upper` (continued-fraction descent).
fn simplest_rational(
    lower: &Rat,
    upper: &Rat,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Rat, GeoError> {
    let mut lower = lower.clone();
    let mut upper = upper.clone();
    let mut parents = RetainedBuilder::default();
    let mut result = loop {
        let (quotient, remainder) =
            meter.run(ExactOperation::RationalDivide, &[&lower], |values| {
                Ok(values
                    .integer_divide(lower.numerator().as_ref(), lower.denominator().as_ref())?
                    .expect("positive denominator"))
            })?;
        let whole = Rat::from_int(if remainder.is_negative() {
            meter.run(Linear, &[&lower], |values| {
                values.integer_subtract(&quotient, &Int::one())
            })?
        } else {
            quotient
        });
        if whole == lower {
            break meter.run(Linear, &[&lower], |values| values.original(&lower))?;
        }
        let next = meter.add(&whole, &Rat::one())?;
        if meter.compare(&next, &upper)?.is_le() {
            break next;
        }
        // The same continued-fraction descent reverses the reciprocal endpoints.
        // Native pending parents replace machine-stack recursion.
        let low = meter.sub(&lower, &whole)?;
        let high = meter.sub(&upper, &whole)?;
        let inner_low = meter.div(&Rat::one(), &high)?;
        let inner_high = meter.div(&Rat::one(), &low)?;
        meter.push(&mut parents, whole)?;
        lower = inner_low;
        upper = inner_high;
    };
    while let Some(whole) = parents.pop() {
        let fraction = meter.div(&Rat::one(), &result)?;
        result = meter.add(&whole, &fraction)?;
    }
    Ok(result)
}

/// Every root of a nonzero polynomial in `[0,1]` as an exact rational, or
/// `None` when some root there is irrational.
fn unit_rational_roots(
    polynomial: &Poly,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Option<RetainedBuilder<Rat>>, GeoError> {
    let mut current = polynomial.clone();
    let mut roots = RetainedBuilder::default();
    'restart: loop {
        if current.is_zero() {
            return Ok(None);
        }
        if current.degree() == Some(0) {
            meter.canonical_roots(&mut roots)?;
            return Ok(Some(roots));
        }
        let mut remaining = current.squarefree(meter)?;
        for endpoint in [Rat::zero(), Rat::one()] {
            if remaining.evaluate(&endpoint, meter)?.is_zero() {
                remaining = remaining
                    .divide(
                        &Poly::linear(meter.negate(&endpoint)?, Rat::one(), meter)?,
                        meter,
                    )?
                    .0;
                meter.push(&mut roots, endpoint)?;
            }
        }
        if remaining.degree().unwrap_or(0) == 0 {
            meter.canonical_roots(&mut roots)?;
            return Ok(Some(roots));
        }
        let sequence = sturm(&remaining, meter)?;
        // The integer-normalized leading coefficient bounds every rational root's
        // denominator; distinct such rationals are at least 1/lead^2 apart.
        let mut denominators = Rat::one();
        for value in &remaining.0 {
            let denominator = meter.run(Linear, &[value], |values| {
                values
                    .integer_absolute(value.denominator().as_ref())
                    .map(Rat::from_int)
            })?;
            let gcd = meter.run(
                ExactOperation::RationalMultiply,
                &[&denominators, &denominator],
                |values| {
                    Ok(Rat::from_int(values.integer_gcd(
                        denominators.numerator().as_ref(),
                        value.denominator().as_ref(),
                    )?))
                },
            )?;
            let product = meter.mul(&denominators, &denominator)?;
            denominators = meter.div(&product, &gcd)?;
        }
        let lead = meter.mul(&remaining.leading(), &denominators)?;
        let lead = meter.absolute(&lead)?;
        let square = meter.mul(&lead, &lead)?;
        let doubled = meter.mul(&square, &Rat::from_i64(2))?;
        let separation = meter.div(&Rat::one(), &doubled)?;
        let two = Rat::from_i64(2);
        let mut pending = meter.buffer(1)?;
        pending.push_reserved((Rat::zero(), Rat::one()));
        while let Some((low, high)) = pending.pop() {
            let count = sign_changes(&sequence, &low, meter)?
                .checked_sub(sign_changes(&sequence, &high, meter)?)
                .ok_or(GeoError::ArithmeticOverflow(
                    "reversed Sturm variation count",
                ))?;
            if count == 0 {
                continue;
            }
            let sum = meter.add(&low, &high)?;
            let middle = meter.div(&sum, &two)?;
            let width = meter.sub(&high, &low)?;
            if count > 1 || meter.compare(&width, &separation)?.is_gt() {
                if remaining.evaluate(&middle, meter)?.is_zero() {
                    // Deflate the exact root and restart on what remains.
                    let reduced = remaining
                        .divide(
                            &Poly::linear(meter.negate(&middle)?, Rat::one(), meter)?,
                            meter,
                        )?
                        .0;
                    meter.push(&mut roots, middle)?;
                    current = reduced;
                    continue 'restart;
                }
                meter.push(&mut pending, (low, middle.clone()))?;
                meter.push(&mut pending, (middle, high))?;
                continue;
            }
            // Exactly one root in (low, high], within the rational separation.
            let candidate = simplest_rational(&low, &high, meter)?;
            if *candidate.denominator() > *lead.numerator()
                || !remaining.evaluate(&candidate, meter)?.is_zero()
            {
                return Ok(None);
            }
            meter.push(&mut roots, candidate)?;
        }
        meter.canonical_roots(&mut roots)?;
        return Ok(Some(roots));
    }
}

/// The exact image polynomials `(X(t), Y(t))` of one affine original edge.
fn image(
    curve: &OperationImageCurve,
    polynomial: &Polynomial2d,
    swapped: bool,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Option<[Poly; 2]>, GeoError> {
    let (start, end) = curve.source_endpoints();
    if start.z().is_some() || end.z().is_some() {
        return Ok(None);
    }
    let mut start = [start.x(), start.y()];
    let mut end = [end.x(), end.y()];
    if swapped {
        start.reverse();
        end.reverse();
    }
    let mut normalized = [Poly::default(), Poly::default()];
    for (axis, slot) in normalized.iter_mut().enumerate() {
        let scale = &polynomial.scale()[axis];
        let offset = meter.sub(start[axis], &polynomial.origin()[axis])?;
        let delta = meter.sub(end[axis], start[axis])?;
        *slot = Poly::linear(meter.div(&offset, scale)?, meter.div(&delta, scale)?, meter)?;
    }
    let mut output = [Poly::default(), Poly::default()];
    for term in polynomial.terms() {
        let x = normalized[0].power(term.x_power, meter)?;
        let y = normalized[1].power(term.y_power, meter)?;
        let monomial = x.mul(&y, meter)?;
        output[0] = output[0].add(&monomial.scale(&term.x_coefficient, meter)?, meter)?;
        output[1] = output[1].add(&monomial.scale(&term.y_coefficient, meter)?, meter)?;
    }
    Ok(Some(output))
}

/// The determinant of an exact rational matrix by Gaussian elimination.
fn determinant(
    mut matrix: RetainedBuilder<RetainedBuilder<Rat>>,
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Rat, GeoError> {
    let size = matrix.len();
    let mut sign = Rat::one();
    for column in 0..size {
        let Some(pivot) = (column..size).find(|&row| !matrix[row][column].is_zero()) else {
            return Ok(Rat::zero());
        };
        if pivot != column {
            matrix.swap(pivot, column);
            sign = meter.negate(&sign)?;
        }
        for row in column + 1..size {
            if matrix[row][column].is_zero() {
                continue;
            }
            let factor = meter.div(&matrix[row][column], &matrix[column][column])?;
            for index in column..size {
                let product = meter.mul(&matrix[column][index], &factor)?;
                matrix[row][index] = meter.sub(&matrix[row][index], &product)?;
            }
        }
    }
    let mut product = sign;
    for (index, row) in matrix.iter().enumerate() {
        product = meter.mul(&product, &row[index])?;
    }
    Ok(product)
}

/// The Sylvester resultant of two nonzero polynomials.
fn resultant(f: &Poly, g: &Poly, meter: &mut Meter<'_, '_, '_>) -> Result<Rat, GeoError> {
    let (m, n) = (f.degree().unwrap_or(0), g.degree().unwrap_or(0));
    let size = m
        .checked_add(n)
        .ok_or(GeoError::ArithmeticOverflow(ADMISSION))?;
    if size == 0 {
        return Ok(Rat::one());
    }
    let mut matrix = meter.buffer(size)?;
    for _ in 0..size {
        matrix.push_reserved(meter.zeros(size)?);
    }
    for row in 0..n {
        for (offset, power) in (0..=m).rev().enumerate() {
            matrix[row][row + offset] = f.coefficient(power);
        }
    }
    for row in 0..m {
        for (offset, power) in (0..=n).rev().enumerate() {
            matrix[n + row][row + offset] = g.coefficient(power);
        }
    }
    determinant(matrix, meter)
}

/// `Res_u(A(t) - B(u), C(t) - D(u))` as a polynomial in `t`, by exact
/// evaluation at integer abscissae and Newton interpolation. The leading
/// `u` coefficients are the constants of `B` and `D`, so specialization
/// commutes with the resultant.
fn eliminate(
    left: &[Poly; 2],
    right: &[Poly; 2],
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Poly, GeoError> {
    let (m, n) = (
        right[0].degree().unwrap_or(0),
        right[1].degree().unwrap_or(0),
    );
    let degree = n
        .checked_mul(left[0].degree().unwrap_or(0))
        .and_then(|degree| {
            m.checked_mul(left[1].degree().unwrap_or(0))
                .and_then(|other| degree.checked_add(other))
        })
        .ok_or(GeoError::ArithmeticOverflow(ADMISSION))?;
    let length = degree
        .checked_add(1)
        .ok_or(GeoError::ArithmeticOverflow(ADMISSION))?;
    let mut abscissae = meter.buffer(length)?;
    for index in 0..=degree {
        let index = i64::try_from(index).map_err(|_| GeoError::ArithmeticOverflow(ADMISSION))?;
        abscissae.push_reserved(Rat::from_i64(index));
    }
    let mut coefficients = meter.buffer(abscissae.len())?;
    for t in &abscissae {
        let f = Poly::constant(left[0].evaluate(t, meter)?, meter)?.sub(&right[0], meter)?;
        let g = Poly::constant(left[1].evaluate(t, meter)?, meter)?.sub(&right[1], meter)?;
        coefficients.push_reserved(resultant(&f, &g, meter)?);
    }
    // Newton divided differences, then expansion into monomials.
    for level in 1..coefficients.len() {
        for index in (level..coefficients.len()).rev() {
            let difference = meter.sub(&coefficients[index], &coefficients[index - 1])?;
            let span = meter.sub(&abscissae[index], &abscissae[index - level])?;
            coefficients[index] = meter.div(&difference, &span)?;
        }
    }
    let mut result = Poly::default();
    for (index, coefficient) in coefficients.iter().enumerate().rev() {
        result = result
            .mul(
                &Poly::linear(meter.negate(&abscissae[index])?, Rat::one(), meter)?,
                meter,
            )?
            .add(&Poly::constant(coefficient.clone(), meter)?, meter)?;
    }
    Ok(result)
}

/// Contacts of two images on one shared axis line: one coordinate is the same
/// constant on both and the other is affine and nonconstant in each parameter.
/// The contact set is the exact intersection of the two coordinate ranges.
fn shared_line(
    left: &[Poly; 2],
    right: &[Poly; 2],
    meter: &mut Meter<'_, '_, '_>,
) -> Result<Option<ContactList>, GeoError> {
    let Some(axis) = (0..2).find(|&axis| {
        left[axis].degree().unwrap_or(0) == 0
            && right[axis].degree().unwrap_or(0) == 0
            && left[axis] == right[axis]
    }) else {
        return Ok(None);
    };
    let other = 1 - axis;
    let (a, b) = (&left[other], &right[other]);
    if a.degree() != Some(1) || b.degree() != Some(1) {
        return Ok(None);
    }
    let range = |image: &Poly, meter: &mut Meter<'_, '_, '_>| -> Result<(Rat, Rat), GeoError> {
        let start = image.coefficient(0);
        let end = meter.add(&start, &image.coefficient(1))?;
        Ok(if meter.compare(&start, &end)?.is_le() {
            (start, end)
        } else {
            (end, start)
        })
    };
    let (left_low, left_high) = range(a, meter)?;
    let (right_low, right_high) = range(b, meter)?;
    let low = if meter.compare(&left_low, &right_low)?.is_ge() {
        left_low
    } else {
        right_low
    };
    let high = if meter.compare(&left_high, &right_high)?.is_le() {
        left_high
    } else {
        right_high
    };
    let order = meter.compare(&low, &high)?;
    if order.is_gt() {
        return Ok(Some(ContactList::default()));
    }
    // Parameter of an exact coordinate value on one affine image.
    let at = |image: &Poly, value: &Rat, meter: &mut Meter<'_, '_, '_>| -> Result<Rat, GeoError> {
        let offset = meter.sub(value, &image.coefficient(0))?;
        meter.div(&offset, &image.coefficient(1))
    };
    let endpoint = |value: &Rat| value.is_zero() || *value == Rat::one();
    if order.is_eq() {
        let (t, u) = (at(a, &low, meter)?, at(b, &low, meter)?);
        return meter
            .contacts([if endpoint(&t) && endpoint(&u) {
                CurveIntersection::SymbolicEndpoint { left: t, right: u }
            } else {
                CurveIntersection::SymbolicSourceContact { left: t, right: u }
            }])
            .map(Some);
    }
    let mut first = (at(a, &low, meter)?, at(b, &low, meter)?);
    let mut second = (at(a, &high, meter)?, at(b, &high, meter)?);
    if meter.compare(&first.0, &second.0)?.is_gt() {
        core::mem::swap(&mut first, &mut second);
    }
    meter
        .contacts([CurveIntersection::BranchOverlap {
            left: (first.0, second.0),
            right: (first.1, second.1),
        }])
        .map(Some)
}

/// Whether every operation after the polynomial is injective on these images:
/// an inverse Mercator whose longitude stays strictly inside its square.
fn injective_tail(
    curve: &OperationImageCurve,
    images: &[&[Poly; 2]],
    meter: &mut Meter<'_, '_, '_>,
) -> Result<bool, GeoError> {
    let operations = curve.chain().operations();
    if operations.len() != 2 {
        return Ok(false);
    }
    let OperationModel::MercatorToGeographic { radius, .. } = operations[1].model() else {
        return Ok(false);
    };
    // |X(t)| <= sum |coefficients| on [0,1]; pi > 3 bounds the seam strictly.
    let seam = meter.mul(radius, &Rat::from_i64(3))?;
    for image in images {
        let mut bound = Rat::zero();
        for value in &image[0].0 {
            let absolute = meter.absolute(value)?;
            bound = meter.add(&bound, &absolute)?;
        }
        if meter.compare(&bound, &seam)?.is_ge() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn contact(t: Rat, u: Rat) -> CurveIntersection {
    let endpoint = |value: &Rat| value.is_zero() || *value == Rat::one();
    if endpoint(&t) && endpoint(&u) {
        CurveIntersection::SymbolicEndpoint { left: t, right: u }
    } else {
        CurveIntersection::SymbolicSourceContact { left: t, right: u }
    }
}

/// The complete exact contact set, or `None` when the certificate does not apply.
pub(super) fn intersections(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ContactList>, GeoError> {
    context.with_metadata(progress, |context, progress, retained| {
        let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right) else {
            return Ok(None);
        };
        if !a.same_mapping_admitted(b, context, progress)? {
            return Ok(None);
        }
        let operation = &a.chain().operations()[0];
        let OperationModel::Polynomial2d(polynomial) = operation.model() else {
            return Ok(None);
        };
        let swapped = operation.source().swapped_axes;
        let meter = &mut Meter {
            context,
            progress,
            retained,
        };
        let (Some(left_image), Some(right_image)) = (
            image(a, polynomial, swapped, meter)?,
            image(b, polynomial, swapped, meter)?,
        ) else {
            return Ok(None);
        };
        if !injective_tail(a, &[&left_image, &right_image], meter)? {
            return Ok(None);
        }
        // A constant image is a point; its own handler owns it.
        let constant = |image: &[Poly; 2]| image.iter().all(|axis| axis.degree().unwrap_or(0) == 0);
        if constant(&left_image) || constant(&right_image) {
            return Ok(None);
        }
        let eliminated = eliminate(&left_image, &right_image, meter)?;
        if eliminated.is_zero() {
            // A shared image component. Its affine shared-line case is exact;
            // any other shared component belongs to the overlap handlers.
            return shared_line(&left_image, &right_image, meter);
        }
        let Some(parameters) = unit_rational_roots(&eliminated, meter)? else {
            return Ok(None);
        };
        let mut contacts = ContactList::default();
        for t in parameters {
            let first = right_image[0].sub(
                &Poly::constant(left_image[0].evaluate(&t, meter)?, meter)?,
                meter,
            )?;
            let second = right_image[1].sub(
                &Poly::constant(left_image[1].evaluate(&t, meter)?, meter)?,
                meter,
            )?;
            let common = first.gcd(&second, meter)?;
            if common.is_zero() {
                return Ok(None);
            }
            let Some(matches) = unit_rational_roots(&common, meter)? else {
                return Ok(None);
            };
            for u in matches {
                let left = meter.run(Linear, &[&t], |values| values.original(&t))?;
                let value = contact(left, u);
                crate::numerical::ExactAdmission::new(meter.context, meter.progress)
                    .push_detached(&mut contacts, value, meter.retained)?;
            }
        }
        Ok(Some(contacts))
    })
}

#[cfg(test)]
mod tests {
    use super::{Meter, Poly, eliminate, simplest_rational, unit_rational_roots};
    use crate::context::WorkProgress;
    use crate::{Int, MetricContext, Rat};

    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).unwrap()
    }

    #[test]
    fn exact_unit_roots_keep_multiplicity_out_and_refuse_irrational_roots() {
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        let mut retained = 0;
        let meter = &mut Meter {
            context: &mut context,
            progress: &mut progress,
            retained: &mut retained,
        };
        let third = Rat::new(Int::one(), Int::from_i64(3)).unwrap();
        // (t - 1/3)^2 (t - 3/4): a double root and a simple root.
        let factor = Poly::linear(third.neg(), Rat::one(), meter).unwrap();
        let product = factor
            .mul(&factor, meter)
            .unwrap()
            .mul(
                &Poly::linear(rat("-0.75"), Rat::one(), meter).unwrap(),
                meter,
            )
            .unwrap();
        assert_eq!(
            unit_rational_roots(&product, meter).unwrap().unwrap(),
            vec![third.clone(), rat("0.75")]
        );
        // t^2 - 1/2 has the irrational root 1/sqrt(2) in [0,1].
        let irrational =
            Poly::from_values([rat("-0.5"), Rat::zero(), Rat::one()].into_iter(), meter).unwrap();
        assert_eq!(unit_rational_roots(&irrational, meter).unwrap(), None);
        // An endpoint root and a root outside the unit interval.
        let outside = Poly::linear(Rat::zero(), Rat::one(), meter)
            .unwrap()
            .mul(&Poly::linear(rat("-2"), Rat::one(), meter).unwrap(), meter)
            .unwrap();
        assert_eq!(
            unit_rational_roots(&outside, meter).unwrap().unwrap(),
            vec![Rat::zero()]
        );
        assert_eq!(
            simplest_rational(&rat("0.3"), &rat("0.35"), meter).unwrap(),
            third
        );
    }

    #[test]
    fn the_fold_eliminant_isolates_its_degenerate_endpoint_contact() {
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        let mut retained = 0;
        let meter = &mut Meter {
            context: &mut context,
            progress: &mut progress,
            retained: &mut retained,
        };
        // Left image (0, -0.1 - 0.1 t); right image ((0.1 u)^2, -0.2): they meet
        // only at t = 1, u = 0, where the right image has zero speed.
        let left = [
            Poly::default(),
            Poly::linear(rat("-0.1"), rat("-0.1"), meter).unwrap(),
        ];
        let right = [
            Poly::from_values([Rat::zero(), Rat::zero(), rat("0.01")].into_iter(), meter).unwrap(),
            Poly::constant(rat("-0.2"), meter).unwrap(),
        ];
        let eliminated = eliminate(&left, &right, meter).unwrap();
        assert_eq!(
            unit_rational_roots(&eliminated, meter).unwrap().unwrap(),
            vec![Rat::one()]
        );
    }
}
