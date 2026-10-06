// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The integer and decimal branch of the numeric operators, computed on
//! [`crate::exact`]: the decimal arithmetic, the division under a policy, the exact
//! comparison with an IEEE value, and the governor's view of an operand's size.

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact::cost::{self, Shape};
use crate::exact::{self, DivisionPolicy, ExactError};
use crate::value::{XsdError, XsdValue};

/// A binary operator of the exact branch.
#[derive(Clone, Copy)]
pub(crate) enum Op {
    Add,
    Sub,
    Mul,
}

/// The exact decimal of an integer or decimal operand. Cloning an inline value is a
/// copy of its machine words.
fn decimal_of(value: &XsdValue) -> exact::Decimal {
    match value {
        XsdValue::Integer { value, .. } => exact::Decimal::from_integer(value.clone()),
        XsdValue::Decimal(decimal) => decimal.clone(),
        _ => unreachable!("the caller passes integer and decimal operands only"),
    }
}

/// `a op b` for two integer or decimal operands of which at least one is a
/// decimal: an exact decimal.
pub(crate) fn decimal_binop(a: &XsdValue, b: &XsdValue, op: Op) -> Result<XsdValue, XsdError> {
    if let (XsdValue::Decimal(x), XsdValue::Decimal(y)) = (a, b) {
        return decimal_op(x, y, op);
    }
    decimal_op(&decimal_of(a), &decimal_of(b), op)
}

fn decimal_op(x: &exact::Decimal, y: &exact::Decimal, op: Op) -> Result<XsdValue, XsdError> {
    Ok(XsdValue::Decimal(match op {
        Op::Add => x + y,
        Op::Sub => x - y,
        Op::Mul => x.try_mul(y).map_err(XsdError::Exact)?,
    }))
}

/// `op:numeric-divide` for two integer or decimal operands under `policy`: always a
/// decimal (integer ÷ integer included, as XPath defines).
pub(crate) fn div(
    a: &XsdValue,
    b: &XsdValue,
    policy: DivisionPolicy,
) -> Result<XsdValue, XsdError> {
    let quotient = match (a, b) {
        (XsdValue::Decimal(x), XsdValue::Decimal(y)) => x.div(y, policy),
        _ => decimal_of(a).div(&decimal_of(b), policy),
    };
    quotient
        .map(XsdValue::Decimal)
        .map_err(|error| match error {
            ExactError::DivisionByZero => XsdError::DivisionByZero {
                datatype: if matches!(a, XsdValue::Integer { .. })
                    && matches!(b, XsdValue::Integer { .. })
                {
                    XsdDatatype::Integer
                } else {
                    XsdDatatype::Decimal
                },
            },
            other => XsdError::Exact(other),
        })
}

/// The exact order of an integer against a decimal.
pub(crate) fn integer_cmp_decimal(x: &exact::Integer, y: &exact::Decimal) -> Ordering {
    if y.is_integer() {
        return x.cmp(y.unscaled());
    }
    exact::Decimal::from_integer(x.clone()).cmp(y)
}

/// The exact order of an integer or decimal operand against a finite or infinite
/// IEEE value; `None` only for `NaN`. Linear in the operand, with no reduction to
/// lowest terms ([`exact::Decimal::cmp_f64`]).
pub(crate) fn cmp_ieee(exact: &XsdValue, ieee: f64) -> Option<Ordering> {
    match exact {
        XsdValue::Integer { value, .. } => value.cmp_f64(ieee),
        XsdValue::Decimal(decimal) => decimal.cmp_f64(ieee),
        _ => None,
    }
}

/// The size of an integer or decimal value, read in constant time without copying
/// or touching its digits.
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
