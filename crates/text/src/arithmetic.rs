// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native fixed-point refusals, retaining their original values and owners.

use crate::{Fixed, fixed::Scaled};

#[derive(Debug)]
pub(crate) enum ArithmeticRefusal {
    FromInteger {
        value: i64,
    },
    Add {
        left: i128,
        right: i128,
    },
    Subtract {
        left: i128,
        right: i128,
    },
    Multiply {
        left: i128,
        right: i128,
    },
    Divide {
        left: i128,
        right: i128,
    },
    DivisionByZero {
        dividend: i128,
    },
    ScaledDivisionByZero {
        dividend: purrdf_sparql_eval::WorkspaceInteger,
    },
    LogDomain {
        raw: i128,
    },
    RangeReduction {
        raw: u128,
        numerator: u128,
        denominator: u128,
        mantissa: Option<u128>,
        cause: Option<std::num::TryFromIntError>,
    },
    LogOverflow {
        raw: i128,
        exponent: i32,
        ln_mantissa: i128,
    },
    Narrow {
        value: Scaled,
    },
    ScoreOutsideBound {
        score: i128,
        maximum: i128,
    },
    FieldNormalization {
        b: i128,
        relative: i128,
        normalized: i128,
    },
    FieldFrequency {
        document: u32,
        predicate: u32,
        current: u64,
        added: u64,
    },
    FieldLength {
        field: usize,
        current: u64,
        added: u64,
    },
    PredicateOrdinal {
        ordinal: usize,
        cause: std::num::TryFromIntError,
    },
    PartitionRank {
        rank: usize,
        cause: std::num::TryFromIntError,
    },
    Exact {
        cause: purrdf_xsd::exact::ExactError,
        product: Scaled,
    },
}

impl ArithmeticRefusal {
    pub(crate) const fn is_domain(&self) -> bool {
        match self {
            Self::DivisionByZero { .. }
            | Self::ScaledDivisionByZero { .. }
            | Self::LogDomain { .. }
            | Self::ScoreOutsideBound { .. }
            | Self::FieldNormalization { .. }
            | Self::Exact { .. } => true,
            Self::FromInteger { .. }
            | Self::Add { .. }
            | Self::Subtract { .. }
            | Self::Multiply { .. }
            | Self::Divide { .. }
            | Self::RangeReduction { .. }
            | Self::LogOverflow { .. }
            | Self::Narrow { .. }
            | Self::FieldFrequency { .. }
            | Self::FieldLength { .. }
            | Self::PredicateOrdinal { .. }
            | Self::PartitionRank { .. } => false,
        }
    }
}

impl purrdf_hash::error::StructuredError for ArithmeticRefusal {
    purrdf_hash::error_visit! {
        FromInteger { value } => "text-fixed-from-integer-overflow" { "value" => Signed(i128::from(*value)) },
        Add { left, right } => "text-fixed-add-overflow" { "left_raw" => Signed(*left), "right_raw" => Signed(*right) },
        Subtract { left, right } => "text-fixed-subtract-overflow" { "left_raw" => Signed(*left), "right_raw" => Signed(*right) },
        Multiply { left, right } => "text-fixed-multiply-overflow" { "left_raw" => Signed(*left), "right_raw" => Signed(*right) },
        Divide { left, right } => "text-fixed-divide-overflow" { "left_raw" => Signed(*left), "right_raw" => Signed(*right) },
        DivisionByZero { dividend } => "text-fixed-division-by-zero" { "dividend_raw" => Signed(*dividend), "divisor_raw" => Signed(0) },
        ScaledDivisionByZero { dividend } => "text-scaled-division-by-zero" { "dividend_raw" => Record(&**dividend), "divisor_raw" => Signed(0) },
        LogDomain { raw } => "text-fixed-log-domain" { "raw" => Signed(*raw) },
        RangeReduction { raw, numerator, denominator, mantissa, cause } => "text-fixed-range-reduction" {
            "raw" => Unsigned(*raw), "numerator" => Unsigned(*numerator), "denominator" => Unsigned(*denominator),
            "mantissa" => Value(mantissa.map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Unsigned)),
            "cause" => Value(cause.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Cause(value))),
        },
        LogOverflow { raw, exponent, ln_mantissa } => "text-fixed-log-overflow" {
            "raw" => Signed(*raw), "exponent" => Signed(i128::from(*exponent)), "ln_mantissa" => Signed(*ln_mantissa),
        },
        Narrow { value } => "text-fixed-narrow-overflow" { "value" => Record(value), "scale_digits" => Unsigned(u128::from(crate::SCALE_DIGITS)) },
        ScoreOutsideBound { score, maximum } => "text-fixed-score-outside-bound" { "score_raw" => Signed(*score), "maximum_raw" => Signed(*maximum) },
        FieldNormalization { b, relative, normalized } => "text-fixed-field-normalization" { "b_raw" => Signed(*b), "relative_raw" => Signed(*relative), "normalized_raw" => Signed(*normalized) },
        FieldFrequency { document, predicate, current, added } => "text-field-frequency-overflow" {
            "document" => Unsigned(u128::from(*document)), "predicate" => Unsigned(u128::from(*predicate)), "current" => Unsigned(u128::from(*current)), "added" => Unsigned(u128::from(*added)),
        },
        FieldLength { field, current, added } => "text-field-length-overflow" { "field" => Unsigned(*field as u128), "current" => Unsigned(u128::from(*current)), "added" => Unsigned(u128::from(*added)) },
        PredicateOrdinal { ordinal, cause } => "text-field-predicate-ordinal-overflow" { "ordinal" => Unsigned(*ordinal as u128), "cause" => Cause(cause) },
        PartitionRank { rank, cause } => "text-partition-rank-overflow" { "rank" => Unsigned(*rank as u128), "limit" => Unsigned(u128::from(u32::MAX)), "cause" => Cause(cause) },
        Exact { cause, product } => "text-fixed-exact-domain" { "cause" => Cause(cause), "dividend" => Record(product), "divisor_raw" => Signed(0) },
    }
}

impl core::fmt::Display for ArithmeticRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(if self.is_domain() {
            "fixed-point domain error: "
        } else {
            "fixed-point overflow: "
        })?;
        match self {
            Self::FromInteger { value } => write!(f, "{value} does not fit the fixed-point range"),
            Self::Add { .. } => f.write_str("addition left the fixed-point range"),
            Self::Subtract { .. } => f.write_str("subtraction left the fixed-point range"),
            Self::Multiply { .. } => f.write_str("multiplication left the fixed-point range"),
            Self::Divide { .. } => f.write_str("division left the fixed-point range"),
            Self::DivisionByZero { .. } | Self::ScaledDivisionByZero { .. } => {
                f.write_str("division by zero")
            }
            Self::LogDomain { raw } => write!(
                f,
                "ln is undefined at {}, which is not positive",
                Fixed::from_raw(*raw).decimal_display()
            ),
            Self::RangeReduction { .. } => {
                f.write_str("range reduction left the fixed-point range")
            }
            Self::LogOverflow { .. } => f.write_str("logarithm left the fixed-point range"),
            Self::Narrow { .. } => f.write_str("result left the fixed-point range"),
            Self::ScoreOutsideBound { score, maximum } => write!(
                f,
                "score {} is outside the prepared interval [0, {}]",
                Fixed::from_raw(*score).decimal_display(),
                Fixed::from_raw(*maximum).decimal_display()
            ),
            Self::FieldNormalization { .. } => {
                f.write_str("field normalization rounds to zero under this profile")
            }
            Self::FieldFrequency { .. } => f.write_str("merged field term frequency exceeds u64"),
            Self::FieldLength { .. } => f.write_str("merged field length exceeds u64"),
            Self::PredicateOrdinal { .. } => {
                f.write_str("field scope predicate ordinal exceeds index representation")
            }
            Self::PartitionRank { .. } => {
                f.write_str("a partition holds more ranked rows than a u32 can number")
            }
            Self::Exact { .. } => f.write_str("division by zero"),
        }
    }
}

impl std::error::Error for ArithmeticRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Exact { cause, .. } => Some(cause),
            Self::RangeReduction {
                cause: Some(cause), ..
            }
            | Self::PredicateOrdinal { cause, .. }
            | Self::PartitionRank { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
