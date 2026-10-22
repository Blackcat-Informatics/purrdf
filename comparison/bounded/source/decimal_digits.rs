// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`DecimalDigits`]: an `xsd:integer` or `xsd:decimal` value of ANY size, held as its
//! normalized decimal digits, and [`LiteralValue`]: the value a literal denotes, past
//! the bounded representation as well as inside it.
//!
//! # Why a digit string, and why no arithmetic
//!
//! The bounded representation ([`crate::Decimal`], `i128`) is this crate's
//! arithmetic domain, and its limits are the documented conformance contract (see the
//! crate docs, *Numeric limits*). Three jobs, however, need no arithmetic at all and so
//! need no limit: ordering two numbers, deciding whether they are equal, and rounding
//! one to the nearest binary floating-point value. A literal past the bounds is still a
//! perfectly good member of the `xsd:integer`/`xsd:decimal` value space, so for those
//! three jobs it is answered exactly, whatever its length.
//!
//! The representation is the normal form of a decimal numeral: a sign, the significant
//! digits with no leading or trailing zero, and the decimal exponent that places them —
//! the value is `±0.d₁d₂…dₙ × 10^exponent`. Two numerals denote one value exactly when
//! their normal forms are identical, so derived equality IS value equality, and the
//! order is sign, then exponent, then the digit strings compared lexicographically.
//! Rounding to `f64`/`f32` hands the normal form to the correctly rounded decimal
//! reader, which rounds any number of digits once.
//!
//! A value here takes no part in `+`, `−`, `×` or `÷`: arithmetic stays in the bounded
//! domain, and an operand outside it is a typed error ([`crate::ErrorCode`]).

use std::cmp::Ordering;

use crate::bigint::BigInt;
use crate::datatype::XsdDatatype;
use crate::numeric::{Decimal, is_decimal_lexical, is_integer_lexical, signed};
use crate::value::{XsdError, XsdValue};

/// An exact `xsd:integer`/`xsd:decimal` value of any magnitude and scale, as its
/// normalized digits: the comparison key and binary-floating-point source for
/// literals past the bounded representation. See the [module docs](self).
///
/// ```rust
/// use std::cmp::Ordering;
///
/// use purrdf_xsd::DecimalDigits;
///
/// let big = DecimalDigits::parse("100000000000000000000000000000000000000000").unwrap();
/// let same = DecimalDigits::parse("+000100000000000000000000000000000000000000000.000").unwrap();
/// assert_eq!(big, same);
/// let fine = DecimalDigits::parse("0.10000000000000000001").unwrap();
/// assert_eq!(fine.cmp(&DecimalDigits::parse("0.1").unwrap()), Ordering::Greater);
/// assert_eq!(big.to_f64(), 1e41);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecimalDigits {
    /// `true` for a negative value; always `false` for zero.
    negative: bool,
    /// The significant digits, ASCII, with no leading and no trailing `0`; empty for
    /// zero.
    digits: Box<str>,
    /// The value is `±0.digits × 10^exponent`; zero for zero.
    exponent: i64,
}

impl DecimalDigits {
    /// The value of an `xsd:decimal` lexical form (which includes every `xsd:integer`
    /// lexical form): an optional sign, then digits with at most one `.` and at least
    /// one digit. `None` for anything else. No whitespace is trimmed, exactly as
    /// [`crate::parse`] trims none.
    #[must_use]
    pub fn parse(lexical: &str) -> Option<Self> {
        if !is_decimal_lexical(lexical) {
            return None;
        }
        let negative = lexical.starts_with('-');
        let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
        let (int, frac) = body.split_once('.').unwrap_or((body, ""));
        let int = int.trim_start_matches('0');
        let (digits, exponent) = if int.is_empty() {
            let significant = frac.trim_start_matches('0');
            let zeros = frac.len() - significant.len();
            (
                significant.trim_end_matches('0').to_owned(),
                -i64::try_from(zeros).ok()?,
            )
        } else {
            let mut digits = String::with_capacity(int.len() + frac.len());
            digits.push_str(int);
            digits.push_str(frac);
            let trimmed = digits.trim_end_matches('0').len();
            digits.truncate(trimmed);
            (digits, i64::try_from(int.len()).ok()?)
        };
        Some(Self::from_normal(negative, digits, exponent))
    }

    /// The value of `lexical` read as `datatype`, which must be `xsd:decimal` or an
    /// `xsd:integer`-family datatype: the lexical form must be in that datatype's
    /// lexical space ([`XsdError::InvalidLexical`] otherwise) and the value inside its
    /// facets — the sign of `xsd:positiveInteger` and its relatives, the ranges of
    /// `xsd:long` and its relatives ([`XsdError::OutOfRange`] otherwise). There is no
    /// other bound: this is the unbounded value space.
    ///
    /// # Errors
    ///
    /// As above, and [`XsdError::TypeMismatch`] for any other datatype.
    pub fn parse_typed(lexical: &str, datatype: XsdDatatype) -> Result<Self, XsdError> {
        let in_lexical_space = match datatype {
            XsdDatatype::Decimal => is_decimal_lexical(lexical),
            other if other.is_integer_family() => is_integer_lexical(lexical),
            _ => {
                return Err(XsdError::TypeMismatch {
                    reason: "an unbounded exact value is an xsd:integer or xsd:decimal",
                });
            }
        };
        let value = Self::parse(lexical)
            .filter(|_| in_lexical_space)
            .ok_or_else(|| {
                XsdError::invalid(
                    datatype,
                    lexical,
                    if datatype == XsdDatatype::Decimal {
                        "expected an optional sign then digits with at most one point"
                    } else {
                        "expected an optional sign then digits"
                    },
                )
            })?;
        let within = match datatype {
            XsdDatatype::Decimal | XsdDatatype::Integer => true,
            XsdDatatype::NonNegativeInteger => !value.negative,
            XsdDatatype::PositiveInteger => !value.negative && !value.is_zero(),
            XsdDatatype::NonPositiveInteger => value.negative || value.is_zero(),
            XsdDatatype::NegativeInteger => value.negative,
            bounded => match (value.to_i128(), bounded.integer_range()) {
                (Some(v), Some((min, max))) => (min..=max).contains(&v),
                _ => false,
            },
        };
        if within {
            Ok(value)
        } else {
            Err(XsdError::OutOfRange {
                datatype,
                lexical: lexical.to_owned(),
                reason: crate::value::reason::OUTSIDE_DATATYPE,
            })
        }
    }

    /// The value of an `i128`.
    #[must_use]
    pub fn from_i128(value: i128) -> Self {
        let digits = value.unsigned_abs().to_string();
        let exponent = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        let trimmed = digits.trim_end_matches('0').to_owned();
        Self::from_normal(value < 0, trimmed, exponent)
    }

    /// The value of a bounded [`Decimal`].
    #[must_use]
    pub fn from_decimal(value: &Decimal) -> Self {
        let digits = value.mantissa().unsigned_abs().to_string();
        let exponent = i64::try_from(digits.len()).unwrap_or(i64::MAX) - i64::from(value.scale());
        let trimmed = digits.trim_end_matches('0').to_owned();
        Self::from_normal(value.mantissa() < 0, trimmed, exponent)
    }

    /// The value of an exact bounded [`XsdValue`] (`xsd:integer` family or
    /// `xsd:decimal`); `None` for every other value.
    #[must_use]
    pub fn from_value(value: &XsdValue) -> Option<Self> {
        match value {
            XsdValue::Integer { value, .. } => Some(Self::from_i128(*value)),
            XsdValue::Decimal(d) => Some(Self::from_decimal(d)),
            _ => None,
        }
    }

    /// Build from an already-normal digit string (no leading or trailing zero).
    fn from_normal(negative: bool, digits: String, exponent: i64) -> Self {
        if digits.is_empty() {
            return Self {
                negative: false,
                digits: Box::from(""),
                exponent: 0,
            };
        }
        Self {
            negative,
            digits: digits.into_boxed_str(),
            exponent,
        }
    }

    /// Whether the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// Whether the value is an integer.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.fraction_digits() == 0
    }

    /// The number of digits before the decimal point in the canonical form, leading
    /// zeros excluded (`0` for a magnitude below one).
    #[must_use]
    pub fn integer_digits(&self) -> u64 {
        if self.is_zero() {
            0
        } else {
            u64::try_from(self.exponent).unwrap_or(0)
        }
    }

    /// The number of digits after the decimal point in the canonical form, trailing
    /// zeros excluded (`0` for an integer).
    #[must_use]
    pub fn fraction_digits(&self) -> u64 {
        let len = i64::try_from(self.digits.len()).unwrap_or(i64::MAX);
        u64::try_from(len.saturating_sub(self.exponent)).unwrap_or(0)
    }

    /// The XSD 1.1 canonical `xsd:decimal` lexical form (`decimalCanonicalMap`): no
    /// decimal point for an integer value, otherwise the shortest fractional part.
    /// For an integer value this is also the `xsd:integer` canonical form.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        if self.is_zero() {
            return "0".to_owned();
        }
        let digits: &str = &self.digits;
        let len = digits.len();
        let mut out = String::with_capacity(len + 3);
        if self.negative {
            out.push('-');
        }
        match usize::try_from(self.exponent) {
            Ok(0) | Err(_) => {
                out.push_str("0.");
                let zeros = usize::try_from(self.exponent.unsigned_abs()).unwrap_or(usize::MAX);
                out.extend(std::iter::repeat_n('0', zeros));
                out.push_str(digits);
            }
            Ok(point) if point < len => {
                out.push_str(&digits[..point]);
                out.push('.');
                out.push_str(&digits[point..]);
            }
            Ok(point) => {
                out.push_str(digits);
                out.extend(std::iter::repeat_n('0', point - len));
            }
        }
        out
    }

    /// The value as an `i128`, when it is an integer that fits one.
    #[must_use]
    pub fn to_i128(&self) -> Option<i128> {
        if self.is_integer() {
            self.truncate_to_i128()
        } else {
            None
        }
    }

    /// The value with its fractional part discarded (rounded toward zero) — the
    /// `xs:decimal` to `xs:integer` cast of XPath F&O 3.1 §19.1.2.4 — when that
    /// integer fits an `i128`; `None` otherwise (`err:FOCA0003`).
    #[must_use]
    pub fn truncate_to_i128(&self) -> Option<i128> {
        if self.is_zero() || self.exponent <= 0 {
            return Some(0);
        }
        // `|i128| < 2^127 < 10^39`, so an integer part of 40 or more digits cannot fit.
        let width = usize::try_from(self.exponent).ok().filter(|&w| w <= 39)?;
        let digits: &str = &self.digits;
        let magnitude = if width <= digits.len() {
            digits[..width].parse::<u128>().ok()?
        } else {
            let head = digits.parse::<u128>().ok()?;
            let shift = u32::try_from(width - digits.len()).ok()?;
            head.checked_mul(10_u128.checked_pow(shift)?)?
        };
        signed(self.negative, magnitude)
    }

    /// The value as a bounded [`Decimal`] — at most 18 fractional digits and an `i128`
    /// mantissa — when it is exactly one; `None` otherwise.
    #[must_use]
    pub fn to_decimal(&self) -> Option<Decimal> {
        if self.is_zero() {
            return Some(Decimal::from_integer(0));
        }
        let scale = u8::try_from(self.fraction_digits())
            .ok()
            .filter(|&scale| scale <= crate::numeric::MAX_DECIMAL_SCALE)?;
        // mantissa = digits × 10^(exponent + scale − len); that power is non-negative
        // because scale ≥ len − exponent.
        let digits: &str = &self.digits;
        let pad = self.exponent + i64::from(scale) - i64::try_from(digits.len()).ok()?;
        if digits.len() > 39 {
            return None;
        }
        let head = digits.parse::<u128>().ok()?;
        let magnitude = head.checked_mul(10_u128.checked_pow(u32::try_from(pad).ok()?)?)?;
        Some(Decimal::from_parts(
            signed(self.negative, magnitude)?,
            scale,
        ))
    }

    /// The value as a bounded [`XsdValue`] of `datatype` (`xsd:decimal` or an integer
    /// family datatype), when the bounded representation holds it exactly.
    #[must_use]
    pub fn to_value(&self, datatype: XsdDatatype) -> Option<XsdValue> {
        if datatype == XsdDatatype::Decimal {
            self.to_decimal().map(XsdValue::Decimal)
        } else if datatype.is_integer_family() {
            self.to_i128()
                .map(|value| XsdValue::Integer { value, datatype })
        } else {
            None
        }
    }

    /// The `f64` nearest the value, ties to even — XSD 1.1 `floatingPointRound`, and
    /// the cast and promotion XPath F&O 3.1 §19.1.2.2 require. Rounded ONCE, from all
    /// the digits, however many there are; a magnitude past the binary64 range is an
    /// infinity and one below its smallest subnormal half a signed zero. A zero value
    /// is `+0.0`.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        match self.binary_extreme() {
            Some(Extreme::Huge) => self.signed_inf_f64(),
            Some(Extreme::Tiny) => {
                if self.negative {
                    -0.0
                } else {
                    0.0
                }
            }
            None => self.scientific().parse::<f64>().unwrap_or(f64::NAN),
        }
    }

    /// The `f32` nearest the value, ties to even, rounded once straight to single
    /// precision (never through `f64`, which would round twice).
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        match self.binary_extreme() {
            Some(Extreme::Huge) => {
                if self.negative {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                }
            }
            Some(Extreme::Tiny) => {
                if self.negative {
                    -0.0
                } else {
                    0.0
                }
            }
            None => self.scientific().parse::<f32>().unwrap_or(f32::NAN),
        }
    }

    /// Order this exact value against a binary floating-point value, exactly: `None`
    /// for `NaN`, the infinities at the two ends, `±0.0` equal to zero.
    #[must_use]
    pub fn cmp_f64(&self, ieee: f64) -> Option<Ordering> {
        if ieee.is_nan() {
            return None;
        }
        if ieee.is_infinite() {
            return Some(if ieee > 0.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let own = self.sign();
        let other = if ieee == 0.0 {
            0
        } else if ieee < 0.0 {
            -1
        } else {
            1
        };
        if own != other {
            return Some(own.cmp(&other));
        }
        if own == 0 {
            return Some(Ordering::Equal);
        }
        let magnitude = self.magnitude_cmp_f64(ieee.abs());
        Some(if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        })
    }

    /// `-1`, `0` or `1`.
    fn sign(&self) -> i8 {
        if self.is_zero() {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }

    /// Compare the strictly positive magnitude with the strictly positive finite
    /// `ieee`, exactly.
    fn magnitude_cmp_f64(&self, ieee: f64) -> Ordering {
        // `10^(exponent − 1) ≤ |self| < 10^exponent`. Every finite double is below
        // `2^1024 < 10^309` and every non-zero one at least `2^-1074 > 10^-324`, so
        // either end of that window decides the pair without touching a digit.
        if self.exponent > 309 {
            return Ordering::Greater;
        }
        if self.exponent < -324 {
            return Ordering::Less;
        }
        let (significand, exponent2) = crate::numeric::dyadic_magnitude(ieee);
        let mut left = BigInt::from_digits(&self.digits).unwrap_or_else(BigInt::zero);
        let mut right = BigInt::from_u128(u128::from(significand));
        // |self| = digits × 10^(exponent − len); |ieee| = significand × 2^exponent2.
        let len = i64::try_from(self.digits.len()).unwrap_or(i64::MAX);
        let power10 = self.exponent - len;
        let shift10 = u32::try_from(power10.unsigned_abs()).unwrap_or(u32::MAX);
        if power10 >= 0 {
            left = left.mul_pow10(shift10);
        } else {
            right = right.mul_pow10(shift10);
        }
        let shift2 = exponent2.unsigned_abs();
        if exponent2 >= 0 {
            right = right.mul_pow2(shift2);
        } else {
            left = left.mul_pow2(shift2);
        }
        left.cmp(&right)
    }

    /// Whether the magnitude lies past every finite binary64/binary32 value or below
    /// half of every non-zero one, so the conversion is decided without reading the
    /// digits — and without handing the reader an exponent it would have to clamp.
    fn binary_extreme(&self) -> Option<Extreme> {
        if self.is_zero() {
            return Some(Extreme::Tiny);
        }
        if self.exponent > 400 {
            Some(Extreme::Huge)
        } else if self.exponent < -400 {
            Some(Extreme::Tiny)
        } else {
            None
        }
    }

    fn signed_inf_f64(&self) -> f64 {
        if self.negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    }

    /// `±0.digits e exponent`, the form the correctly rounded reader takes.
    fn scientific(&self) -> String {
        let mut text = String::with_capacity(self.digits.len() + 26);
        if self.negative {
            text.push('-');
        }
        text.push_str("0.");
        text.push_str(&self.digits);
        text.push('e');
        text.push_str(&self.exponent.to_string());
        text
    }
}

/// A magnitude no binary floating-point format distinguishes from an infinity or a
/// zero.
enum Extreme {
    Huge,
    Tiny,
}

impl Ord for DecimalDigits {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.sign().cmp(&other.sign()) {
            Ordering::Equal => {}
            decided => return decided,
        }
        if self.is_zero() {
            return Ordering::Equal;
        }
        let magnitude = self
            .exponent
            .cmp(&other.exponent)
            .then_with(|| self.digits.as_bytes().cmp(other.digits.as_bytes()));
        if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

impl PartialOrd for DecimalDigits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The value a typed literal denotes: a bounded [`XsdValue`], or — for an
/// `xsd:integer`/`xsd:decimal` literal past the bounded representation — its exact
/// [`DecimalDigits`].
///
/// This is the domain of the operations that need no arithmetic: [`literal_cmp`] (the
/// SPARQL `<` and `=` over the promotion lattice), [`literal_equal`] and
/// [`literal_total_cmp`] (the exact order a sort uses). Over two bounded values each is
/// exactly its [`XsdValue`] counterpart ([`crate::value_cmp`], [`crate::value_equal`],
/// [`crate::value_total_cmp`]); a value past the bounds compares exactly against every
/// exact value and, under promotion, as its correctly rounded `xsd:float`/`xsd:double`.
///
/// ```rust
/// use std::cmp::Ordering;
///
/// use purrdf_xsd::{LiteralValue, XsdDatatype, literal_cmp};
///
/// let big = LiteralValue::parse("100000000000000000000000000000000000000000", XsdDatatype::Integer)?;
/// let max = LiteralValue::parse("170141183460469231731687303715884105727", XsdDatatype::Integer)?;
/// assert!(big.bounded().is_none());
/// assert!(max.bounded().is_some());
/// assert_eq!(literal_cmp(&big, &max), Some(Ordering::Greater));
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LiteralValue {
    /// A value of the bounded value space — everything [`crate::parse`] answers.
    Bounded(XsdValue),
    /// An `xsd:integer`-family or `xsd:decimal` value past the bounded representation.
    Unbounded {
        /// The exact value.
        digits: DecimalDigits,
        /// The literal's datatype (an integer-family datatype or `xsd:decimal`).
        datatype: XsdDatatype,
    },
}

impl LiteralValue {
    /// The value `lexical` denotes as `datatype`: [`crate::parse`]'s answer, or — when
    /// that refuses an `xsd:integer`/`xsd:decimal` value only because it is past the
    /// bounded representation — the exact [`DecimalDigits`].
    ///
    /// # Errors
    ///
    /// Every error [`crate::parse`] reports except the representation limit itself: a
    /// malformed lexical form, a derived integer outside its facets (`xsd:byte` of
    /// `300`), and every non-numeric datatype's errors.
    pub fn parse(lexical: &str, datatype: XsdDatatype) -> Result<Self, XsdError> {
        Self::widen(crate::value::parse(lexical, datatype), lexical, datatype)
    }

    /// [`Self::parse`] under the XSD 1.0 operand rules ([`crate::parse_xsd10`]).
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn parse_xsd10(lexical: &str, datatype: XsdDatatype) -> Result<Self, XsdError> {
        Self::widen(
            crate::value::parse_xsd10(lexical, datatype),
            lexical,
            datatype,
        )
    }

    /// [`Self::parse`] by datatype IRI: `Ok(None)` when the IRI names no XSD value
    /// space, exactly as [`crate::parse_by_iri`].
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn parse_by_iri(lexical: &str, datatype_iri: &str) -> Result<Option<Self>, XsdError> {
        XsdDatatype::from_iri(datatype_iri)
            .map(|datatype| Self::parse(lexical, datatype))
            .transpose()
    }

    fn widen(
        bounded: Result<XsdValue, XsdError>,
        lexical: &str,
        datatype: XsdDatatype,
    ) -> Result<Self, XsdError> {
        match bounded {
            Ok(value) => Ok(Self::Bounded(value)),
            Err(error @ XsdError::OutOfRange { .. })
                if datatype == XsdDatatype::Decimal || datatype.is_integer_family() =>
            {
                DecimalDigits::parse_typed(lexical, datatype)
                    .map(|digits| Self::Unbounded { digits, datatype })
                    .map_err(|_| error)
            }
            Err(error) => Err(error),
        }
    }

    /// The bounded value, when the literal has one.
    #[must_use]
    pub const fn bounded(&self) -> Option<&XsdValue> {
        match self {
            Self::Bounded(value) => Some(value),
            Self::Unbounded { .. } => None,
        }
    }

    /// The exact digits of a value past the bounded representation.
    #[must_use]
    pub const fn unbounded(&self) -> Option<&DecimalDigits> {
        match self {
            Self::Unbounded { digits, .. } => Some(digits),
            Self::Bounded(_) => None,
        }
    }

    /// The literal's datatype.
    #[must_use]
    pub fn datatype(&self) -> XsdDatatype {
        match self {
            Self::Bounded(value) => value.datatype(),
            Self::Unbounded { datatype, .. } => *datatype,
        }
    }

    /// Whether the value is in the SPARQL numeric tower.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        match self {
            Self::Bounded(value) => value.is_numeric(),
            Self::Unbounded { .. } => true,
        }
    }

    /// The bounded value, or — for a value past the bounds — its correctly rounded
    /// promotion to `target` when `target` is `xsd:double` or `xsd:float` (the only
    /// promotion that needs no bounded exact representation). `None` otherwise.
    #[must_use]
    pub fn promoted(&self, target: XsdDatatype) -> Option<XsdValue> {
        match (self, target) {
            (Self::Bounded(value), _) => Some(value.clone()),
            (Self::Unbounded { digits, .. }, XsdDatatype::Double) => {
                Some(XsdValue::Double(digits.to_f64()))
            }
            (Self::Unbounded { digits, .. }, XsdDatatype::Float) => {
                Some(XsdValue::Float(digits.to_f32()))
            }
            (Self::Unbounded { .. }, _) => None,
        }
    }
}

/// SPARQL's `<`/`=` order (§17.3, the promotion lattice) over two literal values:
/// [`crate::value_cmp`] for two bounded values; for a value past the bounds, the exact
/// order against an `xsd:integer`/`xsd:decimal` value and, against an
/// `xsd:float`/`xsd:double`, the order of its correctly rounded conversion to that
/// type. `None` for incomparable values.
#[must_use]
pub fn literal_cmp(a: &LiteralValue, b: &LiteralValue) -> Option<Ordering> {
    mixed(a, b, crate::ops::value_cmp, false)
}

/// The exact total order a sort uses ([`crate::value_total_cmp`]) over two literal
/// values, a value past the bounds included: it compares exactly against every value of
/// the numeric tower, the binary floating-point ones included. `None` for `NaN` and for
/// incomparable values.
#[must_use]
pub fn literal_total_cmp(a: &LiteralValue, b: &LiteralValue) -> Option<Ordering> {
    mixed(a, b, crate::ops::value_total_cmp, true)
}

/// SPARQL value equality ([`crate::value_equal`]) over two literal values; a value past
/// the bounds is equal to another exactly when [`literal_cmp`] calls them equal.
#[must_use]
pub fn literal_equal(a: &LiteralValue, b: &LiteralValue) -> Option<bool> {
    match (a, b) {
        (LiteralValue::Bounded(x), LiteralValue::Bounded(y)) => crate::ops::value_equal(x, y),
        _ => literal_cmp(a, b).map(|ord| ord == Ordering::Equal),
    }
}

fn mixed(
    a: &LiteralValue,
    b: &LiteralValue,
    bounded: fn(&XsdValue, &XsdValue) -> Option<Ordering>,
    exact_ieee: bool,
) -> Option<Ordering> {
    match (a, b) {
        (LiteralValue::Bounded(x), LiteralValue::Bounded(y)) => bounded(x, y),
        (LiteralValue::Unbounded { digits: x, .. }, LiteralValue::Unbounded { digits: y, .. }) => {
            Some(x.cmp(y))
        }
        (LiteralValue::Unbounded { digits, .. }, LiteralValue::Bounded(value)) => {
            unbounded_vs_bounded(digits, value, exact_ieee)
        }
        (LiteralValue::Bounded(value), LiteralValue::Unbounded { digits, .. }) => {
            unbounded_vs_bounded(digits, value, exact_ieee).map(Ordering::reverse)
        }
    }
}

/// Order a value past the bounds against a bounded value.
fn unbounded_vs_bounded(
    digits: &DecimalDigits,
    value: &XsdValue,
    exact_ieee: bool,
) -> Option<Ordering> {
    match value {
        XsdValue::Integer { .. } | XsdValue::Decimal(_) => {
            // Every bounded exact value is below `2^127 < 10^39` in magnitude, so a
            // value with forty or more integer digits is decided by its sign alone.
            if digits.integer_digits() >= 40 {
                return Some(if digits.is_negative() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                });
            }
            DecimalDigits::from_value(value).map(|other| digits.cmp(&other))
        }
        XsdValue::Float(f) if exact_ieee => digits.cmp_f64(f64::from(*f)),
        XsdValue::Double(d) if exact_ieee => digits.cmp_f64(*d),
        XsdValue::Float(f) => digits.to_f32().partial_cmp(f),
        XsdValue::Double(d) => digits.to_f64().partial_cmp(d),
        _ => None,
    }
}

// ── Arithmetic over operands past the bounds ─────────────────────────────────

/// An exact `xsd:integer`/`xsd:decimal` operand of any size: `mantissa / 10^scale`,
/// and whether it is an integer-family value (whose results stay `xsd:integer`).
struct Exact {
    mantissa: BigInt,
    scale: u32,
    integer: bool,
}

impl Exact {
    /// The exact operand `value` denotes; `None` for every value outside the exact
    /// branch of the numeric tower.
    fn of(value: &LiteralValue) -> Option<Self> {
        match value {
            LiteralValue::Bounded(XsdValue::Integer { value, .. }) => Some(Self {
                mantissa: BigInt::from_i128(*value),
                scale: 0,
                integer: true,
            }),
            LiteralValue::Bounded(XsdValue::Decimal(d)) => Some(Self {
                mantissa: BigInt::from_i128(d.mantissa()),
                scale: u32::from(d.scale()),
                integer: false,
            }),
            LiteralValue::Unbounded { digits, datatype } => Some(Self {
                mantissa: BigInt::from_digits(&digits.canonical_lexical().replace('.', ""))?,
                scale: u32::try_from(digits.fraction_digits()).ok()?,
                integer: datatype.is_integer_family(),
            }),
            LiteralValue::Bounded(_) => None,
        }
    }

    /// `self.mantissa` at `scale ≥ self.scale`.
    fn at_scale(&self, scale: u32) -> BigInt {
        self.mantissa.mul_pow10(scale - self.scale)
    }

    /// The bounded result: an `xsd:integer` when `integer` and it fits `i128`, an
    /// `xsd:decimal` under the precision rule otherwise; an integer part past the
    /// bounds is `err:FOAR0002`.
    fn bounded(mantissa: &BigInt, scale: u32, integer: bool) -> Result<XsdValue, XsdError> {
        if integer && scale == 0 {
            return mantissa.to_i128().map_or_else(
                || {
                    Err(XsdError::OutOfRange {
                        datatype: XsdDatatype::Integer,
                        lexical: String::new(),
                        reason: crate::value::reason::INTEGER_OVERFLOW,
                    })
                },
                |value| {
                    Ok(XsdValue::Integer {
                        value,
                        datatype: XsdDatatype::Integer,
                    })
                },
            );
        }
        crate::numeric::decimal_mean(mantissa, scale, 1).map(XsdValue::Decimal)
    }
}

/// `⌊numerator / denominator⌋` for a non-negative `numerator` and a positive
/// `denominator`, by schoolbook long division over the numerator's decimal digits.
fn quotient(numerator: &BigInt, denominator: &BigInt) -> BigInt {
    let mut digits = String::new();
    let mut remainder = BigInt::zero();
    let negated = denominator.negated();
    for digit in numerator.to_decimal_string().bytes() {
        remainder = remainder.mul_small(10);
        remainder.add_assign(&BigInt::from_i128(i128::from(digit - b'0')));
        let mut q = b'0';
        while remainder >= *denominator {
            remainder.add_assign(&negated);
            q += 1;
        }
        digits.push(char::from(q));
    }
    BigInt::from_digits(&digits).unwrap_or_else(BigInt::zero)
}

/// The arithmetic the four operators share once an operand lies past the bounded
/// representation: the IEEE promotion when the other operand is an `xsd:float` or
/// `xsd:double`, otherwise the exact result projected into the bounded value space
/// (`exact`), every refusal typed.
fn mixed_arithmetic(
    a: &LiteralValue,
    b: &LiteralValue,
    bounded: fn(&XsdValue, &XsdValue) -> Result<XsdValue, XsdError>,
    exact: fn(&Exact, &Exact) -> Result<XsdValue, XsdError>,
) -> Result<XsdValue, XsdError> {
    if let (LiteralValue::Bounded(x), LiteralValue::Bounded(y)) = (a, b) {
        return bounded(x, y);
    }
    let ieee = |value: &LiteralValue| {
        matches!(
            value,
            LiteralValue::Bounded(XsdValue::Float(_) | XsdValue::Double(_))
        )
    };
    let promote = |value: &LiteralValue, target: &LiteralValue| {
        value
            .promoted(target.datatype())
            .ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand",
            })
    };
    if ieee(b) {
        return bounded(
            &promote(a, b)?,
            b.bounded().expect("an IEEE operand is bounded"),
        );
    }
    if ieee(a) {
        return bounded(
            a.bounded().expect("an IEEE operand is bounded"),
            &promote(b, a)?,
        );
    }
    match (Exact::of(a), Exact::of(b)) {
        (Some(x), Some(y)) => exact(&x, &y),
        _ => Err(XsdError::TypeMismatch {
            reason: "non-numeric operand",
        }),
    }
}

/// `op:numeric-add` over two literal values of any size ([`LiteralValue`]): exactly
/// [`crate::value_add`] for two bounded values (temporal operands included); with an
/// `xsd:integer`/`xsd:decimal` operand past the bounds, the IEEE sum against an
/// `xsd:float`/`xsd:double`, otherwise the exact sum when the bounded value space
/// holds it — truncated toward zero at the finest scale that holds a decimal one, the
/// precision rule of the crate docs' *Numeric limits* — and `err:FOAR0002` when its
/// integer part does not fit.
///
/// # Errors
///
/// As above, and [`XsdError::TypeMismatch`] for a non-numeric operand.
///
/// ```rust
/// use purrdf_xsd::{LiteralValue, XsdDatatype, literal_sub};
///
/// let big = LiteralValue::parse("100000000000000000000000000000000000000001", XsdDatatype::Integer)?;
/// let less = LiteralValue::parse("100000000000000000000000000000000000000000", XsdDatatype::Integer)?;
/// assert_eq!(literal_sub(&big, &less)?.canonical_lexical(), "1");
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn literal_add(a: &LiteralValue, b: &LiteralValue) -> Result<XsdValue, XsdError> {
    mixed_arithmetic(a, b, crate::ops::value_add, |x, y| {
        let scale = x.scale.max(y.scale);
        let mut sum = x.at_scale(scale);
        sum.add_assign(&y.at_scale(scale));
        Exact::bounded(&sum, scale, x.integer && y.integer)
    })
}

/// `op:numeric-subtract` over two literal values of any size, as [`literal_add`].
///
/// # Errors
///
/// As [`literal_add`].
pub fn literal_sub(a: &LiteralValue, b: &LiteralValue) -> Result<XsdValue, XsdError> {
    mixed_arithmetic(a, b, crate::ops::value_sub, |x, y| {
        let scale = x.scale.max(y.scale);
        let mut difference = x.at_scale(scale);
        difference.add_assign(&y.at_scale(scale).negated());
        Exact::bounded(&difference, scale, x.integer && y.integer)
    })
}

/// `op:numeric-multiply` over two literal values of any size, as [`literal_add`].
///
/// # Errors
///
/// As [`literal_add`].
pub fn literal_mul(a: &LiteralValue, b: &LiteralValue) -> Result<XsdValue, XsdError> {
    mixed_arithmetic(a, b, crate::ops::value_mul, |x, y| {
        Exact::bounded(
            &x.mantissa.mul(&y.mantissa),
            x.scale + y.scale,
            x.integer && y.integer,
        )
    })
}

/// `op:numeric-divide` over two literal values of any size: exactly
/// [`crate::value_div`] for two bounded values; with an `xsd:integer`/`xsd:decimal`
/// operand past the bounds, the exact quotient as one `xsd:decimal` under the
/// division precision rule — truncated toward zero at the finest scale ≤ 18 whose
/// mantissa fits, which is how `SUM(?v) / COUNT(?v)` over a total past the bounds
/// equals `AVG(?v)` — `err:FOAR0001` for a zero divisor and `err:FOAR0002` when the
/// quotient's integer part does not fit.
///
/// # Errors
///
/// As above, and [`XsdError::TypeMismatch`] for a non-numeric operand.
///
/// ```rust
/// use purrdf_xsd::{LiteralValue, XsdDatatype, literal_div};
///
/// // (i128::MAX + i128::MAX - 1) / 2 is MAX - 0.5, which truncates to MAX - 1.
/// let total = LiteralValue::parse("340282366920938463463374607431768211453", XsdDatatype::Integer)?;
/// let two = LiteralValue::parse("2", XsdDatatype::Integer)?;
/// assert_eq!(
///     literal_div(&total, &two)?.canonical_lexical(),
///     (i128::MAX - 1).to_string()
/// );
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn literal_div(a: &LiteralValue, b: &LiteralValue) -> Result<XsdValue, XsdError> {
    mixed_arithmetic(a, b, crate::ops::value_div, |x, y| {
        if y.mantissa.is_zero() {
            return Err(XsdError::DivisionByZero {
                datatype: if x.integer && y.integer {
                    XsdDatatype::Integer
                } else {
                    XsdDatatype::Decimal
                },
            });
        }
        // trunc(x / y × 10^18) = trunc(|xm| × 10^(ys + 18) / (|ym| × 10^xs)), signed.
        let target = u32::from(crate::numeric::MAX_DECIMAL_SCALE);
        let negative = x.mantissa.is_negative() != y.mantissa.is_negative();
        let magnitude = |value: &BigInt| {
            if value.is_negative() {
                value.negated()
            } else {
                value.clone()
            }
        };
        let numerator = magnitude(&x.mantissa).mul_pow10(y.scale + target);
        let denominator = magnitude(&y.mantissa).mul_pow10(x.scale);
        let scaled = quotient(&numerator, &denominator);
        let scaled = if negative { scaled.negated() } else { scaled };
        crate::numeric::decimal_mean(&scaled, target, 1).map(XsdValue::Decimal)
    })
}

/// One of the unary numeric functions over a literal value of any size: `bounded`
/// for a bounded value; for an `xsd:integer`/`xsd:decimal` value past the bounds,
/// `exact` maps the exact `(mantissa, scale)` to the exact result, which is projected
/// into the bounded value space with the operand's type (`xsd:integer` for the
/// integer family) or refused, `err:FOAR0002`.
fn unary(
    a: &LiteralValue,
    bounded: fn(&XsdValue) -> Result<XsdValue, XsdError>,
    exact: fn(&BigInt, u32) -> (BigInt, u32),
) -> Result<XsdValue, XsdError> {
    match a {
        LiteralValue::Bounded(value) => bounded(value),
        unbounded => {
            let x = Exact::of(unbounded).ok_or(XsdError::TypeMismatch {
                reason: "non-numeric operand",
            })?;
            let (mantissa, scale) = exact(&x.mantissa, x.scale);
            Exact::bounded(&mantissa, scale, x.integer)
        }
    }
}

/// `⌊mantissa / 10^scale⌋`, as a scale-0 mantissa.
fn floor_div_pow10(mantissa: &BigInt, scale: u32) -> BigInt {
    let unit = BigInt::from_i128(1).mul_pow10(scale);
    if mantissa.is_negative() {
        // ⌊−n / d⌋ = −⌈n / d⌉ = −⌊(n + d − 1) / d⌋.
        let mut shifted = mantissa.negated();
        shifted.add_assign(&unit);
        shifted.add_assign(&BigInt::from_i128(-1));
        quotient(&shifted, &unit).negated()
    } else {
        quotient(mantissa, &unit)
    }
}

/// `op:numeric-unary-minus` over a literal value of any size ([`literal_add`]'s
/// rules: exact, or typed).
///
/// # Errors
///
/// `err:FOAR0002` when the negation lies past the bounds; [`XsdError::TypeMismatch`]
/// for a non-numeric operand.
pub fn literal_unary_minus(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::ops::value_unary_minus, |m, s| (m.negated(), s))
}

/// `op:numeric-unary-plus` over a literal value of any size (the value itself, when
/// the bounded value space holds it).
///
/// # Errors
///
/// As [`literal_unary_minus`].
pub fn literal_unary_plus(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::numeric::numeric_unary_plus, |m, s| (m.clone(), s))
}

/// `fn:abs` over a literal value of any size.
///
/// # Errors
///
/// As [`literal_unary_minus`].
pub fn literal_abs(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::numeric::numeric_abs, |m, s| {
        (
            if m.is_negative() {
                m.negated()
            } else {
                m.clone()
            },
            s,
        )
    })
}

/// `fn:floor` over a literal value of any size: exact, so a decimal past eighteen
/// fractional digits floors to the integer the bounded value space holds.
///
/// # Errors
///
/// As [`literal_unary_minus`].
pub fn literal_floor(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::numeric::numeric_floor, |m, s| {
        (floor_div_pow10(m, s), 0)
    })
}

/// `fn:ceiling` over a literal value of any size, as [`literal_floor`].
///
/// # Errors
///
/// As [`literal_unary_minus`].
pub fn literal_ceil(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::numeric::numeric_ceil, |m, s| {
        (floor_div_pow10(&m.negated(), s).negated(), 0)
    })
}

/// `fn:round` over a literal value of any size (half toward positive infinity,
/// `⌊x + ½⌋`), as [`literal_floor`].
///
/// # Errors
///
/// As [`literal_unary_minus`].
pub fn literal_round(a: &LiteralValue) -> Result<XsdValue, XsdError> {
    unary(a, crate::numeric::numeric_round, |m, s| {
        // ⌊(2m + 10^s) / (2 · 10^s)⌋.
        let mut doubled = m.mul_small(2);
        doubled.add_assign(&BigInt::from_i128(1).mul_pow10(s));
        let unit = BigInt::from_i128(2).mul_pow10(s);
        let floored = if doubled.is_negative() {
            let mut shifted = doubled.negated();
            shifted.add_assign(&unit);
            shifted.add_assign(&BigInt::from_i128(-1));
            quotient(&shifted, &unit).negated()
        } else {
            quotient(&doubled, &unit)
        };
        (floored, 0)
    })
}
