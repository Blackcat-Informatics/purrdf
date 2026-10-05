// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact branch of the numeric operators: integer and decimal operands of any
//! size, computed on [`crate::exact`].
//!
//! The public operators in [`crate::numeric`] answer an operation over two
//! bounded values ([`XsdValue::Integer`], [`XsdValue::Decimal`]) with the bounded
//! arithmetic whenever its result is exact and representable, and come here
//! otherwise: when an operand is [`XsdValue::BigInteger`] or
//! [`XsdValue::BigDecimal`], when an `i128` result overflows, and when a product
//! needs more than eighteen fractional digits. Every result goes back through
//! [`XsdValue::from_exact_integer`] / [`XsdValue::from_exact_decimal`], so a value
//! the bounded variants hold is always returned in them.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact::{self, DivisionPolicy, ExactError, Rounding};
use crate::value::{XsdError, XsdValue};

/// A binary operator of the exact branch.
#[derive(Clone, Copy)]
pub(crate) enum Op {
    Add,
    Sub,
    Mul,
}

/// Whether either operand is on the exact branch but outside the bounded
/// variants, which the bounded arithmetic cannot read.
pub(crate) const fn involves_big(a: &XsdValue, b: &XsdValue) -> bool {
    matches!(a, XsdValue::BigInteger { .. } | XsdValue::BigDecimal(_))
        || matches!(b, XsdValue::BigInteger { .. } | XsdValue::BigDecimal(_))
}

/// Whether the value belongs to the integer family (of any size).
const fn is_integer_family(value: &XsdValue) -> bool {
    matches!(
        value,
        XsdValue::Integer { .. } | XsdValue::BigInteger { .. }
    )
}

/// The exact decimal of an exact-branch operand.
fn decimal_of(value: &XsdValue) -> exact::Decimal {
    value
        .to_exact_decimal()
        .expect("the caller passes exact-branch operands only")
}

/// The exact integer of an integer-family operand.
fn integer_of(value: &XsdValue) -> exact::Integer {
    value
        .to_exact_integer()
        .expect("the caller passes integer-family operands only")
}

/// `a op b` for two exact-branch operands: an integer result when both are
/// integers, a decimal result otherwise.
pub(crate) fn binop(a: &XsdValue, b: &XsdValue, op: Op) -> Result<XsdValue, XsdError> {
    if is_integer_family(a) && is_integer_family(b) {
        let (x, y) = (integer_of(a), integer_of(b));
        let result = match op {
            Op::Add => &x + &y,
            Op::Sub => &x - &y,
            Op::Mul => &x * &y,
        };
        return Ok(XsdValue::from_exact_integer(result, XsdDatatype::Integer));
    }
    let (x, y) = (decimal_of(a), decimal_of(b));
    let result = match op {
        Op::Add => &x + &y,
        Op::Sub => &x - &y,
        Op::Mul => x.try_mul(&y).map_err(XsdError::Exact)?,
    };
    Ok(XsdValue::from_exact_decimal(result))
}

/// `op:numeric-divide` for two exact-branch operands under `policy`: always a
/// decimal (integer ÷ integer included, as XPath defines).
pub(crate) fn div(
    a: &XsdValue,
    b: &XsdValue,
    policy: DivisionPolicy,
) -> Result<XsdValue, XsdError> {
    let (x, y) = (decimal_of(a), decimal_of(b));
    x.div(&y, policy)
        .map(XsdValue::from_exact_decimal)
        .map_err(|error| match error {
            ExactError::DivisionByZero => XsdError::DivisionByZero {
                datatype: if is_integer_family(a) && is_integer_family(b) {
                    XsdDatatype::Integer
                } else {
                    XsdDatatype::Decimal
                },
            },
            other => XsdError::Exact(other),
        })
}

/// The exact order of two exact-branch operands.
pub(crate) fn cmp(a: &XsdValue, b: &XsdValue) -> Ordering {
    if is_integer_family(a) && is_integer_family(b) {
        return integer_of(a).cmp(&integer_of(b));
    }
    decimal_of(a).cmp(&decimal_of(b))
}

/// The exact order of an exact-branch operand against a finite or infinite IEEE
/// value; `None` only for `NaN`.
pub(crate) fn cmp_ieee(exact: &XsdValue, ieee: f64) -> Option<Ordering> {
    if ieee.is_nan() {
        return None;
    }
    if ieee.is_infinite() {
        return Some(if ieee.is_sign_positive() {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let left = exact::Rational::from_decimal(&decimal_of(exact));
    let right = exact::Rational::from_f64(ieee).expect("finite was just checked");
    Some(left.cmp(&right))
}

/// The correctly rounded `f64` of an exact-branch operand.
pub(crate) fn to_f64(value: &XsdValue) -> f64 {
    match value {
        XsdValue::BigInteger { value, .. } => value.to_f64(),
        _ => decimal_of(value).to_f64(),
    }
}

/// The correctly rounded `f32` of an exact-branch operand, rounded once.
pub(crate) fn to_f32(value: &XsdValue) -> f32 {
    match value {
        XsdValue::BigInteger { value, .. } => value.to_f32(),
        _ => decimal_of(value).to_f32(),
    }
}

/// Negation, keeping the operand's datatype family.
pub(crate) fn neg(value: &XsdValue) -> XsdValue {
    if is_integer_family(value) {
        return XsdValue::from_exact_integer(-&integer_of(value), value.datatype());
    }
    XsdValue::from_exact_decimal(-&decimal_of(value))
}

/// The absolute value, keeping the operand's datatype family.
pub(crate) fn abs(value: &XsdValue) -> XsdValue {
    if is_integer_family(value) {
        return XsdValue::from_exact_integer(integer_of(value).abs(), value.datatype());
    }
    let decimal = decimal_of(value);
    XsdValue::from_exact_decimal(if decimal.is_negative() {
        -&decimal
    } else {
        decimal
    })
}

/// The value rounded to an integer in direction `rounding`, keeping the operand's
/// datatype family (`fn:ceiling`, `fn:floor`, `fn:round` over decimals return a
/// decimal).
pub(crate) fn round_to_integer(value: &XsdValue, rounding: Rounding) -> XsdValue {
    if is_integer_family(value) {
        return value.clone();
    }
    XsdValue::from_exact_decimal(exact::Decimal::from_integer(
        decimal_of(value).round_to_integer(rounding),
    ))
}

/// The cost of one exact-branch operation over these operands, for a governor.
pub(crate) fn cost(a: &XsdValue, b: &XsdValue, op: CostOp) -> exact::Cost {
    if is_integer_family(a) && is_integer_family(b) && !matches!(op, CostOp::Div(_)) {
        let (x, y) = (integer_of(a), integer_of(b));
        return match op {
            CostOp::Add | CostOp::Compare => x.add_cost(&y),
            CostOp::Mul => x.mul_cost(&y),
            CostOp::Div(_) => unreachable!("excluded above"),
        };
    }
    let (x, y) = (decimal_of(a), decimal_of(b));
    match op {
        CostOp::Add | CostOp::Compare => x.add_cost(&y),
        CostOp::Mul => x.mul_cost(&y),
        CostOp::Div(policy) => x.div_cost(&y, policy),
    }
}

/// The operation a [`cost`] estimate is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostOp {
    /// `+` or `−`.
    Add,
    /// `×`.
    Mul,
    /// `÷` under the given policy.
    Div(DivisionPolicy),
    /// A comparison (`=`, `<`, an `ORDER BY` step).
    Compare,
}
