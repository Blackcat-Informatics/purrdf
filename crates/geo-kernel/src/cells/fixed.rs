// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact-coordinate bridge and the frozen Q96/Q62 trigonometric pipeline.

use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::wide::mul_div_round_even;

use crate::error::GeoError;
use crate::exact::{Int, Rat};
use crate::geographic::LonLat;

use super::constants::{COSINE, PI96, Q62, Q96, SINE};

pub(super) trait IntegerAdmission {
    const METERED: bool;
    fn bounded(&mut self, count: u64) -> Result<(), GeoError>;
    fn exact<T>(
        &mut self,
        cost: ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError>;
}
pub(super) struct PureAssignment;
impl IntegerAdmission for PureAssignment {
    const METERED: bool = false;
    fn bounded(&mut self, _: u64) -> Result<(), GeoError> {
        Ok(())
    }
    fn exact<T>(
        &mut self,
        _: ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        evaluate()
    }
}
pub(super) fn integer<T, A: IntegerAdmission>(
    admission: &mut A,
    operation: ExactOperation,
    operands: &[&Int],
    count: u64,
    extra_bits: u64,
    evaluate: impl FnOnce() -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    if !A::METERED {
        return evaluate();
    }
    let bits = operands
        .iter()
        .fold(1, |bits, operand| bits.max(operand.bit_len()));
    let bits = bits
        .checked_add(extra_bits)
        .ok_or(GeoError::ArithmeticOverflow(
            "fixed assignment admission width",
        ))?;
    let cost = ExactArithmeticCost::for_operation(operation, bits, count).ok_or(
        GeoError::ArithmeticOverflow("fixed assignment admission cost"),
    )?;
    admission.exact(cost, evaluate)
}
pub(super) fn rational<T, A: IntegerAdmission>(
    admission: &mut A,
    operation: ExactOperation,
    operands: &[&Rat],
    count: u64,
    evaluate: impl FnOnce() -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    if !A::METERED {
        return evaluate();
    }
    let cost = crate::numerical::rational_cost(operation, operands, count).ok_or(
        GeoError::ArithmeticOverflow("integer rational admission cost"),
    )?;
    admission.exact(cost, evaluate)
}
fn round_ratio(
    numerator: &Int,
    denominator: &Int,
    fractional_bits: u32,
    admission: &mut impl IntegerAdmission,
) -> Result<i128, GeoError> {
    let scaled = integer(
        admission,
        ExactOperation::Linear,
        &[numerator],
        2,
        u64::from(fractional_bits),
        || Ok(numerator.abs().shl(fractional_bits)),
    )?;
    let (quotient, remainder) = integer(
        admission,
        ExactOperation::Divide,
        &[&scaled, denominator],
        1,
        0,
        || {
            scaled
                .div_rem(denominator)
                .ok_or(GeoError::ArithmeticOverflow("fixed-point zero denominator"))
        },
    )?;
    let quotient = quotient.to_i128().ok_or(GeoError::ArithmeticOverflow(
        "fixed-point coordinate conversion",
    ))?;
    let comparison = integer(
        admission,
        ExactOperation::Linear,
        &[&remainder, denominator],
        2,
        1,
        || Ok(remainder.shl(1).cmp(denominator)),
    )?;
    let increment = comparison.is_gt() || (comparison.is_eq() && quotient & 1 == 1);
    let rounded =
        quotient
            .checked_add(i128::from(increment))
            .ok_or(GeoError::ArithmeticOverflow(
                "fixed-point coordinate rounding",
            ))?;
    Ok(if numerator.is_negative() {
        -rounded
    } else {
        rounded
    })
}

fn mul_round(a: i128, b: i128, scale: u128) -> Result<i128, GeoError> {
    let magnitude = mul_div_round_even(a.unsigned_abs(), b.unsigned_abs(), scale)
        .and_then(|value| i128::try_from(value).ok())
        .ok_or(GeoError::ArithmeticOverflow("fixed-point multiplication"))?;
    Ok(if (a < 0) != (b < 0) {
        -magnitude
    } else {
        magnitude
    })
}

fn horner(argument_squared: i128, coefficients: &[i128; 10]) -> Result<i128, GeoError> {
    let mut value = coefficients[9];
    for coefficient in coefficients[..9].iter().rev() {
        value = coefficient
            .checked_add(mul_round(argument_squared, value, Q96)?)
            .ok_or(GeoError::ArithmeticOverflow("fixed-point Horner addition"))?;
    }
    Ok(value)
}

fn sin_cos(degrees: &Rat, admission: &mut impl IntegerAdmission) -> Result<(i128, i128), GeoError> {
    // Quarter turns are selected in the exact degree space, before rounding.
    let ninety = Int::from_i64(90);
    let quarter_denominator = integer(
        admission,
        ExactOperation::Multiply,
        &[degrees.denominator(), &ninety],
        1,
        0,
        || Ok(degrees.denominator().mul(&ninety)),
    )?;
    let quarter = round_ratio(degrees.numerator(), &quarter_denominator, 0, admission)?;
    let quarter_integer = Int::from_i128(quarter);
    let whole = integer(
        admission,
        ExactOperation::Multiply,
        &[&quarter_denominator, &quarter_integer],
        1,
        0,
        || Ok(quarter_denominator.mul(&quarter_integer)),
    )?;
    let remainder = integer(
        admission,
        ExactOperation::Linear,
        &[degrees.numerator(), &whole],
        1,
        0,
        || Ok(degrees.numerator().sub(&whole)),
    )?;
    let magnitude = integer(
        admission,
        ExactOperation::Linear,
        &[&remainder],
        1,
        0,
        || Ok(remainder.abs()),
    )?;
    let degree96 = round_ratio(&magnitude, degrees.denominator(), 96, admission)?;
    let angle = mul_div_round_even(degree96.unsigned_abs(), PI96, 180 * Q96)
        .and_then(|value| i128::try_from(value).ok())
        .ok_or(GeoError::ArithmeticOverflow(
            "fixed-point angular conversion",
        ))?;
    let square = mul_round(angle, angle, Q96)?;
    let sine = mul_round(angle, horner(square, &SINE)?, Q96)?;
    let sine = if remainder.is_negative() { -sine } else { sine };
    let cosine = horner(square, &COSINE)?;
    let (sine, cosine) = match quarter.rem_euclid(4) {
        0 => (sine, cosine),
        1 => (cosine, -sine),
        2 => (-sine, -cosine),
        _ => (-cosine, sine),
    };
    Ok((mul_round(sine, 1, 1 << 34)?, mul_round(cosine, 1, 1 << 34)?))
}

#[cfg(test)]
pub(super) fn normal(point: &LonLat) -> Result<[i128; 3], GeoError> {
    normal_admitted(point, &mut PureAssignment)
}
pub(super) fn normal_admitted(
    point: &LonLat,
    admission: &mut impl IntegerAdmission,
) -> Result<[i128; 3], GeoError> {
    let pole = integer(
        admission,
        ExactOperation::Linear,
        &[point.latitude().numerator(), point.latitude().denominator()],
        2,
        0,
        || {
            Ok(if point.latitude() == &Rat::from_i64(90) {
                1
            } else if point.latitude() == &Rat::from_i64(-90) {
                -1
            } else {
                0
            })
        },
    )?;
    if pole == 1 {
        return Ok([0, 0, Q62]);
    }
    if pole == -1 {
        return Ok([0, 0, -Q62]);
    }
    let (sin_lon, cos_lon) = sin_cos(point.longitude(), admission)?;
    let (sin_lat, cos_lat) = sin_cos(point.latitude(), admission)?;
    let scale =
        u128::try_from(Q62).map_err(|_| GeoError::ArithmeticOverflow("Q62 scale conversion"))?;
    Ok([
        mul_round(cos_lat, cos_lon, scale)?,
        mul_round(cos_lat, sin_lon, scale)?,
        sin_lat,
    ])
}
