// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The integer custom aggregates the aggregate-seam tests register: a `SUM`-alike, a
//! `PRODUCT`-alike that declares exactly like it, and a two-argument weighted sum.
//!
//! One accumulator folds all three, so the step, combine and finish logic every test
//! registers is written once. Non-numeric arguments are ignored (never observed, since
//! every fixture value is an `xsd:integer` literal).

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_core::TermValue;
use purrdf_sparql_eval::{
    AggregateAccumulator, AlgebraicClass, Arity, CustomAggregate, EvalError, Volatility,
};

/// `xsd:integer`, the datatype every fold result is minted with.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// How an integer fixture aggregate folds its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    /// The running sum of the single argument.
    Sum,
    /// The running product of the single argument: declares identically to [`Fold::Sum`]
    /// but computes a different answer.
    Product,
    /// The running sum of `value * weight` over a two-argument tuple.
    WeightedSum,
}

impl Fold {
    /// The fold's identity element, the accumulator's starting total.
    const fn identity(self) -> i64 {
        match self {
            Self::Sum | Self::WeightedSum => 0,
            Self::Product => 1,
        }
    }

    /// `total` with `value` folded in.
    const fn apply(self, total: i64, value: i64) -> i64 {
        match self {
            Self::Sum | Self::WeightedSum => total + value,
            Self::Product => total * value,
        }
    }
}

/// The integer lexical form of `term`, when it is a literal holding one.
fn integer(term: Option<&TermValue>) -> Option<i64> {
    match term {
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.parse::<i64>().ok(),
        _ => None,
    }
}

/// The running total of one [`Fold`].
pub struct FoldAccumulator {
    fold: Fold,
    total: i64,
}

impl AggregateAccumulator for FoldAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let value = match self.fold {
            Fold::Sum | Fold::Product => integer(args.first()),
            Fold::WeightedSum => integer(args.first())
                .zip(integer(args.get(1)))
                .map(|(value, weight)| value * weight),
        };
        if let Some(value) = value {
            self.total = self.fold.apply(self.total, value);
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(value) = integer(other.finish()?.as_ref()) {
            self.total = self.fold.apply(self.total, value);
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

/// A commutative integer aggregate with no state bound, folding by [`Self::fold`] under
/// the declared [`Self::volatility`].
pub struct FoldAggregate {
    /// What the accumulator computes.
    pub fold: Fold,
    /// The declared determinism class.
    pub volatility: Volatility,
}

impl FoldAggregate {
    /// A [`Volatility::Stable`] aggregate folding by `fold`.
    pub const fn stable(fold: Fold) -> Self {
        Self {
            fold,
            volatility: Volatility::Stable,
        }
    }
}

impl CustomAggregate for FoldAggregate {
    fn arity(&self) -> Arity {
        match self.fold {
            Fold::Sum | Fold::Product => Arity::Exact(1),
            Fold::WeightedSum => Arity::Exact(2),
        }
    }
    fn volatility(&self) -> Volatility {
        self.volatility
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(FoldAccumulator {
            fold: self.fold,
            total: self.fold.identity(),
        })
    }
}
