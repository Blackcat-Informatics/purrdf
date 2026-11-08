// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unsigned fusion values with explicit decimal precision.

use core::num::NonZeroU128;

use crate::FusionError;

/// The number of decimal fractional digits in a fusion law, from zero to 38.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Precision(NonZeroU128);

impl Precision {
    // Twelve is inside the admitted range; frozen fixtures use these explicit units.
    pub(crate) const FROZEN_12: Self =
        Self(NonZeroU128::new(1_000_000_000_000).expect("a positive power of ten"));
    /// Admit a decimal scale. Every power through `10^38` fits in `u128`.
    ///
    /// # Errors
    /// Returns [`FusionError::InvalidPrecision`] for more than 38 digits.
    pub const fn new(digits: u32) -> Result<Self, FusionError> {
        if digits <= 38 {
            // The admitted power is computed once. Arithmetic reads the scale
            // directly; its nonzero field also records that the units are valid.
            // INVARIANT: admitted powers lie in 1..=10^38, inside u128.
            Ok(Self(NonZeroU128::new(10_u128.pow(digits)).expect(
                "INVARIANT: an admitted decimal scale is a positive power of ten",
            )))
        } else {
            Err(FusionError::InvalidPrecision { digits })
        }
    }
    /// The declared number of fractional decimal digits.
    #[must_use]
    pub const fn digits(self) -> u32 {
        self.0.ilog10()
    }
    /// The number of raw units in one whole value.
    #[must_use]
    pub const fn scale(self) -> u128 {
        self.0.get()
    }
    /// A value in this precision's raw units.
    #[must_use]
    pub const fn score(self, raw: u128) -> Score {
        Score::from_raw(raw, self)
    }
}

impl core::fmt::Debug for Precision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Precision").field(&self.digits()).finish()
    }
}

/// An unsigned exact fusion weight, contribution or score, carrying its units.
///
/// Arithmetic refuses mismatched precision. Comparisons between different
/// precisions are unordered; callers must choose the law before comparing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Score {
    raw: u128,
    precision: Precision,
}

impl Score {
    /// Preserve the raw value and its declared decimal units without conversion.
    #[must_use]
    pub const fn from_raw(raw: u128, precision: Precision) -> Self {
        Self { raw, precision }
    }
    /// Convert a whole unsigned value to explicitly declared raw units.
    ///
    /// # Errors
    /// Refuses a value whose scaled representation exceeds `u128`.
    pub fn from_integer(value: u128, precision: Precision) -> Result<Self, FusionError> {
        value
            .checked_mul(precision.scale())
            .map(|raw| precision.score(raw))
            .ok_or(FusionError::Overflow)
    }
    /// Zero under an explicitly selected precision.
    #[must_use]
    pub const fn zero(precision: Precision) -> Self {
        precision.score(0)
    }
    /// The unsigned count of raw units.
    #[must_use]
    pub const fn into_raw(self) -> u128 {
        self.raw
    }
    /// The units this value carries.
    #[must_use]
    pub const fn precision(self) -> Precision {
        self.precision
    }
    /// Require this value to belong to a particular fusion law's units.
    ///
    /// # Errors
    /// Returns [`FusionError::PrecisionMismatch`] for different precisions.
    pub fn require_precision(self, expected: Precision) -> Result<(), FusionError> {
        if self.precision == expected {
            Ok(())
        } else {
            Err(FusionError::PrecisionMismatch {
                expected,
                got: self.precision,
            })
        }
    }
    /// Add values in the same units without wrapping.
    ///
    /// # Errors
    /// Refuses mismatched precision and an unrepresentable sum.
    pub fn checked_add(self, other: Self) -> Result<Self, FusionError> {
        other.require_precision(self.precision)?;
        self.raw
            .checked_add(other.raw)
            .map(|raw| self.precision.score(raw))
            .ok_or(FusionError::Overflow)
    }
    /// Subtract values in the same units without wrapping.
    ///
    /// # Errors
    /// Refuses mismatched precision and a negative result.
    pub fn checked_sub(self, other: Self) -> Result<Self, FusionError> {
        other.require_precision(self.precision)?;
        self.raw
            .checked_sub(other.raw)
            .map(|raw| self.precision.score(raw))
            .ok_or(FusionError::Overflow)
    }
    /// Multiply and truncate once at the declared precision.
    ///
    /// # Errors
    /// Refuses mismatched precision and an unrepresentable result.
    pub fn checked_mul(self, other: Self) -> Result<Self, FusionError> {
        other.require_precision(self.precision)?;
        purrdf_xsd::wide::mul_div(self.raw, other.raw, self.precision.scale())
            .map(|raw| self.precision.score(raw))
            .ok_or(FusionError::Overflow)
    }
    /// Render exact decimal units, retaining all declared fractional digits.
    #[must_use]
    pub fn to_decimal_lexical(self) -> String {
        self.to_string()
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        (self.precision == other.precision).then(|| self.raw.cmp(&other.raw))
    }
}

impl core::fmt::Display for Score {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let scale = self.precision.scale();
        if self.precision.digits() == 0 {
            return self.raw.fmt(f);
        }
        write!(
            f,
            "{}.{:0width$}",
            self.raw / scale,
            self.raw % scale,
            width = self.precision.digits() as usize
        )
    }
}

impl purrdf_hash::error::StructuredError for Score {
    purrdf_hash::error_visit! {
        @record self { raw, precision } => "retrieval-score" {
            "raw" => Unsigned(self.raw), "scale_digits" => Unsigned(u128::from(self.precision.digits()))
        }
    }
}

/// Profile-scoped frontier arithmetic. Its units live once on the profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RawScore(u128);

impl RawScore {
    pub(crate) const ZERO: Self = Self(0);
    pub(crate) const fn scaled(self, precision: Precision) -> Score {
        precision.score(self.0)
    }
    pub(crate) fn checked_add(self, other: Self) -> Result<Self, FusionError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(FusionError::Overflow)
    }
}

purrdf_lex::constructors! {
    impl RawScore {
        pub(crate) fn from_raw(raw: u128) -> Self;
    }
}
