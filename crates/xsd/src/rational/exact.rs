// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical arbitrary-precision rational arithmetic over the shared integer.
//!
//! Numerators and positive denominators remain coprime, with zero represented
//! exactly as `0/1`. Arithmetic and decimal/IEEE conversions use exact integer
//! products and remainders. No floating arithmetic or ambient environment enters
//! a rational decision; binary64 is only a correctly rounded boundary value.

use core::cmp::Ordering;

/// The shared exact signed integer, retained under the geographic API path.
use crate::integer::Int;
use crate::integer::{
    ExactArithmeticCost, ExactOperation, LimbScratch, LimbScratchError,
    scratch::{Allocate, Unbounded},
};

/// Maximum absolute written exponent accepted by the exact carrier parser.
const MAX_DECIMAL_EXPONENT: i64 = 100_000;

/// One scoped step of the original rational comparison.
/// The admission callback executes this body exactly once and returns its
/// unchanged result while the supplied scratch allowance remains live.
pub type RationalComparisonBody<'a> = dyn FnMut() -> Result<Ordering, LimbScratchError> + 'a;

/// A checked comparison bound or its caller's admission refused execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RationalComparisonError<E> {
    /// The original comparison's complete work/storage bound overflowed.
    CostOverflow,
    /// A caller's scoped admission or original bounded body refused.
    Admission(E),
}

#[derive(Clone, Copy)]
enum ComparisonPhase {
    Scan,
    Numerator,
    Products,
}

// ---------------------------------------------------------------------------
// Rat
// ---------------------------------------------------------------------------

/// An exact rational number.
///
/// The denominator is always strictly positive, numerator and denominator are
/// always coprime, and zero is exactly `0/1`. That gives exactly one
/// representation per value, so `PartialEq`, `Eq` and `Hash` are derived.
///
/// Every arithmetic operation is exact. The only lossy operation on this type
/// is [`Rat::to_f64`], which is the crate's single floating-point boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rat {
    /// Numerator; carries the sign of the value.
    num: Int,
    /// Denominator; always strictly positive and coprime with `num`.
    den: Int,
}

impl Rat {
    /// The value zero, as `0/1`.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            num: Int::zero(),
            den: Int::one(),
        }
    }

    /// The value one, as `1/1`.
    #[must_use]
    pub fn one() -> Self {
        Self {
            num: Int::one(),
            den: Int::one(),
        }
    }

    /// Builds the rational `value / 1`.
    #[must_use]
    pub fn from_int(value: Int) -> Self {
        Self {
            num: value,
            den: Int::one(),
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
        let (num, den) = crate::ieee::ratio::dyadic_ratio(numerator, fractional_bits);
        Self::from_reduced(num, den)
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
        Ok(Self::from_reduced(num, den))
    }

    /// Builds `num / den`, normalizing the sign and reducing to lowest terms.
    ///
    /// Returns `None` if and only if `den` is zero.
    pub fn new(num: Int, den: Int) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        let (num, den) = if den.is_negative() {
            (num.neg(), den.neg())
        } else {
            (num, den)
        };
        if num.is_zero() {
            return Some(Self::zero());
        }
        if den.is_one() {
            return Some(Self::from_reduced(num, den));
        }
        let g = num.gcd(&den);
        if g.is_one() {
            return Some(Self::from_reduced(num, den));
        }
        let reduced_num = num
            .div_rem(&g)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        let reduced_den = den
            .div_rem(&g)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        Some(Self::from_reduced(reduced_num, reduced_den))
    }

    /// The numerator, carrying the sign of the value.
    #[must_use]
    pub fn numerator(&self) -> &Int {
        &self.num
    }

    /// The denominator, always strictly positive.
    #[must_use]
    pub fn denominator(&self) -> &Int {
        &self.den
    }

    /// Borrow the original numerator and positive denominator, in that order.
    /// Admission and storage visitors can inspect both owners without copying.
    #[must_use]
    pub fn integer_operands(&self) -> [&Int; 2] {
        [&self.num, &self.den]
    }

    /// Heap limb capacity retained by the exact numerator and denominator.
    /// Inline limbs and the enclosing object are accounted by their owner.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.num
            .allocated_bytes()
            .saturating_add(self.den.allocated_bytes())
    }

    /// Share canonical immutable long operands without changing their value or
    /// canonical representation. Preparation admits owned storage before this
    /// operation; later clones allocate no limbs or ownership headers.
    #[must_use]
    pub fn into_shared(self) -> Self {
        Self {
            num: self.num.into_shared(),
            den: self.den.into_shared(),
        }
    }

    /// Complete heap retained by the immutable shared original operands.
    #[must_use]
    pub fn shared_owned_bytes(&self) -> usize {
        self.num
            .shared_owned_bytes()
            .saturating_add(self.den.shared_owned_bytes())
    }

    /// Returns `self + other`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        // Reduce through the gcd of the denominators rather than multiplying
        // them out and reducing afterwards: the intermediate stays small and
        // the expensive gcd runs on `gcd(d1, d2)` instead of on the full
        // cross-product.
        let g = self.den.gcd(&other.den);
        if g.is_one() {
            let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
            return Self::from_reduced(num, self.den.mul(&other.den));
        }
        let d1g = self
            .den
            .div_rem(&g)
            .expect("a gcd of positives is positive")
            .0;
        let d2g = other
            .den
            .div_rem(&g)
            .expect("a gcd of positives is positive")
            .0;
        let t = self.num.mul(&d2g).add(&other.num.mul(&d1g));
        // `gcd(t, lcm(d1, d2)) == gcd(t, g)`, because `t` is coprime with both
        // `d1/g` and `d2/g`.
        let g2 = t.gcd(&g);
        if g2.is_one() {
            return Self::from_reduced(t, d1g.mul(&other.den));
        }
        let num = t
            .div_rem(&g2)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        let den = d1g.mul(
            &other
                .den
                .div_rem(&g2)
                .expect("a gcd of a non-zero pair is non-zero")
                .0,
        );
        Self::from_reduced(num, den)
    }

    /// Returns `self - other`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Returns `self * other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        // Cancel crosswise before multiplying, for the same reason as `add`.
        let g1 = self.num.gcd(&other.den);
        let g2 = other.num.gcd(&self.den);
        let (n1, d2) = if g1.is_one() {
            (self.num.clone(), other.den.clone())
        } else {
            let expect = "a gcd with a positive denominator is non-zero";
            (
                self.num.div_rem(&g1).expect(expect).0,
                other.den.div_rem(&g1).expect(expect).0,
            )
        };
        let (n2, d1) = if g2.is_one() {
            (other.num.clone(), self.den.clone())
        } else {
            let expect = "a gcd with a positive denominator is non-zero";
            (
                other.num.div_rem(&g2).expect(expect).0,
                self.den.div_rem(&g2).expect(expect).0,
            )
        };
        Self::from_reduced(n1.mul(&n2), d1.mul(&d2))
    }

    /// Returns `self / other`, or `None` when `other` is zero.
    pub fn div(&self, other: &Self) -> Option<Self> {
        Some(self.mul(&other.recip()?))
    }

    /// Returns the additive inverse.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self {
            num: self.num.neg(),
            den: self.den.clone(),
        }
    }

    /// Returns the absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Returns `-1`, `0` or `1` according to the sign.
    #[must_use]
    pub fn signum(&self) -> i32 {
        self.num.signum()
    }

    /// Returns `true` when this is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Returns `true` when the denominator is one.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// Returns the multiplicative inverse, or `None` when this is zero.
    pub fn recip(&self) -> Option<Self> {
        if self.num.is_zero() {
            return None;
        }
        Some(if self.num.is_negative() {
            Self {
                num: self.den.neg(),
                den: self.num.abs(),
            }
        } else {
            Self {
                num: self.den.clone(),
                den: self.num.clone(),
            }
        })
    }

    /// Builds the exact value `mantissa / 10^scale`.
    #[must_use]
    // Preserve the established owned constructor while sharing the exact
    // reduction body with the explicitly borrowed reusable-storage entry.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_decimal(mantissa: Int, scale: u32) -> Self {
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
            return Ok(Self::zero());
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
        Ok(Self { num, den })
    }

    /// Decode a finite binary64 value as its exact dyadic rational.
    ///
    /// This is integer-only: NaN and infinity return `None`, while both signed
    /// zero spellings represent the same rational zero.
    #[must_use]
    pub fn from_binary64(value: f64) -> Option<Self> {
        let decoded = crate::ieee::dyadic::Binary64Dyadic::decode(value)?;
        let mut numerator = Int::from_u64(decoded.significand());
        let exponent = decoded.exponent();
        let denominator = if exponent < 0 {
            Int::one().shl(exponent.unsigned_abs())
        } else {
            numerator = numerator.shl(exponent.unsigned_abs());
            Int::one()
        };
        if decoded.negative() {
            numerator = numerator.neg();
        }
        Self::new(numerator, denominator)
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
        let Some(lexeme) = scan_decimal(text) else {
            return Ok(None);
        };
        let Some((digits, power, scale)) = lexeme.construction() else {
            return Ok(None);
        };
        admit(digits, power)?;
        let Some(magnitude) = Int::from_decimal_bytes(
            lexeme
                .int_digits
                .iter()
                .chain(lexeme.frac_digits.iter())
                .copied(),
        ) else {
            return Ok(None);
        };
        let mantissa = if lexeme.negative {
            magnitude.neg()
        } else {
            magnitude
        };
        if mantissa.is_zero() {
            return Ok(Some(Self::zero()));
        }
        // The value is `mantissa * 10^(-scale)`.
        if scale > 0 {
            // A `u32` overflow needs more than four billion fraction digits,
            // which no `&str` this process can hold could supply.
            let Ok(scale) = u32::try_from(scale) else {
                return Ok(None);
            };
            Ok(Some(Self::from_decimal(mantissa, scale)))
        } else {
            let Ok(scale) = u32::try_from(-scale) else {
                return Ok(None);
            };
            Ok(Some(Self::from_int(mantissa.mul(&Int::pow10(scale)))))
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
        let value = crate::ieee::ratio::to_binary64(
            &self.num,
            &self.den,
            crate::ieee::ratio::Rounding::NearestEven,
        )
        .expect("a canonical denominator is nonzero");
        // Preserve the established carrier contract: underflow has canonical +0.
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
        if self.den.bit_len() == 1 {
            return Ok(Some(0));
        }
        let twos = self.den.trailing_zeros().expect("positive denominator");
        let odd = self.den.shr_wide_using(twos, storage)?;
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
        if mantissa.is_zero() {
            return "0".to_owned();
        }
        let digits = mantissa.abs().to_string();
        let scale = max_scale as usize;
        let mut out = String::with_capacity(digits.len() + 3);
        if mantissa.is_negative() {
            out.push('-');
        }
        if scale == 0 {
            out.push_str(&digits);
            return out;
        }
        let padded = if digits.len() <= scale {
            format!("{digits:0>width$}", width = scale + 1)
        } else {
            digits
        };
        let split = padded.len() - scale;
        out.push_str(&padded[..split]);
        let fraction = padded[split..].trim_end_matches('0');
        if !fraction.is_empty() {
            out.push('.');
            out.push_str(fraction);
        }
        out
    }

    /// Returns `round(self * 10^scale)`, rounding half to even.
    ///
    /// This is the mantissa of the value rounded to `scale` decimal places;
    /// `Rat::from_decimal(value.round_to_scale(s), s)` is the rounded value.
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
            &self.num, &factor, &self.den, rounding, storage,
        )?
        .expect("a canonical denominator is nonzero"))
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

    fn compare_admitted_using<E>(
        &self,
        other: &Self,
        storage: &impl Allocate,
        mut run: impl FnMut(ExactArithmeticCost, &mut RationalComparisonBody<'_>) -> Result<Ordering, E>,
    ) -> Result<Ordering, RationalComparisonError<E>> {
        self.compare_using(other, storage, |phase, evaluate| {
            let cost = match phase {
                ComparisonPhase::Scan => ExactArithmeticCost::rational_comparison_scan(
                    self.den.bit_len().max(other.den.bit_len()),
                ),
                ComparisonPhase::Numerator => {
                    let cost = ExactArithmeticCost::for_operation(
                        ExactOperation::Linear,
                        self.num.bit_len().max(other.num.bit_len()).max(1),
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
                        (self.num.bit_len(), self.den.bit_len()),
                        (other.num.bit_len(), other.den.bit_len()),
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
            let (sa, sb) = (self.signum(), other.signum());
            if sa != sb {
                return Ok(sa.cmp(&sb));
            }
            phase = Some(if self.den == other.den {
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
            ComparisonPhase::Numerator => run(phase, &mut || Ok(self.num.cmp(&other.num))),
            // Both denominators are strictly positive, so cross multiplication
            // preserves the exact order independently of storage ownership.
            ComparisonPhase::Products => run(phase, &mut || {
                Int::compare_products_using(&self.num, &other.den, &other.num, &self.den, storage)
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
        let left = self.num.mul_using(&other.den, storage)?;
        let right = other.num.mul_using(&self.den, storage)?;
        let difference = left.sub_using(&right, storage)?;
        let denominator = self.den.mul_using(&other.den, storage)?;
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
        let origin_product = self.den.mul_using(&Int::from_i64(origin), storage)?;
        let shifted = self.num.sub_using(&origin_product, storage)?;
        let modulus = self.den.mul_using(&Int::from_u64(period), storage)?;
        let mut remainder = shifted
            .div_rem_using(&modulus, storage)?
            .expect("positive modulus")
            .1;
        if remainder.is_negative() {
            remainder = remainder.add_using(&modulus, storage)?;
        }
        // Adding/subtracting an integer multiple of den cannot introduce a
        // common factor. A zero numerator likewise implies den=1.
        let value = Self {
            num: remainder.add_using(&origin_product, storage)?,
            den: self.den.abs_using(storage)?,
        };
        debug_assert!(!value.den.is_negative() && !value.den.is_zero());
        debug_assert!(!value.num.is_zero() || value.den.is_one());
        Ok(Some(value))
    }

    /// Wraps an already reduced numerator and denominator.
    pub(super) fn from_reduced(num: Int, den: Int) -> Self {
        let value = Self { num, den };
        value.assert_canonical();
        value
    }

    /// Checks the representation invariant. The checks compile out of release
    /// builds; the call itself is unconditional so the function is never dead.
    fn assert_canonical(&self) {
        debug_assert!(
            !self.den.is_zero() && !self.den.is_negative(),
            "denominator must be strictly positive"
        );
        debug_assert!(
            if self.den.is_one() {
                true
            } else if self.den.trailing_zeros() == self.den.bit_len().checked_sub(1) {
                self.num.is_odd()
            } else {
                self.num.abs().gcd(&self.den).is_one()
            },
            "numerator and denominator must be coprime"
        );
        debug_assert!(
            !self.num.is_zero() || self.den.is_one(),
            "zero must be exactly 0/1"
        );
    }
}

impl Default for Rat {
    fn default() -> Self {
        Self::zero()
    }
}

impl Ord for Rat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_using(other, &Unbounded, |_, evaluate| evaluate())
            .expect("unbounded integer storage")
    }
}

impl PartialOrd for Rat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Decimal lexing and the float boundary
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_canonical_operands_clone_without_allocation_and_remain_immutable() {
        let original = Rat::from_dyadic_integer(&Int::from_i64(-7), 1075);
        let expected = original.round_to_scale(400);
        let shared = original.clone().into_shared();
        let scratch = LimbScratch::new(32, 96).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let clones: [Rat; 16] = core::array::from_fn(|_| shared.clone());
        for value in &clones {
            assert_eq!(value.round_to_scale_in(400, &scratch).unwrap(), expected);
            assert_eq!(
                value.compare_in(&original, &scratch).unwrap(),
                Ordering::Equal
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(shared, original);
        assert_eq!(shared.neg().neg(), original);
        assert_eq!(shared.add(&shared.neg()), Rat::zero());
    }

    #[test]
    fn bounded_order_matches_exact_cross_products_and_refuses_insufficient_storage() {
        let scratch = LimbScratch::new(8, 64).unwrap();
        let mut cases = Vec::new();
        for bits in [193, 224, 448, 512] {
            let unit = Int::one().shl(bits);
            let below = Rat::new(unit.sub(&Int::one()), unit.add(&Int::one())).unwrap();
            let above = Rat::new(unit.add(&Int::one()), unit.sub(&Int::one())).unwrap();
            for a in [below.clone(), below.neg(), above.clone(), above.neg()] {
                for b in [Rat::zero(), Rat::one(), below.clone(), above.clone()] {
                    let order = a.cmp(&b);
                    cases.push((a.clone(), b, order));
                }
            }
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for (a, b, expected) in &cases {
            assert_eq!(a.compare_in(b, &scratch).unwrap(), *expected);
            assert_eq!(b.compare_in(a, &scratch).unwrap(), expected.reverse());
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), scratch.destination_capacity());
        let insufficient = LimbScratch::new(8, 3).unwrap();
        let (a, b, _) = &cases[cases.len() - 13];
        assert!(matches!(
            a.compare_in(b, &insufficient),
            Err(LimbScratchError::Capacity { .. })
        ));
    }

    #[test]
    fn decimal_prime_cancellation_matches_general_reduction_across_chunk_boundaries() {
        let scratch = LimbScratch::new(64, 128).unwrap();
        let mut cases = Vec::new();
        for scale in [0, 1, 15, 26, 27, 28, 54, 55, 112, 224] {
            for twos in [0, 1, 26, 27, 28, 54, 113] {
                for fives in [0, 1, 26, 27, 28, 54, 113] {
                    let source = Int::from_i64(13).shl(twos).mul_pow5(fives);
                    for source in [source.clone(), source.neg()] {
                        let expected = Rat::new(source.clone(), Int::pow10(scale)).unwrap();
                        assert_eq!(Rat::from_decimal(source.clone(), scale), expected);
                        cases.push((source, scale, expected));
                    }
                }
            }
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for (source, scale, expected) in &cases {
            assert_eq!(
                &Rat::from_decimal_in(source, *scale, &scratch).unwrap(),
                expected
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), scratch.destination_capacity());
        assert_eq!(Rat::from_decimal(Int::zero(), 100_000), Rat::zero());
        let insufficient = LimbScratch::new(4, 3).unwrap();
        assert!(matches!(
            Rat::from_decimal_in(&Int::one(), 224, &insufficient),
            Err(LimbScratchError::Capacity { .. })
        ));
    }

    #[test]
    fn finite_decimal_scales_are_exact_minimal_source_properties() {
        let scratch = LimbScratch::new(16, 160).unwrap();
        for twos in [0, 1, 26, 27, 28, 54, 55, 224, 4096] {
            for fives in [0, 1, 26, 27, 28, 54, 55, 224, 4096] {
                let denominator = Int::one().shl(twos).mul_pow5(fives);
                let value = Rat::new(Int::from_i64(-13), denominator).unwrap();
                let expected = twos.max(fives);
                assert_eq!(value.finite_decimal_scale(), Some(u64::from(expected)));
                assert_eq!(
                    value.finite_decimal_scale_in(&scratch).unwrap(),
                    Some(u64::from(expected))
                );
                if expected <= 55 {
                    let text = value.to_decimal_string(expected);
                    assert_eq!(Rat::parse_decimal(&text).unwrap(), value);
                    if expected != 0 {
                        assert_ne!(
                            Rat::parse_decimal(&value.to_decimal_string(expected - 1)).unwrap(),
                            value
                        );
                    }
                }
            }
        }
        assert_eq!(scratch.available(), scratch.destination_capacity());
        for value in [Rat::zero(), Rat::from_i64(-91), Rat::from_i64(1)] {
            assert_eq!(value.finite_decimal_scale(), Some(0));
        }
        let cancelled = Rat::new(Int::from_i64(3), Int::from_i64(24)).unwrap();
        assert_eq!(cancelled.finite_decimal_scale(), Some(3));
        let written = Rat::parse_decimal("0.1234567890123456789").unwrap();
        assert_eq!(written.finite_decimal_scale(), Some(19));
        assert_eq!(written.to_decimal_string(19), "0.1234567890123456789");
    }

    #[test]
    fn finite_decimal_scale_refuses_other_primes_and_insufficient_destinations() {
        let scratch = LimbScratch::new(16, 80).unwrap();
        for prime in [3, 7, 11] {
            for fives in [0, 1, 26, 27, 28, 54, 112] {
                let denominator = Int::from_i64(prime).mul_pow5(fives).shl(1075);
                let value = Rat::new(Int::one(), denominator).unwrap();
                assert_eq!(value.finite_decimal_scale(), None);
                assert_eq!(value.finite_decimal_scale_in(&scratch).unwrap(), None);
            }
        }
        let value = Rat::new(Int::one(), Int::one().mul_pow5(448)).unwrap();
        let insufficient = LimbScratch::new(1, 32).unwrap();
        assert_eq!(
            value.finite_decimal_scale_in(&insufficient),
            Err(LimbScratchError::Exhausted { destinations: 1 })
        );
        assert_eq!(insufficient.available(), 1);
        let narrow = LimbScratch::new(8, 3).unwrap();
        assert!(matches!(
            value.finite_decimal_scale_in(&narrow),
            Err(LimbScratchError::Capacity { .. })
        ));
        assert_eq!(narrow.available(), 8);
    }

    #[test]
    fn integer_difference_intervals_match_independent_integer_oracle() {
        let scratch = LimbScratch::new(8, 32).unwrap();
        for left_numerator in -9_i64..=9 {
            for left_denominator in 1_i64..=5 {
                let left = Rat::new(
                    Int::from_i64(left_numerator),
                    Int::from_i64(left_denominator),
                )
                .unwrap();
                for right_numerator in -7_i64..=7 {
                    for right_denominator in 1_i64..=4 {
                        let right = Rat::new(
                            Int::from_i64(right_numerator),
                            Int::from_i64(right_denominator),
                        )
                        .unwrap();
                        for (lower, upper) in [(-3, 4), (-1, 0), (0, 1), (2, 2), (4, -3)] {
                            let numerator = left_numerator * right_denominator
                                - right_numerator * left_denominator;
                            let denominator = left_denominator * right_denominator;
                            let expected =
                                lower * denominator <= numerator && numerator < upper * denominator;
                            assert_eq!(
                                left.difference_in_integer_interval(&right, lower, upper),
                                expected
                            );
                            assert_eq!(
                                left.difference_in_integer_interval_in(
                                    &right, lower, upper, &scratch,
                                )
                                .unwrap(),
                                expected
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(scratch.available(), scratch.destination_capacity());
    }

    #[test]
    fn integer_difference_interval_endpoints_and_original_storage_are_exact() {
        let zero = Rat::zero();
        let denominator = Int::one().shl(448);
        let epsilon = Rat::new(Int::one(), denominator).unwrap();
        let lower = Rat::from_i64(-180);
        let upper = Rat::from_i64(180);
        let scratch = LimbScratch::new(8, 32).unwrap();
        let cases = [
            (lower.sub(&epsilon), false),
            (lower.clone(), true),
            (lower.add(&epsilon), true),
            (upper.sub(&epsilon), true),
            (upper.clone(), false),
            (upper.add(&epsilon), false),
        ];
        for (value, expected) in cases {
            assert_eq!(
                value.difference_in_integer_interval(&zero, -180, 180),
                expected
            );
            assert_eq!(
                value
                    .difference_in_integer_interval_in(&zero, -180, 180, &scratch)
                    .unwrap(),
                expected
            );
        }
        assert!(Rat::from_i64(i64::MIN).difference_in_integer_interval(&zero, i64::MIN, i64::MAX));
        assert!(!Rat::from_i64(i64::MAX).difference_in_integer_interval(&zero, i64::MIN, i64::MAX));
        let left = Rat::new(
            Int::one().shl(448).add(&Int::from_i64(7)),
            Int::one().shl(449).add(&Int::from_i64(3)),
        )
        .unwrap();
        let right = Rat::new(
            Int::one().shl(440).neg(),
            Int::one().shl(447).add(&Int::one()),
        )
        .unwrap();
        let difference = left.sub(&right);
        let expected = difference >= Rat::from_i64(-180) && difference < Rat::from_i64(180);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert_eq!(
            left.difference_in_integer_interval_in(&right, -180, 180, &scratch)
                .unwrap(),
            expected
        );
        assert_eq!(window.close().allocations, 0);
        let insufficient = LimbScratch::new(1, 32).unwrap();
        assert_eq!(
            left.difference_in_integer_interval_in(&right, -180, 180, &insufficient),
            Err(LimbScratchError::Exhausted { destinations: 1 })
        );
        assert_eq!(insufficient.available(), 1);
        let narrow = LimbScratch::new(8, 3).unwrap();
        assert!(matches!(
            left.difference_in_integer_interval_in(&right, -180, 180, &narrow),
            Err(LimbScratchError::Capacity { .. })
        ));
        assert_eq!(narrow.available(), 8);
        let none = LimbScratch::new(0, 0).unwrap();
        assert_eq!(
            left.difference_in_integer_interval_in(&right, 3, 3, &none),
            Ok(false)
        );
    }

    #[test]
    fn integer_period_reduction_matches_exact_residues_and_bounded_rounding() {
        for numerator in -30..=30 {
            for denominator in 1..=19 {
                let value = Rat::new(Int::from_i64(numerator), Int::from_i64(denominator)).unwrap();
                for origin in [-180, -1, 0, 7] {
                    for period in [1_u64, 7, 360] {
                        let shifted = numerator - origin * denominator;
                        let residue = shifted.rem_euclid(denominator * period as i64);
                        let expected = Rat::new(
                            Int::from_i64(residue + origin * denominator),
                            Int::from_i64(denominator),
                        )
                        .unwrap();
                        assert_eq!(value.modulo_integer(origin, period), Some(expected));
                    }
                }
            }
        }
        let source = Rat::new(
            Int::one().shl(500).add(&Int::from_i64(7)).neg(),
            Int::one().shl(448),
        )
        .unwrap();
        let normalized = source.modulo_integer(-180, 360).unwrap();
        let expected = normalized.round_to_scale(15);
        let scratch = LimbScratch::new(32, 64).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..16 {
            let actual = source
                .modulo_integer_in(-180, 360, &scratch)
                .unwrap()
                .unwrap();
            assert_eq!(actual, normalized);
            assert_eq!(actual.round_to_scale_in(15, &scratch).unwrap(), expected);
        }
        assert_eq!(window.close().allocations, 0);
        assert!(source.modulo_integer(0, 0).is_none());
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Values chosen to straddle every representation boundary the limb layout
    /// has: single limb, the `u64` ceiling, the first two-limb value, and both
    /// `i128` extremes.
    fn oracle_table() -> Vec<i128> {
        vec![
            0,
            1,
            -1,
            2,
            -2,
            7,
            -7,
            12_345,
            -12_345,
            i128::from(u32::MAX),
            i128::from(u64::MAX),
            -i128::from(u64::MAX),
            i128::from(u64::MAX) + 1,
            -(i128::from(u64::MAX) + 1),
            i128::from(i64::MIN),
            1i128 << 100,
            -(1i128 << 100),
            i128::MAX,
            i128::MIN,
        ]
    }

    fn assert_int_canonical(value: &Int, context: &str) {
        assert!(
            value.limbs().last() != Some(&0),
            "{context}: trailing zero limb in {value:?}"
        );
        assert!(
            !(value.is_negative() && value.limbs().is_empty()),
            "{context}: negative zero in {value:?}"
        );
    }

    fn int(value: i64) -> Int {
        Int::from_i64(value)
    }

    fn rat(num: i64, den: i64) -> Rat {
        Rat::new(int(num), int(den)).expect("test denominator is non-zero")
    }

    // -----------------------------------------------------------------------
    // 1. Canonicalization
    // -----------------------------------------------------------------------

    #[test]
    fn zero_has_exactly_one_representation() {
        let zero = Int::zero();
        assert_int_canonical(&zero, "zero");
        assert!(zero.is_zero());
        assert!(!zero.is_negative());
        assert_eq!(zero.signum(), 0);
        assert_eq!(zero, Int::default());
        assert_eq!(zero, Int::from_i64(0));
        assert_eq!(zero, Int::from_i128(0));
        assert_eq!(zero, Int::from_u64(0));
        assert_eq!(zero, zero.neg());
        assert_eq!(zero, zero.abs());
        assert_eq!(zero.to_string(), "0");
    }

    #[test]
    fn subtracting_a_value_from_itself_yields_canonical_zero() {
        for value in oracle_table() {
            let a = Int::from_i128(value);
            let difference = a.sub(&a);
            assert_int_canonical(&difference, "self-subtraction");
            assert_eq!(difference, Int::zero(), "{value} - {value}");
            assert!(!difference.is_negative());
        }
    }

    #[test]
    fn multiplying_by_zero_yields_canonical_zero() {
        for value in oracle_table() {
            let a = Int::from_i128(value);
            for zero in [Int::zero(), Int::zero().neg()] {
                let product = a.mul(&zero);
                assert_int_canonical(&product, "multiply by zero");
                assert_eq!(product, Int::zero());
                assert_eq!(product.limbs(), []);
            }
        }
    }

    #[test]
    fn every_operation_leaves_a_canonical_value() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let a = Int::from_i128(a_value);
                let b = Int::from_i128(b_value);
                assert_int_canonical(&a.add(&b), "add");
                assert_int_canonical(&a.sub(&b), "sub");
                assert_int_canonical(&a.mul(&b), "mul");
                assert_int_canonical(&a.gcd(&b), "gcd");
                assert_int_canonical(&a.abs(), "abs");
                assert_int_canonical(&a.neg(), "neg");
                assert_int_canonical(&a.shl(65), "shl");
                assert_int_canonical(&a.shr(65), "shr");
                if let Some((q, r)) = a.div_rem(&b) {
                    assert_int_canonical(&q, "div_rem quotient");
                    assert_int_canonical(&r, "div_rem remainder");
                }
            }
        }
    }

    #[test]
    fn cancelling_limbs_leaves_no_trailing_zero_limb() {
        // `2^128 - 2^128` and `(2^128 + 1) - 1` both retire a high limb.
        let big = Int::one().shl(128);
        assert_int_canonical(&big.sub(&big), "limb cancellation");
        assert!(big.sub(&big).is_zero());
        let plus_one = big.add(&Int::one());
        let back = plus_one.sub(&Int::one());
        assert_int_canonical(&back, "high limb retained");
        assert_eq!(back, big);
        let down = big.sub(&Int::one());
        assert_int_canonical(&down, "high limb dropped");
        assert_eq!(down.bit_len(), 128);
    }

    // -----------------------------------------------------------------------
    // 2. add / sub / mul against an i128 oracle
    // -----------------------------------------------------------------------

    #[test]
    fn add_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_add(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).add(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} + {b_value}");
            }
        }
    }

    #[test]
    fn sub_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_sub(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).sub(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} - {b_value}");
            }
        }
    }

    #[test]
    fn mul_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_mul(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).mul(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} * {b_value}");
            }
        }
    }

    #[test]
    fn arithmetic_crosses_the_limb_boundary_exactly() {
        let limb = Int::from_u64(u64::MAX);
        assert_eq!(limb.add(&Int::one()).to_i128(), Some(1i128 << 64));
        assert_eq!(limb.add(&Int::one()).bit_len(), 65);
        // (2^64 - 1)^2 == 2^128 - 2^65 + 1, which no primitive can hold.
        assert_eq!(
            limb.mul(&limb),
            Int::one()
                .shl(128)
                .sub(&Int::one().shl(65))
                .add(&Int::one())
        );
        assert_eq!(limb.mul(&limb).to_i128(), None);
        let two_limbs = Int::from_i128(1i128 << 64);
        assert_eq!(two_limbs.sub(&Int::one()), limb);
        assert_eq!(two_limbs.limbs().len(), 2);
        assert_eq!(limb.limbs().len(), 1);
    }

    #[test]
    fn multi_limb_products_exceed_i128_and_still_render_exactly() {
        // 2^64 * 2^64 * 2^64 = 2^192, which no primitive can hold.
        let limb = Int::one().shl(64);
        let cube = limb.mul(&limb).mul(&limb);
        assert_eq!(cube.to_i128(), None);
        assert_eq!(cube.bit_len(), 193);
        assert_eq!(cube, Int::one().shl(192));
        assert_eq!(cube.limbs().len(), 4);
        let rendered = cube.to_string();
        // floor(192 * log10(2)) + 1 == 58 digits.
        assert_eq!(rendered.len(), 58);
        assert!(rendered.starts_with('6'), "2^192 begins with 6: {rendered}");
        assert!(rendered.ends_with("896"), "2^192 ends with 896: {rendered}");
        assert_eq!(Int::from_decimal_digits(&rendered), Some(cube));
    }

    #[test]
    fn to_i128_reports_the_representable_boundary() {
        assert_eq!(Int::from_i128(i128::MIN).to_i128(), Some(i128::MIN));
        assert_eq!(Int::from_i128(i128::MAX).to_i128(), Some(i128::MAX));
        // |i128::MIN| is one past i128::MAX, so its absolute value does not fit.
        assert_eq!(Int::from_i128(i128::MIN).abs().to_i128(), None);
        assert_eq!(Int::from_i128(i128::MAX).add(&Int::one()).to_i128(), None);
        assert_eq!(Int::from_i128(i128::MIN).sub(&Int::one()).to_i128(), None);
    }

    // -----------------------------------------------------------------------
    // 3. div_rem
    // -----------------------------------------------------------------------

    #[test]
    fn div_rem_truncates_toward_zero_like_rust() {
        for a_value in [7i128, -7, 12, -12, 0, 1, -1, i128::MAX, i128::MIN, 1 << 100] {
            for b_value in [3i128, -3, 1, -1, 5, -5, i128::MAX, i128::MIN, 1 << 64] {
                let (q, r) = Int::from_i128(a_value)
                    .div_rem(&Int::from_i128(b_value))
                    .expect("divisor is non-zero");
                let Some(expected_q) = a_value.checked_div(b_value) else {
                    // The only unrepresentable case: `i128::MIN / -1` is exactly
                    // `2^127`, which Rust wraps and this type does not.
                    assert_eq!((a_value, b_value), (i128::MIN, -1));
                    assert_eq!(q, Int::from_i128(i128::MAX).add(&Int::one()));
                    assert!(r.is_zero());
                    continue;
                };
                assert_eq!(
                    q.to_i128(),
                    Some(expected_q),
                    "quotient of {a_value} / {b_value}"
                );
                assert_eq!(
                    r.to_i128(),
                    a_value.checked_rem(b_value),
                    "remainder of {a_value} % {b_value}"
                );
            }
        }
    }

    #[test]
    fn div_rem_remainder_takes_the_sign_of_the_dividend() {
        let cases = [
            (7i64, 3i64, 2i64, 1i64),
            (-7, 3, -2, -1),
            (7, -3, -2, 1),
            (-7, -3, 2, -1),
        ];
        for (a, b, expected_q, expected_r) in cases {
            let (q, r) = int(a).div_rem(&int(b)).expect("divisor is non-zero");
            assert_eq!(q, int(expected_q), "quotient of {a} / {b}");
            assert_eq!(r, int(expected_r), "remainder of {a} % {b}");
            assert_eq!(q.mul(&int(b)).add(&r), int(a), "identity for {a} / {b}");
        }
    }

    #[test]
    fn div_rem_by_zero_is_none() {
        for value in oracle_table() {
            assert!(Int::from_i128(value).div_rem(&Int::zero()).is_none());
        }
        // The neighbouring valid case: dividing by one is always fine.
        for value in oracle_table() {
            let (q, r) = Int::from_i128(value)
                .div_rem(&Int::one())
                .expect("one is a valid divisor");
            assert_eq!(q, Int::from_i128(value));
            assert!(r.is_zero());
        }
    }

    #[test]
    fn div_rem_handles_a_multi_limb_dividend() {
        let dividend = Int::one().shl(200).add(&int(12_345));
        let divisor = Int::one().shl(100).add(&int(7));
        let (q, r) = dividend.div_rem(&divisor).expect("divisor is non-zero");
        assert_eq!(q.mul(&divisor).add(&r), dividend);
        assert!(!r.is_negative());
        assert!(r < divisor);
        assert!(q.bit_len() >= 99);
        // And the negated dividend mirrors it exactly.
        let (nq, nr) = dividend
            .neg()
            .div_rem(&divisor)
            .expect("divisor is non-zero");
        assert_eq!(nq, q.neg());
        assert_eq!(nr, r.neg());
    }

    #[test]
    fn div_rem_satisfies_its_identity_across_the_table() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let a = Int::from_i128(a_value);
                let b = Int::from_i128(b_value);
                let Some((q, r)) = a.div_rem(&b) else {
                    assert_eq!(b_value, 0);
                    continue;
                };
                assert_eq!(q.mul(&b).add(&r), a, "{a_value} / {b_value}");
                assert!(
                    r.abs() < b.abs(),
                    "remainder too large for {a_value}/{b_value}"
                );
                if !r.is_zero() {
                    assert_eq!(r.signum(), a.signum(), "remainder sign for {a_value}");
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. gcd
    // -----------------------------------------------------------------------

    #[test]
    fn gcd_is_non_negative_and_absorbs_zero() {
        assert_eq!(Int::zero().gcd(&Int::zero()), Int::zero());
        for value in oracle_table() {
            let n = Int::from_i128(value);
            assert_eq!(Int::zero().gcd(&n), n.abs(), "gcd(0, {value})");
            assert_eq!(n.gcd(&Int::zero()), n.abs(), "gcd({value}, 0)");
            assert!(!n.gcd(&Int::one()).is_negative());
            assert_eq!(n.gcd(&Int::one()), Int::one(), "gcd({value}, 1)");
        }
    }

    #[test]
    fn gcd_ignores_the_signs_of_its_operands() {
        for (a, b, expected) in [
            (12i64, 18i64, 6i64),
            (-12, 18, 6),
            (12, -18, 6),
            (-12, -18, 6),
            (17, 5, 1),
            (100, 100, 100),
            (-100, 100, 100),
        ] {
            assert_eq!(int(a).gcd(&int(b)), int(expected), "gcd({a}, {b})");
        }
    }

    #[test]
    fn gcd_handles_a_multi_limb_pair() {
        // gcd(3 * 2^200, 5 * 2^200) == 2^200 — well past the u128 fast path.
        let base = Int::one().shl(200);
        let a = base.mul(&int(3));
        let b = base.mul(&int(5));
        assert_eq!(a.gcd(&b), base);
        assert_eq!(b.gcd(&a), base);
        assert_eq!(a.neg().gcd(&b.neg()), base);

        // gcd(p * 7, p * 11) == p, where p = 2^128 + 1.
        let p = Int::from_decimal_digits("340282366920938463463374607431768211457")
            .expect("literal is all digits");
        assert_eq!(p, Int::one().shl(128).add(&Int::one()));
        assert_eq!(p.mul(&int(7)).gcd(&p.mul(&int(11))), p);
    }

    // -----------------------------------------------------------------------
    // 5. sqrt_floor
    // -----------------------------------------------------------------------

    #[test]
    fn sqrt_floor_is_exact_around_every_small_perfect_square() {
        // `k = 0` is the one place `k^2 + 1` is not still `k`: sqrt(1) is 1.
        assert_eq!(Int::zero().sqrt_floor(), Some(Int::zero()));
        assert_eq!(Int::one().sqrt_floor(), Some(Int::one()));
        for k in 1i64..3_000 {
            let square = int(k * k);
            assert_eq!(square.sqrt_floor(), Some(int(k)), "sqrt({})", k * k);
            assert_eq!(
                square.sub(&Int::one()).sqrt_floor(),
                Some(int(k - 1)),
                "sqrt({} - 1)",
                k * k
            );
            assert_eq!(
                square.add(&Int::one()).sqrt_floor(),
                Some(int(k)),
                "sqrt({} + 1)",
                k * k
            );
        }
    }

    #[test]
    fn sqrt_floor_is_exact_for_a_large_multi_limb_square() {
        let base = Int::from_decimal_digits("123456789012345678901234567890123456789")
            .expect("literal is all digits");
        let square = base.mul(&base);
        assert!(square.bit_len() > 128, "the square must miss the u128 path");
        assert_eq!(square.sqrt_floor(), Some(base.clone()));
        assert_eq!(
            square.sub(&Int::one()).sqrt_floor(),
            Some(base.sub(&Int::one()))
        );
        assert_eq!(square.add(&Int::one()).sqrt_floor(), Some(base));

        // A power of two square, so the initial Newton guess is exactly on the
        // boundary of the bit-length estimate.
        let power = Int::one().shl(300);
        assert_eq!(power.mul(&power).sqrt_floor(), Some(power));
    }

    #[test]
    fn sqrt_floor_refuses_negatives_but_accepts_their_magnitudes() {
        for value in [-1i64, -2, -100, -1_000_000] {
            assert_eq!(int(value).sqrt_floor(), None, "sqrt({value})");
            // The neighbouring valid case must still succeed.
            assert!(
                int(value).abs().sqrt_floor().is_some(),
                "sqrt({}) must succeed",
                -value
            );
        }
        assert_eq!(Int::zero().sqrt_floor(), Some(Int::zero()));
        assert_eq!(Int::one().sqrt_floor(), Some(Int::one()));
        // sqrt(2^126) is exactly 2^63.
        assert_eq!(Int::one().shl(126).sqrt_floor(), Some(Int::one().shl(63)));
        // And the defining property holds at the top of the u128 fast path.
        let root = Int::from_i128(i128::MAX)
            .sqrt_floor()
            .expect("i128::MAX is non-negative");
        assert!(
            root.mul(&root) <= Int::from_i128(i128::MAX),
            "root is too large"
        );
        let next = root.add(&Int::one());
        assert!(
            next.mul(&next) > Int::from_i128(i128::MAX),
            "root is too small"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Ord
    // -----------------------------------------------------------------------

    #[test]
    fn ord_is_the_numeric_order_across_signs_and_magnitudes() {
        let mut values: Vec<Int> = oracle_table().into_iter().map(Int::from_i128).collect();
        values.sort();
        let mut expected = oracle_table();
        expected.sort_unstable();
        let sorted: Vec<i128> = values
            .iter()
            .map(|v| v.to_i128().expect("table values fit i128"))
            .collect();
        assert_eq!(sorted, expected);
    }

    #[test]
    fn ord_agrees_with_i128_on_representable_values() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                assert_eq!(
                    Int::from_i128(a_value).cmp(&Int::from_i128(b_value)),
                    a_value.cmp(&b_value),
                    "cmp({a_value}, {b_value})"
                );
            }
        }
    }

    #[test]
    fn ord_puts_negatives_below_zero_below_positives_beyond_i128() {
        let big = Int::one().shl(300);
        let small = Int::one().shl(299);
        assert!(big > small);
        assert!(big.neg() < small.neg());
        assert!(big.neg() < Int::zero());
        assert!(Int::zero() < small);
        assert!(big.neg() < big);
        assert_eq!(big.cmp(&big), Ordering::Equal);
        assert_eq!(big.neg().cmp(&big.neg()), Ordering::Equal);
    }

    // -----------------------------------------------------------------------
    // 7. Display / from_decimal_digits
    // -----------------------------------------------------------------------

    #[test]
    fn display_round_trips_through_from_decimal_digits() {
        let samples = [
            "0",
            "1",
            "9",
            "10",
            "12345",
            "18446744073709551615",
            "18446744073709551616",
            "340282366920938463463374607431768211455",
            "340282366920938463463374607431768211456",
            "1234567890123456789012345678901234567890123456789",
        ];
        for sample in samples {
            let parsed = Int::from_decimal_digits(sample).expect("sample is all digits");
            assert_eq!(parsed.to_string(), sample, "round trip of {sample}");
        }
    }

    #[test]
    fn display_writes_a_sign_only_for_negatives() {
        assert_eq!(int(0).to_string(), "0");
        assert_eq!(int(0).neg().to_string(), "0");
        assert_eq!(int(-1).to_string(), "-1");
        assert_eq!(int(-12_345).to_string(), "-12345");
        assert_eq!(Int::pow10(0).to_string(), "1");
        assert_eq!(Int::pow10(1).to_string(), "10");
        assert_eq!(Int::pow10(19).to_string(), format!("1{}", "0".repeat(19)));
        assert_eq!(Int::pow10(20).to_string(), format!("1{}", "0".repeat(20)));
        assert_eq!(
            Int::pow10(50).neg().to_string(),
            format!("-1{}", "0".repeat(50))
        );
    }

    #[test]
    fn from_decimal_digits_rejects_anything_but_a_digit_run() {
        for bad in [
            "", "-1", "+1", "1 ", " 1", "1_0", "1.0", "12a", "0x10", "1\n", "1e5",
        ] {
            assert!(
                Int::from_decimal_digits(bad).is_none(),
                "{bad:?} must be rejected"
            );
        }
        // The neighbouring valid cases: leading zeros and long runs are fine.
        assert_eq!(Int::from_decimal_digits("007").expect("digits"), int(7));
        assert_eq!(Int::from_decimal_digits("0").expect("digits"), Int::zero());
        assert_eq!(
            Int::from_decimal_digits("00000000000000000000000000000001").expect("digits"),
            Int::one()
        );
    }

    #[test]
    fn bit_len_shl_and_shr_agree_on_the_magnitude() {
        assert_eq!(Int::zero().bit_len(), 0);
        assert_eq!(int(1).bit_len(), 1);
        assert_eq!(int(255).bit_len(), 8);
        assert_eq!(int(256).bit_len(), 9);
        assert_eq!(Int::from_u64(u64::MAX).bit_len(), 64);
        assert_eq!(Int::one().shl(64).bit_len(), 65);
        assert_eq!(Int::one().shl(0), Int::one());
        assert_eq!(Int::zero().shl(1_000), Int::zero());
        assert_eq!(Int::zero().shr(1_000), Int::zero());
    }

    #[test]
    fn shr_truncates_the_magnitude_toward_zero() {
        assert_eq!(int(5).shr(1), int(2));
        assert_eq!(int(-5).shr(1), int(-2));
        assert_eq!(int(-1).shr(1), Int::zero());
        assert!(!int(-1).shr(1).is_negative());
        assert_eq!(int(-20).shl(2), int(-80));
        assert_eq!(int(-5).shl(2), int(-20));
        let big = Int::one().shl(200);
        assert_eq!(big.shr(200), Int::one());
        assert_eq!(big.shr(201), Int::zero());
        assert_eq!(big.neg().shr(199), int(-2));
    }

    // -----------------------------------------------------------------------
    // 8. Rat normalization
    // -----------------------------------------------------------------------

    #[test]
    fn rat_new_reduces_and_normalizes_the_sign() {
        assert_eq!(rat(2, 4), rat(1, 2));
        assert_eq!(rat(-1, -2), rat(1, 2));
        assert_eq!(rat(1, -2), rat(-1, 2));
        assert_eq!(rat(0, 5), Rat::zero());
        assert_eq!(rat(0, -5), Rat::zero());
        assert_eq!(*rat(0, 5).denominator(), Int::one());
        assert_eq!(rat(6, 3), Rat::from_i64(2));
        assert_eq!(rat(-6, 3), Rat::from_i64(-2));
        assert_eq!(*rat(1, -2).denominator(), int(2));
        assert_eq!(*rat(1, -2).numerator(), int(-1));
        assert!(!rat(1, -2).denominator().is_negative());
        assert_eq!(Rat::new(Int::one(), Int::zero()), None);
        // The neighbouring valid case: any non-zero denominator is accepted.
        assert!(Rat::new(Int::one(), int(-1)).is_some());
    }

    #[test]
    fn rat_predicates_report_the_normalized_form() {
        assert!(Rat::zero().is_zero());
        assert!(Rat::zero().is_integer());
        assert_eq!(Rat::zero().signum(), 0);
        assert_eq!(Rat::default(), Rat::zero());
        assert!(Rat::one().is_integer());
        assert!(!rat(1, 2).is_integer());
        assert!(rat(4, 2).is_integer());
        assert_eq!(rat(-3, 4).signum(), -1);
        assert_eq!(rat(3, 4).signum(), 1);
        assert_eq!(rat(-3, 4).abs(), rat(3, 4));
        assert_eq!(rat(3, 4).neg(), rat(-3, 4));
        assert_eq!(Rat::zero().neg(), Rat::zero());
        assert_eq!(Rat::zero().recip(), None);
        assert_eq!(rat(-2, 3).recip(), Some(rat(-3, 2)));
        assert_eq!(rat(2, 3).recip(), Some(rat(3, 2)));
    }

    // -----------------------------------------------------------------------
    // 9. Rat arithmetic identities
    // -----------------------------------------------------------------------

    fn rat_table() -> Vec<Rat> {
        vec![
            Rat::zero(),
            Rat::one(),
            rat(-1, 1),
            rat(1, 2),
            rat(-1, 2),
            rat(2, 3),
            rat(-5, 7),
            rat(22, 7),
            rat(1, 1_000_000),
            rat(999_999, 1_000_000),
            Rat::from_int(Int::one().shl(100)),
            Rat::new(Int::one(), Int::one().shl(100)).expect("non-zero denominator"),
            Rat::new(Int::one().shl(70), Int::from_u64(u64::MAX)).expect("non-zero denominator"),
        ]
    }

    #[test]
    fn rat_addition_and_subtraction_are_inverses() {
        for a in rat_table() {
            for b in rat_table() {
                assert_eq!(a.add(&b).sub(&b), a, "({a:?} + {b:?}) - {b:?}");
                assert_eq!(a.add(&b), b.add(&a), "commutativity");
                assert_eq!(a.sub(&b), a.add(&b.neg()), "sub is add of the negation");
                assert_eq!(a.add(&Rat::zero()), a, "additive identity");
                assert_eq!(a.add(&a.neg()), Rat::zero(), "additive inverse");
            }
        }
    }

    #[test]
    fn rat_multiplication_and_division_are_inverses() {
        for a in rat_table() {
            for b in rat_table() {
                let product = a.mul(&b);
                assert_eq!(product, b.mul(&a), "commutativity");
                assert_eq!(a.mul(&Rat::one()), a, "multiplicative identity");
                assert_eq!(a.mul(&Rat::zero()), Rat::zero(), "absorbing zero");
                if b.is_zero() {
                    assert_eq!(a.div(&b), None, "division by zero");
                } else {
                    assert_eq!(product.div(&b), Some(a.clone()), "({a:?} * {b:?}) / {b:?}");
                    assert_eq!(a.div(&b), Some(a.mul(&b.recip().expect("non-zero"))));
                }
            }
        }
    }

    #[test]
    fn rat_arithmetic_is_associative_and_distributive() {
        let table = rat_table();
        for a in &table {
            for b in &table {
                for c in &table {
                    assert_eq!(a.add(b).add(c), a.add(&b.add(c)), "additive associativity");
                    assert_eq!(
                        a.mul(b).mul(c),
                        a.mul(&b.mul(c)),
                        "multiplicative associativity"
                    );
                    assert_eq!(a.mul(&b.add(c)), a.mul(b).add(&a.mul(c)), "distributivity");
                }
            }
        }
    }

    #[test]
    fn rat_ordering_survives_arithmetic() {
        assert!(rat(1, 3) < rat(1, 2));
        assert!(rat(-1, 2) < rat(-1, 3));
        assert!(rat(-1, 3) < Rat::zero());
        assert!(Rat::zero() < rat(1, 1_000_000));
        assert!(rat(22, 7) > Rat::from_i64(3));
        let mut sorted = rat_table();
        sorted.sort();
        for pair in sorted.windows(2) {
            assert!(pair[0] <= pair[1], "sort produced {pair:?} out of order");
        }
        // Adding the same value to both sides preserves the order.
        let shift = rat(7, 11);
        for a in rat_table() {
            for b in rat_table() {
                assert_eq!(
                    a.cmp(&b),
                    a.add(&shift).cmp(&b.add(&shift)),
                    "order after a common shift"
                );
            }
        }
    }

    #[test]
    fn rat_from_decimal_is_the_exact_scaled_mantissa() {
        assert_eq!(Rat::from_decimal(int(1), 1), rat(1, 10));
        assert_eq!(Rat::from_decimal(int(-25), 4), rat(-1, 400));
        assert_eq!(Rat::from_decimal(int(0), 100), Rat::zero());
        assert_eq!(Rat::from_decimal(int(5), 0), Rat::from_i64(5));
        assert_eq!(
            Rat::from_decimal(int(1_000), 3),
            Rat::one(),
            "trailing zeros must reduce away"
        );
    }

    // -----------------------------------------------------------------------
    // 10. parse_decimal
    // -----------------------------------------------------------------------

    #[test]
    fn parse_decimal_accepts_the_whole_lexical_grammar() {
        let cases: [(&str, Rat); 12] = [
            ("1", Rat::one()),
            ("-1.5", rat(-3, 2)),
            ("+.5", rat(1, 2)),
            ("1.", Rat::one()),
            ("1e10", Rat::from_int(Int::pow10(10))),
            ("-2.5E-3", rat(-1, 400)),
            ("0", Rat::zero()),
            ("00.10", rat(1, 10)),
            ("+1", Rat::one()),
            ("-0", Rat::zero()),
            ("1E+2", Rat::from_i64(100)),
            ("0.000", Rat::zero()),
        ];
        for (text, expected) in cases {
            assert_eq!(
                Rat::parse_decimal(text),
                Some(expected),
                "{text:?} must parse exactly"
            );
        }
    }

    #[test]
    fn parse_decimal_rejects_everything_outside_the_grammar() {
        for bad in [
            "", ".", "e5", "1e", "1.2.3", "NaN", "INF", "-INF", "1 ", " 1", "0x10", "1_000", "+",
            "-", "1e+", "1e-", "--1", "1..2", ".e5", "1e1.5", "0b1",
        ] {
            assert_eq!(Rat::parse_decimal(bad), None, "{bad:?} must be rejected");
        }
        // The neighbouring valid cases for the trickiest rejections.
        assert!(Rat::parse_decimal(".5").is_some(), "a bare .5 is valid");
        assert!(Rat::parse_decimal("1e5").is_some(), "1e5 is valid");
        assert!(Rat::parse_decimal("1e-5").is_some(), "1e-5 is valid");
        assert!(Rat::parse_decimal("1.2").is_some(), "1.2 is valid");
        assert!(Rat::parse_decimal("-1").is_some(), "-1 is valid");
    }

    #[test]
    fn parse_decimal_never_touches_a_float() {
        // 0.1 is not representable in binary. If any step of the parse went
        // through an f64 this would not be exactly one tenth.
        assert_eq!(
            Rat::parse_decimal("0.1"),
            Some(Rat::new(Int::from_i64(1), Int::from_i64(10)).expect("non-zero denominator"))
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").numerator(),
            &Int::one()
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").denominator(),
            &int(10)
        );
        // A 30-digit mantissa survives verbatim; an f64 keeps 17.
        let long = "1.234567890123456789012345678901";
        let parsed = Rat::parse_decimal(long).expect("valid");
        assert_eq!(
            parsed,
            Rat::new(
                Int::from_decimal_digits("1234567890123456789012345678901").expect("digits"),
                Int::pow10(30)
            )
            .expect("non-zero denominator")
        );
        assert_eq!(parsed.to_decimal_string(30), long);
    }

    #[test]
    fn parse_decimal_caps_the_exponent_without_over_refusing() {
        // The refusal.
        assert_eq!(Rat::parse_decimal("1e100001"), None);
        assert_eq!(Rat::parse_decimal("1e-100001"), None);
        assert_eq!(Rat::parse_decimal("1e999999999999999999999"), None);
        assert_eq!(Rat::parse_decimal("0e100001"), None);
        // The over-refusal control: everything at or below the cap still parses.
        let at_cap = Rat::parse_decimal("1e100000").expect("the cap itself must parse");
        assert!(at_cap.is_integer());
        assert_eq!(at_cap.signum(), 1);
        assert_eq!(
            Rat::parse_decimal("1e100"),
            Some(Rat::from_int(Int::pow10(100)))
        );
        assert_eq!(
            Rat::parse_decimal("1e-100"),
            Rat::new(Int::one(), Int::pow10(100))
        );
        let at_cap_negative =
            Rat::parse_decimal("1e-100000").expect("the negative cap must parse too");
        assert!(!at_cap_negative.is_integer());
        assert_eq!(at_cap_negative.signum(), 1);
        assert_eq!(*at_cap_negative.numerator(), Int::one());
    }

    // -----------------------------------------------------------------------
    // 11. to_f64
    // -----------------------------------------------------------------------

    #[test]
    fn to_f64_is_exact_on_powers_of_two() {
        for exponent in -60i32..60 {
            let expected = 2.0f64.powi(exponent);
            let value = if exponent >= 0 {
                Rat::from_int(Int::one().shl(exponent as u32))
            } else {
                Rat::new(Int::one(), Int::one().shl(exponent.unsigned_abs()))
                    .expect("non-zero denominator")
            };
            assert_eq!(value.to_f64().to_bits(), expected.to_bits(), "2^{exponent}");
            assert_eq!(
                value.neg().to_f64().to_bits(),
                (-expected).to_bits(),
                "-2^{exponent}"
            );
        }
    }

    #[test]
    fn to_f64_is_exact_on_small_integers_and_dyadic_fractions() {
        for n in -1_000i64..1_000 {
            assert_eq!(Rat::from_i64(n).to_f64().to_bits(), (n as f64).to_bits());
            assert_eq!(
                rat(n, 4).to_f64().to_bits(),
                ((n as f64) / 4.0).to_bits(),
                "{n}/4"
            );
        }
    }

    #[test]
    fn to_f64_rounds_ties_to_even_in_both_directions() {
        // Doubles just above 1.0 are spaced 2^-52 apart, so `1 + 2^-53` sits
        // exactly on a tie. The even neighbour is 1.0, so it rounds down.
        let two_53 = Int::one().shl(53);
        let tie_down_numerator = two_53.add(&Int::one());
        let down = Rat::new(tie_down_numerator, two_53.clone()).expect("non-zero denominator");
        assert_eq!(down.to_f64().to_bits(), 1.0f64.to_bits());
        // `1 + 3 * 2^-53` is the next tie up; there the even neighbour is the
        // upper one, so the same rule rounds the other way.
        let tie_up_numerator = two_53.add(&int(3));
        let up = Rat::new(tie_up_numerator, two_53).expect("non-zero denominator");
        assert_eq!(up.to_f64().to_bits(), 1.0f64.to_bits() + 2);
        // A quarter of an ulp either side of the first tie resolves by
        // magnitude, not by parity: `1 + 2^-54` rounds down and `1 + 3 * 2^-54`
        // rounds up, even though both quotients are odd.
        let two_54 = Int::one().shl(54);
        let below_numerator = two_54.add(&Int::one());
        let below = Rat::new(below_numerator, two_54.clone()).expect("non-zero denominator");
        assert_eq!(below.to_f64().to_bits(), 1.0f64.to_bits());
        let above_numerator = two_54.add(&int(3));
        let above = Rat::new(above_numerator, two_54).expect("non-zero denominator");
        assert_eq!(above.to_f64().to_bits(), 1.0f64.to_bits() + 1);
    }

    #[test]
    fn to_f64_matches_the_rust_literal_for_inexact_values() {
        assert_eq!(rat(1, 3).to_f64().to_bits(), (1.0f64 / 3.0f64).to_bits());
        assert_eq!(
            rat(-1, 3).to_f64().to_bits(),
            (-(1.0f64 / 3.0f64)).to_bits()
        );
        assert_eq!(
            rat(-1, 3).to_f64().to_bits(),
            (-rat(1, 3).to_f64()).to_bits()
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").to_f64().to_bits(),
            0.1f64.to_bits()
        );
        assert_eq!(
            Rat::parse_decimal("-0.1")
                .expect("valid")
                .to_f64()
                .to_bits(),
            (-0.1f64).to_bits()
        );
        // The shortest decimal that round-trips through an f64 pi must land
        // back on exactly those bits.
        assert_eq!(
            Rat::parse_decimal("3.141592653589793")
                .expect("valid")
                .to_f64()
                .to_bits(),
            std::f64::consts::PI.to_bits()
        );
        assert_eq!(rat(2, 3).to_f64().to_bits(), (2.0f64 / 3.0f64).to_bits());
        assert_eq!(rat(1, 10).to_f64().to_bits(), 0.1f64.to_bits());
        assert_eq!(rat(1, 100).to_f64().to_bits(), 0.01f64.to_bits());
    }

    #[test]
    fn to_f64_reaches_the_subnormal_range_exactly() {
        // The smallest positive subnormal, 2^-1074.
        let tiny = Rat::new(Int::one(), Int::one().shl(1_074)).expect("non-zero denominator");
        assert_eq!(tiny.to_f64().to_bits(), 1);
        assert_eq!(tiny.to_f64(), f64::from_bits(1));
        assert_eq!(tiny.neg().to_f64().to_bits(), (1u64 << 63) | 1);
        // Three times it is still subnormal and still exact.
        let three = Rat::new(int(3), Int::one().shl(1_074)).expect("non-zero denominator");
        assert_eq!(three.to_f64().to_bits(), 3);
        // The largest subnormal, (2^52 - 1) * 2^-1074.
        let largest_subnormal =
            Rat::new(Int::one().shl(52).sub(&Int::one()), Int::one().shl(1_074))
                .expect("non-zero denominator");
        assert_eq!(largest_subnormal.to_f64().to_bits(), (1u64 << 52) - 1);
        // The smallest normal, 2^-1022.
        let smallest_normal =
            Rat::new(Int::one(), Int::one().shl(1_022)).expect("non-zero denominator");
        assert_eq!(
            smallest_normal.to_f64().to_bits(),
            f64::MIN_POSITIVE.to_bits()
        );
        // Rounding up out of the subnormal range lands on the smallest normal.
        let just_under = Rat::new(Int::one().shl(53).sub(&Int::one()), Int::one().shl(1_075))
            .expect("non-zero denominator");
        assert_eq!(just_under.to_f64().to_bits(), f64::MIN_POSITIVE.to_bits());
    }

    #[test]
    fn to_f64_underflows_to_positive_zero_and_overflows_to_infinity() {
        assert_eq!(Rat::zero().to_f64().to_bits(), 0);
        // Half the smallest subnormal is a tie that resolves to even, i.e. zero.
        let half_min = Rat::new(Int::one(), Int::one().shl(1_075)).expect("non-zero");
        assert_eq!(half_min.to_f64().to_bits(), 0);
        // Below that, everything underflows — and never to a negative zero.
        let far_under = Rat::new(Int::one(), Int::one().shl(2_000)).expect("non-zero");
        assert_eq!(far_under.to_f64().to_bits(), 0);
        assert_eq!(far_under.neg().to_f64().to_bits(), 0);
        assert!(far_under.neg().to_f64().is_sign_positive());
        // Just above the tie rounds up to the smallest subnormal instead.
        let just_over = Rat::new(int(3), Int::one().shl(1_076)).expect("non-zero");
        assert_eq!(just_over.to_f64().to_bits(), 1);

        assert_eq!(Rat::from_int(Int::pow10(400)).to_f64(), f64::INFINITY);
        assert_eq!(
            Rat::from_int(Int::pow10(400)).neg().to_f64(),
            f64::NEG_INFINITY
        );
        assert_eq!(Rat::from_int(Int::one().shl(1_024)).to_f64(), f64::INFINITY);
        // The largest finite double is still finite: (2^53 - 1) * 2^971.
        let max = Rat::from_int(Int::one().shl(53).sub(&Int::one()).shl(971));
        assert_eq!(max.to_f64().to_bits(), f64::MAX.to_bits());
        // One ulp of rounding above it overflows.
        let over = Rat::from_int(Int::one().shl(54).sub(&Int::one()).shl(970));
        assert_eq!(over.to_f64(), f64::INFINITY);
    }

    // -----------------------------------------------------------------------
    // 12. to_decimal_string and round_to_scale
    // -----------------------------------------------------------------------

    #[test]
    fn to_decimal_string_rounds_half_to_even_at_the_boundary() {
        assert_eq!(rat(1, 2).to_decimal_string(0), "0");
        assert_eq!(rat(3, 2).to_decimal_string(0), "2");
        assert_eq!(rat(5, 2).to_decimal_string(0), "2");
        assert_eq!(rat(7, 2).to_decimal_string(0), "4");
        assert_eq!(rat(-1, 2).to_decimal_string(0), "0");
        assert_eq!(rat(-3, 2).to_decimal_string(0), "-2");
        assert_eq!(rat(-5, 2).to_decimal_string(0), "-2");
        assert_eq!(rat(1, 3).to_decimal_string(0), "0");
        assert_eq!(rat(2, 3).to_decimal_string(0), "1");
    }

    #[test]
    fn to_decimal_string_suppresses_trailing_zeros_and_the_bare_point() {
        assert_eq!(rat(1, 2).to_decimal_string(5), "0.5");
        assert_eq!(rat(1, 8).to_decimal_string(10), "0.125");
        assert_eq!(rat(5, 1).to_decimal_string(7), "5");
        assert_eq!(rat(-5, 1).to_decimal_string(7), "-5");
        assert_eq!(Rat::zero().to_decimal_string(7), "0");
        assert_eq!(Rat::zero().to_decimal_string(0), "0");
        assert_eq!(rat(-1, 100_000).to_decimal_string(5), "-0.00001");
        assert_eq!(rat(-1, 100_000).to_decimal_string(4), "0");
        assert_eq!(rat(123, 100).to_decimal_string(4), "1.23");
        assert_eq!(rat(1, 3).to_decimal_string(5), "0.33333");
        assert_eq!(rat(2, 3).to_decimal_string(5), "0.66667");
        assert_eq!(rat(-2, 3).to_decimal_string(5), "-0.66667");
        assert_eq!(rat(10, 4).to_decimal_string(3), "2.5");
    }

    #[test]
    fn to_decimal_string_never_writes_a_negative_zero() {
        for value in [rat(-1, 2), rat(-1, 3), rat(-1, 1_000_000), rat(-1, 4)] {
            let rendered = value.to_decimal_string(0);
            assert_eq!(rendered, "0", "{value:?} at scale 0");
            assert!(!rendered.starts_with('-'));
        }
        assert_eq!(Rat::zero().neg().to_decimal_string(5), "0");
    }

    #[test]
    fn round_to_scale_rounds_half_to_even() {
        assert_eq!(rat(1, 2).round_to_scale(0), Int::zero());
        assert_eq!(rat(3, 2).round_to_scale(0), int(2));
        assert_eq!(rat(5, 2).round_to_scale(0), int(2));
        assert_eq!(rat(-5, 2).round_to_scale(0), int(-2));
        assert_eq!(rat(1, 4).round_to_scale(1), int(2));
        assert_eq!(rat(3, 4).round_to_scale(1), int(8));
        assert_eq!(rat(-3, 4).round_to_scale(1), int(-8));
        assert_eq!(rat(1, 3).round_to_scale(3), int(333));
        assert_eq!(rat(2, 3).round_to_scale(3), int(667));
        assert_eq!(Rat::from_i64(7).round_to_scale(2), int(700));
        assert_eq!(Rat::zero().round_to_scale(9), Int::zero());
    }

    #[test]
    fn directed_decimal_spellings_preserve_the_grid_and_canonical_zero() {
        use crate::ieee::ratio::Rounding;
        for (value, scale, lower, upper) in [
            (rat(1, 3), 3, "0.333", "0.334"),
            (rat(-1, 3), 3, "-0.334", "-0.333"),
            (rat(1, 8), 3, "0.125", "0.125"),
            (rat(-1, 8), 3, "-0.125", "-0.125"),
            (rat(1, 2), 0, "0", "1"),
            (rat(-1, 2), 0, "-1", "0"),
            (rat(1, 20_000), 4, "0", "0.0001"),
            (rat(-1, 20_000), 4, "-0.0001", "0"),
            (Rat::zero(), 15, "0", "0"),
        ] {
            assert_eq!(value.to_decimal_string_with(scale, Rounding::Down), lower);
            assert_eq!(value.to_decimal_string_with(scale, Rounding::Up), upper);
            assert_eq!(
                value.to_decimal_string_with(scale, Rounding::NearestEven),
                value.to_decimal_string(scale),
            );
        }
    }

    #[test]
    fn directed_decimal_walls_enclose_exact_sources_on_the_declared_grid() {
        use crate::ieee::ratio::Rounding;
        let scratch = LimbScratch::new(64, 12).unwrap();
        for (value, scale, expected) in [
            (rat(1, 3), 3, [333, 334]),
            (rat(-1, 3), 3, [-334, -333]),
            (rat(1, 8), 3, [125, 125]),
            (rat(-1, 8), 3, [-125, -125]),
            (rat(1, 2), 0, [0, 1]),
            (rat(-1, 2), 0, [-1, 0]),
            (Rat::zero(), 15, [0, 0]),
        ] {
            for (rounding, expected) in [Rounding::Down, Rounding::Up].into_iter().zip(expected) {
                let ordinary = value.round_to_scale_with(scale, rounding);
                assert_eq!(ordinary, Int::from_i64(expected));
                assert_eq!(
                    value
                        .round_to_scale_with_in(scale, rounding, &scratch)
                        .unwrap(),
                    ordinary,
                );
            }
        }
        let value = Rat::new(
            Int::one().shl(447).add(&Int::from_i64(13)),
            Int::one().shl(448).add(&Int::from_i64(29)),
        )
        .unwrap();
        for value in [&value, &value.neg()] {
            let lower = Rat::from_decimal(value.round_to_scale_with(15, Rounding::Down), 15);
            let upper = Rat::from_decimal(value.round_to_scale_with(15, Rounding::Up), 15);
            assert!(lower <= *value && *value <= upper);
            assert!(upper.sub(&lower) <= Rat::from_decimal(Int::one(), 15));
            for rounding in [Rounding::Down, Rounding::Up] {
                assert_eq!(
                    value
                        .round_to_scale_with_in(15, rounding, &scratch)
                        .unwrap(),
                    value.round_to_scale_with(15, rounding),
                );
            }
        }
        let narrow = LimbScratch::new(32, 1).unwrap();
        assert!(matches!(
            value.round_to_scale_with_in(15, Rounding::Down, &narrow),
            Err(LimbScratchError::Capacity { .. }),
        ));
    }

    #[test]
    fn round_to_scale_and_from_decimal_are_inverse_at_exact_scales() {
        for (num, den, scale) in [(1i64, 2i64, 1u32), (1, 8, 3), (-3, 4, 2), (22, 25, 2)] {
            let value = rat(num, den);
            let mantissa = value.round_to_scale(scale);
            assert_eq!(Rat::from_decimal(mantissa, scale), value);
        }
    }

    // -----------------------------------------------------------------------
    // 13. Determinism of the float boundary
    // -----------------------------------------------------------------------

    #[test]
    fn to_f64_depends_only_on_the_value_not_on_how_it_was_built() {
        let spellings = [
            rat(1, 2),
            rat(3, 6),
            rat(-2, -4),
            rat(50, 100),
            Rat::from_decimal(int(5), 1),
            Rat::parse_decimal("0.5").expect("valid"),
            Rat::parse_decimal(".5").expect("valid"),
            Rat::parse_decimal("5e-1").expect("valid"),
            Rat::parse_decimal("50e-2").expect("valid"),
            Rat::parse_decimal("+0.500").expect("valid"),
            Rat::one().div(&Rat::from_i64(2)).expect("non-zero divisor"),
            rat(1, 4).add(&rat(1, 4)),
        ];
        let first = spellings[0].clone();
        for spelling in &spellings {
            assert_eq!(*spelling, first, "{spelling:?} must be the same value");
            assert_eq!(
                spelling.to_f64().to_bits(),
                first.to_f64().to_bits(),
                "{spelling:?} must produce identical bits"
            );
        }
    }

    #[test]
    fn parse_decimal_is_bit_identical_across_lexical_spellings() {
        let groups: [(&[&str], Rat); 4] = [
            (
                &["1", "1.0", "1.", "+1", "01", "0.1e1", "10e-1"],
                Rat::one(),
            ),
            (
                &["0", "-0", "+0", "0.0", ".0", "0e100", "0.000e-5"],
                Rat::zero(),
            ),
            (
                &["-1.5", "-1.50", "-15e-1", "-0.15e1", "-150e-2"],
                rat(-3, 2),
            ),
            (&["0.1", ".1", "1e-1", "10e-2", "0.100", "00.1"], rat(1, 10)),
        ];
        for (spellings, expected) in groups {
            for spelling in spellings {
                let parsed = Rat::parse_decimal(spelling).expect("spelling is valid");
                assert_eq!(parsed, expected, "{spelling:?}");
                assert_eq!(
                    parsed.to_f64().to_bits(),
                    expected.to_f64().to_bits(),
                    "{spelling:?} bits"
                );
                assert_eq!(
                    parsed.numerator(),
                    expected.numerator(),
                    "{spelling:?} numerator"
                );
                assert_eq!(
                    parsed.denominator(),
                    expected.denominator(),
                    "{spelling:?} denominator"
                );
            }
        }
    }

    #[test]
    fn equal_values_hash_and_compare_identically() {
        let mut set: std::collections::HashSet<Rat, purrdf_hash::fixed::FixedState> =
            std::collections::HashSet::default();
        set.insert(rat(1, 2));
        assert!(set.contains(&rat(3, 6)));
        assert!(set.contains(&rat(-1, -2)));
        assert!(!set.insert(rat(50, 100)), "an equal value must not be new");
        assert_eq!(set.len(), 1);

        let mut ints: std::collections::HashSet<Int, purrdf_hash::fixed::FixedState> =
            std::collections::HashSet::default();
        ints.insert(Int::zero());
        assert!(ints.contains(&Int::zero().neg()));
        assert!(ints.contains(&int(5).sub(&int(5))));
        assert_eq!(ints.len(), 1);
    }
}
