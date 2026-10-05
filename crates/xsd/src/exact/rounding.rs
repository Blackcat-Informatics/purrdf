// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Rounding directions and the caller-configurable decimal-quotient precision
//! policy.

use std::cmp::Ordering;

/// A direction for rounding an exact value to a coarser decimal grid.
///
/// The XPath F&O 3.1 functions map onto these: `fn:round` is
/// [`Rounding::HalfCeiling`], `fn:round-half-to-even` is [`Rounding::HalfEven`],
/// `fn:floor` is [`Rounding::Floor`], `fn:ceiling` is [`Rounding::Ceiling`], the
/// cast to `xs:integer` is [`Rounding::TowardZero`], and the float-to-decimal
/// cast's "closest, ties toward zero" is [`Rounding::HalfTowardZero`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rounding {
    /// Discard the excess digits (truncation).
    TowardZero,
    /// Increase the magnitude whenever any excess digit is nonzero.
    AwayFromZero,
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceiling,
    /// To the nearest; a tie goes to the even neighbour (IEEE 754
    /// `roundTiesToEven`, `fn:round-half-to-even`).
    HalfEven,
    /// To the nearest; a tie goes away from zero.
    HalfAwayFromZero,
    /// To the nearest; a tie goes toward zero.
    HalfTowardZero,
    /// To the nearest; a tie goes toward positive infinity (`fn:round`).
    HalfCeiling,
    /// To the nearest; a tie goes toward negative infinity.
    HalfFloor,
}

impl Rounding {
    /// Whether a truncated magnitude must be increased by one unit.
    ///
    /// `negative` is the sign of the exact value, `odd` whether the truncated
    /// magnitude is odd, `half` the excess compared with half a unit (`Less`,
    /// `Equal` or `Greater`), and `inexact` whether the excess is nonzero at all.
    #[must_use]
    pub(crate) const fn increments(
        self,
        negative: bool,
        odd: bool,
        half: Ordering,
        inexact: bool,
    ) -> bool {
        if !inexact {
            return false;
        }
        let above = matches!(half, Ordering::Greater);
        let tie = matches!(half, Ordering::Equal);
        match self {
            Self::TowardZero => false,
            Self::AwayFromZero => true,
            Self::Floor => negative,
            Self::Ceiling => !negative,
            Self::HalfEven => above || (tie && odd),
            Self::HalfAwayFromZero => above || tie,
            Self::HalfTowardZero => above,
            Self::HalfCeiling => above || (tie && !negative),
            Self::HalfFloor => above || (tie && negative),
        }
    }
}

/// How [`crate::exact::Decimal::div`] (`op:numeric-divide` over decimals) forms
/// a quotient that may have no finite decimal expansion.
///
/// XPath F&O 3.1 §4.2 leaves the precision of a decimal quotient
/// implementation-defined; this is where the implementation lets its caller
/// define it. The default, [`DivisionPolicy::xsd_default`], is eighteen
/// fractional digits truncated toward zero: exactly the quotient the 3.x bounded
/// [`crate::numeric::Decimal`] produces wherever its `i128` mantissa holds the
/// result, so adopting the exact tower as the default representation changes no
/// quotient that is representable today. A reasoner that needs the exact value
/// of a non-terminating quotient divides [`crate::exact::Rational`]s instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DivisionPolicy {
    /// Round the exact quotient to `scale` fractional digits in direction
    /// `rounding`. A quotient that terminates within `scale` digits is exact.
    Scale {
        /// Fractional digits kept.
        scale: u32,
        /// How the excess is rounded away.
        rounding: Rounding,
    },
    /// The exact quotient, at whatever scale its expansion terminates; a
    /// quotient with no finite expansion is [`crate::exact::ExactError::NonTerminating`].
    /// The scale of a terminating quotient is bounded by the divisor: it is at
    /// most the divisor's scale plus the larger of the powers of two and five
    /// in the reduced divisor, which [`crate::exact::Decimal::div_cost`] charges
    /// for.
    Exact,
}

impl DivisionPolicy {
    /// The scale of [`Self::xsd_default`]: eighteen fractional digits, the 3.x
    /// bounded decimal's maximum scale.
    pub const DEFAULT_SCALE: u32 = 18;

    /// Eighteen fractional digits, truncated toward zero — see the type docs.
    #[must_use]
    pub const fn xsd_default() -> Self {
        Self::Scale {
            scale: Self::DEFAULT_SCALE,
            rounding: Rounding::TowardZero,
        }
    }

    /// Round to `scale` fractional digits in direction `rounding`.
    #[must_use]
    pub const fn scale(scale: u32, rounding: Rounding) -> Self {
        Self::Scale { scale, rounding }
    }
}

purrdf_hash::default_from_new!(DivisionPolicy => xsd_default);
