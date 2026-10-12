// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact branch of the numeric operators: integer and decimal operands of any
//! size, computed on [`crate::exact`].
//!
//! [`XsdValue::Integer`] and [`XsdValue::Decimal`] carry one exact representation
//! at every size. Their inline values still use machine-word operations.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact::cost::{self, Shape};
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
    super::is_big(a) || super::is_big(b)
}

/// Whether the value belongs to the integer family (of any size).
const fn is_integer_family(value: &XsdValue) -> bool {
    matches!(value, XsdValue::Integer { .. })
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
#[cold]
#[inline(never)]
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

/// The admitted exact branch borrows input coefficients instead of cloning
/// magnitudes for numeric promotion.
pub(crate) fn binary_admitted(
    a: &XsdValue,
    b: &XsdValue,
    op: super::NumericBinaryOperator,
    policy: DivisionPolicy,
    admit: &mut impl FnMut(cost::NumericOperationLayout) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<XsdValue, super::NumericOperationError> {
    use super::{NumericBinaryOperator as Operator, NumericOperationError as Error};
    let integer_pair = is_integer_family(a) && is_integer_family(b);
    with_coefficient(a, |left, ls| {
        with_coefficient(b, |right, rs| {
            if integer_pair && op != Operator::Divide {
                let operation = match op {
                    Operator::Add => cost::IntegerOperation::Add,
                    Operator::Subtract => cost::IntegerOperation::Subtract,
                    Operator::Multiply => cost::IntegerOperation::Multiply,
                    Operator::Divide => unreachable!("division excluded"),
                };
                admit(left.operation_layout(right, operation)?)?;
                let result = match op {
                    Operator::Add => left.try_add(right)?,
                    Operator::Subtract => left.try_sub(right)?,
                    Operator::Multiply => left.try_mul(right)?,
                    Operator::Divide => unreachable!("division excluded"),
                };
                return Ok(XsdValue::from_exact_integer(result, XsdDatatype::Integer));
            }
            let result = match op {
                Operator::Add => exact::Decimal::try_add_parts_admitted(left, ls, right, rs, admit)
                    .map_err(exact::ExactOperationError::Storage),
                Operator::Subtract => {
                    exact::Decimal::try_sub_parts_admitted(left, ls, right, rs, admit)
                        .map_err(exact::ExactOperationError::Storage)
                }
                Operator::Multiply => {
                    exact::Decimal::try_mul_parts_admitted(left, ls, right, rs, admit)
                }
                Operator::Divide => {
                    exact::Decimal::try_div_parts_admitted(left, ls, right, rs, policy, admit)
                }
            };
            result
                .map(XsdValue::from_exact_decimal)
                .map_err(|error| match error {
                    exact::ExactOperationError::Storage(error) => Error::Storage(error),
                    exact::ExactOperationError::Value(ExactError::DivisionByZero) => {
                        Error::Value(XsdError::DivisionByZero {
                            datatype: if integer_pair {
                                XsdDatatype::Integer
                            } else {
                                XsdDatatype::Decimal
                            },
                        })
                    }
                    exact::ExactOperationError::Value(error) => {
                        Error::Value(XsdError::Exact(error))
                    }
                })
        })
    })
}

/// `op:numeric-divide` for two exact-branch operands under `policy`: always a
/// decimal (integer ÷ integer included, as XPath defines).
#[cold]
#[inline(never)]
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

pub(crate) fn with_coefficient<T>(
    value: &XsdValue,
    use_value: impl FnOnce(&exact::Integer, u32) -> T,
) -> T {
    match value {
        XsdValue::Integer { value, .. } => use_value(value, 0),
        XsdValue::Decimal(value) => use_value(value.unscaled(), value.scale()),
        _ => unreachable!("exact-branch operands were checked before dispatch"),
    }
}

pub(crate) fn cmp_admitted(
    a: &XsdValue,
    b: &XsdValue,
    admit: &mut impl FnMut(cost::NumericOperationLayout) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Ordering, crate::bigint::LimbScratchError> {
    with_coefficient(a, |left, ls| {
        with_coefficient(b, |right, rs| {
            exact::Decimal::try_cmp_parts_admitted(left, ls, right, rs, admit)
        })
    })
}

pub(crate) fn cmp_ieee_admitted_using(
    exact: &XsdValue,
    ieee: f64,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(cost::NumericOperationLayout) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<Option<Ordering>, crate::bigint::LimbScratchError> {
    with_coefficient(exact, |coefficient, scale| {
        exact::decimal::cmp_scaled_f64_admitted_using(coefficient, scale, ieee, storage, admit)
    })
}

/// Negation: an `xsd:integer` for an integer-family operand (F&O 3.1 §4.2), a
/// decimal otherwise.
#[cold]
#[inline(never)]
pub(crate) fn neg(value: &XsdValue) -> XsdValue {
    if is_integer_family(value) {
        return XsdValue::from_exact_integer(-&integer_of(value), XsdDatatype::Integer);
    }
    XsdValue::from_exact_decimal(-&decimal_of(value))
}

/// The absolute value: an `xsd:integer` for an integer-family operand, a decimal
/// otherwise.
#[cold]
#[inline(never)]
pub(crate) fn abs(value: &XsdValue) -> XsdValue {
    if is_integer_family(value) {
        return XsdValue::from_exact_integer(integer_of(value).abs(), XsdDatatype::Integer);
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
#[cold]
#[inline(never)]
pub(crate) fn round_to_integer(value: &XsdValue, rounding: Rounding) -> XsdValue {
    if is_integer_family(value) {
        return XsdValue::from_exact_integer(integer_of(value), XsdDatatype::Integer);
    }
    XsdValue::from_exact_decimal(exact::Decimal::from_integer(
        decimal_of(value).round_to_integer(rounding),
    ))
}

/// The size of an exact-branch value, read in constant time without copying or
/// touching its digits.
pub(crate) fn shape_of(value: &XsdValue) -> Option<Shape> {
    Some(match value {
        XsdValue::Integer { value, .. } => value.shape(),
        XsdValue::Decimal(decimal) => decimal.shape(),
        _ => return None,
    })
}

/// The cost of one exact-branch operation over these operands, for a governor:
/// computed from their shapes alone, in constant time.
pub(crate) fn cost(a: Shape, b: Shape, integers: bool, op: CostOp) -> exact::Cost {
    match op {
        CostOp::Add if integers => cost::add(a.limbs, b.limbs),
        CostOp::Mul if integers => cost::mul(a.limbs, b.limbs),
        CostOp::Add => cost::decimal_add(a, b),
        CostOp::Mul => cost::decimal_mul(a, b),
        CostOp::Compare => cost::decimal_cmp(a, b),
        CostOp::Div(policy) => cost::decimal_div(a, b, policy),
    }
}

/// The cost of an exact-branch value meeting an IEEE operand: its correctly rounded
/// conversion (arithmetic, and the promoting comparison), or its exact comparison
/// with the binary value (the total order), whichever is dearer.
pub(crate) const fn ieee_cost(value: Shape) -> exact::Cost {
    value.ieee_cost()
}

/// The operation an [`exact::Cost`](crate::exact::Cost) estimate is for.
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
