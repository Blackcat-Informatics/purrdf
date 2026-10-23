// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Geometric API face over the workspace's canonical exact rational owner.

use super::Rational;
use super::certified::{
    DecimalConstructionError, RationalComparisonBody, RationalComparisonError,
    RationalNativeComparisonBody,
};
use crate::bigint::{BigInt as Int, LimbScratch, LimbScratchError};
use crate::exact::{ExactArithmeticCost, Integer};
use core::cmp::Ordering;
use std::borrow::Cow;

/// Canonical exact rational with the established geometric constructor surface.
/// All arithmetic, parsing and comparison execute the shared exact tower body.
/// Borrowed operands preserve original heap identity; inline views allocate no heap.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rat {
    value: Rational,
}

impl purrdf_hash::error::StructuredError for Rat {
    purrdf_hash::error_visit! {
        @record self { value } => "geometric-rational" { "value" => Record(&self.value) }
    }
}

impl Rat {
    const fn wrap(value: Rational) -> Self {
        Self { value }
    }

    /// The value zero, as `0/1`.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            value: Rational::ZERO,
        }
    }

    /// The value one, as `1/1`.
    #[must_use]
    pub fn one() -> Self {
        Self {
            value: Rational::ONE,
        }
    }

    /// Builds the rational `value / 1`.
    #[must_use]
    pub fn from_int(value: Int) -> Self {
        Self {
            value: Rational::from_bigint_integer(value),
        }
    }

    /// Builds the rational `value / 1` from an `i64`.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self::from_int(Int::from_i64(value))
    }

    /// Exact canonical value `numerator / 2^fractional_bits`.
    #[must_use]
    pub fn from_dyadic_integer(numerator: &Int, fractional_bits: u32) -> Self {
        Self::wrap(Rational::from_dyadic_integer(numerator, fractional_bits))
    }

    /// The same exact value in already admitted reusable limb destinations.
    ///
    /// # Errors
    /// Refuses destination exhaustion or capacity without allocating a fallback.
    pub fn from_dyadic_integer_in(
        numerator: &Int,
        fractional_bits: u32,
        scratch: &LimbScratch,
    ) -> Result<Self, LimbScratchError> {
        Rational::from_dyadic_integer_in(numerator, fractional_bits, scratch).map(Self::wrap)
    }

    /// Builds `num / den`, normalizing the sign and reducing to lowest terms.
    ///
    /// Returns `None` if and only if `den` is zero.
    pub fn new(num: Int, den: Int) -> Option<Self> {
        Rational::new(Integer::from_bigint(num), Integer::from_bigint(den))
            .ok()
            .map(Self::wrap)
    }

    /// The numerator, carrying the sign of the value.
    #[must_use]
    pub fn numerator(&self) -> Cow<'_, Int> {
        self.value.bigint_numerator()
    }

    /// The denominator, always strictly positive.
    #[must_use]
    pub fn denominator(&self) -> Cow<'_, Int> {
        self.value.bigint_denominator()
    }

    /// Borrow the original numerator and positive denominator, in that order.
    /// Admission and storage visitors can inspect both owners without copying.
    #[must_use]
    pub fn integer_operands(&self) -> [Cow<'_, Int>; 2] {
        self.value.integer_operands()
    }

    /// Heap limb capacity retained by the exact numerator and denominator.
    /// Inline limbs and the enclosing object are accounted by their owner.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.value.allocated_bytes()
    }

    /// Share canonical immutable long operands without changing their value or
    /// canonical representation. Preparation admits owned storage before this
    /// operation; later clones allocate no limbs or ownership headers.
    #[must_use]
    pub fn into_shared(self) -> Self {
        Self::wrap(self.value.into_shared())
    }

    /// Complete heap retained by the immutable shared original operands.
    #[must_use]
    pub fn shared_owned_bytes(&self) -> usize {
        self.value.shared_owned_bytes()
    }

    /// Copy the canonical coefficients into an original preparation account.
    ///
    /// # Errors
    /// Preserves the first native admission or allocation failure.
    pub fn detached_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        self.value.detached_with_memory(memory).map(Self::wrap)
    }

    /// Keep detached coefficients covered through their last immutable clone.
    ///
    /// # Errors
    /// Preserves a refused native shared-control allocation.
    pub fn publish_detached(
        &mut self,
        original: &purrdf_lex::allocation::SharedOwned<()>,
    ) -> Result<(), purrdf_lex::allocation::StorageError> {
        self.value.publish_detached(original)
    }

    /// Exact native coefficient and control layouts for detached publication.
    ///
    /// # Errors
    /// Refuses checked address-space layout overflow.
    pub fn detached_required_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
        self.value.detached_required_bytes()
    }

    /// Detached canonical numerator/denominator layouts for certified widths.
    /// This quotes the same BigInt factory before derived values are computed;
    /// canonical reduction can only reduce either resulting magnitude width.
    ///
    /// # Errors
    /// Refuses address-space, array or summed-control overflow.
    pub fn detached_layout_for_bits(
        widths: (u64, u64),
    ) -> Result<usize, purrdf_lex::allocation::StorageError> {
        use purrdf_lex::allocation::StorageError;
        <[u64; 2]>::from(widths)
            .into_iter()
            .try_fold(0_usize, |bytes, bits| {
                let limbs =
                    usize::try_from(bits.div_ceil(64)).map_err(|_| StorageError::SizeOverflow)?;
                bytes
                    .checked_add(Int::detached_layout_for_limbs(limbs)?)
                    .ok_or(StorageError::SizeOverflow)
            })
    }

    /// Copy the original canonical operands into admitted destinations.
    /// Already pooled operands keep their immutable original owner.
    ///
    /// # Errors
    /// Refuses capacity or destination exhaustion without a heap fallback.
    pub fn copy_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.copy_in(scratch).map(Self::wrap)
    }

    /// Absolute value in the original reusable destinations.
    ///
    /// # Errors
    /// Preserves the actual destination refusal without a heap fallback.
    pub fn abs_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.abs_in(scratch).map(Self::wrap)
    }

    /// Additive inverse in the original reusable destinations.
    ///
    /// # Errors
    /// Preserves the actual destination refusal without a heap fallback.
    pub fn neg_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.neg_in(scratch).map(Self::wrap)
    }

    /// Returns `self + other`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        Self::wrap(&self.value + &other.value)
    }

    /// Returns `self - other`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        Self::wrap(&self.value - &other.value)
    }

    /// Returns `self * other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        Self::wrap(&self.value * &other.value)
    }

    /// Returns `self / other`, or `None` when `other` is zero.
    pub fn div(&self, other: &Self) -> Option<Self> {
        self.value.checked_div(&other.value).ok().map(Self::wrap)
    }

    /// The identical exact sum in admitted reusable destinations.
    /// # Errors
    /// Refuses capacity or destination exhaustion without allocating a fallback.
    pub fn add_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.add_in(&other.value, scratch).map(Self::wrap)
    }

    /// The identical exact difference in admitted reusable destinations.
    /// # Errors
    /// Refuses capacity or destination exhaustion without allocating a fallback.
    pub fn sub_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.sub_in(&other.value, scratch).map(Self::wrap)
    }

    /// The identical exact product in admitted reusable destinations.
    /// # Errors
    /// Refuses capacity or destination exhaustion without allocating a fallback.
    pub fn mul_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.value.mul_in(&other.value, scratch).map(Self::wrap)
    }

    /// The identical quotient in admitted destinations, `None` for a zero divisor.
    /// # Errors
    /// Refuses physical capacity separately from the zero-divisor decision.
    pub fn div_in(
        &self,
        other: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        self.value
            .div_in(&other.value, scratch)
            .map(|value| value.map(Self::wrap))
    }

    /// Returns the additive inverse.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self::wrap(-&self.value)
    }

    /// Returns the absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self::wrap(self.value.abs())
    }

    /// Returns `-1`, `0` or `1` according to the sign.
    #[must_use]
    pub fn signum(&self) -> i32 {
        self.value.numerator().signum()
    }

    /// Returns `true` when this is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Returns `true` when the denominator is one.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.value.is_integer()
    }

    /// Returns the multiplicative inverse, or `None` when this is zero.
    pub fn recip(&self) -> Option<Self> {
        self.value.recip().ok().map(Self::wrap)
    }

    /// Builds the exact value `mantissa / 10^scale`.
    #[must_use]
    // Preserve the established owned constructor while sharing the exact
    // reduction body with the explicitly borrowed reusable-storage entry.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_decimal(mantissa: Int, scale: u32) -> Self {
        Self::wrap(Rational::from_scaled_integer(mantissa, scale))
    }

    /// The identical exact decimal in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating fallback.
    pub fn from_decimal_in(
        mantissa: &Int,
        scale: u32,
        scratch: &LimbScratch,
    ) -> Result<Self, LimbScratchError> {
        Rational::from_decimal_in(mantissa, scale, scratch).map(Self::wrap)
    }

    /// Decode a finite binary64 value as its exact dyadic rational.
    ///
    /// This is integer-only: NaN and infinity return `None`, while both signed
    /// zero spellings represent the same rational zero.
    #[must_use]
    pub fn from_binary64(value: f64) -> Option<Self> {
        Rational::from_binary64(value).map(Self::wrap)
    }

    /// Decode the same finite value using original admitted limb destinations.
    ///
    /// # Errors
    /// Refuses physical destination exhaustion without an allocating fallback.
    pub fn from_binary64_in(
        value: f64,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        Rational::from_binary64_in(value, scratch).map(|value| value.map(Self::wrap))
    }

    /// Parses an XSD-style decimal or double lexical form **exactly**.
    ///
    /// The accepted grammar is an optional sign, then either digits with an
    /// optional `.` and optional fraction digits or a bare `.` with fraction
    /// digits, then an optional exponent (`e` or `E`, an optional sign, and at
    /// least one digit). At least one mantissa digit is required. Nothing else
    /// is accepted: no whitespace on either side, no digit separators, no
    /// radix prefix, and none of the special values `NaN`, `INF` or `-INF`.
    ///
    /// No floating-point value is constructed anywhere on this path. The
    /// mantissa digits become an [`Int`] and the exponent becomes an exact
    /// power of ten, so `parse_decimal("0.1")` is precisely `1/10` and not the
    /// binary approximation the `f64` literal `0.1` would give.
    ///
    /// # The one refusal this parser makes
    ///
    /// The lexical form allows an unbounded exponent, but `1e999999999` would
    /// ask for a power of ten with a billion digits and exhaust memory before
    /// producing anything. So the **absolute** value of the exponent is capped
    /// at 100000 and anything above that returns `None`. The cap is on the
    /// exponent as written, so it applies even when the mantissa is zero
    /// (`0e200000` is refused); that keeps the refusal a property of the
    /// lexical form alone, and therefore identical on every host. `1e100000` is
    /// at the cap and parses; only `1e100001` and beyond are refused.
    pub fn parse_decimal(text: &str) -> Option<Self> {
        Rational::parse_decimal(text).map(Self::wrap)
    }

    /// Parse the same exact grammar after admitting original digit/power counts.
    /// The callback runs before constructing any integer or power of ten.
    ///
    /// # Errors
    /// Returns the callback's refusal without constructing the value.
    pub fn parse_decimal_admitted<E>(
        text: &str,
        admit: &mut impl FnMut(usize, u64) -> Result<(), E>,
    ) -> Result<Option<Self>, E> {
        Rational::parse_decimal_admitted(text, admit).map(|value| value.map(Self::wrap))
    }

    /// Parse the original decimal/double carrier grammar in original native
    /// limb destinations, with admission before the first coefficient birth.
    /// # Errors
    /// Preserves the exact admission or native destination refusal independently
    /// of a malformed spelling, which remains `Ok(None)`.
    pub fn parse_decimal_in<E>(
        text: &str,
        admit: &mut impl FnMut(usize, u64) -> Result<(), E>,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, DecimalConstructionError<E>> {
        Rational::parse_decimal_in(text, admit, scratch).map(|value| value.map(Self::wrap))
    }

    /// Inspect the original grammar without constructing an integer. Strict
    /// record adapters share these digit/power pre-construction counts.
    #[must_use]
    pub fn decimal_construction(text: &str) -> Option<(usize, u64)> {
        Rational::decimal_construction(text)
    }

    /// Exact native destination count and limb capacity for the shared decimal
    /// body, including its simultaneously live retained and working operands.
    ///
    /// # Errors
    /// Refuses checked destination layout overflow before construction.
    pub fn decimal_scratch_layout(
        digits: usize,
        power: u64,
    ) -> Result<(usize, usize), LimbScratchError> {
        Rational::decimal_scratch_layout(digits, power)
    }

    /// Returns the `f64` nearest to the exact value, rounding half to even.
    ///
    /// This is the rational's binary64 boundary. Exact arithmetic and rational
    /// decisions retain the original numerator and denominator.
    ///
    /// The value is not computed with floating-point operations. The sign,
    /// biased exponent and 52-bit trailing significand are derived by exact
    /// integer division of the numerator by the denominator — the quotient is
    /// the significand and the remainder decides the rounding, by comparing
    /// twice the remainder against the divisor — and the resulting `u64` bit
    /// pattern is handed to [`f64::from_bits`]. The result is therefore the
    /// correctly rounded double, bit-for-bit identical on every target, in every
    /// case **except the sign of a zero**, which is recorded below.
    ///
    /// Boundary behaviour:
    ///
    /// * Zero returns positive zero.
    /// * Values too small to represent round to **positive** zero even when
    ///   negative. IEEE-754 round-to-nearest would give `-0.0` here, so this is
    ///   the one place the result is not the correctly rounded double: `-1e-400`
    ///   returns `+0.0` where `str::parse::<f64>` returns `-0.0`.
    ///
    ///   This is a consequence of the number model rather than an oversight.
    ///   [`Rat`] has no signed zero — there is exactly one canonical
    ///   representation per value, which is what makes `Eq` and `Hash`
    ///   derivable — so by the time a value has underflowed there is no sign
    ///   left to carry. `-0.0` is likewise unrepresentable on the way IN:
    ///   [`Rat::parse_decimal`] reads `-0.0` as zero, so a `POINT(-0.0 5)`
    ///   ordinate has already lost its sign before this function sees it.
    ///   Consequently **this function never produces `-0.0` at all**, and a
    ///   consumer must not read the sign of a zero as information.
    /// * Subnormals are produced exactly, including the smallest one.
    /// * Values too large to represent return [`f64::INFINITY`] or
    ///   [`f64::NEG_INFINITY`].
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        let value = self.value.to_f64();
        if value.to_bits() << 1 == 0 {
            f64::from_bits(0)
        } else {
            value
        }
    }

    /// The minimum fraction scale that represents this rational exactly as a
    /// finite decimal, or `None` when its decimal expansion is nonterminating.
    ///
    /// For the normalized positive denominator `2^a * 5^b`, this returns
    /// `max(a,b)`; any remaining prime factor makes the expansion infinite.
    /// Integers and zero return `Some(0)`. This is an exact property of the
    /// original value, independent of a requested output grid or proof precision.
    /// The full-width count does not conflate a writer's narrower scale limit
    /// with nontermination. A writer checks that limit separately.
    #[must_use]
    pub fn finite_decimal_scale(&self) -> Option<u64> {
        self.value.finite_decimal_scale()
    }

    /// The identical finite-decimal decision in admitted reusable destinations.
    ///
    /// [`crate::integer::ExactArithmeticCost::finite_decimal_scale`] supplies
    /// the checked preflight from the actual denominator width. The caller
    /// admits that work and storage before entering this body.
    /// # Errors
    /// Refuses destination exhaustion or capacity without an allocating fallback.
    pub fn finite_decimal_scale_in(
        &self,
        scratch: &LimbScratch,
    ) -> Result<Option<u64>, LimbScratchError> {
        self.value.finite_decimal_scale_in(scratch)
    }

    /// Renders the value as a decimal literal with at most `max_scale` fraction
    /// digits, rounding half to even.
    ///
    /// Trailing zeros in the fraction are suppressed and a value that needs no
    /// fraction gets no trailing `.`, so an exact short value such as `1/2`
    /// renders as `0.5` whatever `max_scale` is. A value that rounds to zero
    /// renders as `0` with no sign — this function never emits `-0`.
    #[must_use]
    pub fn to_decimal_string(&self, max_scale: u32) -> String {
        self.value.to_decimal_string(max_scale)
    }

    /// Canonical decimal spelling under the declared exact rounding law.
    /// Directed forms are outward bounds on the same decimal grid. Trailing
    /// zeros and negative zero follow [`Self::to_decimal_string`].
    /// [`crate::integer::ExactArithmeticCost::rational_decimal`] admits the
    /// original rounding and formatting bodies before any output allocation.
    #[must_use]
    pub fn to_decimal_string_with(
        &self,
        max_scale: u32,
        rounding: crate::ieee::ratio::Rounding,
    ) -> String {
        self.value.to_decimal_string_with(max_scale, rounding)
    }

    /// Returns `round(self * 10^scale)`, rounding half to even.
    ///
    /// This is the mantissa of the value rounded to `scale` decimal places;
    /// `Rat::from_decimal(value.round_to_scale(s), s)` is the rounded value.
    #[must_use]
    pub fn round_to_scale(&self, scale: u32) -> Int {
        self.value.round_to_scale(scale)
    }

    /// Identical half-even decimal rounding in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating fallback.
    pub fn round_to_scale_in(
        &self,
        scale: u32,
        scratch: &LimbScratch,
    ) -> Result<Int, LimbScratchError> {
        self.value.round_to_scale_in(scale, scratch)
    }

    /// Integer mantissa rounded on a declared decimal grid. Directed laws give
    /// exact outward bounds: `Down <= self*10^scale <= Up`. Neither an
    /// intermediate binary value nor proof precision chooses the grid.
    #[must_use]
    pub fn round_to_scale_with(&self, scale: u32, rounding: crate::ieee::ratio::Rounding) -> Int {
        self.value.round_to_scale_with(scale, rounding)
    }

    /// The same decimal rounding using only admitted reusable destinations.
    /// [`crate::integer::ExactArithmeticCost::rational_round`] bounds the power,
    /// product quotient and final increment before this operation.
    /// # Errors
    /// Refuses destination exhaustion without an allocation fallback.
    pub fn round_to_scale_with_in(
        &self,
        scale: u32,
        rounding: crate::ieee::ratio::Rounding,
        scratch: &LimbScratch,
    ) -> Result<Int, LimbScratchError> {
        self.value.round_to_scale_with_in(scale, rounding, scratch)
    }

    /// Identical exact rational ordering in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating product fallback.
    pub fn compare_in(
        &self,
        other: &Self,
        scratch: &LimbScratch,
    ) -> Result<Ordering, LimbScratchError> {
        self.value.compare_in(&other.value, scratch)
    }

    /// Execute the original exact ordering with admission for each actual step.
    ///
    /// The callback first receives the complete sign/denominator scan bound,
    /// before those sources are inspected. If signs differ, that scan completes
    /// the comparison. Otherwise it receives either the borrowed signed
    /// numerator scan or the original cross-product bound. The callback must
    /// execute its supplied body exactly once, keep the allowance live through
    /// execution, and return the unchanged result. This lets governors poll
    /// before each step without copying the branch decision in another crate.
    /// # Errors
    /// Returns checked-bound overflow or the callback's first refusal.
    pub fn compare_admitted<E>(
        &self,
        other: &Self,
        run: impl FnMut(ExactArithmeticCost, &mut RationalComparisonBody<'_>) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        self.value.compare_admitted(&other.value, run)
    }

    /// The same scoped ordering, with products in an admitted destination arena.
    /// # Errors
    /// Returns checked-bound overflow or the callback's admission/body refusal;
    /// the supplied body has no heap fallback on arena exhaustion.
    pub fn compare_admitted_in<E>(
        &self,
        other: &Self,
        scratch: &LimbScratch,
        run: impl FnMut(ExactArithmeticCost, &mut RationalComparisonBody<'_>) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        self.value.compare_admitted_in(&other.value, scratch, run)
    }

    /// Use the original comparison with an arena selected after branch admission.
    ///
    /// # Errors
    /// Preserves checked-bound overflow and the original caller/native failure.
    pub fn compare_admitted_by<E>(
        &self,
        other: &Self,
        run: impl FnMut(
            ExactArithmeticCost,
            &mut RationalNativeComparisonBody<'_>,
        ) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        self.value.compare_admitted_by(&other.value, run)
    }

    /// Decide exactly whether `lower <= self - other < upper`.
    ///
    /// The integer interval is closed on its lower bound and open on its upper
    /// bound. Empty or reversed intervals return false. Cross products use the
    /// positive original denominators directly, without constructing a reduced
    /// rational difference or performing a greatest common divisor.
    #[must_use]
    pub fn difference_in_integer_interval(&self, other: &Self, lower: i64, upper: i64) -> bool {
        self.value
            .difference_in_integer_interval(&other.value, lower, upper)
    }

    /// The identical half-open difference decision in admitted destinations.
    ///
    /// [`crate::integer::ExactArithmeticCost::rational_difference_integer_interval`]
    /// supplies the checked preflight from both original operand shapes and the
    /// largest integer-bound magnitude width. The caller admits that work and
    /// storage before entering this body.
    /// # Errors
    /// Refuses destination exhaustion or capacity without an allocating fallback.
    pub fn difference_in_integer_interval_in(
        &self,
        other: &Self,
        lower: i64,
        upper: i64,
        scratch: &LimbScratch,
    ) -> Result<bool, LimbScratchError> {
        self.value
            .difference_in_integer_interval_in(&other.value, lower, upper, scratch)
    }

    /// Exact reduction into `[origin,origin+period)`, or `None` for period zero.
    /// Numerator congruence preserves the existing coprime denominator.
    #[must_use]
    pub fn modulo_integer(&self, origin: i64, period: u64) -> Option<Self> {
        self.value.modulo_integer(origin, period).map(Self::wrap)
    }

    /// The same exact integer-period reduction in admitted destinations.
    /// # Errors
    /// Refuses destination exhaustion; a zero period returns `Ok(None)`.
    pub fn modulo_integer_in(
        &self,
        origin: i64,
        period: u64,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        self.value
            .modulo_integer_in(origin, period, scratch)
            .map(|value| value.map(Self::wrap))
    }
}

impl Default for Rat {
    fn default() -> Self {
        Self::zero()
    }
}

impl core::fmt::Debug for Rat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rat")
            .field("num", &self.numerator())
            .field("den", &self.denominator())
            .finish()
    }
}
