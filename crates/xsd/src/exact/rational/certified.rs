// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scoped certified operations on the canonical exact rational owner.

use super::Rational;
use crate::bigint::scratch::{Allocate, Unbounded};
use crate::bigint::{BigInt as Int, LimbScratch, LimbScratchError};
use crate::exact::{ExactArithmeticCost, ExactOperation, Integer};
use core::cmp::Ordering;
use std::borrow::Cow;

/// Maximum absolute written exponent accepted by the exact carrier parser.
const MAX_DECIMAL_EXPONENT: i64 = 100_000;

/// One scoped step of the original rational comparison.
/// The admission callback executes this body exactly once and returns its
/// unchanged result while the supplied scratch allowance remains live.
pub type RationalComparisonBody<'a> = dyn FnMut() -> Result<Ordering, LimbScratchError> + 'a;

/// Original comparison phase with the caller's arena supplied after branch admission.
/// A borrowed scan accepts `None`; a product phase requires its admitted arena.
pub type RationalNativeComparisonBody<'a> =
    dyn FnMut(Option<&LimbScratch>) -> Result<Ordering, LimbScratchError> + 'a;

/// A checked comparison bound or its caller's admission refused execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RationalComparisonError<E> {
    /// The original comparison's complete work/storage bound overflowed.
    CostOverflow,
    /// A caller's scoped admission or original bounded body refused.
    Admission(E),
}

/// Admission and physical destination failures of the original decimal body.
/// Lexically invalid spellings remain `Ok(None)` before any destination birth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecimalConstructionError<E> {
    /// The original caller refused the scanned digit and exponent counts.
    Admission(E),
    /// The original native limb destination could not be produced.
    Storage(LimbScratchError),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for RationalComparisonError<E>
{
    purrdf_hash::error_visit! {
        CostOverflow => "rational-comparison-cost-overflow" {},
        Admission(error) => "rational-comparison-admission" { "cause" => Cause(error) },
    }
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for DecimalConstructionError<E>
{
    purrdf_hash::error_visit! {
        Admission(error) => "decimal-construction-admission" { "cause" => Cause(error) },
        Storage(error) => "decimal-construction-storage" { "cause" => Cause(error) },
    }
}

#[derive(Clone, Copy)]
enum ComparisonPhase {
    Scan,
    Numerator,
    Products,
}

impl Rational {
    /// Borrow the reduced numerator without copying a heap magnitude.
    #[must_use]
    pub fn bigint_numerator(&self) -> Cow<'_, Int> {
        self.numerator.bigint_operand()
    }

    /// Borrow the positive denominator without copying a heap magnitude.
    #[must_use]
    pub fn bigint_denominator(&self) -> Cow<'_, Int> {
        self.denominator.bigint_operand()
    }

    /// Inspect original numerator and denominator owners without a heap copy.
    #[must_use]
    pub fn integer_operands(&self) -> [Cow<'_, Int>; 2] {
        [self.bigint_numerator(), self.bigint_denominator()]
    }

    /// Original heap limb capacity, excluding this inline object.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.bigint_numerator()
            .allocated_bytes()
            .saturating_add(self.bigint_denominator().allocated_bytes())
    }

    /// Move original canonical operands to immutable shared magnitude storage.
    #[must_use]
    pub fn into_shared(self) -> Self {
        Self {
            numerator: Integer::from_bigint(self.numerator.into_bigint().into_shared()),
            denominator: Integer::from_bigint(self.denominator.into_bigint().into_shared()),
        }
    }

    /// Complete shared original magnitude storage; inline values need no heap.
    #[must_use]
    pub fn shared_owned_bytes(&self) -> usize {
        self.bigint_numerator()
            .shared_owned_bytes()
            .saturating_add(self.bigint_denominator().shared_owned_bytes())
    }

    /// Detach both reduced coefficients into the original preparation account.
    /// Canonical operands require no repeated reduction or unbounded gcd.
    ///
    /// # Errors
    /// Returns the first admission or allocation refusal before publication.
    pub fn detached_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        Ok(Self {
            numerator: self.numerator.detached_with_memory(memory)?,
            denominator: self.denominator.detached_with_memory(memory)?,
        })
    }

    /// Preserve the original detached allocation through every shallow clone.
    ///
    /// # Errors
    /// Returns a refused shared-control allocation with the original value live.
    pub fn publish_detached(
        &mut self,
        original: &purrdf_lex::allocation::SharedOwned<()>,
    ) -> Result<(), purrdf_lex::allocation::StorageError> {
        self.numerator.publish_detached(original)?;
        self.denominator.publish_detached(original)
    }

    /// Exact requested layouts of both detached canonical coefficients.
    ///
    /// # Errors
    /// Refuses address-space overflow before native allocation.
    pub fn detached_required_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
        self.bigint_numerator()
            .detached_required_bytes()?
            .checked_add(self.bigint_denominator().detached_required_bytes()?)
            .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)
    }

    fn from_reduced_bigints(num: Int, den: Int) -> Self {
        // Every caller is a canonical construction proof: cancelling powers
        // of two/five, or an integer congruence preserving the original gcd.
        // Validation must not secretly allocate an unbounded debug-only gcd.
        debug_assert!(!den.is_zero() && !den.is_negative());
        debug_assert!(!num.is_zero() || den.is_one());
        Self {
            numerator: Integer::from_bigint(num),
            denominator: Integer::from_bigint(den),
        }
    }

    /// Copy canonical operands into the original reusable destinations without
    /// another reduction or a detached heap allocation. Existing pooled
    /// operands share their immutable original destination.
    ///
    /// # Errors
    /// Refuses capacity or destination exhaustion before publishing a copy.
    pub fn copy_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        let numerator = self.bigint_numerator().copy_in(scratch)?;
        let denominator = self.bigint_denominator().copy_in(scratch)?;
        Ok(Self::from_reduced_bigints(numerator, denominator))
    }

    /// The same canonical absolute value in original reusable destinations.
    ///
    /// # Errors
    /// Refuses destination capacity or availability without a heap fallback.
    pub fn abs_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        let numerator = self.bigint_numerator().abs_using(scratch)?;
        let denominator = self.bigint_denominator().copy_in(scratch)?;
        Ok(Self::from_reduced_bigints(numerator, denominator))
    }

    /// The same canonical additive inverse in original reusable destinations.
    ///
    /// # Errors
    /// Refuses destination capacity or availability without a heap fallback.
    pub fn neg_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        let numerator = self.bigint_numerator().neg_using(scratch)?;
        let denominator = self.bigint_denominator().copy_in(scratch)?;
        Ok(Self::from_reduced_bigints(numerator, denominator))
    }

    /// Add canonical fractions using denominator cancellation before products.
    ///
    /// # Errors
    /// Refuses the original reusable destination's capacity or availability.
    pub fn add_in(&self, rhs: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.sum_using(rhs, false, scratch)
    }

    /// Subtract under the same cancellation body and original destination owner.
    ///
    /// # Errors
    /// Refuses capacity or availability without a heap fallback.
    pub fn sub_in(&self, rhs: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.sum_using(rhs, true, scratch)
    }

    /// Multiply canonical fractions after cancelling the two crosswise gcds.
    ///
    /// # Errors
    /// Refuses capacity or availability without a heap fallback.
    pub fn mul_in(&self, rhs: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.product_using(rhs, false, scratch)
            .map(|value| value.expect("multiplication has no zero divisor"))
    }

    /// Divide canonical fractions; `None` identifies the original zero divisor.
    ///
    /// # Errors
    /// Refuses capacity or availability separately from division by zero.
    pub fn div_in(
        &self,
        rhs: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        self.product_using(rhs, true, scratch)
    }

    pub(super) fn sum_using(
        &self,
        rhs: &Self,
        subtract: bool,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        let [left_n, left_d] = self.integer_operands();
        let [right_n, right_d] = rhs.integer_operands();
        let common = left_d.gcd_using(&right_d, storage)?;
        let left_factor = cancelled_operand(&left_d, &common, storage)?;
        let right_factor = cancelled_operand(&right_d, &common, storage)?;
        let left = left_n.mul_using(&right_factor, storage)?;
        let right = right_n.mul_using(&left_factor, storage)?;
        let sum = if subtract {
            left.sub_using(&right, storage)?
        } else {
            left.add_using(&right, storage)?
        };
        drop(left);
        drop(right);
        if sum.is_zero() {
            return Ok(Self::ZERO);
        }
        // Each canonical numerator is coprime with its denominator. The new
        // numerator is coprime with d1/g and d2/g, so only gcd(sum,g) remains.
        let reduction = sum.gcd_using(&common, storage)?;
        let denominator_factor = cancelled_operand(&right_d, &reduction, storage)?;
        let denominator = left_factor.mul_using(&denominator_factor, storage)?;
        let numerator = if reduction.is_one() {
            sum
        } else {
            sum.div_rem_using(&reduction, storage)?
                .expect("a gcd with a positive denominator is nonzero")
                .0
        };
        Ok(Self::from_reduced_bigints(numerator, denominator))
    }

    pub(super) fn product_using(
        &self,
        rhs: &Self,
        divide: bool,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, LimbScratchError> {
        if divide && rhs.is_zero() {
            return Ok(None);
        }
        if self.is_zero() || (!divide && rhs.is_zero()) {
            return Ok(Some(Self::ZERO));
        }
        let [left_n, left_d] = self.integer_operands();
        let [rhs_n, rhs_d] = rhs.integer_operands();
        let (right_n, right_d) = if divide {
            (&rhs_d, &rhs_n)
        } else {
            (&rhs_n, &rhs_d)
        };
        let left_gcd = left_n.gcd_using(right_d, storage)?;
        let right_gcd = right_n.gcd_using(&left_d, storage)?;
        let left_n = cancelled_operand(&left_n, &left_gcd, storage)?;
        let right_d = cancelled_operand(right_d, &left_gcd, storage)?;
        let right_n = cancelled_operand(right_n, &right_gcd, storage)?;
        let left_d = cancelled_operand(&left_d, &right_gcd, storage)?;
        let numerator = left_n.mul_using(&right_n, storage)?;
        let denominator = left_d.mul_using(&right_d, storage)?;
        // Division swaps the divisor's signed numerator into the denominator.
        // Sign normalization moves both original magnitudes without copying.
        let negative_denominator = denominator.is_negative();
        let negative_numerator = numerator.is_negative() ^ negative_denominator;
        Ok(Some(Self::from_reduced_bigints(
            numerator.with_sign(negative_numerator),
            denominator.with_sign(false),
        )))
    }

    /// The original exact binary integer as a canonical rational integer.
    #[must_use]
    pub fn from_bigint_integer(value: Int) -> Self {
        Self::from_integer(Integer::from_bigint(value))
    }

    /// Exact canonical value `numerator / 2^fractional_bits`.
    #[must_use]
    pub fn from_dyadic_integer(numerator: &Int, fractional_bits: u32) -> Self {
        let (num, den) = crate::ieee::ratio::dyadic_ratio(numerator, fractional_bits);
        Self::from_reduced_bigints(num, den)
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
        let (num, den) = crate::ieee::ratio::dyadic_ratio_in(numerator, fractional_bits, scratch)?;
        Ok(Self::from_reduced_bigints(num, den))
    }

    /// Builds the exact value `mantissa / 10^scale`.
    #[must_use]
    // Preserve the established owned constructor while sharing the exact
    // reduction body with the explicitly borrowed reusable-storage entry.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_scaled_integer(mantissa: Int, scale: u32) -> Self {
        Self::from_decimal_using(&mantissa, scale, &Unbounded).expect("unbounded integer storage")
    }

    /// The identical exact decimal in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating fallback.
    pub fn from_decimal_in(
        mantissa: &Int,
        scale: u32,
        scratch: &LimbScratch,
    ) -> Result<Self, LimbScratchError> {
        Self::from_decimal_using(mantissa, scale, scratch)
    }

    fn from_decimal_using(
        mantissa: &Int,
        scale: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        if mantissa.is_zero() {
            return Ok(Self::ZERO);
        }
        // The denominator's only primes are 2 and 5. Binary valuation plus
        // exact 5-adic quotients supplies complete canonical reduction.
        let twos = mantissa
            .trailing_zeros()
            .expect("nonzero magnitude")
            .min(u64::from(scale)) as u32;
        let binary_reduced = mantissa.shr_using(twos, storage)?;
        let (num, fives) = binary_reduced.strip_fives_using(scale, storage)?;
        let den = Int::one()
            .mul_pow5_using(scale - fives, storage)?
            .shl_using(scale - twos, storage)?;
        debug_assert!(!den.is_zero() && !den.is_negative());
        debug_assert!(twos == scale || num.is_odd());
        #[cfg(debug_assertions)]
        {
            // Checking the remaining prime requires one small-word quotient;
            // it uses the same admitted storage as the construction itself.
            let remainder = num
                .div_rem_using(&Int::from_i64(5), storage)?
                .expect("nonzero factor")
                .1;
            debug_assert!(fives == scale || !remainder.is_zero());
        }
        Ok(Self::from_reduced_bigints(num, den))
    }

    /// Decode a finite binary64 value as its exact dyadic rational.
    ///
    /// This is integer-only: NaN and infinity return `None`, while both signed
    /// zero spellings represent the same rational zero.
    #[must_use]
    pub fn from_binary64(value: f64) -> Option<Self> {
        Self::from_binary64_using(value, &Unbounded).expect("unbounded integer storage")
    }

    /// Decode the same finite binary64 value in original admitted destinations.
    /// Both zero signs give canonical zero; nonfinite values remain `Ok(None)`.
    ///
    /// # Errors
    /// Refuses capacity or destination exhaustion without a resident fallback.
    pub fn from_binary64_in(
        value: f64,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        Self::from_binary64_using(value, scratch)
    }

    fn from_binary64_using(
        value: f64,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, LimbScratchError> {
        let Some(decoded) = crate::ieee::dyadic::Binary64Dyadic::decode(value) else {
            return Ok(None);
        };
        let mut numerator = Int::from_u64(decoded.significand()).with_sign(decoded.negative());
        let exponent = decoded.exponent();
        let denominator = if exponent < 0 {
            Int::one().shl_using(exponent.unsigned_abs(), storage)?
        } else {
            numerator = numerator.shl_using(exponent.unsigned_abs(), storage)?;
            Int::one()
        };
        // The shared decoder already removed every power of two from its
        // nonzero significand. These two branches are canonical construction
        // proofs and require no additional gcd or independent reduction body.
        Ok(Some(Self::from_reduced_bigints(numerator, denominator)))
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
        match Self::parse_decimal_admitted(
            text,
            &mut |_, _| Ok::<(), core::convert::Infallible>(()),
        ) {
            Ok(value) => value,
            Err(error) => match error {},
        }
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
        match Self::parse_decimal_using(text, admit, &Unbounded) {
            Ok(value) => Ok(value),
            Err(DecimalConstructionError::Admission(error)) => Err(error),
            Err(DecimalConstructionError::Storage(_)) => {
                unreachable!("resident unbounded integer construction cannot refuse a destination")
            }
        }
    }

    /// Run the same scanner, signed coefficient and canonical decimal reduction
    /// in original preadmitted destinations. Returned limbs retain that arena's
    /// original owner after either parser or context is destroyed.
    ///
    /// # Errors
    /// Distinguishes the caller's original admission from native destination
    /// capacity or allocator refusal; no resident destination is substituted.
    pub fn parse_decimal_in<E>(
        text: &str,
        admit: &mut impl FnMut(usize, u64) -> Result<(), E>,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, DecimalConstructionError<E>> {
        Self::parse_decimal_using(text, admit, scratch)
    }

    fn parse_decimal_using<E>(
        text: &str,
        admit: &mut impl FnMut(usize, u64) -> Result<(), E>,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, DecimalConstructionError<E>> {
        let Some(lexeme) = scan_decimal(text) else {
            return Ok(None);
        };
        let Some((digits, power, scale)) = lexeme.construction() else {
            return Ok(None);
        };
        admit(digits, power).map_err(DecimalConstructionError::Admission)?;
        let Some(mantissa) = Int::from_decimal_bytes_using(
            lexeme
                .int_digits
                .iter()
                .chain(lexeme.frac_digits.iter())
                .copied(),
            lexeme.negative,
            storage,
        )
        .map_err(DecimalConstructionError::Storage)?
        else {
            return Ok(None);
        };
        if mantissa.is_zero() {
            return Ok(Some(Self::ZERO));
        }
        // The scanner's original scale, including the written exponent, owns
        // exactly the same mantissa * 10^(-scale) law as the resident entry.
        if scale > 0 {
            let Ok(scale) = u32::try_from(scale) else {
                return Ok(None);
            };
            Self::from_decimal_using(&mantissa, scale, storage)
                .map(Some)
                .map_err(DecimalConstructionError::Storage)
        } else {
            let Ok(scale) = u32::try_from(-scale) else {
                return Ok(None);
            };
            mantissa
                .mul_pow10_using(scale, storage)
                .map(Self::from_bigint_integer)
                .map(Some)
                .map_err(DecimalConstructionError::Storage)
        }
    }

    /// Inspect the original grammar without constructing an integer. Strict
    /// record adapters share these digit/power pre-construction counts.
    #[must_use]
    pub fn decimal_construction(text: &str) -> Option<(usize, u64)> {
        scan_decimal(text)?
            .construction()
            .map(|(digits, power, _)| (digits, power))
    }

    /// Native reusable destinations for the original decimal construction body.
    ///
    /// Six simultaneous destinations enclose the retained mantissa, binary
    /// reduced mantissa, stripped numerator, current power, its replacement,
    /// and shifted denominator. The prime-removal quotient replaces its input
    /// before denominator construction; its remainder is a single inline word.
    /// The checked-profile quotient also fits after the power has died. No
    /// original destination has more than `4 * (digits + power)` significant
    /// bits; a further limb covers the small-multiply/shift carry destination.
    /// An entirely inline body needs no pooled destinations.
    ///
    /// # Errors
    /// Refuses checked bit/limb layout overflow before any native allocation.
    pub fn decimal_scratch_layout(
        digits: usize,
        power: u64,
    ) -> Result<(usize, usize), LimbScratchError> {
        let bits = ExactArithmeticCost::decimal(digits, power)
            .ok_or(LimbScratchError::SizeOverflow)?
            .output_bits;
        let limbs = usize::try_from(bits.div_ceil(64))
            .ok()
            .and_then(|limbs| limbs.checked_add(1))
            .ok_or(LimbScratchError::SizeOverflow)?;
        Ok(if limbs <= 3 { (0, 0) } else { (6, limbs) })
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
        self.finite_decimal_scale_using(&Unbounded)
            .expect("unbounded integer storage")
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
        self.finite_decimal_scale_using(scratch)
    }

    fn finite_decimal_scale_using(
        &self,
        storage: &impl Allocate,
    ) -> Result<Option<u64>, LimbScratchError> {
        if self.bigint_denominator().bit_len() == 1 {
            return Ok(Some(0));
        }
        let twos = self
            .bigint_denominator()
            .trailing_zeros()
            .expect("positive denominator");
        let odd = self.bigint_denominator().shr_wide_using(twos, storage)?;
        if odd.bit_len() == 1 {
            return Ok(Some(twos));
        }
        // 5^b > 2^(2b), so a nonzero B-bit integer has at most (B-1)/2
        // factors of five. The original 27-factor division body strips all
        // of them; testing the residue proves both outcomes without a GCD.
        let limit = (odd.bit_len() - 1) / 2;
        let (remaining, fives) = odd.strip_fives_wide_using(limit, storage)?;
        Ok((remaining.bit_len() == 1).then_some(twos.max(fives)))
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
        self.to_decimal_string_with(max_scale, crate::ieee::ratio::Rounding::NearestEven)
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
        let mantissa = self.round_to_scale_with(max_scale, rounding);
        let mut out = String::new();
        let mut resident = purrdf_lex::allocation::Resident;
        crate::exact::append_scaled_integer_with_memory(
            &mantissa,
            max_scale,
            &mut out,
            &mut purrdf_lex::allocation::Memory::new(&mut resident),
        )
        .expect("resident rational decimal rendering storage");
        out
    }

    /// Returns `round(self * 10^scale)`, rounding half to even.
    ///
    /// This is the mantissa of the value rounded to `scale` decimal places;
    /// `Rational::from_decimal(value.round_to_scale(s), s)` is the rounded value.
    #[must_use]
    pub fn round_to_scale(&self, scale: u32) -> Int {
        self.round_to_scale_with(scale, crate::ieee::ratio::Rounding::NearestEven)
    }

    /// Identical half-even decimal rounding in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating fallback.
    pub fn round_to_scale_in(
        &self,
        scale: u32,
        scratch: &LimbScratch,
    ) -> Result<Int, LimbScratchError> {
        self.round_to_scale_with_in(scale, crate::ieee::ratio::Rounding::NearestEven, scratch)
    }

    /// Integer mantissa rounded on a declared decimal grid. Directed laws give
    /// exact outward bounds: `Down <= self*10^scale <= Up`. Neither an
    /// intermediate binary value nor proof precision chooses the grid.
    #[must_use]
    pub fn round_to_scale_with(&self, scale: u32, rounding: crate::ieee::ratio::Rounding) -> Int {
        self.round_to_scale_using(scale, rounding, &Unbounded)
            .expect("unbounded integer storage")
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
        self.round_to_scale_using(scale, rounding, scratch)
    }

    fn round_to_scale_using(
        &self,
        scale: u32,
        rounding: crate::ieee::ratio::Rounding,
        storage: &impl Allocate,
    ) -> Result<Int, LimbScratchError> {
        let factor = Int::pow10_using(scale, storage)?;
        Ok(crate::ieee::ratio::round_product_using(
            &self.bigint_numerator(),
            &factor,
            &self.bigint_denominator(),
            rounding,
            storage,
        )?
        .expect("a canonical denominator is nonzero"))
    }

    pub(super) fn compare_resident(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Ordering, LimbScratchError> {
        self.compare_using(other, storage, |_, evaluate| evaluate())
    }

    /// Identical exact rational ordering in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion without an allocating product fallback.
    pub fn compare_in(
        &self,
        other: &Self,
        scratch: &LimbScratch,
    ) -> Result<Ordering, LimbScratchError> {
        self.compare_using(other, scratch, |_, evaluate| evaluate())
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
        self.compare_admitted_using(other, &Unbounded, run)
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
        self.compare_admitted_using(other, scratch, run)
    }

    /// Execute the same comparison while admitting an arena only for products.
    /// The supplied original phase must execute once. Every arena reference is
    /// released when that phase returns, including caller or native failure.
    ///
    /// # Errors
    /// Returns checked-bound overflow or the caller's first admission/body failure.
    pub fn compare_admitted_by<E>(
        &self,
        other: &Self,
        mut run: impl FnMut(
            ExactArithmeticCost,
            &mut RationalNativeComparisonBody<'_>,
        ) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        let storage = crate::bigint::scratch::ProvidedScratch::default();
        self.compare_admitted_using(other, &storage, |cost, body| {
            run(cost, &mut |scratch| storage.within(scratch, &mut *body))
        })
    }

    fn compare_admitted_using<E>(
        &self,
        other: &Self,
        storage: &impl Allocate,
        mut run: impl FnMut(ExactArithmeticCost, &mut RationalComparisonBody<'_>) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        self.compare_using(other, storage, |phase, evaluate| {
            let cost = match phase {
                ComparisonPhase::Scan => ExactArithmeticCost::rational_comparison_scan(
                    self.bigint_denominator()
                        .bit_len()
                        .max(other.bigint_denominator().bit_len()),
                ),
                ComparisonPhase::Numerator => {
                    let cost = ExactArithmeticCost::for_operation(
                        ExactOperation::Linear,
                        self.bigint_numerator()
                            .bit_len()
                            .max(other.bigint_numerator().bit_len())
                            .max(1),
                        1,
                    );
                    cost.map(|mut cost| {
                        // Ordering borrowed magnitudes allocates no result.
                        cost.workspace_bytes = 0;
                        cost
                    })
                }
                ComparisonPhase::Products => ExactArithmeticCost::for_rational_operands(
                    ExactOperation::RationalCompare,
                    [
                        (
                            self.bigint_numerator().bit_len(),
                            self.bigint_denominator().bit_len(),
                        ),
                        (
                            other.bigint_numerator().bit_len(),
                            other.bigint_denominator().bit_len(),
                        ),
                    ],
                    1,
                ),
            }
            .ok_or(RationalComparisonError::CostOverflow)?;
            run(cost, evaluate).map_err(RationalComparisonError::Admission)
        })
    }

    fn compare_using<E>(
        &self,
        other: &Self,
        storage: &impl Allocate,
        mut run: impl FnMut(ComparisonPhase, &mut RationalComparisonBody<'_>) -> Result<Ordering, E>,
    ) -> Result<Ordering, E> {
        let mut phase = None;
        let result = run(ComparisonPhase::Scan, &mut || {
            let (sa, sb) = (self.numerator.signum(), other.numerator.signum());
            if sa != sb {
                return Ok(sa.cmp(&sb));
            }
            phase = Some(if self.bigint_denominator() == other.bigint_denominator() {
                ComparisonPhase::Numerator
            } else {
                ComparisonPhase::Products
            });
            Ok(Ordering::Equal)
        })?;
        let Some(phase) = phase else {
            return Ok(result);
        };
        match phase {
            ComparisonPhase::Numerator => run(phase, &mut || {
                Ok(self.bigint_numerator().cmp(&other.bigint_numerator()))
            }),
            // Both denominators are strictly positive, so cross multiplication
            // preserves the exact order independently of storage ownership.
            ComparisonPhase::Products => run(phase, &mut || {
                Int::compare_products_using(
                    &self.bigint_numerator(),
                    &other.bigint_denominator(),
                    &other.bigint_numerator(),
                    &self.bigint_denominator(),
                    storage,
                )
            }),
            ComparisonPhase::Scan => unreachable!("the scan selects an arithmetic phase"),
        }
    }

    /// Decide exactly whether `lower <= self - other < upper`.
    ///
    /// The integer interval is closed on its lower bound and open on its upper
    /// bound. Empty or reversed intervals return false. Cross products use the
    /// positive original denominators directly, without constructing a reduced
    /// rational difference or performing a greatest common divisor.
    #[must_use]
    pub fn difference_in_integer_interval(&self, other: &Self, lower: i64, upper: i64) -> bool {
        self.difference_in_integer_interval_using(other, lower, upper, &Unbounded)
            .expect("unbounded integer storage")
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
        self.difference_in_integer_interval_using(other, lower, upper, scratch)
    }

    fn difference_in_integer_interval_using(
        &self,
        other: &Self,
        lower: i64,
        upper: i64,
        storage: &impl Allocate,
    ) -> Result<bool, LimbScratchError> {
        if lower >= upper {
            return Ok(false);
        }
        let left = self
            .bigint_numerator()
            .mul_using(&other.bigint_denominator(), storage)?;
        let right = other
            .bigint_numerator()
            .mul_using(&self.bigint_denominator(), storage)?;
        let difference = left.sub_using(&right, storage)?;
        let denominator = self
            .bigint_denominator()
            .mul_using(&other.bigint_denominator(), storage)?;
        let lower = denominator.mul_using(&Int::from_i64(lower), storage)?;
        let upper = denominator.mul_using(&Int::from_i64(upper), storage)?;
        Ok(lower <= difference && difference < upper)
    }

    /// Exact reduction into `[origin,origin+period)`, or `None` for period zero.
    /// Numerator congruence preserves the existing coprime denominator.
    #[must_use]
    pub fn modulo_integer(&self, origin: i64, period: u64) -> Option<Self> {
        self.modulo_integer_using(origin, period, &Unbounded)
            .expect("unbounded integer storage")
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
        self.modulo_integer_using(origin, period, scratch)
    }

    fn modulo_integer_using(
        &self,
        origin: i64,
        period: u64,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, LimbScratchError> {
        if period == 0 {
            return Ok(None);
        }
        let origin_product = self
            .bigint_denominator()
            .mul_using(&Int::from_i64(origin), storage)?;
        let shifted = self
            .bigint_numerator()
            .sub_using(&origin_product, storage)?;
        let modulus = self
            .bigint_denominator()
            .mul_using(&Int::from_u64(period), storage)?;
        let mut remainder = shifted
            .div_rem_using(&modulus, storage)?
            .expect("positive modulus")
            .1;
        if remainder.is_negative() {
            remainder = remainder.add_using(&modulus, storage)?;
        }
        // Adding/subtracting an integer multiple of den cannot introduce a
        // common factor. A zero numerator likewise implies den=1.
        let value = Self::from_reduced_bigints(
            remainder.add_using(&origin_product, storage)?,
            self.bigint_denominator().abs_using(storage)?,
        );
        debug_assert!(
            !value.bigint_denominator().is_negative() && !value.bigint_denominator().is_zero()
        );
        debug_assert!(!value.bigint_numerator().is_zero() || value.bigint_denominator().is_one());
        Ok(Some(value))
    }
}

/// The pieces of a decimal lexical form, before any arithmetic happens.
#[derive(Clone, Debug)]
struct DecimalLexeme<'a> {
    /// `true` when the mantissa carried a `-` sign.
    negative: bool,
    /// Digits before the `.`, possibly empty.
    int_digits: &'a [u8],
    /// Digits after the `.`, possibly empty.
    frac_digits: &'a [u8],
    /// Signed exponent; zero when no exponent was written.
    exponent: i64,
}

impl DecimalLexeme<'_> {
    fn construction(&self) -> Option<(usize, u64, i64)> {
        if self.exponent > MAX_DECIMAL_EXPONENT || self.exponent < -MAX_DECIMAL_EXPONENT {
            return None;
        }
        let scale = i64::try_from(self.frac_digits.len())
            .ok()?
            .checked_sub(self.exponent)?;
        let digits = self.int_digits.len().checked_add(self.frac_digits.len())?;
        let power = if self
            .int_digits
            .iter()
            .chain(self.frac_digits)
            .all(|digit| *digit == b'0')
        {
            0
        } else {
            scale.unsigned_abs()
        };
        Some((digits, power, scale))
    }
}

/// Splits a decimal lexical form into its parts, or returns `None` if the whole
/// string is not one.
fn scan_decimal(text: &str) -> Option<DecimalLexeme<'_>> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let negative = match bytes.first() {
        None => return None,
        Some(&c) if c == b'+' || c == b'-' => {
            i = 1;
            c == b'-'
        }
        Some(_) => false,
    };
    let int_start = i;
    i = skip_digits(bytes, i);
    let int_digits = &bytes[int_start..i];
    let frac_digits: &[u8] = if i < bytes.len() && bytes[i] == b'.' {
        let frac_start = i + 1;
        i = skip_digits(bytes, frac_start);
        &bytes[frac_start..i]
    } else {
        &[]
    };
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }
    let exponent = if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let (value, after) = scan_exponent(bytes, i + 1)?;
        i = after;
        value
    } else {
        0
    };
    if i != bytes.len() {
        return None;
    }
    Some(DecimalLexeme {
        negative,
        int_digits,
        frac_digits,
        exponent,
    })
}

/// Returns the index just past the run of ASCII digits starting at `from`.
fn skip_digits(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

/// Scans an exponent body (an optional sign and at least one digit) starting at
/// `from`, returning its signed value and the index just past it.
///
/// Returns `None` when no digit follows, which is the `1e` and `1e+` rejection.
fn scan_exponent(bytes: &[u8], from: usize) -> Option<(i64, usize)> {
    let signed = from < bytes.len() && (bytes[from] == b'+' || bytes[from] == b'-');
    let negative = from < bytes.len() && bytes[from] == b'-';
    let start = if signed { from + 1 } else { from };
    let end = skip_digits(bytes, start);
    if end == start {
        return None;
    }
    let mut acc = 0i64;
    for &byte in &bytes[start..end] {
        // Saturating: any value that saturates is far above the cap the caller
        // applies, so the exact digits stop mattering.
        acc = acc
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'));
    }
    Some((if negative { -acc } else { acc }, end))
}

/// Borrow an unchanged factor or divide it exactly under the original storage.
/// Unit gcds must not introduce a deep clone of already owned heap limbs.
fn cancelled_operand<'a>(
    value: &'a Int,
    divisor: &Int,
    storage: &impl Allocate,
) -> Result<Cow<'a, Int>, LimbScratchError> {
    if divisor.is_one() {
        Ok(Cow::Borrowed(value))
    } else {
        let (quotient, remainder) = value
            .div_rem_using(divisor, storage)?
            .expect("a gcd with a positive denominator is nonzero");
        debug_assert!(remainder.is_zero(), "a gcd divides each operand exactly");
        Ok(Cow::Owned(quotient))
    }
}

#[cfg(test)]
mod arithmetic_tests {
    use super::*;
    use purrdf_alloc_probe::CurrentThreadWindow;

    fn value(n: Int, d: Int) -> Rational {
        Rational::new(Integer::from_bigint(n), Integer::from_bigint(d))
            .expect("nonzero denominator")
    }

    #[test]
    fn binary64_birth_preserves_exact_values_and_refuses_without_fallback() {
        let scratch = LimbScratch::new(8, 32).expect("binary64 native destinations");
        let cases = [
            0,
            1_u64 << 63,
            1,
            (1_u64 << 63) | 1,
            (1_u64 << 52) - 1,
            1_u64 << 52,
            0.5_f64.to_bits(),
            (-1.5_f64).to_bits(),
            f64::MAX.to_bits(),
            (-f64::MAX).to_bits(),
        ];
        for bits in cases {
            let source = f64::from_bits(bits);
            let expected = Rational::from_binary64(source).expect("finite original value");
            let window = CurrentThreadWindow::open();
            let native = Rational::from_binary64_in(source, &scratch)
                .expect("original native storage")
                .expect("finite value");
            assert_eq!(window.close().allocations, 0);
            assert_eq!(native, expected);
            drop(native);
            assert_eq!(scratch.available(), scratch.destination_capacity());
        }
        let insufficient = LimbScratch::new(1, 4).expect("small destination");
        let window = CurrentThreadWindow::open();
        assert!(matches!(
            Rational::from_binary64_in(f64::from_bits(1), &insufficient),
            Err(LimbScratchError::Capacity { .. })
        ));
        for source in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert!(
                Rational::from_binary64_in(source, &insufficient)
                    .expect("nonfinite lexical decision needs no destination")
                    .is_none()
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(
            insufficient.available(),
            insufficient.destination_capacity()
        );
    }

    #[test]
    fn canonical_cancellation_preserves_cross_product_oracle_and_all_signs() {
        for (an, ad, bn, bd) in [
            (2, 3, -4, 9),
            (-7, 12, -5, 18),
            (0, 1, 19, 31),
            (13, 1, 17, 1),
        ] {
            let a = value(Int::from_i64(an), Int::from_i64(ad));
            let b = value(Int::from_i64(bn), Int::from_i64(bd));
            let [a_n, a_d] = a.integer_operands();
            let [b_n, b_d] = b.integer_operands();
            let sum = value(a_n.mul(&b_d).add(&b_n.mul(&a_d)), a_d.mul(&b_d));
            let difference = value(a_n.mul(&b_d).sub(&b_n.mul(&a_d)), a_d.mul(&b_d));
            let product = value(a_n.mul(&b_n), a_d.mul(&b_d));
            let quotient = value(a_n.mul(&b_d), a_d.mul(&b_n));
            assert_eq!(&a + &b, sum);
            assert_eq!(&a - &b, difference);
            assert_eq!(&a * &b, product);
            assert_eq!(a.checked_div(&b).expect("nonzero divisor"), quotient);
        }
        assert_eq!(
            Rational::ONE.checked_div(&Rational::ZERO),
            Err(crate::exact::ExactError::DivisionByZero)
        );
    }

    #[test]
    fn canonical_native_cancellation_uses_only_prepared_original_destinations() {
        let numerator = Int::one().shl(399).add(&Int::from_u64(719));
        let denominator = Int::one().shl(201).sub(&Int::from_u64(37));
        let a = value(numerator.clone(), denominator.clone());
        let b = value(numerator.neg(), denominator);
        let inverse = a.recip().expect("nonzero value");
        let scratch = LimbScratch::new(32, 32).expect("prepared exact destinations");
        let window = CurrentThreadWindow::open();
        let sum = a.add_in(&b, &scratch).expect("exact cancelling sum");
        let product = a
            .mul_in(&inverse, &scratch)
            .expect("exact cancelling product");
        let quotient = a
            .div_in(&a, &scratch)
            .expect("admitted quotient")
            .expect("nonzero divisor");
        let negative = b
            .div_in(&a, &scratch)
            .expect("admitted signed quotient")
            .expect("nonzero divisor");
        let measured = window.close();
        assert_eq!(sum, Rational::ZERO);
        assert_eq!(product, Rational::ONE);
        assert_eq!(quotient, Rational::ONE);
        assert_eq!(negative, Rational::from_integer(Integer::from_i128(-1)));
        assert_eq!(
            measured.allocations, 0,
            "all native arithmetic remains inside its original arena"
        );
        assert_eq!(scratch.available(), scratch.destination_capacity());
        let narrow = LimbScratch::new(1, 4).expect("small admitted destination");
        assert!(matches!(
            a.mul_in(&inverse, &narrow),
            Err(LimbScratchError::Capacity { .. })
        ));
        assert!(
            a.div_in(&Rational::ZERO, &narrow)
                .expect("zero-divisor decision allocates no destination")
                .is_none()
        );
    }
}
