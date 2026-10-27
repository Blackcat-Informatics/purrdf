// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The XSD numeric value space: `integer`, `decimal`, `float`, `double`, their
//! lexical↔value parsing + canonical mapping, and the SPARQL numeric promotion
//! lattice (`integer ⊂ decimal ⊂ float ⊂ double`) used for cross-type comparison.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact::DivisionPolicy;
use crate::ieee;
use crate::value::{XsdError, XsdValue};

pub(crate) mod exact_path;

pub use exact_path::CostOp;

/// The cost of one numeric operation over `a` and `b`, for a governor to charge
/// before the operation runs, computed in constant time from bounded words or
/// the operands' sizes:
///
/// * two `xsd:integer`/`xsd:decimal` operands run on the exact tower when one
///   exceeds the inline representation, or when their inline result cannot be
///   represented exactly; the cost follows
///   [`crate::exact::cost`];
/// * every division of two exact numeric operands runs on the tower under its
///   caller's [`DivisionPolicy`] and costs the same;
/// * one such big operand meeting an `xsd:float`/`xsd:double` costs its correctly
///   rounded conversion or its exact comparison with the binary value;
/// * duration and calendar operators charge their exact seconds copy, alignment,
///   day split or instant difference when those seconds leave inline storage;
/// * everything else computes in machine words: [`crate::exact::Cost::ZERO`].
///
/// The cost covers the operation, not the rendering of its result: a caller that
/// writes the result as a lexical form charges [`numeric_render_cost`] of it too,
/// which is where a product's growing scale is paid for.
#[must_use]
#[inline]
pub fn numeric_cost(a: &XsdValue, b: &XsdValue, op: CostOp) -> crate::exact::Cost {
    if let Some(cost) = duration_arithmetic_cost(a, b, op) {
        return cost;
    }
    if let Some(cost) = calendar_arithmetic_cost(a, b, op) {
        return cost;
    }
    if let (XsdValue::Duration(left), XsdValue::Duration(right)) = (a, b)
        && matches!(op, CostOp::Add)
    {
        let x = left.seconds_ref();
        let y = right.seconds_ref();
        let bounded_spill = match (x.to_bounded(), y.to_bounded()) {
            (Some(x), Some(y)) => align_decimals(&x, &y)
                .is_none_or(|(x, y, _)| x.checked_add(y).is_none() || x.checked_sub(y).is_none()),
            _ => true,
        };
        if bounded_spill
            || x.unscaled().as_i128().is_none()
            || y.unscaled().as_i128().is_none()
            || x.scale() > 18
            || y.scale() > 18
        {
            // Both duration addition and subtraction perform one aligned
            // exact-seconds operation; their bounded month step is constant.
            return x.add_cost(y);
        }
    }
    let tower_division = matches!(op, CostOp::Div(_));
    let has_big_operand = is_big(a) || is_big(b);
    // A bounded pair can still overflow into the tower. Classify it before
    // returning zero, so the caller admits the fallback computation.
    let bounded_fallback = matches!(op, CostOp::Add | CostOp::Mul)
        && !has_big_operand
        && bounded_needs_tower(a, b, op);
    if !tower_division && !has_big_operand && !bounded_fallback {
        return crate::exact::Cost::ZERO;
    }
    tower_cost(a, b, op, tower_division || bounded_fallback)
}

/// Price the exact seconds computation performed by the temporal dispatch.
/// Calendar fields and months remain bounded words; the only tower traffic is
/// the borrowed seconds, the duration's seconds, and their staged day split.
fn calendar_arithmetic_cost(a: &XsdValue, b: &XsdValue, op: CostOp) -> Option<crate::exact::Cost> {
    use crate::exact::{
        Cost,
        cost::{self, Shape},
    };
    if !matches!(op, CostOp::Add) {
        return None;
    }
    let wide = |seconds: &crate::exact::Decimal| {
        seconds.unscaled().as_i128().is_none() || seconds.scale() > 18
    };
    let ((XsdValue::Duration(duration), calendar) | (calendar, XsdValue::Duration(duration))) =
        (a, b)
    else {
        return None;
    };
    let seconds = duration.seconds_ref();
    let base_seconds = match calendar {
        XsdValue::DateTime(value) => {
            i128::from(value.hour()) * 3600 + i128::from(value.minute()) * 60
        }
        XsdValue::Time(value) => i128::from(value.hour()) * 3600 + i128::from(value.minute()) * 60,
        _ => 0,
    };
    let source = match calendar {
        XsdValue::DateTime(value) => Some(value.second_ref()),
        XsdValue::Time(value) if duration.months() == 0 => Some(value.second_ref()),
        XsdValue::Date(_) | XsdValue::Gregorian(_) => None,
        _ => return None,
    };
    if matches!(calendar, XsdValue::Gregorian(_)) && !seconds.is_integer() {
        return Some(Cost::ZERO);
    }
    let source_wide = source.is_some_and(wide);
    let bounded_spill = match (
        source.map(crate::exact::Decimal::to_bounded),
        seconds.to_bounded(),
    ) {
        (Some(Some(source)), Some(delta)) => {
            let base = Decimal::from_integer(base_seconds);
            align_decimals(&base, &source)
                .and_then(|(x, y, scale)| {
                    x.checked_add(y).map(|sum| Decimal::from_parts(sum, scale))
                })
                .and_then(|with_second| align_decimals(&with_second, &delta))
                .is_none_or(|(x, y, _)| x.checked_add(y).is_none() || x.checked_sub(y).is_none())
        }
        (None, Some(delta)) => {
            let base = Decimal::from_integer(base_seconds);
            align_decimals(&base, &delta)
                .is_none_or(|(x, y, _)| x.checked_add(y).is_none() || x.checked_sub(y).is_none())
        }
        _ => true,
    };
    if !source_wide && !wide(seconds) && !bounded_spill {
        return Some(Cost::ZERO);
    }
    let source_shape = source.map_or(Shape::of_i128(0, 0), crate::exact::Decimal::shape);
    let seconds_shape = seconds.shape();
    let base = Shape::of_i128(86_400, 0);
    let with_second = base.sum(source_shape);
    let total = with_second.sum(seconds_shape);
    let arithmetic = cost::decimal_add(base, source_shape)
        .saturating_add(cost::decimal_add(with_second, seconds_shape));
    // The day split truncates, subtracts its fraction, divides the whole
    // seconds by 86,400, then constructs the seconds within the minute.
    let split = cost::decimal_unary(total)
        .saturating_add(cost::decimal_add(total, total))
        .saturating_add(cost::div(total.limbs(), 1))
        .saturating_add(cost::decimal_add(total, base));
    Some(arithmetic.saturating_add(split))
}

/// Subtraction's exact-work charge. Same-sort calendar pairs subtract their
/// instants; addition of those pairs is a type error and must not be charged
/// ahead of that diagnostic. Other subtraction pairs share addition's cost.
#[must_use]
pub fn temporal_subtract_cost(a: &XsdValue, b: &XsdValue) -> crate::exact::Cost {
    if matches!(a, XsdValue::Duration(_))
        && matches!(
            b,
            XsdValue::DateTime(_) | XsdValue::Date(_) | XsdValue::Time(_) | XsdValue::Gregorian(_)
        )
    {
        return crate::exact::Cost::ZERO;
    }
    instant_difference_cost(a, b).unwrap_or_else(|| numeric_cost(a, b, CostOp::Add))
}

fn instant_difference_cost(a: &XsdValue, b: &XsdValue) -> Option<crate::exact::Cost> {
    use crate::exact::{
        Cost,
        cost::{self, Shape},
    };
    let (left, right) = match (a, b) {
        (XsdValue::DateTime(left), XsdValue::DateTime(right)) => {
            if left.timezone_minutes().is_some() != right.timezone_minutes().is_some() {
                return Some(Cost::ZERO);
            }
            (Some(left.second_ref()), Some(right.second_ref()))
        }
        (XsdValue::Time(left), XsdValue::Time(right)) => {
            if left.timezone_minutes().is_some() != right.timezone_minutes().is_some() {
                return Some(Cost::ZERO);
            }
            (Some(left.second_ref()), Some(right.second_ref()))
        }
        (XsdValue::Date(_), XsdValue::Date(_)) => (None, None),
        _ => return None,
    };
    let wide = |seconds: &crate::exact::Decimal| {
        seconds.unscaled().as_i128().is_none() || seconds.scale() > 18
    };
    if !left.is_some_and(wide) && !right.is_some_and(wide) {
        return Some(Cost::ZERO);
    }
    let base = Shape::of_i128(i128::MAX, 0);
    let left = left.map_or(Shape::of_i128(0, 0), crate::exact::Decimal::shape);
    let right = right.map_or(Shape::of_i128(0, 0), crate::exact::Decimal::shape);
    let left_total = base.sum(left);
    let right_total = base.sum(right);
    Some(
        cost::decimal_add(base, left)
            .saturating_add(cost::decimal_add(base, right))
            .saturating_add(cost::decimal_add(left_total, right_total)),
    )
}

/// Exact-work bound for the seconds copy and optional whole-minute shift made
/// by `fn:adjust-*-to-timezone`. Invalid offsets reach the XSD domain error
/// before any exact arithmetic; their work charge stays zero.
#[must_use]
pub fn timezone_adjust_cost(value: &XsdValue, timezone: Option<i64>) -> crate::exact::Cost {
    use crate::exact::{
        Cost,
        cost::{self, Shape},
    };
    if timezone.is_some_and(|seconds| seconds % 60 != 0 || !(-50_400..=50_400).contains(&seconds)) {
        return Cost::ZERO;
    }
    let (seconds, source_timezone) = match value {
        XsdValue::DateTime(value) => (value.second_ref(), value.timezone_minutes()),
        XsdValue::Time(value) => (value.second_ref(), value.timezone_minutes()),
        _ => return Cost::ZERO,
    };
    if seconds.unscaled().as_i128().is_some() && seconds.scale() <= 18 {
        return Cost::ZERO;
    }
    let shape = seconds.shape();
    // Retagging copies the coefficient once; no division or canonical text is
    // produced until the separate governed rendering step.
    let copy = Cost::new(
        shape.limbs().saturating_add(1),
        shape.limbs().saturating_add(1).saturating_mul(8),
    );
    if source_timezone.is_none() || timezone.is_none() {
        return copy;
    }
    let base = Shape::of_i128(86_400, 0);
    let delta = Shape::of_i128(100_800, 0);
    let with_second = base.sum(shape);
    let total = with_second.sum(delta);
    copy.saturating_add(cost::decimal_add(base, shape))
        .saturating_add(cost::decimal_add(with_second, delta))
        .saturating_add(cost::decimal_unary(total))
        .saturating_add(cost::decimal_add(total, total))
        .saturating_add(cost::div(total.limbs(), 1))
        .saturating_add(cost::decimal_add(total, base))
}

fn duration_arithmetic_cost(a: &XsdValue, b: &XsdValue, op: CostOp) -> Option<crate::exact::Cost> {
    use crate::exact::{
        Cost, DivisionPolicy,
        cost::{self, Shape},
    };
    match op {
        CostOp::Mul => {
            let ((XsdValue::Duration(duration), factor) | (factor, XsdValue::Duration(duration))) =
                (a, b)
            else {
                return None;
            };
            let factor_shape = Shape::of_value(factor)?;
            let factor_small = match factor {
                XsdValue::Integer { value, .. } => value.as_i128().map(|value| (value, 0)),
                XsdValue::Decimal(value) => value
                    .unscaled()
                    .as_i128()
                    .map(|coefficient| (coefficient, value.scale())),
                _ => None,
            };
            let seconds = duration.seconds_ref();
            if let (Some((factor, scale)), Some(coefficient)) =
                (factor_small, seconds.unscaled().as_i128())
                && scale.saturating_add(seconds.scale()) <= 18
                && i128::from(duration.months()).checked_mul(factor).is_some()
                && coefficient.checked_mul(factor).is_some()
            {
                return Some(Cost::ZERO);
            }
            let months_shape = Shape::of_i128(i128::from(duration.months()), 0);
            let month_cost = if duration.months() == 0 {
                Cost::ZERO
            } else {
                cost::decimal_mul(months_shape, factor_shape)
                    .saturating_add(cost::decimal_unary(months_shape.product(factor_shape)))
            };
            let seconds_cost = if seconds.is_zero() {
                Cost::ZERO
            } else {
                cost::decimal_mul(seconds.shape(), factor_shape)
            };
            Some(month_cost.saturating_add(seconds_cost))
        }
        CostOp::Div(_) => {
            let XsdValue::Duration(dividend) = a else {
                return None;
            };
            let policy = DivisionPolicy::xsd_default();
            let left_seconds = dividend.seconds_ref();
            let (numerator, denominator) = match b {
                XsdValue::Integer { .. } | XsdValue::Decimal(_) => {
                    let denominator = Shape::of_value(b)?;
                    let months = Shape::of_i128(i128::from(dividend.months()), 0);
                    let month_cost = if dividend.months() == 0 {
                        Cost::ZERO
                    } else {
                        cost::decimal_div(months, denominator, policy).saturating_add(
                            cost::decimal_unary(months.quotient(denominator, policy)),
                        )
                    };
                    let seconds_cost = if left_seconds.is_zero() {
                        Cost::ZERO
                    } else {
                        cost::decimal_div(left_seconds.shape(), denominator, policy)
                    };
                    return Some(month_cost.saturating_add(seconds_cost));
                }
                XsdValue::Duration(divisor)
                    if dividend.months() != 0
                        && left_seconds.is_zero()
                        && divisor.months() != 0
                        && divisor.seconds_ref().is_zero() =>
                {
                    (
                        Shape::of_i128(i128::from(dividend.months()), 0),
                        Shape::of_i128(i128::from(divisor.months()), 0),
                    )
                }
                XsdValue::Duration(divisor)
                    if dividend.months() == 0
                        && divisor.months() == 0
                        && !divisor.seconds_ref().is_zero() =>
                {
                    (left_seconds.shape(), divisor.seconds_ref().shape())
                }
                XsdValue::Duration(_) => return Some(Cost::ZERO),
                _ => return None,
            };
            Some(cost::decimal_div(numerator, denominator, policy))
        }
        _ => None,
    }
}

/// Whether bounded exact arithmetic can leave its machine-word representation.
/// `CostOp::Add` covers both addition and subtraction, so either overflow is
/// conservatively charged. A comparison of bounded exact values stays bounded.
fn bounded_needs_tower(a: &XsdValue, b: &XsdValue, op: CostOp) -> bool {
    if !a.is_exact_numeric() || !b.is_exact_numeric() {
        return false;
    }
    match (a, b, op) {
        (XsdValue::Integer { value: x, .. }, XsdValue::Integer { value: y, .. }, CostOp::Add) => {
            let x = x.as_i128().expect("bounded operands checked");
            let y = y.as_i128().expect("bounded operands checked");
            x.checked_add(y).is_none() || x.checked_sub(y).is_none()
        }
        (XsdValue::Integer { value: x, .. }, XsdValue::Integer { value: y, .. }, CostOp::Mul) => x
            .as_i128()
            .expect("bounded operands checked")
            .checked_mul(y.as_i128().expect("bounded operands checked"))
            .is_none(),
        (_, _, CostOp::Add) => {
            let x = bounded_decimal(a);
            let y = bounded_decimal(b);
            align_decimals(&x, &y)
                .is_none_or(|(x, y, _)| x.checked_add(y).is_none() || x.checked_sub(y).is_none())
        }
        (_, _, CostOp::Mul) => {
            let x = bounded_decimal(a);
            let y = bounded_decimal(b);
            u32::from(x.scale()) + u32::from(y.scale()) > u32::from(MAX_DECIMAL_SCALE)
                || x.mantissa().checked_mul(y.mantissa()).is_none()
        }
        (_, _, CostOp::Compare | CostOp::Div(_)) => false,
    }
}

fn bounded_decimal(value: &XsdValue) -> Decimal {
    match value {
        XsdValue::Integer { value, .. } => {
            Decimal::from_integer(value.as_i128().expect("bounded operands checked"))
        }
        XsdValue::Decimal(value) => value.to_bounded().expect("bounded operands checked"),
        _ => unreachable!("bounded exact operands were checked before dispatch"),
    }
}

/// [`numeric_cost`] once a big operand, a bounded overflow, or division runs on
/// the tower. Out of line, so the fast path above stays small.
#[cold]
#[inline(never)]
fn tower_cost(a: &XsdValue, b: &XsdValue, op: CostOp, force_exact: bool) -> crate::exact::Cost {
    match (exact_path::shape_of(a), exact_path::shape_of(b)) {
        (Some(x), Some(y)) if exact_path::involves_big(a, b) || force_exact => {
            let integer = |v: &XsdValue| matches!(v, XsdValue::Integer { .. });
            let integers = integer(a) && integer(b) && !matches!(op, CostOp::Div(_));
            exact_path::cost(x, y, integers, op)
        }
        (Some(x), None) if is_big(a) && matches!(b, XsdValue::Float(_) | XsdValue::Double(_)) => {
            exact_path::ieee_cost(x)
        }
        (None, Some(y)) if is_big(b) && matches!(a, XsdValue::Float(_) | XsdValue::Double(_)) => {
            exact_path::ieee_cost(y)
        }
        _ => crate::exact::Cost::ZERO,
    }
}

/// The cost of a unary numeric function over `value` — negation, `fn:abs`,
/// `fn:ceiling`, `fn:floor`, `fn:round` — when it runs on the exact tower (an
/// exact integer or decimal operand outside the inline representation); zero otherwise.
#[must_use]
pub fn numeric_unary_cost(value: &XsdValue) -> crate::exact::Cost {
    match value {
        XsdValue::Duration(duration) => {
            let seconds = duration.seconds_ref();
            if seconds.unscaled().as_i128().is_none() || seconds.scale() > 18 {
                seconds.unary_cost()
            } else {
                crate::exact::Cost::ZERO
            }
        }
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.unary_cost(),
        XsdValue::Decimal(decimal)
            if decimal.unscaled().as_i128().is_none() || decimal.scale() > 18 =>
        {
            decimal.unary_cost()
        }
        _ => crate::exact::Cost::ZERO,
    }
}

/// The cost of converting `value` to the nearest `xsd:double`/`xsd:float` — a cast,
/// or a promotion against an IEEE operand — when it is an
/// exact integer or decimal outside the inline representation; zero otherwise.
#[must_use]
pub fn numeric_to_float_cost(value: &XsdValue) -> crate::exact::Cost {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.to_float_cost(),
        XsdValue::Decimal(decimal)
            if decimal.unscaled().as_i128().is_none() || decimal.scale() > 18 =>
        {
            decimal.to_float_cost()
        }
        _ => crate::exact::Cost::ZERO,
    }
}

/// The cost of rendering `value`'s canonical lexical form
/// ([`XsdValue::canonical_lexical`]; [`computed_numeric_render_cost`] prices the
/// form an operation emits) when its exact payload exceeds the inline
/// representation: every byte of the text, the leading zeros of a long
/// fraction included, so a value whose coefficient is short but whose scale is
/// vast is priced at the text it becomes. Wide temporal seconds use the same
/// decimal groups; duration additionally scans them for day/time components.
/// Zero for bounded values, whose text is bounded by machine words.
#[must_use]
pub fn numeric_render_cost(value: &XsdValue) -> crate::exact::Cost {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => value.render_cost(),
        XsdValue::Decimal(decimal) => decimal_render_cost(decimal),
        XsdValue::DateTime(value) => decimal_render_cost(value.second_ref()),
        XsdValue::Time(value) => decimal_render_cost(value.second_ref()),
        XsdValue::Duration(value) => {
            let base = decimal_render_cost(value.seconds_ref());
            // One radix conversion plus up to three linear digit visits:
            // day summary, quotient emission, and fractional suffix.
            crate::exact::Cost::new(base.work().saturating_mul(4), base.bytes())
        }
        _ => crate::exact::Cost::ZERO,
    }
}

/// [`numeric_render_cost`] for the text an operation COMPUTES, which is what
/// [`prepare_computed_numeric_text`] emits: a wide whole decimal is spelled with
/// `.0`, two bytes past its canonical text, and is priced at those bytes. Every
/// other value is spelled as its canonical form.
#[must_use]
pub fn computed_numeric_render_cost(value: &XsdValue) -> crate::exact::Cost {
    match value {
        XsdValue::Decimal(decimal)
            if decimal.unscaled().as_i128().is_none() || decimal.scale() > 18 =>
        {
            decimal.computed_render_cost()
        }
        other => numeric_render_cost(other),
    }
}

/// The shared bounded-aware work charge for rendering a borrowed exact decimal.
/// Small values retain the original zero-cost scalar path; a wide coefficient or
/// scale pays for its canonical digits before rendering begins.
#[must_use]
pub fn decimal_render_cost(value: &crate::exact::Decimal) -> crate::exact::Cost {
    if value.unscaled().as_i128().is_none() || value.scale() > 18 {
        value.render_cost()
    } else {
        crate::exact::Cost::ZERO
    }
}

/// Whether an exact numeric value exceeds the machine-word fast path.
const fn is_big(value: &XsdValue) -> bool {
    match value {
        XsdValue::Integer { value, .. } => value.as_i128().is_none(),
        XsdValue::Decimal(value) => value.unscaled().as_i128().is_none() || value.scale() > 18,
        _ => false,
    }
}

/// An exact decimal: `value = mantissa × 10^(-scale)`, `i128`-backed (scale
/// bounded so the mantissa stays in `i128`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Decimal {
    mantissa: i128,
    scale: u8,
}

/// Max fractional digits we retain; keeps the mantissa within `i128` headroom.
const MAX_DECIMAL_SCALE: u8 = 18;

impl Decimal {
    /// Construct from raw mantissa + scale (internal/testing).
    #[must_use]
    pub(crate) fn from_parts(mantissa: i128, scale: u8) -> Self {
        Self { mantissa, scale }
    }

    /// The decimal whose value is exactly the integer `value` (scale 0). Every
    /// `i128` is representable, so the conversion never rounds and never fails —
    /// the `xs:integer` to `xs:decimal` cast of XPath F&O 3.1 §19.1.2.3.
    #[must_use]
    pub(crate) const fn from_integer(value: i128) -> Self {
        Self {
            mantissa: value,
            scale: 0,
        }
    }

    /// The decimal numerically closest to the binary64 `value` among those this
    /// type represents (at most 18 fractional digits), ties going to the one
    /// closer to zero — the `xs:float`/`xs:double` to `xs:decimal` cast of XPath
    /// F&O 3.1 §19.1.2.3. A float source widens to `f64` exactly first.
    ///
    /// Returns `None` for `NaN` and the infinities (which have no decimal value,
    /// `err:FOCA0002`) and for a magnitude of `2^127` or more (too large to be
    /// accommodated, `err:FOCA0001`). The conversion reads the binary value
    /// itself, never a decimal rendering of it: `0.1_f32` is
    /// `0.100000001490116119384765625`, so the result is `0.100000001490116119`.
    #[must_use]
    #[cfg(test)]
    fn from_f64_closest(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        let bits = value.to_bits();
        let negative = bits >> 63 == 1;
        let biased = i32::try_from((bits >> 52) & 0x7ff).ok()?;
        let fraction = bits & ((1_u64 << 52) - 1);
        // `|value| = significand × 2^exponent`, exactly.
        let (significand, exponent) = if biased == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1_u64 << 52), biased - 1075)
        };
        if significand == 0 {
            return Some(Self::from_integer(0));
        }
        let magnitude: u128 = if exponent >= 0 {
            // An integer: exact at scale 0 while it stays below 2^127.
            let width = 64 - significand.leading_zeros();
            if width + exponent.unsigned_abs() > 127 {
                return None;
            }
            u128::from(significand) << exponent.unsigned_abs()
        } else {
            // `significand × 10^18 / 2^shift`, rounded to the nearest integer with
            // ties toward zero. The product is below 2^53 × 10^18 < 2^113, so
            // it neither overflows nor survives a shift of 114 or more except
            // as a remainder short of one half.
            let shift = exponent.unsigned_abs();
            let product = u128::from(significand) * 10_u128.pow(u32::from(MAX_DECIMAL_SCALE));
            let rounded = if shift >= 114 {
                0
            } else {
                let quotient = product >> shift;
                let remainder = product & ((1_u128 << shift) - 1);
                quotient + u128::from(remainder > 1_u128 << (shift - 1))
            };
            let mut decimal = Self {
                mantissa: i128::try_from(rounded).ok()?,
                scale: MAX_DECIMAL_SCALE,
            };
            while decimal.scale > 0 && decimal.mantissa % 10 == 0 {
                decimal.mantissa /= 10;
                decimal.scale -= 1;
            }
            if negative {
                decimal.mantissa = -decimal.mantissa;
            }
            return Some(decimal);
        };
        let mantissa = i128::try_from(magnitude).ok()?;
        Some(Self::from_integer(if negative {
            -mantissa
        } else {
            mantissa
        }))
    }

    /// The mantissa (signed significant digits).
    #[must_use]
    pub(crate) fn mantissa(&self) -> i128 {
        self.mantissa
    }

    /// The scale (number of fractional digits).
    #[must_use]
    pub(crate) fn scale(&self) -> u8 {
        self.scale
    }

    /// The correctly rounded `f64` value (round-to-nearest, ties-to-even) — the
    /// conversion XPath `xs:double` casting (F&O §19.1.2) and SPARQL's
    /// `decimal ⊂ double` promotion require. Every decimal is finite and at most
    /// `2^127` in magnitude, so the result is always finite; zero is `+0.0`.
    ///
    /// The exact value `mantissa / 10^scale` is rounded ONCE (see
    /// `decimal_float`'s module docs for the algorithm and its cost). An earlier
    /// revision computed `mantissa as f64 / 10^scale`, rounding in the cast
    /// whenever `|mantissa| > 2^53` and again in the division, which could land
    /// one ulp away from the correctly rounded value.
    #[must_use]
    #[cfg(test)]
    fn to_f64(self) -> f64 {
        crate::decimal_float::decimal_to_f64(self.mantissa, self.scale)
    }

    /// The integer (truncated-toward-zero) part of the value.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn whole_part(&self) -> i128 {
        self.mantissa / 10i128.pow(u32::from(self.scale))
    }

    /// The fractional part of the value as a `Decimal` (same scale).
    #[must_use]
    #[cfg(test)]
    pub(crate) fn frac_part(&self) -> Self {
        Self {
            mantissa: self.mantissa % 10i128.pow(u32::from(self.scale)),
            scale: self.scale,
        }
    }

    /// Exact comparison of two decimals (total order — decimals are never NaN).
    ///
    /// ## Overflow-safety argument
    ///
    /// `scale` is a `u8` capped at `MAX_DECIMAL_SCALE` (= 18) at every construction
    /// site (`parse_decimal` enforces `frac_str.len() <= 18`; `frac_part` inherits the
    /// parent scale; `from_parts(_, 0)` for integer promotion is scale 0).
    ///
    /// For the fractional-alignment step the two frac mantissas satisfy:
    ///   `|frac_m| < 10^scale ≤ 10^18`
    /// After scaling to the common (higher) scale we multiply by at most `10^diff`
    /// where `diff ≤ 18`, giving a product `< 10^18 × 10^18 = 10^36`.
    /// `i128::MAX ≈ 1.7 × 10^38 > 10^36`, so the multiplication cannot overflow.
    ///
    /// The integer-part comparison uses `whole_part()` which returns `i128` and is
    /// exact (no multiplication); it is compared directly.
    ///
    /// There is NO `f64` path and NO `unwrap_or` swallowing a failure.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn cmp_exact(&self, other: &Self) -> Ordering {
        // Fast path: identical scale — single cmp, no arithmetic needed.
        if self.scale == other.scale {
            return self.mantissa.cmp(&other.mantissa);
        }

        // Step 1 — sign comparison.  Negative < zero < positive.
        let s_sign = self.mantissa.signum();
        let o_sign = other.mantissa.signum();
        if s_sign != o_sign {
            return s_sign.cmp(&o_sign);
        }
        // Both zero (mantissa == 0 regardless of scale) → Equal.
        if s_sign == 0 {
            return Ordering::Equal;
        }

        // Step 2 — integer part comparison (both same sign, non-zero).
        let s_whole = self.whole_part();
        let o_whole = other.whole_part();
        let whole_ord = s_whole.cmp(&o_whole);
        if whole_ord != Ordering::Equal {
            return whole_ord;
        }

        // Step 3 — fractional part comparison.
        // Each frac mantissa satisfies |frac_m| < 10^scale ≤ 10^18.
        // We scale the lower-scale fraction up to the higher scale by multiplying by
        // 10^diff (diff ≤ 18).  Product < 10^18 × 10^18 = 10^36 < i128::MAX → no
        // overflow.  (Debug assertion guards the invariant during development.)
        debug_assert!(
            self.scale <= MAX_DECIMAL_SCALE && other.scale <= MAX_DECIMAL_SCALE,
            "scale invariant violated: self.scale={}, other.scale={}",
            self.scale,
            other.scale,
        );
        let s_frac = self.frac_part().mantissa;
        let o_frac = other.frac_part().mantissa;
        let frac_ord = if self.scale > other.scale {
            let diff = u32::from(self.scale - other.scale);
            // SAFETY: o_frac < 10^other.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
            let o_scaled = o_frac * 10i128.pow(diff);
            s_frac.cmp(&o_scaled)
        } else {
            let diff = u32::from(other.scale - self.scale);
            // SAFETY: s_frac < 10^self.scale ≤ 10^18; diff ≤ 18; product < 10^36 < i128::MAX
            let s_scaled = s_frac * 10i128.pow(diff);
            s_scaled.cmp(&o_frac)
        };
        // For negative numbers the frac mantissas are negative too (they inherit the
        // sign from `mantissa % 10^scale`), so the direct comparison is already
        // correct: a more-negative fraction means a smaller (more negative) value.
        frac_ord
    }

    /// XSD 1.1 canonical lexical form (Part 2 §3.3.3.1, whose canonical
    /// representation is `decimalCanonicalMap`, defined in §E.1): an
    /// integer-valued decimal has NO decimal point (`3` → `"3"`, `-2` → `"-2"`,
    /// `0` → `"0"`); a non-integer decimal keeps its fractional part with trailing
    /// zeros trimmed (`2.50` → `"2.5"`, `-0.250` → `"-0.25"`).
    #[must_use]
    #[cfg(test)]
    pub(crate) fn canonical_lexical(&self) -> String {
        self.to_string()
    }
}

impl core::fmt::Display for Decimal {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // A bounded coefficient has at most two binary limbs and three base-ten
        // groups: the native Mag inline capacity covers every preparation buffer.
        crate::exact::Decimal::from_bounded(self)
            .prepare_text()
            .map_err(|_| core::fmt::Error)?
            .write_fmt(out)
    }
}

/// Whether `lexical` is in the `xsd:integer` lexical space (XSD 1.1 Part 2
/// §3.4.13.1): an optional `+` or `-`, then one or more ASCII digits. Unbounded:
/// this is the lexical space, not the `i128` this crate parses into. No
/// whitespace is trimmed; a caller applying the datatype's `collapse` facet
/// trims first.
#[must_use]
pub fn is_integer_lexical(lexical: &str) -> bool {
    let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
}

/// Whether `lexical` is in the `xsd:decimal` lexical space (XSD 1.1 Part 2
/// §3.3.3.1): an optional `+` or `-`, then digits with at most one `.`, and at
/// least one digit (`.5`, `1.`, `1.5` and `12` all qualify; no exponent).
/// Unbounded, and no whitespace is trimmed, as for [`is_integer_lexical`].
#[must_use]
pub fn is_decimal_lexical(lexical: &str) -> bool {
    decimal_lexical_parts(lexical).is_some()
}

/// Validate once and borrow the sign and both digit runs for exact conversion.
#[inline]
pub(crate) fn decimal_lexical_parts(lexical: &str) -> Option<(bool, &str, &str)> {
    let first = lexical.as_bytes().first().copied();
    let negative = first == Some(b'-');
    let body = if matches!(first, Some(b'+' | b'-')) {
        &lexical[1..]
    } else {
        lexical
    };
    let mut point = None;
    let mut any_digit = false;
    for (index, byte) in body.bytes().enumerate() {
        if byte.is_ascii_digit() {
            any_digit = true;
        } else if byte == b'.' && point.is_none() {
            point = Some(index);
        } else {
            return None;
        }
    }
    if !any_digit {
        return None;
    }
    let (whole, fraction) = point.map_or((body, ""), |index| (&body[..index], &body[index + 1..]));
    Some((negative, whole, fraction))
}

/// `xsd:integer`: optional leading `+`/`-`, then one or more ASCII digits of
/// any length. A derived integer type uses [`parse_integer_typed`] for its facets.
///
/// # Errors
/// Returns a lexical error for a non-integer spelling.
pub fn parse_integer(s: &str) -> Result<crate::exact::Integer, XsdError> {
    parse_integer_typed(s, XsdDatatype::Integer)
}

/// Parse an integer-family value exactly, hard-failing with
/// [`XsdError::OutOfRange`] if it is outside `datatype`'s inclusive bounds.
///
/// This is the unified entry point for all integer-family datatypes; `parse` in
/// `value.rs` routes every integer-family IRI through here.
///
/// # Errors
/// Returns a lexical error for a non-integer spelling or an out-of-range
/// error for a derived-type facet violation.
pub fn parse_integer_typed(
    lexical: &str,
    datatype: XsdDatatype,
) -> Result<crate::exact::Integer, XsdError> {
    if !is_integer_lexical(lexical) {
        return Err(XsdError::invalid(
            datatype,
            lexical,
            "expected an optional sign then digits",
        ));
    }
    let value = lexical
        .parse::<crate::exact::Integer>()
        .map_err(XsdError::Exact)?;
    if !datatype.admits_integer(&value) {
        return Err(XsdError::OutOfRange {
            datatype,
            lexical: lexical.to_owned(),
            reason: crate::value::reason::OUTSIDE_DATATYPE,
        });
    }
    Ok(value)
}

pub(crate) fn read_integer_typed(
    lexical: &str,
    datatype: XsdDatatype,
) -> Result<i128, NumericReadError> {
    if !is_integer_lexical(lexical) {
        return Err(NumericReadError::Invalid);
    }
    let value = lexical
        .parse::<i128>()
        .map_err(|_| NumericReadError::OutOfRange(crate::value::reason::INTEGER_TOO_LARGE))?;
    if let Some((min, max)) = datatype.integer_range()
        && (value < min || value > max)
    {
        return Err(NumericReadError::OutOfRange(
            crate::value::reason::OUTSIDE_DATATYPE,
        ));
    }
    Ok(value)
}

/// `xsd:decimal`: optional sign, digits with an optional single `.` (at least one
/// digit overall; `.5`, `1.`, `1.5`, `12` all valid), at any coefficient size or
/// fractional scale representable by the exact tower.
///
/// # Errors
/// Returns a lexical or scale error without narrowing to the old decimal type.
pub fn parse_decimal(s: &str) -> Result<crate::exact::Decimal, XsdError> {
    s.parse().map_err(XsdError::Exact)
}

/// A borrowed classification of the bounded numeric readers. Exact fallback
/// and physical sizing inspect it without first allocating an owned diagnostic.
#[derive(Debug)]
pub(crate) enum NumericReadError {
    Invalid,
    OutOfRange(&'static str),
}

impl purrdf_hash::error::StructuredError for NumericReadError {
    purrdf_hash::error_visit! {
        Invalid => "numeric-read-invalid" {},
        OutOfRange(reason) => "numeric-read-out-of-range" { "reason" => Text(reason) },
    }
}

#[cfg(test)]
impl NumericReadError {
    pub(crate) fn owned(self, datatype: XsdDatatype, lexical: &str) -> XsdError {
        match self {
            Self::Invalid => XsdError::invalid(datatype, lexical, "invalid numeric lexical"),
            Self::OutOfRange(reason) => XsdError::OutOfRange {
                datatype,
                lexical: lexical.to_owned(),
                reason,
            },
        }
    }
}

pub(crate) fn read_decimal(s: &str) -> Result<Decimal, NumericReadError> {
    let neg = s.starts_with('-');
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int_str, frac_str) = body.split_once('.').unwrap_or((body, ""));
    if frac_str.contains('.') {
        return Err(NumericReadError::Invalid);
    }
    if int_str.is_empty() && frac_str.is_empty() {
        return Err(NumericReadError::Invalid);
    }
    if !int_str.bytes().all(|byte| byte.is_ascii_digit())
        || !frac_str.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(NumericReadError::Invalid);
    }
    if frac_str.len() > usize::from(MAX_DECIMAL_SCALE) {
        return Err(NumericReadError::OutOfRange(
            crate::value::reason::DECIMAL_TOO_PRECISE,
        ));
    }
    let magnitude = int_str
        .bytes()
        .chain(frac_str.bytes())
        .try_fold(0u128, |acc, digit| {
            acc.checked_mul(10)?.checked_add(u128::from(digit - b'0'))
        })
        .ok_or(NumericReadError::OutOfRange(
            crate::value::reason::INTEGER_TOO_LARGE,
        ))?;
    let mantissa = if neg {
        0i128.checked_sub_unsigned(magnitude)
    } else {
        i128::try_from(magnitude).ok()
    }
    .ok_or(NumericReadError::OutOfRange(
        crate::value::reason::INTEGER_TOO_LARGE,
    ))?;
    Ok(Decimal::from_parts(mantissa, frac_str.len() as u8))
}

/// `xsd:double`: XSD numeric float lexical, or `INF`/`+INF`/`-INF`/`NaN`.
pub fn parse_double(s: &str) -> Result<f64, XsdError> {
    parse_ieee(s, XsdDatatype::Double)
}

/// `xsd:float`: as `double` but single-precision.
pub fn parse_float(s: &str) -> Result<f32, XsdError> {
    let dt = XsdDatatype::Float;
    match s {
        "INF" | "+INF" => return Ok(f32::INFINITY),
        "-INF" => return Ok(f32::NEG_INFINITY),
        "NaN" => return Ok(f32::NAN),
        _ => {}
    }
    reject_non_xsd_numeric(s, dt)?;
    s.parse::<f32>()
        .map_err(|_| XsdError::invalid(dt, s, "not a valid float lexical"))
}

/// XSD **1.0**-pinned `xsd:double` parse: identical to [`parse_double`] but rejects
/// the XSD 1.1-only `+INF` spelling of positive infinity. XSD 1.0 (referenced by
/// the ShEx/SHACL/SPARQL conformance suites) spells positive infinity only `INF`;
/// spec-pinned consumers call this to opt out of the 1.1-only lexical without
/// turning the whole crate back to 1.0.
pub fn parse_double_xsd10(s: &str) -> Result<f64, XsdError> {
    if s == "+INF" {
        return Err(XsdError::invalid(
            XsdDatatype::Double,
            s,
            "XSD 1.0 spells positive infinity INF",
        ));
    }
    parse_double(s)
}

/// XSD **1.0**-pinned `xsd:float` parse: identical to [`parse_float`] but rejects
/// the XSD 1.1-only `+INF` spelling of positive infinity. XSD 1.0 (referenced by
/// the ShEx/SHACL/SPARQL conformance suites) spells positive infinity only `INF`;
/// spec-pinned consumers call this to opt out of the 1.1-only lexical without
/// turning the whole crate back to 1.0.
pub fn parse_float_xsd10(s: &str) -> Result<f32, XsdError> {
    if s == "+INF" {
        return Err(XsdError::invalid(
            XsdDatatype::Float,
            s,
            "XSD 1.0 spells positive infinity INF",
        ));
    }
    parse_float(s)
}

/// Shared finite-numeric parse for double; returns `f64`.
fn parse_ieee(s: &str, dt: XsdDatatype) -> Result<f64, XsdError> {
    match s {
        "INF" | "+INF" => return Ok(f64::INFINITY),
        "-INF" => return Ok(f64::NEG_INFINITY),
        "NaN" => return Ok(f64::NAN),
        _ => {}
    }
    reject_non_xsd_numeric(s, dt)?;
    s.parse::<f64>()
        .map_err(|_| XsdError::invalid(dt, s, "not a valid double lexical"))
}

/// Reject lexicals Rust's float parser would accept but XSD forbids (`inf`,
/// `infinity`, `nan`, etc.): any ASCII letter other than the `e`/`E` exponent
/// marker disqualifies the form (the `INF`/`NaN` keywords are handled before here).
fn reject_non_xsd_numeric(s: &str, dt: XsdDatatype) -> Result<(), XsdError> {
    if s.bytes()
        .any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E')
    {
        return Err(XsdError::invalid(dt, s, "non-XSD numeric token"));
    }
    Ok(())
}

/// Refusal while writing into a caller-owned numeric lexical destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericRenderError {
    /// The caller has not admitted enough available destination capacity.
    DestinationTooSmall {
        /// Complete bytes the writer will append.
        required_bytes: usize,
        /// Bytes available without growing the caller's destination.
        available_bytes: usize,
    },
}

impl purrdf_hash::error::StructuredError for NumericRenderError {
    purrdf_hash::error_visit! {
        DestinationTooSmall { required_bytes, available_bytes } => "numeric-render-destination" {
            "required_bytes" => ByteCount(*required_bytes as u128),
            "available_bytes" => ByteCount(*available_bytes as u128),
        }
    }
}

/// Inline shortest IEEE text, already normalized to its XSD canonical spelling.
///
/// The longest binary64 scientific spelling has 17 significant digits, a sign,
/// point, exponent marker/sign and three exponent digits (24 bytes). The
/// `.0` normalization affects only a one-digit mantissa. This array also
/// covers XPath's plain spelling in its stated [10^-6, 10^6) interval.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalFloatText {
    bytes: [u8; 32],
    length: usize,
}

impl CanonicalFloatText {
    const fn empty() -> Self {
        Self {
            bytes: [0; 32],
            length: 0,
        }
    }

    /// Actual final lexical length, known without allocating.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Numeric text is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Borrow the inline ASCII lexical bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.length])
            .expect("the native IEEE formatter writes ASCII only")
    }

    /// Append without allocating or growing the caller's String.
    /// The full destination check happens before the first write.
    ///
    /// # Errors
    /// Refuses insufficient admitted capacity without modifying the destination.
    pub fn write_to(&self, output: &mut String) -> Result<(), NumericRenderError> {
        let available = output.capacity() - output.len();
        if self.length > available {
            return Err(NumericRenderError::DestinationTooSmall {
                required_bytes: self.length,
                available_bytes: available,
            });
        }
        output.push_str(self.as_str());
        Ok(())
    }
}

impl std::fmt::Write for CanonicalFloatText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.length.checked_add(text.len()).ok_or(std::fmt::Error)?;
        let destination = self
            .bytes
            .get_mut(self.length..end)
            .ok_or(std::fmt::Error)?;
        destination.copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

/// Native canonical text prepared without a coefficient String or magnitude clone.
/// Its surviving group capacity must stay admitted until the final write.
#[derive(Debug)]
pub enum PreparedNumericText {
    /// Exact signed base-ten integer groups.
    Integer(crate::bigint::PreparedDecimalDigits),
    /// Exact decimal groups, point and fractional padding.
    Decimal(crate::exact::PreparedDecimalText),
    /// Inline IEEE text with its original precision.
    Ieee(CanonicalFloatText),
}

impl PreparedNumericText {
    /// Exact destination length after preparation and before output allocation.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Integer(value) => value.len(),
            Self::Decimal(value) => value.len(),
            Self::Ieee(value) => value.len(),
        }
    }
    /// Canonical numeric text is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
    /// Actual remaining native temporary capacity.
    #[must_use]
    pub fn temporary_bytes(&self) -> usize {
        match self {
            Self::Integer(value) => value.heap_bytes(),
            Self::Decimal(value) => value.temporary_bytes(),
            Self::Ieee(_) => 0,
        }
    }
    /// Write into a preallocated final destination without hidden growth.
    /// # Errors
    /// Refuses insufficient capacity before writing any bytes.
    pub fn write_to(&self, output: &mut String) -> Result<(), NumericRenderError> {
        match self {
            Self::Integer(value) => value.write_to(output),
            Self::Decimal(value) => value.write_to(output),
            Self::Ieee(value) => value.write_to(output),
        }
    }
}

/// Prepare the existing native numeric renderer under checked physical admission.
/// A nonnumeric value belongs to its separate native renderer.
/// # Errors
/// Returns checked layout/admission or allocator refusal before lexical allocation.
pub fn prepare_numeric_text(
    value: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<PreparedNumericText>, crate::bigint::LimbScratchError> {
    use crate::exact::cost::NumericOperationLayout;
    let integer = |value: &crate::exact::Integer,
                   admit: &mut dyn FnMut(
        NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>| {
        let layout = value.render_layout(0)?;
        admit(NumericOperationLayout::bytes(layout.temporary_bytes(), 0)?)?;
        value
            .prepare_decimal_digits()
            .map(PreparedNumericText::Integer)
            .map(Some)
    };
    match value {
        XsdValue::Integer { value, .. } => integer(value, admit),
        XsdValue::Decimal(value) => prepare_decimal_text(value, admit).map(Some),
        XsdValue::Float(value) => Ok(Some(PreparedNumericText::Ieee(canonical_float_text(
            *value,
        )))),
        XsdValue::Double(value) => Ok(Some(PreparedNumericText::Ieee(canonical_double_text(
            *value,
        )))),
        _ => Ok(None),
    }
}

/// Prepare the text of a numeric value an operation COMPUTED. It is
/// [`prepare_numeric_text`] except that a decimal always carries a point and a
/// fractional digit (`4.0`, never `4`): the specifications leave the lexical form
/// of a computed literal to the processor, and this is the one a processor emits
/// so that a whole-valued decimal is never mistaken for an `xsd:integer` spelling.
/// The XSD 1.1 canonical text of a stored value stays [`prepare_numeric_text`].
/// # Errors
/// As [`prepare_numeric_text`].
pub fn prepare_computed_numeric_text(
    value: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<PreparedNumericText>, crate::bigint::LimbScratchError> {
    match value {
        XsdValue::Decimal(decimal) => match prepare_decimal_text(decimal, admit)? {
            PreparedNumericText::Decimal(text) => Ok(Some(PreparedNumericText::Decimal(
                text.with_required_point(),
            ))),
            other => Ok(Some(other)),
        },
        other => prepare_numeric_text(other, admit),
    }
}

/// Prepare a borrowed exact decimal through the same native renderer used by
/// numeric values, without copying its coefficient out of a temporal owner.
/// # Errors
/// Returns physical admission or allocation refusal before group conversion.
pub fn prepare_decimal_text(
    value: &crate::exact::Decimal,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<PreparedNumericText, crate::bigint::LimbScratchError> {
    let layout = value.render_layout()?;
    admit(crate::exact::cost::NumericOperationLayout::bytes(
        layout.temporary_bytes(),
        0,
    )?)?;
    value.prepare_text().map(PreparedNumericText::Decimal)
}

/// XSD canonical double in inline caller-borrowable storage. No allocation.
#[must_use]
pub fn canonical_double_text(value: f64) -> CanonicalFloatText {
    canonical_ieee(
        value.is_nan(),
        value.is_infinite(),
        value.is_sign_negative(),
        |output| std::fmt::write(output, format_args!("{value:e}")),
    )
}

/// XSD canonical float at its original single precision. No allocation.
#[must_use]
pub fn canonical_float_text(value: f32) -> CanonicalFloatText {
    canonical_ieee(
        value.is_nan(),
        value.is_infinite(),
        value.is_sign_negative(),
        |output| std::fmt::write(output, format_args!("{value:e}")),
    )
}

fn xpath_ieee_text(
    value: f64,
    plain: impl FnOnce(&mut CanonicalFloatText) -> std::fmt::Result,
    canonical: impl FnOnce() -> CanonicalFloatText,
) -> CanonicalFloatText {
    use std::fmt::Write;
    let special = if value.is_nan() {
        Some("NaN")
    } else if value.is_infinite() {
        Some(if value > 0.0 { "INF" } else { "-INF" })
    } else {
        None
    };
    if let Some(text) = special {
        let mut output = CanonicalFloatText::empty();
        output
            .write_str(text)
            .expect("XPath IEEE special fits inline");
        return output;
    }
    let magnitude = value.abs();
    // The double written 1e-6 lies just below one millionth: retain the
    // existing F&O test rather than changing the lower-bound rounding law.
    if magnitude > 1e-6 && magnitude < 1e6 {
        let mut output = CanonicalFloatText::empty();
        plain(&mut output).expect("XPath plain IEEE text fits inline");
        output
    } else {
        let mut output = canonical();
        // Classify the formatted bytes: LLVM can turn even an original-bit
        // zero test back into a floating comparison that DAZ changes.
        match output.as_str() {
            "0.0E0" => output.length = 1,
            "-0.0E0" => output.length = 2,
            _ => {}
        }
        output
    }
}

/// XPath's double-to-string text, distinct from canonical RDF literal text.
#[must_use]
pub fn xpath_double_text(value: f64) -> CanonicalFloatText {
    xpath_ieee_text(
        value,
        |output| std::fmt::write(output, format_args!("{value}")),
        || canonical_double_text(value),
    )
}

/// XPath's float-to-string text at the original single precision.
#[must_use]
pub fn xpath_float_text(value: f32) -> CanonicalFloatText {
    xpath_ieee_text(
        f64::from(value),
        |output| std::fmt::write(output, format_args!("{value}")),
        || canonical_float_text(value),
    )
}

/// Prepare the numeric/boolean XPath cast-to-string law under the same native
/// exact renderer admission. Boolean and IEEE text stay inline.
/// # Errors
/// Returns physical native render/admission refusal before final allocation.
pub fn prepare_xpath_text(
    value: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<PreparedNumericText>, crate::bigint::LimbScratchError> {
    match value {
        XsdValue::Boolean(value) => {
            use std::fmt::Write;
            let mut text = CanonicalFloatText::empty();
            text.write_str(if *value { "true" } else { "false" })
                .expect("Boolean lexical fits inline");
            Ok(Some(PreparedNumericText::Ieee(text)))
        }
        XsdValue::Float(value) => Ok(Some(PreparedNumericText::Ieee(xpath_float_text(*value)))),
        XsdValue::Double(value) => Ok(Some(PreparedNumericText::Ieee(xpath_double_text(*value)))),
        other => prepare_numeric_text(other, admit),
    }
}

/// XSD canonical double as a resident owned String.
#[must_use]
pub fn canonical_double(value: f64) -> String {
    canonical_double_text(value).as_str().to_owned()
}

/// XSD canonical float as a resident owned String.
#[must_use]
pub fn canonical_float(value: f32) -> String {
    canonical_float_text(value).as_str().to_owned()
}

fn canonical_ieee(
    is_nan: bool,
    is_inf: bool,
    is_neg: bool,
    scientific: impl FnOnce(&mut CanonicalFloatText) -> std::fmt::Result,
) -> CanonicalFloatText {
    use std::fmt::Write;
    let mut output = CanonicalFloatText::empty();
    let special = if is_nan {
        Some("NaN")
    } else if is_inf {
        Some(if is_neg { "-INF" } else { "INF" })
    } else {
        None
    };
    if let Some(special) = special {
        output
            .write_str(special)
            .expect("special IEEE lexical fits inline");
        return output;
    }
    // The formatter handles signed zero from its original bits too, without
    // a floating zero comparison that could flush a subnormal operand.
    let mut raw = CanonicalFloatText::empty();
    scientific(&mut raw).expect("shortest IEEE scientific lexical fits inline");
    let (mantissa, exponent) = raw.as_str().split_once('e').unwrap_or((raw.as_str(), "0"));
    output
        .write_str(mantissa)
        .expect("IEEE mantissa fits inline");
    if !mantissa.contains('.') {
        output
            .write_str(".0")
            .expect("normalized IEEE mantissa fits inline");
    }
    output
        .write_str("E")
        .expect("IEEE exponent marker fits inline");
    output
        .write_str(exponent)
        .expect("IEEE exponent fits inline");
    output
}

/// SPARQL numeric promotion comparison. Promotes both operands to the least type
/// that contains them (`integer ⊂ decimal ⊂ float ⊂ double`) and compares. Returns
/// `None` when an operand is `NaN` (genuinely unordered) or non-numeric (the caller
/// — `value_cmp` — only routes numeric operands here; non-numeric → `None`).
///
/// Integer-vs-integer comparison is by value only (ignoring the subtype): per the
/// SPARQL promotion rules, `xsd:int 5 = xsd:long 5`.
#[must_use]
pub fn numeric_cmp(a: &XsdValue, b: &XsdValue) -> Option<Ordering> {
    numeric_cmp_admitted(a, b, &mut |_| Ok(())).expect("resident numeric comparison allocation")
}

/// Promotion comparison with physical admission for exact alignment and IEEE conversion.
///
/// # Errors
/// Returns native admission or allocation failure independently of incomparability.
pub fn numeric_cmp_admitted(
    a: &XsdValue,
    b: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    Ok(match (a, b) {
        // Same exact integer / decimal cases keep full precision.
        // Integer-vs-integer: compare by value, ignore subtype (xsd:int 5 == xsd:long 5).
        (Integer { value: x, .. }, Integer { value: y, .. }) => Some(x.cmp(y)),
        (Dec(_) | Integer { .. }, Dec(_)) | (Dec(_), Integer { .. }) => {
            Some(exact_path::cmp_admitted(a, b, admit)?)
        }
        // Any `double` operand → compare as f64.
        (Double(_), _) | (_, Double(_)) => {
            match (
                numeric_to_f64_admitted(a, admit)?,
                numeric_to_f64_admitted(b, admit)?,
            ) {
                (Some(a), Some(b)) => a.partial_cmp(&b),
                _ => None,
            }
        }
        // Else any `float` operand → compare as f32.
        (Float(_), _) | (_, Float(_)) => {
            match (
                numeric_to_f32_admitted(a, admit)?,
                numeric_to_f32_admitted(b, admit)?,
            ) {
                (Some(a), Some(b)) => a.partial_cmp(&b),
                _ => None,
            }
        }
        // At least one operand is non-numeric.
        _ => None,
    })
}

/// SPARQL numeric value equality (`=`) via the promotion comparison.
#[must_use]
pub fn numeric_eq(a: &XsdValue, b: &XsdValue) -> bool {
    numeric_cmp(a, b) == Some(Ordering::Equal)
}

/// The **exact** total order over the XSD numeric tower — the relation `ORDER BY`,
/// `DISTINCT`, `MIN` and `MAX` sort by, and a genuine total order, which
/// [`numeric_cmp`] is not.
///
/// Every value in the tower except `NaN` is exactly a rational number:
/// `xsd:integer` is `m/1`, `xsd:decimal` is `mantissa / 10^scale`, and a finite
/// `xsd:float`/`xsd:double` is a **dyadic** rational `significand × 2^exponent`
/// (that is what an IEEE binary format *is*). This function compares those
/// rationals exactly, cross-multiplying into [`crate::BigInt`] on the rare pair
/// where a `u128` cannot hold the products. `NaN` is the one input with no place
/// in the order and answers `None`; the infinities sit at the two ends.
///
/// # This deliberately diverges from SPARQL's promotion-based `<`
///
/// SPARQL 1.1/1.2 §17.3 maps a cross-type numeric comparison onto the promotion
/// lattice `integer ⊂ decimal ⊂ float ⊂ double`, which routes ANY pair containing
/// a float or double operand through `f32`/`f64` — a lossy projection.
/// [`numeric_cmp`] implements that mapping faithfully and is what this crate's
/// `<`, `>`, `=` operators keep using, because that is what the operators are
/// specified to mean. This function does NOT, and the divergence is not a liberty
/// taken for convenience:
///
/// **The two disagree in exactly the cases where the spec's own mapping is
/// intransitive.** Promotion and exactness agree whenever the promotion is
/// lossless; they can only disagree when rounding maps two *distinct* exact values
/// onto one IEEE value. But the moment it does, the promoted relation has a cycle.
/// Take `a = "1.000000000000000001"^^xsd:decimal`, `b = "1"^^xsd:integer` and
/// `c = "1.0E0"^^xsd:double`:
///
/// * `a` vs `b` is decimal-vs-integer, which promotion compares EXACTLY, so
///   `a > b`;
/// * `a` vs `c` promotes `a` to `f64`, which rounds it to `1.0`, so `a = c`;
/// * `b` vs `c` promotes `b` to `f64` exactly, so `b = c`.
///
/// From `a = c` and `b = c` a transitive relation must conclude `a = b`, and the
/// first line says otherwise. That is a genuine cycle over three ordinary literals,
/// and `ORDER BY` hands its comparator to a Rust sort, which is entitled to
/// **panic** when the comparator is not a total order — reachable from any query
/// whose sort key is an attacker-supplied literal with more than sixteen
/// significant digits. There is no way to be both transitive and promotion-faithful
/// here, so the ordering picks transitive; the exact answer is also strictly more
/// informative, since it distinguishes values the promoted relation conflates and
/// never conflates values the promoted relation distinguishes.
///
/// The divergence is confined to the ORDER. `=` and `<` still answer exactly what
/// §17.3 says, through [`numeric_cmp`].
///
/// # Examples
///
/// ```rust
/// use std::cmp::Ordering;
///
/// use purrdf_xsd::{XsdDatatype, numeric_cmp, numeric_total_cmp, parse};
///
/// let a = parse("1.000000000000000001", XsdDatatype::Decimal)?;
/// let c = parse("1.0E0", XsdDatatype::Double)?;
///
/// // Promotion rounds the decimal to 1.0 and calls the two equal.
/// assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
/// // The exact order sees the nineteenth significant digit.
/// assert_eq!(numeric_total_cmp(&a, &c), Some(Ordering::Greater));
///
/// // NaN is the one value with no place in the order.
/// let nan = parse("NaN", XsdDatatype::Double)?;
/// assert_eq!(numeric_total_cmp(&nan, &c), None);
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn numeric_total_cmp(a: &XsdValue, b: &XsdValue) -> Option<Ordering> {
    numeric_total_cmp_using(a, b, &crate::bigint::scratch::Unbounded, &mut |_| Ok(()))
        .expect("unbounded integer storage")
}

/// Exact numeric total order with physical admission before every native destination.
/// # Errors
/// Returns admission/allocation refusal separately from NaN incomparability.
pub fn numeric_total_cmp_admitted(
    a: &XsdValue,
    b: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    numeric_total_cmp_using(a, b, &crate::bigint::scratch::Fallible, admit)
}

fn numeric_total_cmp_using(
    a: &XsdValue,
    b: &XsdValue,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    use XsdValue::{Decimal as Dec, Double, Float, Integer};
    Ok(match (a, b) {
        // ── Pairs the promotion lattice already decides exactly ──────────────
        // Integer-vs-integer ignores the subtype, exactly as `numeric_cmp` does.
        (Integer { value: x, .. }, Integer { value: y, .. }) => Some(x.cmp(y)),
        (Dec(_) | Integer { .. }, Dec(_)) | (Dec(_), Integer { .. }) => {
            Some(exact_path::cmp_admitted(a, b, admit)?)
        }
        // `f32 → f64` is an exact widening (every `f32` is an `f64`), so every
        // IEEE-vs-IEEE pair is already decided without loss.
        (Float(x), Float(y)) => x.partial_cmp(y),
        (Double(x), Double(y)) => x.partial_cmp(y),
        (Float(x), Double(y)) => f64::from(*x).partial_cmp(y),
        (Double(x), Float(y)) => x.partial_cmp(&f64::from(*y)),
        // ── The exact-vs-IEEE pairs, the only lossy ones ─────────────────────
        // Bounded coefficients retain the allocation-free cross-product path;
        // larger coefficients use the same admitted exact tower.
        (_, Float(y)) if a.is_exact_numeric() => {
            exact_vs_ieee_numeric_using(a, f64::from(*y), storage, admit)?
        }
        (_, Double(y)) if a.is_exact_numeric() => {
            exact_vs_ieee_numeric_using(a, *y, storage, admit)?
        }
        (Float(x), _) if b.is_exact_numeric() => {
            exact_vs_ieee_numeric_using(b, f64::from(*x), storage, admit)?.map(Ordering::reverse)
        }
        (Double(x), _) if b.is_exact_numeric() => {
            exact_vs_ieee_numeric_using(b, *x, storage, admit)?.map(Ordering::reverse)
        }
        // At least one operand is non-numeric.
        _ => None,
    })
}

fn exact_vs_ieee_numeric_using(
    exact: &XsdValue,
    ieee: f64,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    let bounded = match exact {
        XsdValue::Integer { value, .. } => value.as_i128().map(Decimal::from_integer),
        XsdValue::Decimal(value) => value.to_bounded(),
        _ => None,
    };
    if let Some(bounded) = bounded {
        exact_vs_ieee_using(&bounded, ieee, storage, admit)
    } else {
        exact_path::cmp_ieee_admitted_using(exact, ieee, storage, admit)
    }
}

/// `|value| = significand × 2^exponent` with an ODD `significand` — the exact dyadic
/// decomposition every finite IEEE double has, normalized so a value like `0.5`
/// comes back as `(1, -1)` rather than `(2^52, -53)`.
///
/// Normalizing matters for more than tidiness: it is what keeps the exponent small
/// for the values that actually have short binary expansions, which is what keeps
/// [`exact_vs_ieee_using`]'s `u128` fast path reachable instead of pushed onto
/// [`crate::BigInt`] by 52 trailing zero bits nobody needs.
fn dyadic_magnitude(value: f64) -> (u64, i32) {
    const SIGNIFICAND_BITS: u32 = 52;
    let bits = value.abs().to_bits();
    // The IEEE-754 binary64 fields: an 11-bit biased exponent above a 52-bit
    // fraction. `value` is finite here (the caller decides the specials first).
    let biased = ((bits >> SIGNIFICAND_BITS) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << SIGNIFICAND_BITS) - 1);
    let (mut significand, mut exponent) = if biased == 0 {
        // Subnormal (and zero): no implicit leading bit, fixed exponent.
        (fraction, -1074)
    } else {
        // Normal: value = (2^52 + fraction) × 2^(biased - 1023 - 52).
        (fraction | (1u64 << SIGNIFICAND_BITS), biased - 1075)
    };
    if significand != 0 {
        let trailing = significand.trailing_zeros();
        significand >>= trailing;
        // `trailing < 64`, so the cast is exact and the sum cannot overflow an
        // exponent already inside [-1074, 971].
        exponent += trailing as i32;
    }
    (significand, exponent)
}

/// Order an exact `mantissa / 10^scale` against a `f64`, exactly.
///
/// The `f64` carries the IEEE side for BOTH `xsd:float` and `xsd:double`, because
/// widening `f32 → f64` is lossless; the integer branch of the tower arrives as a
/// scale-0 [`Decimal`]. `None` only for `NaN`.
fn exact_vs_ieee_using(
    exact: &Decimal,
    ieee: f64,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    if ieee.is_nan() {
        return Ok(None);
    }
    if ieee.is_infinite() {
        return Ok(Some(if ieee.is_sign_positive() {
            Ordering::Less
        } else {
            Ordering::Greater
        }));
    }
    let exact_sign = exact.mantissa().signum();
    let ieee_sign = if ieee == 0.0 {
        0
    } else if ieee < 0.0 {
        -1
    } else {
        1
    };
    if exact_sign != ieee_sign {
        return Ok(Some(exact_sign.cmp(&ieee_sign)));
    }
    if exact_sign == 0 {
        return Ok(Some(Ordering::Equal));
    }
    let magnitude = magnitude_cmp_using(
        exact.mantissa().unsigned_abs(),
        exact.scale(),
        ieee,
        storage,
        admit,
    )?;
    Ok(Some(if exact_sign > 0 {
        magnitude
    } else {
        magnitude.reverse()
    }))
}

/// Compare the strictly-positive magnitudes `mantissa / 10^scale` and `|ieee|`.
///
/// # The cost model
///
/// This is the `ORDER BY`/`DISTINCT`/`MIN`/`MAX` hot path, so nothing here
/// allocates unless the answer genuinely needs more than 128 bits:
///
/// 1. **A magnitude window.** The exact side is bounded — `1 ≤ mantissa < 2^127`
///    and `scale ≤ 18` give `10^-18 ≤ value ≤ 2^127` — so an IEEE magnitude at
///    least `2^128` or under `2^-60` cannot alias it at all, and one comparison on
///    the extracted exponent settles the pair. That is the whole subnormal range
///    and the whole top two decades of the double range, decided without touching
///    the mantissa.
/// 2. **A `u128` cross-multiplication.** Inside the window the two sides are
///    compared as integers — `mantissa` against `significand × 2^exp × 10^scale`
///    for a non-negative exponent, or `mantissa × 2^-exp` against
///    `significand × 10^scale` for a negative one. When the products fit `u128`,
///    which they do for every pair whose magnitudes are close enough for the
///    answer to be interesting, this is a handful of machine multiplies.
/// 3. **[`crate::BigInt`], only then.** The fallback is reached only when a product
///    genuinely exceeds 128 bits, which needs the two magnitudes to differ by
///    enough that step 1 nearly caught them.
fn magnitude_cmp_using(
    mantissa: u128,
    scale: u8,
    ieee: f64,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Ordering, crate::bigint::LimbScratchError> {
    let (significand, exponent) = dyadic_magnitude(ieee);
    // `2^(high - 1) <= |ieee| < 2^high`.
    let high = exponent + bit_length(significand);
    if high >= 129 {
        // `|ieee| >= 2^128 > 2^127 >= mantissa >= mantissa / 10^scale`.
        return Ok(Ordering::Less);
    }
    if high <= -60 {
        // `|ieee| < 2^-60 < 10^-18 <= mantissa / 10^scale`, the smallest magnitude
        // a non-zero decimal at this crate's maximum scale can denote.
        return Ok(Ordering::Greater);
    }
    // Inside the window: `exponent` is in [-112, 127] and the shifts below are small.
    let ten_pow = 10u128.checked_pow(u32::from(scale));
    let significand = u128::from(significand);
    let shift = exponent.unsigned_abs();
    // `shl_exact` answers `None` rather than wrapping, so an overflowing product
    // falls through to the arbitrary-precision comparison instead of lying.
    let decided = if exponent >= 0 {
        shl_exact(significand, shift)
            .zip(ten_pow)
            .and_then(|(scaled, ten_pow)| scaled.checked_mul(ten_pow))
            .map(|right| mantissa.cmp(&right))
    } else {
        shl_exact(mantissa, shift).zip(ten_pow).map(|(left, pow)| {
            // `significand < 2^53` and `pow <= 10^18 < 2^60`, so the right-hand
            // product is under `2^113` and cannot overflow.
            left.cmp(&(significand * pow))
        })
    };
    if let Some(decided) = decided {
        Ok(decided)
    } else {
        big_magnitude_cmp_using(mantissa, scale, significand, exponent, storage, admit)
    }
}

/// `value << shift`, or `None` when that would discard a high bit.
fn shl_exact(value: u128, shift: u32) -> Option<u128> {
    if shift >= u128::BITS {
        return (value == 0).then_some(0);
    }
    let shifted = value << shift;
    (shifted >> shift == value).then_some(shifted)
}

/// The exact cross-multiplication once a product has outgrown `u128`.
fn big_magnitude_cmp_using(
    mantissa: u128,
    scale: u8,
    significand: u128,
    exponent: i32,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Ordering, crate::bigint::LimbScratchError> {
    use crate::exact::{Integer, cost};
    let left = Integer::from(mantissa);
    let right = Integer::from(significand);
    let shifted_left = if exponent < 0 {
        let bits = u64::from(mantissa.ilog2()) + 1;
        admit(cost::binary_shift_layout(bits, exponent.unsigned_abs(), 0)?)?;
        Some(Integer::from_bigint(left.with_bigint(|big| {
            big.shl_using(exponent.unsigned_abs(), storage)
        })?))
    } else {
        None
    };
    let left_live = shifted_left
        .as_ref()
        .map_or(Ok(0), |value| cost::live_integer_bytes(&[value]))?;
    let shifted_right = if exponent >= 0 {
        let bits = u64::from(significand.ilog2()) + 1;
        admit(cost::binary_shift_layout(bits, exponent.unsigned_abs(), 0)?.with_live(left_live)?)?;
        Integer::from_bigint(
            right.with_bigint(|big| big.shl_using(exponent.unsigned_abs(), storage))?,
        )
    } else {
        right
    };
    let live = left_live
        .checked_add(cost::live_integer_bytes(&[&shifted_right])?)
        .ok_or(crate::bigint::LimbScratchError::SizeOverflow)?;
    admit(cost::scale_up_layout(&shifted_right, u32::from(scale))?.with_live(live)?)?;
    let scaled = crate::exact::decimal::scale_up_using(&shifted_right, u32::from(scale), storage)?;
    Ok(shifted_left.as_ref().unwrap_or(&left).cmp(&scaled))
}

/// The number of bits in `value`'s binary representation (`0` for zero).
fn bit_length(value: u64) -> i32 {
    // `bit_width()` is in [0, 64], so the cast is exact.
    value.bit_width() as i32
}

/// The numeric value as `f64`, or `None` if `v` is not a numeric value.
fn num_f64(v: &XsdValue) -> Option<f64> {
    Some(match v {
        // Spec-mandated lossy promotion: integer ⊂ double (SPARQL §17.3 numeric tower).
        // Large i128 values (> 2^53) lose low-order bits; this is required behaviour,
        // not an accident.
        XsdValue::Integer { value, .. } => value.to_f64(),
        // decimal ⊂ double: correctly rounded, once (see `Decimal::to_f64`).
        XsdValue::Decimal(d) => d.to_f64(),
        XsdValue::Float(f) => f64::from(*f),
        XsdValue::Double(d) => *d,
        _ => return None,
    })
}

/// The numeric value as `f32`, or `None` if `v` is not a numeric value.
fn num_f32(v: &XsdValue) -> Option<f32> {
    Some(match v {
        // Spec-mandated lossy promotion: integer ⊂ float (SPARQL §17.3 numeric tower).
        // Large i128 values (> 2^24) lose precision; required by IEEE promotion semantics.
        XsdValue::Integer { value, .. } => value.to_f32(),
        // decimal ⊂ float: the exact decimal rounded ONCE, straight to single
        // precision (XPath `xs:float` casting). Going through `f64` first would round
        // twice — nothing in the spec asks for that, and it can land one ulp off.
        XsdValue::Decimal(d) => d.to_f32(),
        XsdValue::Float(f) => *f,
        // double → float narrowing: required by the numeric tower when a float operand
        // forces promotion of the other operand down (SPARQL §17.3).
        XsdValue::Double(d) => *d as f32,
        _ => return None,
    })
}

/// Arithmetic selected by the caller without allocating an operator closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericBinaryOperator {
    /// Numeric/value-space addition.
    Add,
    /// Numeric/value-space subtraction.
    Subtract,
    /// Numeric/value-space multiplication.
    Multiply,
    /// Numeric/value-space division.
    Divide,
}

/// A numerical refusal is distinct from a native destination refusal.
#[derive(Debug)]
pub enum NumericOperationError {
    /// Existing value-space and F&O classification.
    Value(XsdError),
    /// Checked physical layout or allocator refusal.
    Storage(crate::bigint::LimbScratchError),
}

impl purrdf_hash::error::StructuredError for NumericOperationError {
    purrdf_hash::error_visit! {
        Value(error) => "numeric-operation-value" { "cause" => Cause(error) },
        Storage(error) => "numeric-operation-storage" { "cause" => Cause(error) },
    }
}

purrdf_lex::variant_from!(NumericOperationError { Value(XsdError) });
purrdf_lex::variant_from!(NumericOperationError { Storage(crate::bigint::LimbScratchError) });

/// Promote to binary64 through the existing correctly rounded native kernels.
/// # Errors
/// Returns physical admission/refusal separately from a nonnumeric operand.
pub fn numeric_to_f64_admitted(
    value: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<f64>, crate::bigint::LimbScratchError> {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => {
            value.try_to_f64_admitted(admit).map(Some)
        }
        XsdValue::Decimal(value) if value.unscaled().as_i128().is_none() || value.scale() > 18 => {
            value.try_to_f64_admitted(admit).map(Some)
        }
        other => Ok(num_f64(other)),
    }
}

/// Promote directly to binary32, preserving the single-rounding law.
/// # Errors
/// Returns physical admission/refusal separately from a nonnumeric operand.
pub fn numeric_to_f32_admitted(
    value: &XsdValue,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<f32>, crate::bigint::LimbScratchError> {
    match value {
        XsdValue::Integer { value, .. } if value.as_i128().is_none() => {
            value.try_to_f32_admitted(admit).map(Some)
        }
        XsdValue::Decimal(value) if value.unscaled().as_i128().is_none() || value.scale() > 18 => {
            value.try_to_f32_admitted(admit).map(Some)
        }
        other => Ok(num_f32(other)),
    }
}

/// Numeric arithmetic with checked phase admission and fallible native destinations.
/// Parsed inputs are borrowed; the caller retains the returned native payload
/// under its admitted frame before rendering or sharing it.
/// # Errors
/// Preserves all existing F&O/precision failures separately from storage refusal.
pub fn numeric_binary_admitted(
    a: &XsdValue,
    b: &XsdValue,
    op: NumericBinaryOperator,
    policy: DivisionPolicy,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<XsdValue, NumericOperationError> {
    if matches!(a, XsdValue::Double(_)) || matches!(b, XsdValue::Double(_)) {
        let x = numeric_to_f64_admitted(a, admit)?.ok_or(XsdError::TypeMismatch {
            reason: "non-numeric arithmetic operand",
        })?;
        let y = numeric_to_f64_admitted(b, admit)?.ok_or(XsdError::TypeMismatch {
            reason: "non-numeric arithmetic operand",
        })?;
        return Ok(XsdValue::Double(match op {
            NumericBinaryOperator::Add => ieee::f64_add(x, y),
            NumericBinaryOperator::Subtract => ieee::f64_sub(x, y),
            NumericBinaryOperator::Multiply => ieee::f64_mul(x, y),
            NumericBinaryOperator::Divide => ieee::f64_div(x, y),
        }));
    }
    if matches!(a, XsdValue::Float(_)) || matches!(b, XsdValue::Float(_)) {
        let x = numeric_to_f32_admitted(a, admit)?.ok_or(XsdError::TypeMismatch {
            reason: "non-numeric arithmetic operand",
        })?;
        let y = numeric_to_f32_admitted(b, admit)?.ok_or(XsdError::TypeMismatch {
            reason: "non-numeric arithmetic operand",
        })?;
        return Ok(XsdValue::Float(match op {
            NumericBinaryOperator::Add => ieee::f32_add(x, y),
            NumericBinaryOperator::Subtract => ieee::f32_sub(x, y),
            NumericBinaryOperator::Multiply => ieee::f32_mul(x, y),
            NumericBinaryOperator::Divide => ieee::f32_div(x, y),
        }));
    }
    let exact_pair = a.is_exact_numeric() && b.is_exact_numeric();
    if exact_pair {
        // The old bounded int_binop renders its overflow before the exact
        // fallback. Classify it in machine words without that doomed allocation.
        if let (XsdValue::Integer { value: x, .. }, XsdValue::Integer { value: y, .. }) = (a, b)
            && op != NumericBinaryOperator::Divide
            && let (Some(x), Some(y)) = (x.as_i128(), y.as_i128())
        {
            let value = match op {
                NumericBinaryOperator::Add => x.checked_add(y),
                NumericBinaryOperator::Subtract => x.checked_sub(y),
                NumericBinaryOperator::Multiply => x.checked_mul(y),
                NumericBinaryOperator::Divide => unreachable!("division excluded"),
            };
            return if let Some(value) = value {
                Ok(integer_result(value))
            } else {
                exact_path::binary_admitted(a, b, op, policy, admit)
            };
        }
        return exact_path::binary_admitted(a, b, op, policy, admit);
    }
    let bounded = match op {
        NumericBinaryOperator::Add => numeric_add_bounded(a, b),
        NumericBinaryOperator::Subtract => numeric_sub_bounded(a, b),
        NumericBinaryOperator::Multiply => numeric_mul_bounded(a, b),
        NumericBinaryOperator::Divide => numeric_div_ieee(a, b),
    };
    match bounded {
        Err(XsdError::OutOfRange { .. } | XsdError::TypeMismatch { .. }) if exact_pair => {
            exact_path::binary_admitted(a, b, op, policy, admit)
        }
        other => other.map_err(NumericOperationError::Value),
    }
}

/// The fixed unary operations whose native destinations must be admitted first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericUnaryOperator {
    /// Numeric identity; evaluator carriers can share their existing payload.
    Plus,
    /// Negation, including the value-space duration extension.
    Minus,
    /// Absolute value.
    Abs,
    /// Ceiling, keeping the numeric family.
    Ceil,
    /// Floor, keeping the numeric family.
    Floor,
    /// XPath round with ties toward positive infinity.
    Round,
}

/// Unary arithmetic using existing kernels with physical admission before copies.
/// # Errors
/// Numerical/type failures are distinct from native destination refusal.
pub fn numeric_unary_admitted(
    value: &XsdValue,
    op: NumericUnaryOperator,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<XsdValue, NumericOperationError> {
    use crate::exact::{Rounding, cost::NumericOperationLayout};
    let copied = |bytes: u64| -> Result<NumericOperationLayout, crate::bigint::LimbScratchError> {
        let bytes =
            usize::try_from(bytes).map_err(|_| crate::bigint::LimbScratchError::SizeOverflow)?;
        NumericOperationLayout::bytes(bytes, bytes)
    };
    match value {
        XsdValue::Integer { value, datatype } => {
            admit(copied(value.heap_bytes())?)?;
            let result = match op {
                NumericUnaryOperator::Minus => value.try_neg()?,
                NumericUnaryOperator::Abs => value.try_abs()?,
                _ => value.try_copy()?,
            };
            Ok(XsdValue::from_exact_integer(
                result,
                if op == NumericUnaryOperator::Plus {
                    *datatype
                } else {
                    XsdDatatype::Integer
                },
            ))
        }
        XsdValue::Decimal(value) => {
            let result = match op {
                NumericUnaryOperator::Ceil => {
                    value.try_round_admitted(0, Rounding::Ceiling, admit)?
                }
                NumericUnaryOperator::Floor => {
                    value.try_round_admitted(0, Rounding::Floor, admit)?
                }
                NumericUnaryOperator::Round => {
                    value.try_round_admitted(0, Rounding::HalfCeiling, admit)?
                }
                _ => {
                    admit(copied(value.unscaled().heap_bytes())?)?;
                    match op {
                        NumericUnaryOperator::Minus => value.try_neg()?,
                        NumericUnaryOperator::Abs => value.try_abs()?,
                        _ => value.try_copy()?,
                    }
                }
            };
            Ok(XsdValue::from_exact_decimal(result))
        }
        XsdValue::Duration(duration) if op == NumericUnaryOperator::Minus => {
            crate::temporal::negate_duration_admitted(duration, admit)
                .map(XsdValue::Duration)
                .map_err(|error| match error {
                    XsdError::Storage(cause) => NumericOperationError::Storage(cause),
                    other => NumericOperationError::Value(other),
                })
        }
        _ => match op {
            NumericUnaryOperator::Plus => numeric_unary_plus(value),
            NumericUnaryOperator::Minus => crate::ops::value_unary_minus(value),
            NumericUnaryOperator::Abs => numeric_abs_bounded(value),
            NumericUnaryOperator::Ceil => numeric_ceil_bounded(value),
            NumericUnaryOperator::Floor => numeric_floor_bounded(value),
            NumericUnaryOperator::Round => numeric_round_bounded(value),
        }
        .map_err(NumericOperationError::Value),
    }
}

/// The numeric/boolean cast law using the existing exact native conversion bodies.
/// # Errors
/// Physical admission/refusal is separate from an out-of-domain cast (None).
pub fn numeric_cast_admitted(
    source: &XsdValue,
    target: XsdDatatype,
    admit: &mut impl FnMut(
        crate::exact::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<XsdValue>, crate::bigint::LimbScratchError> {
    use crate::exact::{
        Decimal as ExactDecimal, ExactOperationError, Integer, cost::NumericOperationLayout,
    };
    let copy_integer = |value: &Integer,
                        admit: &mut dyn FnMut(
        NumericOperationLayout,
    )
        -> Result<(), crate::bigint::LimbScratchError>| {
        let bytes = usize::try_from(value.heap_bytes())
            .map_err(|_| crate::bigint::LimbScratchError::SizeOverflow)?;
        admit(NumericOperationLayout::bytes(bytes, bytes)?)?;
        value.try_copy()
    };
    let exact_binary = |value: f64,
                        admit: &mut dyn FnMut(
        NumericOperationLayout,
    )
        -> Result<(), crate::bigint::LimbScratchError>|
     -> Result<Option<XsdValue>, crate::bigint::LimbScratchError> {
        if target == XsdDatatype::Decimal {
            return match ExactDecimal::try_from_f64_admitted(value, &mut |layout| admit(layout)) {
                Ok(value) => Ok(Some(XsdValue::from_exact_decimal(value))),
                Err(ExactOperationError::Value(_)) => Ok(None),
                Err(ExactOperationError::Storage(error)) => Err(error),
            };
        }
        match Integer::try_from_f64_truncated_admitted(value, &mut |layout| admit(layout)) {
            Ok(value) => Ok(target
                .admits_integer(&value)
                .then(|| XsdValue::from_exact_integer(value, target))),
            Err(ExactOperationError::Value(_)) => Ok(None),
            Err(ExactOperationError::Storage(error)) => Err(error),
        }
    };
    Ok(match target {
        XsdDatatype::Double => {
            let number = match source {
                XsdValue::Boolean(value) => Some(f64::from(u8::from(*value))),
                other => numeric_to_f64_admitted(other, admit)?,
            };
            number.map(XsdValue::Double)
        }
        XsdDatatype::Float => {
            let number = match source {
                XsdValue::Boolean(value) => Some(f32::from(u8::from(*value))),
                other => numeric_to_f32_admitted(other, admit)?,
            };
            number.map(XsdValue::Float)
        }
        XsdDatatype::Boolean => match source {
            XsdValue::Boolean(value) => Some(XsdValue::Boolean(*value)),
            other if other.is_numeric() => {
                crate::effective_boolean_value(other).map(XsdValue::Boolean)
            }
            _ => None,
        },
        XsdDatatype::Decimal => match source {
            XsdValue::Integer { value, .. } => Some(XsdValue::Decimal(ExactDecimal::from_integer(
                copy_integer(value, admit)?,
            ))),
            XsdValue::Decimal(value) => {
                let bytes = usize::try_from(value.unscaled().heap_bytes())
                    .map_err(|_| crate::bigint::LimbScratchError::SizeOverflow)?;
                admit(NumericOperationLayout::bytes(bytes, bytes)?)?;
                Some(XsdValue::Decimal(value.try_copy()?))
            }
            XsdValue::Boolean(value) => Some(XsdValue::Decimal(ExactDecimal::from_integer(
                Integer::from_i128(i128::from(*value)),
            ))),
            XsdValue::Float(value) => return exact_binary(f64::from(*value), admit),
            XsdValue::Double(value) => return exact_binary(*value, admit),
            _ => None,
        },
        target if target.is_integer_family() => {
            let value = match source {
                XsdValue::Integer { value, .. } => copy_integer(value, admit)?,
                XsdValue::Decimal(value) => value.try_to_integer_truncated_admitted(admit)?,
                XsdValue::Boolean(value) => Integer::from_i128(i128::from(*value)),
                XsdValue::Float(value) => return exact_binary(f64::from(*value), admit),
                XsdValue::Double(value) => return exact_binary(*value, admit),
                _ => return Ok(None),
            };
            target
                .admits_integer(&value)
                .then(|| XsdValue::from_exact_integer(value, target))
        }
        _ => None,
    })
}

// ── Numeric arithmetic (SPARQL §17.4 / XPath op:numeric-*) ──────────────────

/// Align two decimals to the same (higher) scale by scaling up the mantissa of
/// the lower-scale operand. Returns `(a_mantissa, b_mantissa, common_scale)`, or
/// `None` when the scaled-up mantissa leaves `i128` — the same overflow the
/// following addition/subtraction would report, reached one step earlier.
///
/// The scale-up factor is `10^diff` with `diff ≤ MAX_DECIMAL_SCALE` (18), but
/// the mantissa being scaled is NOT bounded by `10^scale`: a scale-0 decimal
/// may carry any `i128` mantissa (`1e30` is a valid `xsd:decimal`), so
/// `1e30 + 0.000000000000000001` must scale `1e30` by `10^18` — past `i128`.
/// The multiplication is therefore checked; an unchecked one panicked under
/// overflow checks and silently wrapped to a wrong sum without them.
pub(crate) fn align_decimals(a: &Decimal, b: &Decimal) -> Option<(i128, i128, u8)> {
    if a.scale() == b.scale() {
        return Some((a.mantissa(), b.mantissa(), a.scale()));
    }
    if a.scale() > b.scale() {
        let diff = u32::from(a.scale() - b.scale());
        let b_scaled = b.mantissa().checked_mul(10i128.pow(diff))?;
        Some((a.mantissa(), b_scaled, a.scale()))
    } else {
        let diff = u32::from(b.scale() - a.scale());
        let a_scaled = a.mantissa().checked_mul(10i128.pow(diff))?;
        Some((a_scaled, b.mantissa(), b.scale()))
    }
}

/// `op:numeric-add` — the numeric-TIER `+` operator only. Follows the numeric
/// promotion tower: `integer ⊂ decimal ⊂ float ⊂ double`. Integer addition is
/// exact at every magnitude; decimal addition retains the exact coefficient and scale;
/// float/double are IEEE: the sum correctly rounded once into binary32/binary64 on
/// every target, the x87 included ([`crate::ieee`]). `numeric_sub`, `numeric_mul`
/// and `numeric_div` hold the same law.
///
/// This is a narrower contract than [`crate::ops::value_add`], the SPARQL-
/// facing `+`: every call site over an `XsdValue` of unknown family should go
/// through `value_add`, which also accepts SEP-0002's temporal operand pairs;
/// call this function directly only where the operands are already known
/// numeric.
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, numeric_add, parse};
///
/// let a = parse("40", XsdDatatype::Integer)?;
/// let b = parse("2", XsdDatatype::Integer)?;
/// assert_eq!(numeric_add(&a, &b)?.canonical_lexical(), "42");
///
/// // Mixed integer + decimal promotes to decimal, exactly.
/// let half = parse("0.5", XsdDatatype::Decimal)?;
/// assert_eq!(numeric_add(&a, &half)?.canonical_lexical(), "40.5");
///
/// // A non-numeric operand is a type error.
/// let s = parse("oops", XsdDatatype::String)?;
/// assert!(numeric_add(&a, &s).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_add(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() && b.is_exact_numeric() {
        return exact_path::binop(a, b, exact_path::Op::Add);
    }
    numeric_add_bounded(a, b)
}

/// [`numeric_add`] over the bounded variants only.
fn numeric_add_bounded(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Double, Float};
    match (a, b) {
        // Both double OR either double → f64
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            Ok(Double(ieee::f64_add(x, y)))
        }
        // Either float (no double) → f32
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in add",
            })?;
            Ok(Float(ieee::f32_add(x, y)))
        }
        // Either decimal (no float/double) → exact decimal
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in add",
        }),
    }
}

/// `op:numeric-subtract` — the numeric-TIER `-` operator only. Same promotion
/// tower as [`numeric_add`]; see its doc for why [`crate::ops::value_sub`],
/// not this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family.
///
/// Integer and decimal subtraction retain the exact result at every magnitude.
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
pub fn numeric_sub(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() && b.is_exact_numeric() {
        return exact_path::binop(a, b, exact_path::Op::Sub);
    }
    numeric_sub_bounded(a, b)
}

/// [`numeric_sub`] over the bounded variants only.
fn numeric_sub_bounded(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Double, Float};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            Ok(Double(ieee::f64_sub(x, y)))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in sub",
            })?;
            Ok(Float(ieee::f32_sub(x, y)))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in sub",
        }),
    }
}

/// `op:numeric-multiply` — the numeric-TIER `*` operator only. Same promotion
/// tower as [`numeric_add`]; see its doc for why [`crate::ops::value_mul`],
/// not this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family (it also accepts `xsd:duration × xsd:integer|xsd:decimal`,
/// which this function does not).
///
/// Decimal multiplication multiplies the exact coefficients and adds their
/// scales. It introduces no rounding or eighteen-place ceiling. A scale sum
/// exceeding `u32::MAX` returns `XsdError::Exact(ExactError::ScaleOverflow)`.
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
pub fn numeric_mul(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() && b.is_exact_numeric() {
        return exact_path::binop(a, b, exact_path::Op::Mul);
    }
    numeric_mul_bounded(a, b)
}

/// [`numeric_mul`] over the bounded variants only.
fn numeric_mul_bounded(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Double, Float};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            Ok(Double(ieee::f64_mul(x, y)))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in mul",
            })?;
            Ok(Float(ieee::f32_mul(x, y)))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in mul",
        }),
    }
}

/// `op:numeric-divide` — the numeric-TIER `/` operator only. Integer ÷ integer
/// returns **decimal** (not integer), per XPath `op:numeric-divide` semantics.
/// All other pairs follow the numeric promotion tower.
///
/// See [`numeric_add`]'s doc for why [`crate::ops::value_div`], not this
/// function, is the SPARQL-facing entry point for an `XsdValue` of unknown
/// family (it also accepts `xsd:duration ÷ xsd:integer|xsd:decimal|xsd:duration`,
/// which this function does not).
///
/// Division by zero:
/// - `xsd:integer` or `xsd:decimal` divisor = 0 → `Err(DivisionByZero)` (hard error).
/// - `xsd:float` or `xsd:double` divisor = 0.0 → IEEE result (±INF, or NaN for 0÷0).
///
/// Returns `Err(TypeMismatch)` if either operand is not numeric.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, numeric_div, parse};
///
/// // Integer ÷ integer yields DECIMAL (XPath rule), exactly.
/// let seven = parse("7", XsdDatatype::Integer)?;
/// let two = parse("2", XsdDatatype::Integer)?;
/// let q = numeric_div(&seven, &two)?;
/// assert_eq!(q.datatype(), XsdDatatype::Decimal);
/// assert_eq!(q.canonical_lexical(), "3.5");
///
/// // Exact-type division by zero is a hard error (not a saturated value).
/// let zero = parse("0", XsdDatatype::Integer)?;
/// assert!(numeric_div(&seven, &zero).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_div(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    numeric_div_with_policy(a, b, DivisionPolicy::xsd_default())
}

/// [`numeric_div`] with a caller-chosen precision for exact-branch quotients
/// ([`DivisionPolicy`]); `float`/`double` operands divide in IEEE arithmetic as
/// before.
///
/// Under [`DivisionPolicy::xsd_default`] a terminating decimal quotient stays
/// exact; a repeating quotient is rounded to eighteen fractional digits,
/// half to even. Every exact-branch quotient uses the same tower operation,
/// including inputs that fit the older bounded representation.
///
/// # Errors
///
/// `Err(DivisionByZero)` for a zero exact-branch divisor;
/// `Err(Exact(NonTerminating))` under [`DivisionPolicy::Exact`] for a quotient
/// with no finite decimal expansion; `Err(TypeMismatch)` for a non-numeric
/// operand.
///
/// ```rust
/// use purrdf_xsd::exact::DivisionPolicy;
/// use purrdf_xsd::numeric::numeric_div_with_policy;
/// use purrdf_xsd::{XsdDatatype, parse};
///
/// let one = parse("1", XsdDatatype::Integer)?;
/// let eight = parse("8", XsdDatatype::Integer)?;
/// let three = parse("3", XsdDatatype::Integer)?;
/// let exact = DivisionPolicy::Exact;
/// assert_eq!(numeric_div_with_policy(&one, &eight, exact)?.canonical_lexical(), "0.125");
/// assert!(numeric_div_with_policy(&one, &three, exact).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn numeric_div_with_policy(
    a: &XsdValue,
    b: &XsdValue,
    policy: DivisionPolicy,
) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() && b.is_exact_numeric() {
        return exact_path::div(a, b, policy);
    }
    numeric_div_ieee(a, b)
}

/// The IEEE tiers of [`numeric_div`]. Exact pairs were dispatched to the tower
/// before this function, so no bounded decimal quotient is reachable here.
fn numeric_div_ieee(a: &XsdValue, b: &XsdValue) -> Result<XsdValue, XsdError> {
    use XsdValue::{Double, Float};
    match (a, b) {
        (Double(_), _) | (_, Double(_)) => {
            let x = num_f64(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            let y = num_f64(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            Ok(Double(ieee::f64_div(x, y)))
        }
        (Float(_), _) | (_, Float(_)) => {
            let x = num_f32(a).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            let y = num_f32(b).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand in div",
            })?;
            Ok(Float(ieee::f32_div(x, y)))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand in div",
        }),
    }
}

/// `dividend ÷ count` at eighteen fractional digits, truncated toward zero, as a
/// bounded `Decimal` — `None` when that quotient's mantissa does not fit `i128`.
///
/// A 3.0 helper, kept for its callers: the evaluator's `AVG` now divides through
/// [`numeric_div_with_policy`] like `/`, which answers every quotient exactly at
/// eighteen digits whatever its size. `numeric_div_with_policy(&XsdValue::from_exact_integer(..), &count,
/// DivisionPolicy::default())` is the same quotient with no `None` case.
///
/// `count` must be nonzero; `AVG`'s one caller only reaches this with a folded
/// row count, which is never zero for a non-empty group.
///
/// # Panics
///
/// If `count == 0` — the caller's contract, not a runtime input.
#[must_use]
#[cfg(test)]
fn bigint_avg_decimal(dividend: &crate::bigint::BigInt, count: u64) -> Option<XsdValue> {
    assert!(
        count != 0,
        "AVG's divisor is a folded row count, never zero"
    );
    let scaled = dividend.mul_pow10(u32::from(MAX_DECIMAL_SCALE));
    let (quotient, _remainder) = scaled
        .div_rem_u64(count)
        .expect("count != 0 was just asserted");
    let mantissa = quotient.to_i128()?;
    Some(XsdValue::Decimal(crate::exact::Decimal::from_bounded(
        &Decimal::from_parts(mantissa, MAX_DECIMAL_SCALE),
    )))
}

/// [`bigint_avg_decimal`]'s quotient as canonical `xsd:decimal` lexical text, at any
/// size: the exact eighteen-digit quotient, truncated toward zero, rendered through
/// [`crate::bigint::BigInt::to_decimal_lexical`]. The text always reads back
/// through [`crate::parse`] as the same exact decimal value, so it is a value of the decimal value
/// space, like every result of [`numeric_div_with_policy`], which computes the same
/// quotient as a value.
///
/// Always succeeds: `BigInt` has no magnitude bound beyond available memory, so
/// unlike [`bigint_avg_decimal`] this has no `None` case at all.
///
/// # Panics
///
/// If `count == 0` — the caller's contract, not a runtime input (identical to
/// [`bigint_avg_decimal`]).
#[must_use]
#[cfg(test)]
fn bigint_avg_decimal_lexical(dividend: &crate::bigint::BigInt, count: u64) -> String {
    assert!(
        count != 0,
        "AVG's divisor is a folded row count, never zero"
    );
    let scaled = dividend.mul_pow10(u32::from(MAX_DECIMAL_SCALE));
    let (quotient, _remainder) = scaled
        .div_rem_u64(count)
        .expect("count != 0 was just asserted");
    quotient.to_decimal_lexical(u32::from(MAX_DECIMAL_SCALE))
}

/// `op:numeric-unary-minus` — the numeric-TIER unary `-` only. Negates the
/// value, preserving its type.
///
/// See [`numeric_add`]'s doc for why [`crate::ops::value_unary_minus`], not
/// this function, is the SPARQL-facing entry point for an `XsdValue` of
/// unknown family (it also accepts `xsd:duration`, a PurRDF extension this
/// function does not implement).
///
/// Integer and decimal negation are exact, including the negation of values
/// beyond `i128`. For float/double, IEEE negation (−0.0 negates to +0.0 and
/// vice-versa; NaN negates to NaN with sign flipped per IEEE 754-2008 §6.3).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_unary_minus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() {
        return Ok(exact_path::neg(a));
    }
    numeric_unary_minus_bounded(a)
}

/// [`numeric_unary_minus`] over the bounded variants only.
fn numeric_unary_minus_bounded(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(-f)),
        XsdValue::Double(d) => Ok(XsdValue::Double(-d)),
        _ => Err(XsdError::TypeMismatch {
            reason: "unary minus applied to non-numeric value",
        }),
    }
}

/// The `xsd:integer` result of a unary operator over an integer-family operand.
///
/// F&O 3.1 §4.2 types these results as `xs:integer`, and only permits a subtype
/// when the value is in it: `-5` is not an `xsd:unsignedByte`, so keeping the
/// operand's subtype would mint a literal outside its own value space.
const fn integer_result(value: i128) -> XsdValue {
    XsdValue::Integer {
        value: crate::exact::Integer::from_i128(value),
        datatype: XsdDatatype::Integer,
    }
}

/// SPARQL `op:numeric-unary-plus` (unary `+`). Identity for numeric types.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands (e.g. `+true` is a type
/// error in SPARQL/XPath, not a no-op).
pub fn numeric_unary_plus(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Integer { .. }
        | XsdValue::Decimal(_)
        | XsdValue::Float(_)
        | XsdValue::Double(_) => Ok(a.clone()),
        _ => Err(XsdError::TypeMismatch {
            reason: "unary plus applied to non-numeric value",
        }),
    }
}

// ── Numeric math functions (SPARQL §17.4.4 / XPath fn:abs, fn:ceiling, etc.) ──

/// SPARQL `fn:abs` — absolute value, preserving the operand's numeric type.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_abs(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() {
        return Ok(exact_path::abs(a));
    }
    numeric_abs_bounded(a)
}

/// [`numeric_abs`] over the bounded variants only.
fn numeric_abs_bounded(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(f.abs())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.abs())),
        _ => Err(XsdError::TypeMismatch {
            reason: "abs applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:ceiling` — smallest integer not less than the value, preserving the
/// operand's numeric type (`integer → integer`, `decimal → decimal` exact,
/// `float/double → float/double`).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_ceil(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() {
        return Ok(exact_path::round_to_integer(
            a,
            crate::exact::Rounding::Ceiling,
        ));
    }
    numeric_ceil_bounded(a)
}

/// [`numeric_ceil`] over the bounded variants only.
fn numeric_ceil_bounded(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(f.ceil())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.ceil())),
        _ => Err(XsdError::TypeMismatch {
            reason: "ceiling applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:floor` — largest integer not greater than the value, preserving the
/// operand's numeric type (`integer → integer`, `decimal → decimal` exact,
/// `float/double → float/double`).
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_floor(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() {
        return Ok(exact_path::round_to_integer(
            a,
            crate::exact::Rounding::Floor,
        ));
    }
    numeric_floor_bounded(a)
}

/// [`numeric_floor`] over the bounded variants only.
fn numeric_floor_bounded(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => Ok(XsdValue::Float(f.floor())),
        XsdValue::Double(d) => Ok(XsdValue::Double(d.floor())),
        _ => Err(XsdError::TypeMismatch {
            reason: "floor applied to non-numeric value",
        }),
    }
}

/// SPARQL `fn:round` — round to the nearest integer, with half-values rounded
/// toward positive infinity (`fn:round` semantics per XPath §4.4.5). Preserves the
/// operand's numeric type.
///
/// Examples: `round(2.5) = 3`, `round(-2.5) = -2`, `round(2.4999) = 2`.
///
/// Returns `Err(TypeMismatch)` for non-numeric operands.
pub fn numeric_round(a: &XsdValue) -> Result<XsdValue, XsdError> {
    if a.is_exact_numeric() {
        return Ok(exact_path::round_to_integer(
            a,
            crate::exact::Rounding::HalfCeiling,
        ));
    }
    numeric_round_bounded(a)
}

/// [`numeric_round`] over the bounded variants only.
fn numeric_round_bounded(a: &XsdValue) -> Result<XsdValue, XsdError> {
    match a {
        XsdValue::Float(f) => {
            // f32::round() is round-half-away-from-zero; XPath fn:round is
            // round-half-toward-+infinity. For positive they agree. For negative halves
            // they differ: f32::round(-2.5) = -3 but fn:round(-2.5) = -2.
            // Correction: for negative values at the half-point, add 1.0.
            let r = if *f == ieee::f32_add(f.floor(), 0.5) && *f < 0.0 {
                f.ceil()
            } else {
                f.round()
            };
            Ok(XsdValue::Float(r))
        }
        XsdValue::Double(d) => {
            // Same correction as float.
            let r = if *d == ieee::f64_add(d.floor(), 0.5) && *d < 0.0 {
                d.ceil()
            } else {
                d.round()
            };
            Ok(XsdValue::Double(r))
        }
        _ => Err(XsdError::TypeMismatch {
            reason: "round applied to non-numeric value",
        }),
    }
}

#[cfg(test)]
mod tests {
    fn text(prepared: Option<PreparedNumericText>) -> String {
        let prepared = prepared.expect("numeric");
        let mut out = String::with_capacity(prepared.len());
        prepared.write_to(&mut out).expect("fits");
        assert_eq!(out.len(), prepared.len(), "the prepared length is exact");
        out
    }

    /// A computed decimal carries a point and a fractional digit; the XSD 1.1
    /// canonical text of a stored decimal, an integer, and a decimal that already
    /// has a fraction are unchanged by it.
    #[test]
    fn a_computed_whole_decimal_carries_a_point_and_nothing_else_changes() {
        let decimal =
            |lexical: &str| crate::value::parse(lexical, XsdDatatype::Decimal).expect("decimal");
        for (lexical, computed, canonical) in [
            ("4", "4.0", "4"),
            ("-3", "-3.0", "-3"),
            ("0", "0.0", "0"),
            ("2.5", "2.5", "2.5"),
            ("-0.025", "-0.025", "-0.025"),
        ] {
            let value = decimal(lexical);
            assert_eq!(
                text(prepare_computed_numeric_text(&value, &mut |_| Ok(())).unwrap()),
                computed
            );
            assert_eq!(
                text(prepare_numeric_text(&value, &mut |_| Ok(())).unwrap()),
                canonical
            );
        }
        let integer = crate::value::parse("4", XsdDatatype::Integer).expect("integer");
        assert_eq!(
            text(prepare_computed_numeric_text(&integer, &mut |_| Ok(())).unwrap()),
            "4"
        );
    }

    /// The render cost of a computed wide decimal prices the text it emits: a whole
    /// value carries `.0`, two bytes past its canonical form, and the charge
    /// follows. Neighbours: a wide fraction and a bounded value gain nothing.
    #[test]
    fn computed_render_cost_prices_the_emitted_form() {
        let decimal = |lexical: &str| crate::value::parse(lexical, XsdDatatype::Decimal).unwrap();
        let whole = decimal(&format!("1{}", "0".repeat(60)));
        let XsdValue::Decimal(inner) = &whole else {
            unreachable!("a decimal parses to a decimal");
        };
        assert!(inner.unscaled().as_i128().is_none());
        let canonical = whole.canonical_lexical().len();
        let emitted = prepare_computed_numeric_text(&whole, &mut |_| Ok(())).unwrap();
        assert_eq!(text(emitted).len(), canonical + 2);
        let plain = numeric_render_cost(&whole);
        let computed = computed_numeric_render_cost(&whole);
        assert_eq!(computed.bytes() - plain.bytes(), 2 * 6);
        assert!(computed.work() > plain.work());
        // Neighbours.
        let fraction = decimal(&format!("1{}.5", "0".repeat(60)));
        assert_eq!(
            computed_numeric_render_cost(&fraction),
            numeric_render_cost(&fraction)
        );
        let small = decimal("4");
        assert_eq!(
            computed_numeric_render_cost(&small),
            numeric_render_cost(&small)
        );
    }

    #[test]
    fn bounded_ieee_canonical_writers_preserve_frozen_extremes() {
        for (value, expected) in [
            (0.0, "0.0E0"),
            (-0.0, "-0.0E0"),
            (f64::from_bits(1), "5.0E-324"),
            (-f64::from_bits(1), "-5.0E-324"),
            (f64::from_bits((1u64 << 52) - 1), "2.225073858507201E-308"),
            (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
            (-f64::MIN_POSITIVE, "-2.2250738585072014E-308"),
            (f64::MAX, "1.7976931348623157E308"),
            (-f64::MAX, "-1.7976931348623157E308"),
            (f64::INFINITY, "INF"),
            (f64::NEG_INFINITY, "-INF"),
            (f64::NAN, "NaN"),
        ] {
            let out = canonical_double_text(value);
            assert_eq!(out.as_str(), expected);
            assert_eq!(canonical_double(value), expected);
        }
        for (value, expected) in [
            (0.0, "0.0E0"),
            (-0.0, "-0.0E0"),
            (0.1, "1.0E-1"),
            (f32::from_bits(1), "1.0E-45"),
            (f32::MIN_POSITIVE, "1.1754944E-38"),
            (-f32::MIN_POSITIVE, "-1.1754944E-38"),
            (f32::MAX, "3.4028235E38"),
            (-f32::MAX, "-3.4028235E38"),
            (f32::INFINITY, "INF"),
            (f32::NEG_INFINITY, "-INF"),
            (f32::NAN, "NaN"),
        ] {
            let out = canonical_float_text(value);
            assert_eq!(out.as_str(), expected);
            assert_eq!(canonical_float(value), expected);
        }
        assert!("-2.2250738585072014E-308".len() <= CanonicalFloatText::empty().bytes.len());
    }

    #[test]
    fn bounded_double_writer_round_trips_every_binade_and_refuses_full_sinks() {
        let fraction_mask = (1u64 << 52) - 1;
        for exponent in 0..2047u64 {
            for fraction in [0, 1, fraction_mask / 2, fraction_mask - 1, fraction_mask] {
                for sign in [0, 1u64 << 63] {
                    let value = f64::from_bits(sign | (exponent << 52) | fraction);
                    let out = canonical_double_text(value);
                    assert_eq!(
                        out.as_str().parse::<f64>().unwrap().to_bits(),
                        value.to_bits()
                    );
                    assert!(out.len() <= out.bytes.len());
                    assert_eq!(canonical_double(value), out.as_str());
                }
            }
        }
        let mut full = CanonicalFloatText::empty();
        full.length = full.bytes.len();
        let before = full.bytes;
        assert_eq!(
            std::fmt::Write::write_str(&mut full, "1"),
            Err(std::fmt::Error)
        );
        assert_eq!(full.bytes, before);
        assert_eq!(full.length, full.bytes.len());
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64"))]
    #[test]
    fn canonical_ieee_original_zero_bits_survive_input_flush() {
        use ieee::control;

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        let _guard = {
            if !control::mxcsr_available() {
                return;
            }
            // DAZ may make a floating comparison read a subnormal as zero;
            // FTZ also exercises a changed environment with input bits intact.
            control::Mxcsr::load(control::mxcsr() | (1 << 6) | control::MXCSR_FTZ)
        };
        #[cfg(target_arch = "aarch64")]
        let _guard = control::Fpcr::load(control::fpcr() | (1 << 24) | 1);

        for (bits, expected) in [
            (0, "0.0E0"),
            (1u64 << 63, "-0.0E0"),
            (1, "5.0E-324"),
            ((1u64 << 63) | 1, "-5.0E-324"),
            ((1u64 << 52) - 1, "2.225073858507201E-308"),
        ] {
            let value = f64::from_bits(std::hint::black_box(bits));
            let out = canonical_double_text(value);
            assert_eq!(out.as_str(), expected);
            assert_eq!(canonical_double(value), expected);
            assert_eq!(
                xpath_double_text(value).as_str(),
                expected.strip_suffix(".0E0").unwrap_or(expected)
            );
        }
        for (bits, expected) in [
            (0, "0.0E0"),
            (1u32 << 31, "-0.0E0"),
            (1, "1.0E-45"),
            ((1u32 << 31) | 1, "-1.0E-45"),
            ((1u32 << 23) - 1, "1.1754942E-38"),
        ] {
            let value = f32::from_bits(std::hint::black_box(bits));
            let out = canonical_float_text(value);
            assert_eq!(out.as_str(), expected);
            assert_eq!(canonical_float(value), expected);
            assert_eq!(
                xpath_float_text(value).as_str(),
                expected.strip_suffix(".0E0").unwrap_or(expected)
            );
        }
    }
    use super::*;
    use crate::XsdDatatype as D;

    /// The pre-single-buffer `canonical_lexical` (split/pad/`format!` form), kept
    /// verbatim as the byte-for-byte oracle for the exact-fit rewrite.
    fn canonical_lexical_reference(d: &Decimal) -> String {
        let neg = d.mantissa < 0;
        let digits = d.mantissa.unsigned_abs().to_string();
        let scale = usize::from(d.scale);

        let (int_part, frac_part) = if scale == 0 {
            (digits, String::new())
        } else if digits.len() > scale {
            let split = digits.len() - scale;
            (digits[..split].to_string(), digits[split..].to_string())
        } else {
            let pad = "0".repeat(scale - digits.len());
            ("0".to_string(), format!("{pad}{digits}"))
        };

        let frac_trimmed = frac_part.trim_end_matches('0');
        let sign = if neg { "-" } else { "" };
        if frac_trimmed.is_empty() {
            format!("{sign}{int_part}")
        } else {
            format!("{sign}{int_part}.{frac_trimmed}")
        }
    }

    #[test]
    fn canonical_lexical_matches_reference_over_mantissa_scale_grid() {
        let mantissas: [i128; 24] = [
            0,
            1,
            -1,
            5,
            -5,
            10,
            -10,
            100,
            -100,
            1_000_000,
            -1_000_000,
            123,
            -123,
            120,
            -120,
            102_030,
            -102_030,
            1_234_567_890_123_456_789,
            -1_234_567_890_123_456_789,
            10_000_000_000_000_000_000,
            -10_000_000_000_000_000_000,
            i128::MAX,
            i128::MIN,
            i128::MIN + 1,
        ];
        for &mantissa in &mantissas {
            for scale in 0..=MAX_DECIMAL_SCALE {
                let d = Decimal::from_parts(mantissa, scale);
                assert_eq!(
                    d.canonical_lexical(),
                    canonical_lexical_reference(&d),
                    "mantissa = {mantissa}, scale = {scale}"
                );
            }
        }
    }

    fn parse_bounded_decimal(s: &str) -> Result<Decimal, XsdError> {
        read_decimal(s).map_err(|error| error.owned(XsdDatatype::Decimal, s))
    }

    fn dec(s: &str) -> Decimal {
        parse_bounded_decimal(s).unwrap()
    }

    fn dec_value(s: &str) -> XsdValue {
        XsdValue::Decimal(crate::exact::Decimal::from_bounded(&dec(s)))
    }

    fn xsd_decimal(decimal: Decimal) -> XsdValue {
        XsdValue::Decimal(crate::exact::Decimal::from_bounded(&decimal))
    }

    fn int_val(n: i128) -> XsdValue {
        XsdValue::Integer {
            value: crate::exact::Integer::from_i128(n),
            datatype: D::Integer,
        }
    }

    // ── the exact numeric order ─────────────────────────────────────────────

    /// THE CYCLE the exact order exists to close.
    ///
    /// Under §17.3 promotion these three literals form `a > b`, `a = c`, `b = c`,
    /// which no transitive relation can hold — and `ORDER BY` hands its comparator
    /// to a Rust sort, which may panic on one. The exact order sees the nineteenth
    /// significant digit and answers consistently.
    #[test]
    fn the_promotion_cycle_at_nineteen_significant_digits_is_closed() {
        let a = dec_value("1.000000000000000001");
        let b = int_val(1);
        let c = XsdValue::Double(1.0);

        // The promoted relation, as SPARQL `<` still means it.
        assert_eq!(numeric_cmp(&a, &b), Some(Ordering::Greater));
        assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
        assert_eq!(numeric_cmp(&b, &c), Some(Ordering::Equal));

        // The exact relation, which `ORDER BY` uses.
        assert_eq!(numeric_total_cmp(&a, &b), Some(Ordering::Greater));
        assert_eq!(numeric_total_cmp(&a, &c), Some(Ordering::Greater));
        assert_eq!(numeric_total_cmp(&b, &c), Some(Ordering::Equal));
        assert_eq!(numeric_total_cmp(&c, &a), Some(Ordering::Less));
    }

    /// The same failure one type up: `2^53 + 1` is an ordinary `xsd:integer` and an
    /// unrepresentable `xsd:double`, so promotion rounds it onto `2^53`.
    #[test]
    fn an_integer_beyond_the_double_significand_stays_distinct() {
        let big = int_val((1i128 << 53) + 1);
        let rounded = XsdValue::Double((1u64 << 53) as f64);
        assert_eq!(numeric_cmp(&big, &rounded), Some(Ordering::Equal));
        assert_eq!(numeric_total_cmp(&big, &rounded), Some(Ordering::Greater));
        assert_eq!(numeric_total_cmp(&rounded, &big), Some(Ordering::Less));
        // And the same magnitude one below is genuinely smaller, both ways.
        let smaller = int_val((1i128 << 53) - 1);
        assert_eq!(numeric_total_cmp(&smaller, &rounded), Some(Ordering::Less));
    }

    /// `xsd:float` is exercised on the same seam, and `f32 → f64` widening must not
    /// be what decides it: `0.1f32` is a DIFFERENT rational from `0.1` the decimal.
    #[test]
    fn a_float_operand_is_compared_as_the_dyadic_rational_it_is() {
        let tenth = dec_value("0.1");
        let float_tenth = XsdValue::Float(0.1f32);
        let double_tenth = XsdValue::Double(0.1f64);
        // 0.1f32 = 13421773 / 2^27 > 1/10; 0.1f64 = 3602879701896397 / 2^55 > 1/10.
        assert_eq!(
            numeric_total_cmp(&tenth, &float_tenth),
            Some(Ordering::Less)
        );
        assert_eq!(
            numeric_total_cmp(&tenth, &double_tenth),
            Some(Ordering::Less)
        );
        // ...and the f32 rounding is coarser, so it overshoots further.
        assert_eq!(
            numeric_total_cmp(&double_tenth, &float_tenth),
            Some(Ordering::Less)
        );
        // A dyadic decimal is EXACTLY its IEEE counterpart, in both widths.
        let half = dec_value("0.5");
        assert_eq!(
            numeric_total_cmp(&half, &XsdValue::Float(0.5f32)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            numeric_total_cmp(&half, &XsdValue::Double(0.5f64)),
            Some(Ordering::Equal)
        );
    }

    /// The specials keep their places, and `NaN` keeps its absence of one.
    #[test]
    fn the_specials_and_the_zeroes_order_as_the_value_space_says() {
        let one = int_val(1);
        for infinite in [
            XsdValue::Double(f64::INFINITY),
            XsdValue::Float(f32::INFINITY),
        ] {
            assert_eq!(numeric_total_cmp(&one, &infinite), Some(Ordering::Less));
            assert_eq!(numeric_total_cmp(&infinite, &one), Some(Ordering::Greater));
        }
        for infinite in [
            XsdValue::Double(f64::NEG_INFINITY),
            XsdValue::Float(f32::NEG_INFINITY),
        ] {
            assert_eq!(numeric_total_cmp(&one, &infinite), Some(Ordering::Greater));
            assert_eq!(numeric_total_cmp(&infinite, &one), Some(Ordering::Less));
        }
        assert_eq!(
            numeric_total_cmp(
                &XsdValue::Double(f64::INFINITY),
                &XsdValue::Float(f32::INFINITY)
            ),
            Some(Ordering::Equal)
        );
        // Both zeroes are one value, whichever spelling and whichever branch.
        for zero in [
            XsdValue::Double(0.0),
            XsdValue::Double(-0.0),
            XsdValue::Float(-0.0f32),
        ] {
            assert_eq!(numeric_total_cmp(&int_val(0), &zero), Some(Ordering::Equal));
            assert_eq!(
                numeric_total_cmp(&dec_value("0.000"), &zero),
                Some(Ordering::Equal)
            );
        }
        // NaN has no place in the order at all — never a silent "greater".
        for nan in [XsdValue::Double(f64::NAN), XsdValue::Float(f32::NAN)] {
            assert_eq!(numeric_total_cmp(&one, &nan), None);
            assert_eq!(numeric_total_cmp(&nan, &one), None);
            assert_eq!(numeric_total_cmp(&nan, &nan), None);
        }
    }

    /// The magnitude window and the `BigInt` fallback: a double far outside the
    /// exact side's reach, a subnormal far under it, and a pair close enough that
    /// the answer needs the exact cross-multiplication.
    #[test]
    fn magnitudes_outside_and_inside_the_aliasing_window_both_decide_exactly() {
        let biggest = xsd_decimal(Decimal::from_parts(i128::MAX, 0));
        let smallest = xsd_decimal(Decimal::from_parts(1, 18));
        let most_negative = xsd_decimal(Decimal::from_parts(i128::MIN, 0));

        // Outside the window on the high side: 1e300 dwarfs any i128 mantissa.
        assert_eq!(
            numeric_total_cmp(&biggest, &XsdValue::Double(1e300)),
            Some(Ordering::Less)
        );
        assert_eq!(
            numeric_total_cmp(&most_negative, &XsdValue::Double(-1e300)),
            Some(Ordering::Greater)
        );
        // Outside on the low side: the smallest subnormal is under 1e-18.
        assert_eq!(
            numeric_total_cmp(&smallest, &XsdValue::Double(f64::from_bits(1))),
            Some(Ordering::Greater)
        );
        assert_eq!(
            numeric_total_cmp(&XsdValue::Double(f64::from_bits(1)), &smallest),
            Some(Ordering::Less)
        );

        // Inside the window and genuinely close: 2^127 as a double is exactly
        // representable, and i128::MAX = 2^127 - 1 sits one below it.
        let two_127 = XsdValue::Double(170_141_183_460_469_231_731_687_303_715_884_105_728.0);
        assert_eq!(numeric_total_cmp(&biggest, &two_127), Some(Ordering::Less));
        assert_eq!(
            numeric_total_cmp(
                &most_negative,
                &XsdValue::Double(-170_141_183_460_469_231_731_687_303_715_884_105_728.0)
            ),
            Some(Ordering::Equal),
            "i128::MIN is exactly -2^127, which is exactly a double"
        );
        // A dyadic decimal at full scale still lands exactly on its double.
        let five_tenths_at_scale_18 = xsd_decimal(Decimal::from_parts(500_000_000_000_000_000, 18));
        assert_eq!(
            numeric_total_cmp(&five_tenths_at_scale_18, &XsdValue::Double(0.5)),
            Some(Ordering::Equal)
        );
    }

    /// The two ways a `u128` cross-multiplication runs out of room, both still
    /// inside the aliasing window so the cheap bound cannot answer for them. The
    /// `BigInt` fallback is what decides these, and it must decide them exactly.
    #[test]
    fn the_bigint_fallback_answers_the_pairs_that_overflow_a_u128() {
        // Non-negative IEEE exponent: `significand × 2^exp × 10^18` exceeds 2^128
        // whenever |ieee| is past ~3.4e20, while the exact side at scale 18 tops out
        // near 1.7e20.
        let biggest_at_full_scale = xsd_decimal(Decimal::from_parts(i128::MAX, 18));
        assert_eq!(
            numeric_total_cmp(&biggest_at_full_scale, &XsdValue::Double(1e21)),
            Some(Ordering::Less)
        );
        assert_eq!(
            numeric_total_cmp(&XsdValue::Double(1e21), &biggest_at_full_scale),
            Some(Ordering::Greater)
        );

        // Negative IEEE exponent: `mantissa × 2^-exp` exceeds 2^128 once the mantissa
        // is near the i128 ceiling and the double needs two or more fractional bits
        // (`0.5` alone would not — `(2^127 - 1) << 1` still fits `u128`).
        let biggest = xsd_decimal(Decimal::from_parts(i128::MAX, 0));
        assert_eq!(
            numeric_total_cmp(&biggest, &XsdValue::Double(0.25)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            numeric_total_cmp(&XsdValue::Double(0.25), &biggest),
            Some(Ordering::Less)
        );
        // And the near-tie, one ulp of the EXACT side at a magnitude where a
        // promotion through `f64` could not have resolved it at all: `2^126` is
        // exactly a double, and the two decimals either side of it are not.
        let two_126 = xsd_decimal(Decimal::from_parts(1i128 << 126, 0));
        assert_eq!(
            numeric_total_cmp(
                &two_126,
                &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
            ),
            Some(Ordering::Equal)
        );
        assert_eq!(
            numeric_total_cmp(
                &xsd_decimal(Decimal::from_parts((1i128 << 126) - 1, 0)),
                &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
            ),
            Some(Ordering::Less)
        );
        assert_eq!(
            numeric_total_cmp(
                &xsd_decimal(Decimal::from_parts((1i128 << 126) + 1, 0)),
                &XsdValue::Double(85_070_591_730_234_615_865_843_651_857_942_052_864.0)
            ),
            Some(Ordering::Greater)
        );
    }

    /// Exhaustive antisymmetry and transitivity over a sample that spans every
    /// branch of the tower and every seam between them. A cycle anywhere here is a
    /// sort that may abort the process, so this is the property that must not rot.
    #[test]
    fn the_exact_numeric_order_is_a_total_order_over_the_whole_tower() {
        let samples = vec![
            XsdValue::Double(f64::NEG_INFINITY),
            XsdValue::Float(f32::NEG_INFINITY),
            xsd_decimal(Decimal::from_parts(i128::MIN, 0)),
            int_val(-3),
            dec_value("-1.000000000000000001"),
            XsdValue::Float(-1.0f32),
            int_val(-1),
            XsdValue::Double(-0.0),
            int_val(0),
            dec_value("0.000"),
            XsdValue::Double(f64::from_bits(1)),
            xsd_decimal(Decimal::from_parts(1, 18)),
            dec_value("0.1"),
            XsdValue::Double(0.1f64),
            XsdValue::Float(0.1f32),
            dec_value("0.5"),
            XsdValue::Float(0.5f32),
            int_val(1),
            XsdValue::Double(1.0),
            XsdValue::Float(1.0f32),
            dec_value("1.000000000000000001"),
            int_val((1i128 << 53) - 1),
            XsdValue::Double((1u64 << 53) as f64),
            int_val((1i128 << 53) + 1),
            xsd_decimal(Decimal::from_parts(i128::MAX, 0)),
            XsdValue::Double(1e300),
            XsdValue::Double(f64::INFINITY),
            XsdValue::Float(f32::INFINITY),
        ];

        for (i, a) in samples.iter().enumerate() {
            for (j, b) in samples.iter().enumerate() {
                let forward = numeric_total_cmp(a, b).expect("no NaN in the sample");
                let back = numeric_total_cmp(b, a).expect("no NaN in the sample");
                assert_eq!(forward, back.reverse(), "not antisymmetric at ({i}, {j})");
            }
        }
        for (i, a) in samples.iter().enumerate() {
            for (j, b) in samples.iter().enumerate() {
                if numeric_total_cmp(a, b) != Some(Ordering::Less)
                    && numeric_total_cmp(a, b) != Some(Ordering::Equal)
                {
                    continue;
                }
                for (k, c) in samples.iter().enumerate() {
                    if numeric_total_cmp(b, c) == Some(Ordering::Greater) {
                        continue;
                    }
                    assert_ne!(
                        numeric_total_cmp(a, c),
                        Some(Ordering::Greater),
                        "not transitive at ({i}, {j}, {k})"
                    );
                }
            }
        }

        // And the whole sample sorts through it without tripping the total-order
        // check `sort_by` runs in debug builds.
        let mut order: Vec<usize> = (0..samples.len()).collect();
        order.sort_by(|x, y| {
            numeric_total_cmp(&samples[*x], &samples[*y]).expect("no NaN in the sample")
        });
        assert_eq!(order.len(), samples.len());
    }

    /// The promoted relation is left EXACTLY as it was: this is the divergence
    /// stated as a test, so nobody can "tidy" `numeric_cmp` into the exact order and
    /// silently change what SPARQL `<` and `=` mean.
    #[test]
    fn the_promotion_based_operator_relation_is_untouched() {
        let a = dec_value("1.000000000000000001");
        let c = XsdValue::Double(1.0);
        assert!(
            numeric_eq(&a, &c),
            "SPARQL `=` still promotes, and still rounds"
        );
        assert_eq!(numeric_cmp(&a, &c), Some(Ordering::Equal));
        assert_ne!(numeric_total_cmp(&a, &c), numeric_cmp(&a, &c));
    }

    #[test]
    fn integer_parse_and_bounds() {
        let small = |text| parse_integer(text).unwrap().as_i128();
        assert_eq!(small("42"), Some(42));
        assert_eq!(small("-7"), Some(-7));
        assert_eq!(small("+7"), Some(7));
        assert_eq!(small("007"), Some(7));
        assert_eq!(small(&i128::MAX.to_string()), Some(i128::MAX));
        // The public reader keeps the first value past i128 exactly.
        let past = "170141183460469231731687303715884105728";
        let parsed = parse_integer(past).unwrap();
        assert_eq!(parsed.as_i128(), None);
        assert_eq!(parsed.canonical_lexical(), past);
        assert!(parse_integer("1.0").is_err());
        assert!(parse_integer("").is_err());
        assert!(parse_integer("abc").is_err());
    }

    #[test]
    fn parse_integer_typed_range_checks() {
        let small = |text, datatype| parse_integer_typed(text, datatype).unwrap().as_i128();
        // xsd:byte: -128..127
        assert_eq!(small("127", D::Byte), Some(127));
        assert_eq!(small("-128", D::Byte), Some(-128));
        assert!(parse_integer_typed("128", D::Byte).is_err());
        assert!(parse_integer_typed("-129", D::Byte).is_err());

        // xsd:unsignedByte: 0..255
        assert_eq!(small("255", D::UnsignedByte), Some(255));
        assert_eq!(small("0", D::UnsignedByte), Some(0));
        assert!(parse_integer_typed("256", D::UnsignedByte).is_err());
        assert!(parse_integer_typed("-1", D::UnsignedByte).is_err());

        // xsd:positiveInteger: >= 1
        assert_eq!(small("1", D::PositiveInteger), Some(1));
        assert!(parse_integer_typed("0", D::PositiveInteger).is_err());

        // xsd:negativeInteger: <= -1
        assert_eq!(small("-1", D::NegativeInteger), Some(-1));
        assert!(parse_integer_typed("0", D::NegativeInteger).is_err());

        // xsd:nonNegativeInteger: >= 0
        assert_eq!(small("0", D::NonNegativeInteger), Some(0));
        assert!(parse_integer_typed("-1", D::NonNegativeInteger).is_err());

        // xsd:nonPositiveInteger: <= 0
        assert_eq!(small("0", D::NonPositiveInteger), Some(0));
        assert!(parse_integer_typed("1", D::NonPositiveInteger).is_err());

        // xsd:unsignedLong boundary: u64::MAX should pass; u64::MAX+1 should fail
        let u64max = u64::MAX.to_string();
        assert_eq!(small(&u64max, D::UnsignedLong), Some(i128::from(u64::MAX)));
        assert!(parse_integer_typed("18446744073709551616", D::UnsignedLong).is_err());

        // xsd:int: 2147483647 ok, 2147483648 fails
        assert_eq!(small("2147483647", D::Int), Some(2_147_483_647));
        assert!(parse_integer_typed("2147483648", D::Int).is_err());

        let past = "170141183460469231731687303715884105728";
        assert_eq!(
            parse_integer_typed(past, D::NonNegativeInteger)
                .unwrap()
                .canonical_lexical(),
            past
        );
        assert!(matches!(
            parse_integer_typed(past, D::NegativeInteger),
            Err(XsdError::OutOfRange { .. })
        ));
    }

    #[test]
    fn decimal_parse_and_canonical() {
        assert_eq!(dec("12.34").canonical_lexical(), "12.34");
        // XSD 1.1 §E.1 `decimalCanonicalMap`: integer-valued decimals have no point.
        assert_eq!(dec("12.00").canonical_lexical(), "12");
        assert_eq!(dec("100").canonical_lexical(), "100");
        assert_eq!(dec("-0.5").canonical_lexical(), "-0.5");
        assert_eq!(dec(".5").canonical_lexical(), "0.5");
        assert_eq!(dec("1.").canonical_lexical(), "1");
        assert_eq!(dec("0.005").canonical_lexical(), "0.005");
        assert!(parse_decimal("1.2.3").is_err());
        assert!(parse_decimal("").is_err());
    }

    #[test]
    fn round_decimal_nearest_ties_toward_positive_infinity() {
        // `fn:round` = floor(x + 0.5): round to nearest, ties toward +infinity.
        let round = |s: &str| as_decimal(&numeric_round(&dec_val(s)).unwrap());
        // Positive: half rounds up, sub-half stays.
        assert_eq!(round("2.5").cmp_exact(&dec("3")), Ordering::Equal);
        assert_eq!(round("1.1").cmp_exact(&dec("1")), Ordering::Equal);
        // Negative below the half-point rounds AWAY from zero (the fixed bug:
        // `-1.6` must be `-2`, not `-1`).
        assert_eq!(round("-1.6").cmp_exact(&dec("-2")), Ordering::Equal);
        // Negative sub-half stays toward zero.
        assert_eq!(round("-1.4").cmp_exact(&dec("-1")), Ordering::Equal);
        // Negative ties round toward +infinity (toward zero): `-1.5` → `-1`,
        // `-2.5` → `-2`.
        assert_eq!(round("-1.5").cmp_exact(&dec("-1")), Ordering::Equal);
        assert_eq!(round("-2.5").cmp_exact(&dec("-2")), Ordering::Equal);
    }

    #[test]
    fn decimal_exact_comparison_across_scales() {
        assert_eq!(dec("1.5").cmp_exact(&dec("1.50")), Ordering::Equal);
        assert_eq!(dec("1.5").cmp_exact(&dec("1.05")), Ordering::Greater);
        assert_eq!(dec("0.1").cmp_exact(&dec("0.2")), Ordering::Less);
    }

    // ── cmp_exact correctness tests ──────────────────────────────────────────────

    /// Cross-scale equality: 1.50 (mantissa=150, scale=2) == 1.5 (mantissa=15, scale=1).
    #[test]
    fn cmp_exact_cross_scale_equal() {
        let a = Decimal::from_parts(150, 2); // 1.50
        let b = Decimal::from_parts(15, 1); // 1.5
        assert_eq!(a.cmp_exact(&b), Ordering::Equal);
        assert_eq!(b.cmp_exact(&a), Ordering::Equal);
    }

    /// Cross-scale strict order: 1.5 < 1.50001.
    #[test]
    fn cmp_exact_cross_scale_strict() {
        let a = dec("1.5");
        let b = dec("1.50001");
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    /// Negative cross-scale: -1.5 vs -1.50001.
    /// -1.50001 < -1.5 (more negative).
    #[test]
    fn cmp_exact_negative_cross_scale() {
        let a = dec("-1.5");
        let b = dec("-1.50001");
        assert_eq!(a.cmp_exact(&b), Ordering::Greater); // -1.5 > -1.50001
        assert_eq!(b.cmp_exact(&a), Ordering::Less);
    }

    /// Mixed signs: any positive > any negative.
    #[test]
    fn cmp_exact_mixed_signs() {
        assert_eq!(dec("0.001").cmp_exact(&dec("-999.9")), Ordering::Greater);
        assert_eq!(dec("-0.001").cmp_exact(&dec("999.9")), Ordering::Less);
    }

    /// Both-zero regardless of scale.
    #[test]
    fn cmp_exact_zero_any_scale() {
        let z0 = Decimal::from_parts(0, 0);
        let z5 = Decimal::from_parts(0, 5);
        let z18 = Decimal::from_parts(0, 18);
        assert_eq!(z0.cmp_exact(&z5), Ordering::Equal);
        assert_eq!(z5.cmp_exact(&z18), Ordering::Equal);
        assert_eq!(z18.cmp_exact(&z0), Ordering::Equal);
    }

    /// Large-mantissa regression: two large decimals at scale 0 vs scale 1 that the
    /// old 10^diff widening path would overflow on (mantissa near i128::MAX).
    ///
    /// The old code attempted: (i128::MAX / 10) * 10  which checks out but
    /// i128::MAX * 10 overflows — so we construct a pair where the lower-scale value's
    /// mantissa is large enough that multiplying by 10^diff would exceed i128::MAX.
    ///
    /// Specifically: mantissa = i128::MAX (scale 0) vs mantissa = i128::MAX (scale 1).
    /// Value A = i128::MAX × 10^0 = i128::MAX (≈ 1.70141…×10^38)
    /// Value B = i128::MAX × 10^(-1) ≈ 1.70141…×10^37
    /// So A > B.  The old code would try to scale A's mantissa up by 10 → overflow.
    #[test]
    fn cmp_exact_large_mantissa_no_overflow() {
        // A = i128::MAX at scale 0; B = i128::MAX at scale 1
        // A = 170141183460469231731687303715884105727
        // B = 17014118346046923173168730371588410572.7
        // True order: A > B
        let a = Decimal::from_parts(i128::MAX, 0);
        let b = Decimal::from_parts(i128::MAX, 1);
        assert_eq!(a.cmp_exact(&b), Ordering::Greater);
        assert_eq!(b.cmp_exact(&a), Ordering::Less);
    }

    /// Regression vector for the exact f64 collapse bug: two large unequal decimals
    /// at different scales that the old f64 path would round to the same f64 value
    /// and therefore return Equal incorrectly.
    ///
    /// f64 has ~15.9 significant decimal digits.  Construct two values that differ
    /// only in the 18th digit — well below f64 resolution — but whose true order
    /// is strict.
    ///
    /// A = 100000000000000000.1  (mantissa=1000000000000000001, scale=1)
    /// B = 100000000000000000.2  (mantissa=1000000000000000002, scale=1)
    /// Both have the same f64 representation (the fractional digit is lost), but
    /// A < B is exact.
    #[test]
    fn cmp_exact_large_f64_collapse_regression() {
        // 100000000000000000.1 and 100000000000000000.2 — same scale, near i64::MAX magnitude
        let a = Decimal::from_parts(1_000_000_000_000_000_001, 1);
        let b = Decimal::from_parts(1_000_000_000_000_000_002, 1);
        // Both collapse to the same f64 — the old path returns Equal incorrectly.
        assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
        // cmp_exact must return Less (A < B), not Equal.
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    /// Same as above but across scales (scale 1 vs scale 2).
    #[test]
    fn cmp_exact_large_f64_collapse_cross_scale_regression() {
        // A = 100000000000000000.1  (scale 1)
        // B = 100000000000000000.11 (scale 2) = 10000000000000000011 mantissa
        // A < B (0.1 < 0.11).  Both f64-identical at this magnitude.
        let a = Decimal::from_parts(1_000_000_000_000_000_001, 1); // .1 at scale 1
        let b = Decimal::from_parts(10_000_000_000_000_000_011, 2); // .11 at scale 2
        assert_eq!(a.to_f64(), b.to_f64(), "f64 collapse precondition");
        assert_eq!(a.cmp_exact(&b), Ordering::Less);
        assert_eq!(b.cmp_exact(&a), Ordering::Greater);
    }

    #[test]
    fn double_specials_and_canonical() {
        assert_eq!(parse_double("INF").unwrap(), f64::INFINITY);
        assert_eq!(parse_double("-INF").unwrap(), f64::NEG_INFINITY);
        assert!(parse_double("NaN").unwrap().is_nan());
        assert!(parse_double("inf").is_err());
        assert!(parse_double("Infinity").is_err());
        assert_eq!(canonical_double(1.0), "1.0E0");
        assert_eq!(canonical_double(1.5), "1.5E0");
        assert_eq!(canonical_double(100.0), "1.0E2");
        assert_eq!(canonical_double(0.005), "5.0E-3");
        assert_eq!(canonical_double(f64::INFINITY), "INF");
        assert_eq!(canonical_double(f64::NEG_INFINITY), "-INF");
        assert_eq!(canonical_double(f64::NAN), "NaN");
    }

    #[test]
    fn parse_float_still_accepts_plus_inf() {
        // Regression guard: the XSD 1.1 default parse must keep accepting `+INF`.
        assert_eq!(parse_float("+INF").unwrap(), f32::INFINITY);
    }

    #[test]
    fn parse_double_still_accepts_plus_inf() {
        // Regression guard: the XSD 1.1 default parse must keep accepting `+INF`.
        assert_eq!(parse_double("+INF").unwrap(), f64::INFINITY);
    }

    #[test]
    fn parse_float_xsd10_rejects_plus_inf() {
        assert!(parse_float_xsd10("+INF").is_err());
    }

    #[test]
    fn parse_double_xsd10_rejects_plus_inf() {
        assert!(parse_double_xsd10("+INF").is_err());
    }

    #[test]
    fn parse_float_xsd10_accepts_xsd10_lexicals() {
        assert_eq!(parse_float_xsd10("INF").unwrap(), f32::INFINITY);
        assert_eq!(parse_float_xsd10("-INF").unwrap(), f32::NEG_INFINITY);
        assert!(parse_float_xsd10("NaN").unwrap().is_nan());
        assert_eq!(parse_float_xsd10("1.5").unwrap(), 1.5f32);
        assert_eq!(parse_float_xsd10("1e10").unwrap(), 1e10f32);
    }

    #[test]
    fn parse_double_xsd10_accepts_xsd10_lexicals() {
        assert_eq!(parse_double_xsd10("INF").unwrap(), f64::INFINITY);
        assert_eq!(parse_double_xsd10("-INF").unwrap(), f64::NEG_INFINITY);
        assert!(parse_double_xsd10("NaN").unwrap().is_nan());
        assert_eq!(parse_double_xsd10("1.5").unwrap(), 1.5f64);
        assert_eq!(parse_double_xsd10("1e10").unwrap(), 1e10f64);
    }

    #[test]
    fn value_parse_xsd10_rejects_plus_inf_for_float_and_double() {
        assert!(crate::value::parse_xsd10("+INF", D::Double).is_err());
        assert!(crate::value::parse_xsd10("+INF", D::Float).is_err());
    }

    #[test]
    fn value_parse_xsd10_unaffected_for_non_float_double_datatypes() {
        // Non-float/double datatypes must behave exactly as `parse`.
        let xsd10 = crate::value::parse_xsd10("1", D::Integer).unwrap();
        let xsd11 = crate::value::parse("1", D::Integer).unwrap();
        assert_eq!(xsd10.canonical_lexical(), xsd11.canonical_lexical());
    }

    #[test]
    fn numeric_promotion() {
        // "1"^^integer = "1.0"^^decimal
        assert!(numeric_eq(&int_val(1), &dec_value("1.0")));
        // integer vs double
        assert_eq!(
            numeric_cmp(&int_val(2), &XsdValue::Double(2.5)),
            Some(Ordering::Less)
        );
        // decimal vs float
        assert_eq!(
            numeric_cmp(&dec_value("1.5"), &XsdValue::Float(1.25)),
            Some(Ordering::Greater)
        );
        // NaN is unordered and unequal.
        assert_eq!(numeric_cmp(&XsdValue::Double(f64::NAN), &int_val(1)), None);
        assert!(!numeric_eq(
            &XsdValue::Double(f64::NAN),
            &XsdValue::Double(f64::NAN)
        ));
        // +0 == -0.
        assert!(numeric_eq(&XsdValue::Double(0.0), &XsdValue::Double(-0.0)));

        // Cross-subtype integer equality: xsd:int 5 == xsd:long 5.
        let int5 = XsdValue::Integer {
            value: crate::exact::Integer::from_i128(5),
            datatype: D::Int,
        };
        let long5 = XsdValue::Integer {
            value: crate::exact::Integer::from_i128(5),
            datatype: D::Long,
        };
        assert!(numeric_eq(&int5, &long5));
        assert_eq!(numeric_cmp(&int5, &long5), Some(Ordering::Equal));
    }

    // ── Arithmetic tests ─────────────────────────────────────────────────────

    fn dec_val(s: &str) -> XsdValue {
        dec_value(s)
    }

    fn float_val(f: f32) -> XsdValue {
        XsdValue::Float(f)
    }

    fn double_val(d: f64) -> XsdValue {
        XsdValue::Double(d)
    }

    /// Helper: extract the Decimal from an XsdValue::Decimal, panic otherwise.
    fn as_decimal(v: &XsdValue) -> Decimal {
        match v {
            XsdValue::Decimal(d) => d.to_bounded().expect("bounded decimal test value"),
            other => panic!("expected Decimal, got {other:?}"),
        }
    }

    /// Helper: extract the i128 from an XsdValue::Integer, panic otherwise.
    fn as_integer(v: &XsdValue) -> i128 {
        match v {
            XsdValue::Integer { value, .. } => value.as_i128().expect("bounded integer test value"),
            other => panic!("expected Integer, got {other:?}"),
        }
    }

    /// Helper: extract f64 from XsdValue::Double, panic otherwise.
    fn as_double(v: &XsdValue) -> f64 {
        match v {
            XsdValue::Double(d) => *d,
            other => panic!("expected Double, got {other:?}"),
        }
    }

    /// Helper: extract f32 from XsdValue::Float, panic otherwise.
    fn as_float(v: &XsdValue) -> f32 {
        match v {
            XsdValue::Float(f) => *f,
            other => panic!("expected Float, got {other:?}"),
        }
    }

    // -- integer + integer → integer --

    #[test]
    fn add_integer_integer() {
        let result = numeric_add(&int_val(3), &int_val(4)).unwrap();
        assert_eq!(as_integer(&result), 7);
    }

    #[test]
    fn add_integer_overflow() {
        // i128::MAX + 1 is the exact integer past i128, never a wrap or a refusal.
        let max = int_val(i128::MAX);
        let one = int_val(1);
        let sum = numeric_add(&max, &one).expect("exact");
        assert!(matches!(sum, XsdValue::Integer { ref value, .. } if value.as_i128().is_none()));
        assert_eq!(
            sum.canonical_lexical(),
            "170141183460469231731687303715884105728"
        );
    }

    // -- integer division returns Decimal (SPARQL §17.4 / XPath op:numeric-divide) --

    #[test]
    fn div_integer_integer_returns_decimal() {
        // 1 / 2 must be Decimal(0.5), NOT Integer(0)
        let result = numeric_div(&int_val(1), &int_val(2)).unwrap();
        assert!(
            matches!(result, XsdValue::Decimal(_)),
            "expected Decimal, got {result:?}"
        );
        let d = as_decimal(&result);
        // 0.5 at scale 18: mantissa = 5×10^17
        assert_eq!(d.to_f64(), 0.5, "1/2 must equal 0.5");
    }

    #[test]
    fn div_4_2_is_decimal_two() {
        // 4 / 2 must be Decimal(2.0), NOT Integer(2)
        let result = numeric_div(&int_val(4), &int_val(2)).unwrap();
        assert!(matches!(result, XsdValue::Decimal(_)));
        let d = as_decimal(&result);
        assert_eq!(d.to_f64(), 2.0, "4/2 must equal 2.0 as decimal");
    }

    #[test]
    fn div_1_3_is_18_digit_decimal() {
        // 1 / 3 → Decimal, 18 fractional digits of 3s
        let result = numeric_div(&int_val(1), &int_val(3)).unwrap();
        let d = as_decimal(&result);
        // Canonical form should start with "0.333333333333333333"
        let lex = d.canonical_lexical();
        assert!(
            lex.starts_with("0.333333333333333333"),
            "expected 0.333...333 (18 threes), got {lex}"
        );
        // Exactly 18 fractional digits
        let frac = lex.split('.').nth(1).unwrap_or("");
        assert_eq!(
            frac.len(),
            18,
            "should have 18 fractional digits, got {frac}"
        );
    }

    // -- `bigint_avg_decimal`: AVG's finish once a running SUM has escaped `i128` --

    #[test]
    fn bigint_avg_matches_numeric_div_within_i128_range() {
        // A dividend still small enough to fit `i128` must agree byte-for-byte with
        // the ordinary `numeric_div` path — `bigint_avg_decimal` is a strict
        // generalization, not a different algorithm.
        let dividend = crate::bigint::BigInt::from_i128(1);
        let via_bigint = bigint_avg_decimal(&dividend, 3).expect("1/3 fits");
        let via_numeric_div = numeric_div(&int_val(1), &int_val(3)).unwrap();
        assert_eq!(
            as_decimal(&via_bigint).cmp_exact(&as_decimal(&via_numeric_div)),
            Ordering::Equal
        );
    }

    #[test]
    fn bigint_avg_answers_when_the_dividend_exceeds_i128_but_the_quotient_does_not() {
        // dividend = 2 * i128::MAX (exceeds i128; only representable as a BigInt),
        // divided by a count large enough that the QUOTIENT'S 18-fractional-digit
        // mantissa still fits i128 — proving the escaped-i128 SUM does not have to
        // poison AVG when the answer itself is ordinary.
        let mut dividend = crate::bigint::BigInt::from_i128(i128::MAX);
        dividend.add_i128(i128::MAX);
        assert_eq!(
            dividend.to_i128(),
            None,
            "fixture must genuinely exceed i128"
        );
        let avg = bigint_avg_decimal(&dividend, 10_000_000_000_000_000_000)
            .expect("quotient magnitude is well under i128::MAX");
        let d = as_decimal(&avg);
        // (2 * i128::MAX) / 1e19, truncated toward zero at 18 fractional digits.
        assert_eq!(
            d.canonical_lexical(),
            "34028236692093846346.337460743176821145"
        );
    }

    #[test]
    fn bigint_avg_poisons_when_the_mantissa_still_does_not_fit_i128() {
        // dividend = 2 * i128::MAX, divided by 2: the mathematically exact quotient
        // (i128::MAX) is an ordinary integer, but `bigint_avg_decimal`'s result type
        // is the `i128`-mantissa `Decimal`, and its mantissa must hold BOTH the
        // integer part AND 18 fractional digits — `i128::MAX` scaled to 18
        // fractional digits does not fit `i128` either. This is `xsd:decimal`'s own
        // documented bound (`Decimal`'s deliberate, unchanged design), not a
        // limitation `BigInt` introduces, and it already applied — for the same
        // reason — to sufficiently large in-range `AVG`s before this module existed.
        //
        // `bigint_avg_decimal` genuinely has no answer here (there is no `i128`
        // mantissa to return); [`bigint_avg_decimal_lexical`] is the escape hatch
        // that answers this exact case by rendering the exact result as text
        // instead — see the next test.
        let mut dividend = crate::bigint::BigInt::from_i128(i128::MAX);
        dividend.add_i128(i128::MAX);
        assert!(bigint_avg_decimal(&dividend, 2).is_none());
    }

    #[test]
    fn bigint_avg_decimal_lexical_answers_exactly_where_bigint_avg_decimal_poisons() {
        // Same fixture as the test above (dividend = 2 * i128::MAX, count = 2):
        // `bigint_avg_decimal` has no `i128`-mantissa `Decimal` to return, but the
        // exact scale-18 quotient is perfectly representable as TEXT, with no
        // magnitude bound at all — `170141183460469231731687303715884105727` (i.e.
        // `i128::MAX`) followed by 18 zero fractional digits, trimmed to an
        // integer-valued canonical form per XSD 1.1 §E.1 `decimalCanonicalMap`.
        let mut dividend = crate::bigint::BigInt::from_i128(i128::MAX);
        dividend.add_i128(i128::MAX);
        let lexical = bigint_avg_decimal_lexical(&dividend, 2);
        assert_eq!(lexical, i128::MAX.to_string());
    }

    // -- decimal exactness: 0.1 + 0.2 == 0.3 (the classic float failure) --

    #[test]
    fn decimal_add_exact_no_float_error() {
        // IEEE double: 0.1 + 0.2 ≠ 0.3; exact decimal: 0.1 + 0.2 == 0.3.
        let result = numeric_add(&dec_val("0.1"), &dec_val("0.2")).unwrap();
        let d = as_decimal(&result);
        let expected = parse_bounded_decimal("0.3").unwrap();
        assert_eq!(
            d.cmp_exact(&expected),
            Ordering::Equal,
            "0.1 + 0.2 must equal 0.3 exactly in decimal; got {}",
            d.canonical_lexical()
        );
    }

    /// Aligning a large scale-0 mantissa to scale 18 leaves `i128` before the
    /// addition does: `1e30 + 1e-18` needs the mantissa `1e48`. That is an
    /// `OutOfRange` overflow (it used to panic under overflow checks and wrap
    /// to a wrong sum without them); the neighbouring `1e19 + 1e-18` (mantissa
    /// `1e37`) still fits and must still add exactly — for `-` too.
    #[test]
    fn decimal_alignment_overflow_is_exact_and_its_neighbour_adds() {
        let tiny = dec_val("0.000000000000000001");
        let huge = dec_val("1000000000000000000000000000000");
        for (result, expected) in [
            (
                numeric_add(&huge, &tiny),
                "1000000000000000000000000000000.000000000000000001",
            ),
            (
                numeric_add(&tiny, &huge),
                "1000000000000000000000000000000.000000000000000001",
            ),
            (
                numeric_sub(&huge, &tiny),
                "999999999999999999999999999999.999999999999999999",
            ),
            (
                numeric_add(&int_val(10i128.pow(30)), &tiny),
                "1000000000000000000000000000000.000000000000000001",
            ),
        ] {
            let result = result.expect("exact");
            assert!(
                matches!(result, XsdValue::Decimal(ref value) if value.to_bounded().is_none()),
                "{result:?}"
            );
            assert_eq!(result.canonical_lexical(), expected);
        }
        let fits = dec_val("10000000000000000000");
        assert_eq!(
            numeric_add(&fits, &tiny).unwrap().canonical_lexical(),
            "10000000000000000000.000000000000000001"
        );
        assert_eq!(
            numeric_sub(&fits, &tiny).unwrap().canonical_lexical(),
            "9999999999999999999.999999999999999999"
        );
    }

    // -- numeric promotion: integer + double → double --

    #[test]
    fn add_integer_double_promotes_to_double() {
        let result = numeric_add(&int_val(1), &double_val(1.5)).unwrap();
        assert!(
            matches!(result, XsdValue::Double(_)),
            "expected Double, got {result:?}"
        );
        let d = as_double(&result);
        assert_eq!(d, 2.5);
    }

    // -- numeric promotion: decimal + float → float --

    #[test]
    fn add_decimal_float_promotes_to_float() {
        let result = numeric_add(&dec_val("1.5"), &float_val(0.5)).unwrap();
        assert!(
            matches!(result, XsdValue::Float(_)),
            "expected Float, got {result:?}"
        );
        // 1.5 + 0.5 = 2.0
        assert_eq!(as_float(&result), 2.0_f32);
    }

    /// Promotion reads the correctly rounded value: `72922151633738826.80` is
    /// `7.292215163373883e16` as a double (the old two-rounding conversion gave
    /// `…882e16`, so `=` against the correctly rounded double was false), and
    /// `18446745173221179393.0` is `(2^24 + 2) × 2^40` as a float (narrowing the
    /// double rounds twice, to `2^64`). The neighbouring double one ulp down
    /// still compares unequal, so the `=` is observed, not assumed.
    #[test]
    fn decimal_promotion_rounds_once_to_double_and_to_float() {
        let witness = dec_val("72922151633738826.80");
        let correct = 7.292_215_163_373_883e16_f64;
        assert_eq!(
            numeric_cmp(&witness, &XsdValue::Double(correct)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            numeric_cmp(&witness, &XsdValue::Double(correct.next_down())),
            Some(Ordering::Greater)
        );
        let sum = numeric_add(&witness, &XsdValue::Double(0.0)).unwrap();
        assert_eq!(as_double(&sum).to_bits(), correct.to_bits());

        let above_tie = dec_val("18446745173221179393.0");
        let correct_f32 = f32::from_bits(((127 + 64) << 23) | 1);
        let sum = numeric_add(&above_tie, &float_val(0.0)).unwrap();
        assert_eq!(as_float(&sum).to_bits(), correct_f32.to_bits());
        assert_eq!(
            numeric_cmp(&above_tie, &float_val(correct_f32)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            numeric_cmp(&above_tie, &float_val(2f32.powi(64))),
            Some(Ordering::Greater)
        );
    }

    // -- numeric promotion: integer × decimal → decimal --

    #[test]
    fn mul_integer_decimal_promotes_to_decimal() {
        // 3 × 1.5 = 4.5
        let result = numeric_mul(&int_val(3), &dec_val("1.5")).unwrap();
        assert!(
            matches!(result, XsdValue::Decimal(_)),
            "expected Decimal, got {result:?}"
        );
        let d = as_decimal(&result);
        let expected = parse_bounded_decimal("4.5").unwrap();
        assert_eq!(
            d.cmp_exact(&expected),
            Ordering::Equal,
            "3 × 1.5 must equal 4.5; got {}",
            d.canonical_lexical()
        );
    }

    // -- division by zero --

    #[test]
    fn div_integer_by_zero_is_error() {
        assert!(matches!(
            numeric_div(&int_val(5), &int_val(0)),
            Err(XsdError::DivisionByZero {
                datatype: XsdDatatype::Integer
            })
        ));
    }

    #[test]
    fn div_decimal_by_zero_is_error() {
        assert!(matches!(
            numeric_div(&dec_val("5.0"), &dec_val("0")),
            Err(XsdError::DivisionByZero {
                datatype: XsdDatatype::Decimal
            })
        ));
    }

    #[test]
    fn div_double_by_zero_is_inf_not_error() {
        // IEEE 754: positive / +0.0 = +INF
        let result = numeric_div(&double_val(5.0), &double_val(0.0)).unwrap();
        let d = as_double(&result);
        assert!(
            d.is_infinite() && d.is_sign_positive(),
            "5.0 / 0.0 must be +INF"
        );
    }

    #[test]
    fn div_double_zero_by_zero_is_nan_not_error() {
        // IEEE 754: 0.0 / 0.0 = NaN (no error)
        let result = numeric_div(&double_val(0.0), &double_val(0.0)).unwrap();
        let d = as_double(&result);
        assert!(d.is_nan(), "0.0 / 0.0 must be NaN");
    }

    // -- unary minus --

    #[test]
    fn unary_minus_integer() {
        assert_eq!(as_integer(&numeric_unary_minus(&int_val(5)).unwrap()), -5);
        assert_eq!(as_integer(&numeric_unary_minus(&int_val(-3)).unwrap()), 3);
    }

    #[test]
    fn unary_minus_decimal() {
        let result = numeric_unary_minus(&dec_val("1.5")).unwrap();
        let d = as_decimal(&result);
        assert_eq!(d.canonical_lexical(), "-1.5");
    }

    #[test]
    fn unary_minus_float() {
        let result = numeric_unary_minus(&float_val(2.5)).unwrap();
        assert_eq!(as_float(&result), -2.5_f32);
    }

    #[test]
    fn unary_minus_double() {
        // Use a value that is not an approx of a named constant (clippy::approx_constant).
        let result = numeric_unary_minus(&double_val(1.23456)).unwrap();
        assert!((as_double(&result) - (-1.23456)).abs() < 1e-12);
    }

    // -- unary plus --

    #[test]
    fn unary_plus_is_identity_for_numerics() {
        // integer
        let i = int_val(42);
        let r = numeric_unary_plus(&i).unwrap();
        assert_eq!(as_integer(&r), 42);
        // decimal
        let d_in = dec_val("1.5");
        let d_out = numeric_unary_plus(&d_in).unwrap();
        assert_eq!(as_decimal(&d_out).canonical_lexical(), "1.5");
        // float
        let f_in = float_val(3.0);
        let f_out = numeric_unary_plus(&f_in).unwrap();
        assert_eq!(as_float(&f_out), 3.0_f32);
        // double — use a value that is not an approx of a named constant
        let dbl_in = double_val(9.876);
        let dbl_out = numeric_unary_plus(&dbl_in).unwrap();
        assert!((as_double(&dbl_out) - 9.876).abs() < 1e-12);
    }

    #[test]
    fn unary_plus_non_numeric_is_error() {
        let boolean = XsdValue::Boolean(true);
        assert!(matches!(
            numeric_unary_plus(&boolean),
            Err(XsdError::TypeMismatch { .. })
        ));
        let string = XsdValue::String("hello".to_string());
        assert!(matches!(
            numeric_unary_plus(&string),
            Err(XsdError::TypeMismatch { .. })
        ));
    }

    /// `xsd:double` and `xsd:float` arithmetic is the IEEE result on every target, the x87
    /// included: each operand pair below is a double-rounding witness -- the reference
    /// shows the result rounded through the x87's register format differs -- and the
    /// numeric operators return the correctly rounded bits.
    #[test]
    fn binary_arithmetic_rounds_once_on_double_rounding_witnesses() {
        use crate::ieee::reference as soft;

        let one = 1.0_f64;
        let little = f64::from_bits(0x3ca0_0000_0800_0000); // 2^-53 + 2^-78
        // The observing oracle: through the register's 64 bits the sum is 1.
        assert_eq!(soft::add_via(one, little, soft::X87_EXTENDED), 1.0);
        let succ = 1.0 + f64::EPSILON;
        // Through the lexical space, as a query writes them.
        let lexical = canonical_double(little);
        let parsed = parse_double(&lexical).expect("round-trips");
        assert_eq!(parsed.to_bits(), little.to_bits(), "{lexical}");
        let sum = numeric_add(&double_val(one), &double_val(parsed)).unwrap();
        assert_eq!(as_double(&sum).to_bits(), succ.to_bits());
        assert_eq!(sum.canonical_lexical(), "1.0000000000000002E0");
        // Promotion from an integer operand does not change the law.
        let sum = numeric_add(&int_val(1), &double_val(little)).unwrap();
        assert_eq!(as_double(&sum).to_bits(), succ.to_bits());
        // `(1 + ulp) − (−(2^-53 − 2^-78))` is the same witness as a difference.
        let less = f64::from_bits(0x3c9f_ffff_f000_0000);
        let difference = numeric_sub(&double_val(succ), &double_val(-less)).unwrap();
        assert_eq!(as_double(&difference).to_bits(), succ.to_bits());
        assert_eq!(
            soft::add_via(succ, less, soft::X87_EXTENDED),
            f64::from_bits(0x3ff0_0000_0000_0002)
        );

        // Subnormal products and quotients: rounded at 53 bits and again when stored
        // without the scaling.
        let half_min_plus_one = f64::from_bits(0x0008_0000_0000_0001);
        let (a, b) = (
            f64::from_bits(((1023 - 512) << 52) + 4),
            f64::from_bits(((1023 - 511) << 52) - 2),
        );
        assert_ne!(soft::mul_via(a, b, soft::X87_DOUBLE), half_min_plus_one);
        let product = numeric_mul(&double_val(a), &double_val(b)).unwrap();
        assert_eq!(as_double(&product).to_bits(), half_min_plus_one.to_bits());
        let (a, b) = (
            f64::from_bits((1023 - 512) << 52),
            f64::from_bits(((1023 + 511) << 52) - 2),
        );
        assert_ne!(soft::div_via(a, b, soft::X87_DOUBLE), half_min_plus_one);
        let quotient = numeric_div(&double_val(a), &double_val(b)).unwrap();
        assert_eq!(as_double(&quotient).to_bits(), half_min_plus_one.to_bits());

        // xsd:float: the binary32 subnormal witnesses, rounded at 24 bits and again when
        // stored without the scaling.
        let f32_half_min_plus_one = f32::from_bits(0x0040_0001);
        let (a, b) = (
            f32::from_bits(((127 - 64) << 23) + 4),
            f32::from_bits(((127 - 63) << 23) - 2),
        );
        assert_ne!(
            soft::mul32_via(a, b, soft::X87_SINGLE),
            f32_half_min_plus_one
        );
        let product = numeric_mul(&float_val(a), &float_val(b)).unwrap();
        assert_eq!(
            as_float(&product).to_bits(),
            f32_half_min_plus_one.to_bits()
        );
        let (a, b) = (
            f32::from_bits((127 - 64) << 23),
            f32::from_bits(((127 + 63) << 23) - 2),
        );
        assert_ne!(
            soft::div32_via(a, b, soft::X87_SINGLE),
            f32_half_min_plus_one
        );
        let quotient = numeric_div(&float_val(a), &float_val(b)).unwrap();
        assert_eq!(
            as_float(&quotient).to_bits(),
            f32_half_min_plus_one.to_bits()
        );
    }

    /// Random `xsd:double`/`xsd:float` operands through all four operators, held to the
    /// integer reference bit for bit.
    #[test]
    fn binary_arithmetic_equals_the_software_reference() {
        use crate::ieee::reference as soft;

        let mut state = 0x0b1a_4e57_u64;
        let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
        for index in 0..20_000_u32 {
            // Near one (dense ties) or anywhere in the finite range.
            let draw = |bits: u64| {
                if index % 2 == 0 {
                    f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000)
                } else {
                    f64::from_bits(bits & 0xffef_ffff_ffff_ffff)
                }
            };
            let (x, y) = (draw(next()), draw(next()));
            let d = |v: &XsdValue| as_double(v).to_bits();
            assert_eq!(
                d(&numeric_add(&double_val(x), &double_val(y)).unwrap()),
                soft::add(x, y).to_bits()
            );
            assert_eq!(
                d(&numeric_sub(&double_val(x), &double_val(y)).unwrap()),
                soft::sub(x, y).to_bits()
            );
            assert_eq!(
                d(&numeric_mul(&double_val(x), &double_val(y)).unwrap()),
                soft::mul(x, y).to_bits()
            );
            assert_eq!(
                d(&numeric_div(&double_val(x), &double_val(y)).unwrap()),
                soft::div(x, y).to_bits()
            );
            let (p, q) = (x as f32, y as f32);
            if p.is_finite() && q.is_finite() {
                let f = |v: &XsdValue| as_float(v).to_bits();
                assert_eq!(
                    f(&numeric_add(&float_val(p), &float_val(q)).unwrap()),
                    soft::add32(p, q).to_bits()
                );
                assert_eq!(
                    f(&numeric_sub(&float_val(p), &float_val(q)).unwrap()),
                    soft::sub32(p, q).to_bits()
                );
                assert_eq!(
                    f(&numeric_mul(&float_val(p), &float_val(q)).unwrap()),
                    soft::mul32(p, q).to_bits()
                );
                if q != 0.0 {
                    assert_eq!(
                        f(&numeric_div(&float_val(p), &float_val(q)).unwrap()),
                        soft::div32(p, q).to_bits()
                    );
                }
            }
        }
    }

    /// The decimal-to-double conversion's exact fast path divides once: a decimal whose
    /// quotient `mantissa / 10^scale` rounds differently through the x87's 64-bit register
    /// converts to the correctly rounded double, and so compares and promotes by it.
    #[test]
    fn decimal_to_double_fast_path_rounds_once_on_a_double_rounding_witness() {
        use crate::ieee::reference as soft;

        // The first scale, and the first mantissa from 2^52 upward at it, whose quotient
        // is a witness. (No quotient by 10 is one: the binary expansion of a tenth
        // repeats `0011`, which never rounds onto a midpoint at 64 bits.)
        let (scale, witness) = (1_u8..=22)
            .find_map(|scale| {
                let power = 10_f64.powi(i32::from(scale));
                (1_i128 << 52..(1 << 52) + 5_000)
                    .find(|&m| {
                        let a = m as f64;
                        soft::div_via(a, power, soft::X87_EXTENDED).to_bits()
                            != soft::div(a, power).to_bits()
                    })
                    .map(|m| (scale, m))
            })
            .expect("a double-rounding witness at some scale");
        let expected = soft::div(witness as f64, 10_f64.powi(i32::from(scale)));
        let decimal = Decimal::from_parts(witness, scale);
        assert_eq!(
            decimal.to_f64().to_bits(),
            expected.to_bits(),
            "{witness} / 10^{scale}"
        );
        // The same bits through promotion into double arithmetic.
        let sum = numeric_add(&xsd_decimal(decimal), &double_val(0.0)).unwrap();
        assert_eq!(as_double(&sum).to_bits(), expected.to_bits());
    }
    /// The closest 18-fractional-digit decimal to `value`, ties toward zero, read
    /// off the exact decimal expansion `{:.1100}` prints (every binary64 has at
    /// most 1074 fractional digits).
    fn closest_decimal_reference(value: f64) -> Option<String> {
        let exact = format!("{:.1100}", value.abs());
        let (int, frac) = exact.split_once('.').expect("a fractional part");
        if value.abs() >= 2f64.powi(53) {
            // An integer, exact at scale 0.
            let mantissa = i128::try_from(int.parse::<u128>().ok()?).ok()?;
            let signed = if value.is_sign_negative() {
                -mantissa
            } else {
                mantissa
            };
            return Some(signed.to_string());
        }
        let (kept, rest) = frac.split_at(18);
        let above_half = match rest.as_bytes()[0] {
            b'6'..=b'9' => true,
            b'5' => rest.bytes().skip(1).any(|b| b != b'0'),
            _ => false,
        };
        let mut digits: u128 = format!("{int}{kept}").parse().ok()?;
        digits += u128::from(above_half);
        let mantissa = i128::try_from(digits).ok()?;
        let decimal = Decimal::from_parts(
            if value.is_sign_negative() {
                -mantissa
            } else {
                mantissa
            },
            MAX_DECIMAL_SCALE,
        );
        Some(decimal.canonical_lexical())
    }

    /// The former bounded decimal quotient, from the exact rational
    /// `dividend / divisor` with integer arithmetic only
    /// ([`purrdf_testkit::exact`]): truncated toward zero at the finest scale `S ≤ 18`
    /// whose mantissa fits an `i128`, as `(mantissa, S)`; `None` when none does.
    fn decimal_quotient_oracle(dividend: Decimal, divisor: Decimal) -> Option<(i128, u8)> {
        use purrdf_testkit::exact::{Natural, Rational};
        let negative = (dividend.mantissa < 0) != (divisor.mantissa < 0);
        (0..=MAX_DECIMAL_SCALE).rev().find_map(|scale| {
            // dividend / divisor × 10^S = dm · 10^(vs + S) / (vm · 10^ds).
            let numerator = Natural::from_u128(dividend.mantissa.unsigned_abs())
                .mul_pow10(u32::from(divisor.scale) + u32::from(scale));
            let denominator = Natural::from_u128(divisor.mantissa.unsigned_abs())
                .mul_pow10(u32::from(dividend.scale));
            Rational::new(negative, numerator, denominator)
                .truncate_toward_zero()
                .map(|mantissa| (mantissa, scale))
        })
    }

    /// Hold the explicit legacy `18:toward-zero` policy on two decimals to the
    /// oracle. This preserves the original baseline independently of the new
    /// exact-or-rounded default.
    fn assert_decimal_quotient(dividend: Decimal, divisor: Decimal) {
        use purrdf_testkit::exact::{Direction, Rational as Oracle};
        let got = numeric_div_with_policy(
            &xsd_decimal(dividend),
            &xsd_decimal(divisor),
            DivisionPolicy::scale(18, crate::exact::Rounding::TowardZero),
        )
        .unwrap_or_else(|error| panic!("{dividend:?} / {divisor:?}: {error}"));
        // The quotient at eighteen digits, truncated — from the oracle, at any size.
        let exact = Oracle::from_decimal(dividend.mantissa(), u32::from(dividend.scale()))
            .div(&Oracle::from_decimal(
                divisor.mantissa(),
                u32::from(divisor.scale()),
            ))
            .expect("a nonzero divisor");
        let expected = exact.round_to_scale(18, Direction::TowardZero);
        let value = Oracle::parse(&got.canonical_lexical()).expect("a numeral");
        assert!(
            value.value_eq(&expected),
            "{dividend:?} / {divisor:?} = {got:?}"
        );
        // Wherever the bounded decimal holds the eighteen digits, it is the answer.
        if let Some((mantissa, 18)) = decimal_quotient_oracle(dividend, divisor) {
            let XsdValue::Decimal(quotient) = got else {
                panic!("{dividend:?} / {divisor:?}: {got:?}, expected bounded {mantissa}e-18");
            };
            assert_eq!(
                quotient
                    .to_bounded()
                    .expect("bounded quotient test case")
                    .cmp_exact(&Decimal::from_parts(mantissa, 18)),
                Ordering::Equal
            );
        }
    }

    #[test]
    fn explicit_legacy_decimal_division_matches_the_exact_oracle() {
        let mut state = 0xD1F1_DE5A_u64;
        let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
        let int = |next: &mut dyn FnMut() -> u64| {
            let width = 1 + (next() % 127) as u32;
            let raw = (u128::from(next()) << 64) | u128::from(next());
            let magnitude = (raw >> (128 - width)) as i128;
            if next() & 1 == 1 {
                -magnitude
            } else {
                magnitude
            }
        };
        let scale = |next: &mut dyn FnMut() -> u64| (next() % 19) as u8;
        let mut cases = 0_usize;
        let extremes = [
            i128::MIN,
            i128::MIN + 1,
            i128::MAX,
            i128::MAX - 1,
            -1,
            1,
            2,
            3,
            7,
            10,
            -10,
            1_000_000_000_000_000_000_000,
            100_000_000_000_000_000_000,
        ];
        // Every extreme over every extreme, at a spread of scales — the overflow
        // boundary (MAX / 0.1, MIN / -1) and the exact large quotients (10^21 / 2,
        // MIN / 2) among them.
        for &dm in &extremes {
            for &vm in &extremes {
                for (ds, vs) in [(0, 0), (0, 1), (1, 0), (0, 18), (18, 0), (18, 18), (5, 9)] {
                    assert_decimal_quotient(
                        Decimal::from_parts(dm, ds),
                        Decimal::from_parts(vm, vs),
                    );
                    cases += 1;
                }
            }
        }
        for _ in 0..30_000 {
            let (dm, vm) = (int(&mut next), int(&mut next));
            if vm == 0 {
                continue;
            }
            let (ds, vs) = (scale(&mut next), scale(&mut next));
            assert_decimal_quotient(Decimal::from_parts(dm, ds), Decimal::from_parts(vm, vs));
            cases += 1;
        }
        // Exact quotients: dividend = quotient × divisor, whenever that product fits.
        for _ in 0..20_000 {
            let (q, vm) = (int(&mut next) >> 40, int(&mut next) >> 40);
            let Some(dm) = q.checked_mul(vm).filter(|_| vm != 0) else {
                continue;
            };
            let (ds, vs) = (scale(&mut next), scale(&mut next));
            assert_decimal_quotient(Decimal::from_parts(dm, ds), Decimal::from_parts(vm, vs));
            cases += 1;
        }
        // Near the overflow boundary: large dividends over small divisors.
        for _ in 0..10_000 {
            let dm =
                (i128::MAX - (int(&mut next) >> 64).abs()) * if next() & 1 == 1 { -1 } else { 1 };
            let vm = int(&mut next) >> 100;
            if vm == 0 {
                continue;
            }
            let (ds, vs) = (scale(&mut next), scale(&mut next));
            assert_decimal_quotient(Decimal::from_parts(dm, ds), Decimal::from_parts(vm, vs));
            cases += 1;
        }
        assert!(cases > 50_000, "{cases}");
    }

    #[test]
    fn decimal_negation_and_abs_of_the_smallest_mantissa_are_exact() {
        let min = xsd_decimal(Decimal::from_parts(i128::MIN, 0));
        let two_pow_127 = "170141183460469231731687303715884105728";
        assert_eq!(
            numeric_unary_minus(&min)
                .expect("exact")
                .canonical_lexical(),
            two_pow_127
        );
        assert_eq!(
            numeric_abs(&min).expect("exact").canonical_lexical(),
            two_pow_127
        );
        let scaled = xsd_decimal(Decimal::from_parts(i128::MIN, 18));
        assert_eq!(
            numeric_abs(&scaled).expect("exact").canonical_lexical(),
            "170141183460469231731.687303715884105728"
        );
        // One above it negates and takes its absolute value.
        let next = xsd_decimal(Decimal::from_parts(i128::MIN + 1, 0));
        let Ok(XsdValue::Decimal(abs)) = numeric_abs(&next) else {
            panic!("abs of i128::MIN + 1");
        };
        assert_eq!(abs.unscaled().as_i128(), Some(i128::MAX));
        let Ok(XsdValue::Decimal(negated)) = numeric_unary_minus(&next) else {
            panic!("negation of i128::MIN + 1");
        };
        assert_eq!(negated.unscaled().as_i128(), Some(i128::MAX));
    }

    #[test]
    fn decimal_mantissa_spans_the_whole_i128_range() {
        let min = i128::MIN.to_string();
        let parsed = parse_bounded_decimal(&min).expect("i128::MIN is a decimal");
        assert_eq!(parsed.mantissa(), i128::MIN);
        assert_eq!(parsed.canonical_lexical(), min);
        assert_eq!(
            Decimal::from_integer(i128::MIN).canonical_lexical(),
            min,
            "the canonical lexical of the cast reads back"
        );
        let scaled =
            parse_bounded_decimal("-170141183460469231731.687303715884105728").expect("scaled");
        assert_eq!((scaled.mantissa(), scaled.scale()), (i128::MIN, 18));
        assert_eq!(
            parse_bounded_decimal(&i128::MAX.to_string())
                .expect("max")
                .mantissa(),
            i128::MAX
        );
        // One past either end is still out of range.
        for beyond in [
            "-170141183460469231731687303715884105729",
            "170141183460469231731687303715884105728",
            "-170141183460469231731.687303715884105729",
        ] {
            assert!(
                matches!(
                    parse_bounded_decimal(beyond),
                    Err(XsdError::OutOfRange { .. })
                ),
                "{beyond}"
            );
        }
    }

    #[test]
    fn from_f64_closest_matches_the_exact_expansion() {
        for (value, expected) in [
            (0.1_f64, Some("0.100000000000000006")),
            (f64::from(0.1_f32), Some("0.100000001490116119")),
            (3.0 * 2f64.powi(-19), Some("0.000005722045898437")),
            (-3.0 * 2f64.powi(-19), Some("-0.000005722045898437")),
            (5.0 * 2f64.powi(-19), Some("0.000009536743164062")),
            (1e30, Some("1000000000000000019884624838656")),
            (-0.0, Some("0")),
            (2f64.powi(-1074), Some("0")),
            (2f64.powi(127), None),
            (-(2f64.powi(127)), None),
            (f64::NAN, None),
            (f64::INFINITY, None),
        ] {
            assert_eq!(
                Decimal::from_f64_closest(value)
                    .map(|d| d.canonical_lexical())
                    .as_deref(),
                expected,
                "{value:e}"
            );
        }
        let below = f64::from_bits(2f64.powi(127).to_bits() - 1);
        assert_eq!(
            Decimal::from_f64_closest(below).map(|d| d.mantissa()),
            i128::try_from(below as u128).ok()
        );
        let mut state = 0xdec1_3a1f_u64;
        let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
        for index in 0..20_000_u32 {
            let bits = next();
            // Anywhere in the finite range, or a magnitude in [2^-70, 2^70).
            let value = if index % 2 == 0 {
                f64::from_bits(bits & 0xffef_ffff_ffff_ffff)
            } else {
                let exponent = (bits >> 52) % 140;
                f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | ((953 + exponent) << 52))
            };
            let reference = if value.abs() < 2f64.powi(127) {
                closest_decimal_reference(value)
            } else {
                None
            };
            assert_eq!(
                Decimal::from_f64_closest(value).map(|d| d.canonical_lexical()),
                reference,
                "{value:e}"
            );
        }
    }
}
