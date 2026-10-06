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
/// fractional digits truncated toward zero: exactly the quotient the bounded
/// [`crate::numeric::Decimal`] produces wherever its `i128` mantissa holds the
/// result. A reasoner that needs the exact value of a non-terminating quotient
/// divides [`crate::exact::Rational`]s instead.
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
    /// The scale of [`Self::xsd_default`]: eighteen fractional digits, the bounded
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

impl Rounding {
    /// Every direction, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::TowardZero,
        Self::AwayFromZero,
        Self::Floor,
        Self::Ceiling,
        Self::HalfEven,
        Self::HalfAwayFromZero,
        Self::HalfTowardZero,
        Self::HalfCeiling,
        Self::HalfFloor,
    ];

    /// The direction's stable kebab-case name (`"half-even"`), as
    /// [`DivisionPolicy`]'s text form spells it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TowardZero => "toward-zero",
            Self::AwayFromZero => "away-from-zero",
            Self::Floor => "floor",
            Self::Ceiling => "ceiling",
            Self::HalfEven => "half-even",
            Self::HalfAwayFromZero => "half-away-from-zero",
            Self::HalfTowardZero => "half-toward-zero",
            Self::HalfCeiling => "half-ceiling",
            Self::HalfFloor => "half-floor",
        }
    }
}

/// [`DivisionPolicy`]'s text form was not one of the forms it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivisionPolicyError {
    text: String,
}

impl std::fmt::Display for DivisionPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "expected a division policy `exact`, `N` or `N:ROUNDING` with N a digit count and \
             ROUNDING one of {}, got {:?}",
            Rounding::ALL.map(Rounding::label).join(", "),
            self.text
        )
    }
}

impl std::error::Error for DivisionPolicyError {}

impl std::str::FromStr for DivisionPolicy {
    type Err = DivisionPolicyError;

    /// The policy's one text form, shared by every surface that takes it (the
    /// command line, the C, WebAssembly and Python bindings): `exact`; `N`, `N`
    /// fractional digits truncated toward zero; or `N:ROUNDING`, rounded in the named
    /// direction ([`Rounding::label`]).
    ///
    /// ```rust
    /// use purrdf_xsd::exact::{DivisionPolicy, Rounding};
    ///
    /// assert_eq!("exact".parse(), Ok(DivisionPolicy::Exact));
    /// assert_eq!("18".parse(), Ok(DivisionPolicy::xsd_default()));
    /// assert_eq!("5:half-even".parse(), Ok(DivisionPolicy::scale(5, Rounding::HalfEven)));
    /// assert!("5:sideways".parse::<DivisionPolicy>().is_err());
    /// assert_eq!(DivisionPolicy::scale(5, Rounding::HalfEven).to_string(), "5:half-even");
    /// ```
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refuse = || DivisionPolicyError {
            text: text.to_owned(),
        };
        if text == "exact" {
            return Ok(Self::Exact);
        }
        let (digits, rounding) = text.split_once(':').unwrap_or((text, "toward-zero"));
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(refuse());
        }
        let scale = digits.parse::<u32>().map_err(|_| refuse())?;
        let rounding = Rounding::ALL
            .into_iter()
            .find(|candidate| candidate.label() == rounding)
            .ok_or_else(refuse)?;
        Ok(Self::scale(scale, rounding))
    }
}

impl std::fmt::Display for DivisionPolicy {
    /// The text form [`std::str::FromStr`] reads: `exact`, or `N:ROUNDING`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact => f.write_str("exact"),
            Self::Scale { scale, rounding } => write!(f, "{scale}:{}", rounding.label()),
        }
    }
}
