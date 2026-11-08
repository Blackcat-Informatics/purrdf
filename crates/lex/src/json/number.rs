// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A JSON number as the text the document wrote (RFC 8259 §6), and the exact
//! decimal value that text denotes.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::RangeInclusive;
use core::str::FromStr;

use super::{Error, ErrorKind};
use crate::allocation::{Admission, Memory, Resident, StorageError};

/// A JSON number, held as its lexeme.
///
/// The lexeme always matches the RFC 8259 §6 grammar
///
/// ```text
/// number = [ "-" ] ( "0" / %x31-39 *DIGIT ) [ "." 1*DIGIT ] [ ( "e" / "E" ) [ "+" / "-" ] 1*DIGIT ]
/// ```
///
/// and nothing has rounded it: `1.5`, `1.50` and `15e-1` are three spellings.
///
/// # Equality
///
/// `==` and [`Hash`] compare the exact decimal value the lexeme denotes
/// ([`Number::decimal`]), never a rounded machine number: `1.5`, `1.50` and
/// `15e-1` are equal, `1`, `1.0`, `1e0` and `100e-2` are equal, `-0` equals
/// `0`, and `1e400` is a number like any other. There is no unequal-but-same
/// integer/float distinction (a JSON number is one kind of thing, RFC 8259
/// §6). A caller that needs the spelling itself, such as a byte-stable
/// round-trip check, asks [`Number::same_text`].
///
/// A reader that must decide a value exactly (an XSD decimal, a geometry
/// coordinate) reads [`Number::lexeme`] or [`Number::decimal`];
/// [`Number::as_u64`], [`Number::as_i64`], [`Number::as_i128`] and
/// [`Number::as_f64`] are the machine conversions.
#[derive(Clone)]
pub struct Number(String);

impl Number {
    pub(super) fn owned_text_bytes(&self) -> usize {
        self.0.capacity()
    }
}

/// The base-ten exponent of a [`Decimal`]: a machine word until the exponent
/// leaves `i64`, and its digits after that, so the representation is
/// canonical and derived equality and hashing are the value's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Exponent {
    /// An exponent inside `i64`.
    Small(i64),
    /// An exponent outside `i64`: its sign and its decimal magnitude, without
    /// leading zeros.
    Large {
        /// Whether the exponent is negative.
        negative: bool,
        /// The magnitude's ASCII digits.
        digits: String,
    },
}

impl Exponent {
    /// `[ "+" / "-" ] 1*DIGIT`, already matched by the JSON number grammar.
    fn from_written<S: Admission + ?Sized>(
        sign_and_digits: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let (negative, digits) = match sign_and_digits.as_bytes().first() {
            Some(b'-') => (true, &sign_and_digits[1..]),
            Some(b'+') => (false, &sign_and_digits[1..]),
            _ => (false, sign_and_digits),
        };
        Self::from_magnitude(negative, digits.trim_start_matches('0'), memory)
    }

    /// The exponent with sign `negative` and magnitude `digits` (no leading
    /// zeros; empty is zero).
    fn from_magnitude<S: Admission + ?Sized>(
        negative: bool,
        digits: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let signed = if negative && !digits.is_empty() {
            memory.format(&format_args!("-{digits}"))?
        } else {
            memory.string(digits)?
        };
        let result = match signed.parse::<i64>() {
            Err(_) => {
                if digits.is_empty() {
                    Self::Small(0)
                } else {
                    Self::Large {
                        negative,
                        digits: memory.string(digits)?,
                    }
                }
            }
            Ok(value) => Self::Small(value),
        };
        memory.release_string(signed)?;
        Ok(result)
    }

    /// `self + amount`, exactly.
    fn offset<S: Admission + ?Sized>(
        self,
        amount: i128,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        match self {
            Self::Small(value) => {
                let sum = i128::from(value).saturating_add(amount);
                if let Ok(value) = i64::try_from(sum) {
                    return Ok(Self::Small(value));
                }
                let digits = memory.format(&sum.unsigned_abs())?;
                let result = Self::from_magnitude(sum < 0, &digits, memory)?;
                memory.release_string(digits)?;
                Ok(result)
            }
            Self::Large { negative, digits } => {
                let step = memory.format(&amount.unsigned_abs())?;
                let adjusted = if negative == (amount < 0) {
                    add_digits(&digits, &step, memory)?
                } else {
                    // The exponent is outside `i64` and a step is a slice
                    // length, so the magnitude is the larger and stays
                    // positive; the sign is the exponent's.
                    subtract_digits(&digits, &step, memory)?
                };
                let result = Self::from_magnitude(negative, &adjusted, memory)?;
                memory.release_string(adjusted)?;
                memory.release_string(step)?;
                memory.release_string(digits)?;
                Ok(result)
            }
        }
    }
}

/// `left + right` over ASCII digit strings.
fn add_digits<S: Admission + ?Sized>(
    left: &str,
    right: &str,
    memory: &mut Memory<'_, S>,
) -> Result<String, StorageError> {
    let (long, short) = if left.len() >= right.len() {
        (left.as_bytes(), right.as_bytes())
    } else {
        (right.as_bytes(), left.as_bytes())
    };
    let mut out = Vec::new();
    memory.reserve(
        &mut out,
        long.len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?,
    )?;
    let mut carry = 0_u8;
    for (index, digit) in long.iter().rev().enumerate() {
        let other = short
            .len()
            .checked_sub(index + 1)
            .map_or(0, |at| short[at] - b'0');
        let sum = (digit - b'0') + other + carry;
        out.push(b'0' + sum % 10);
        carry = sum / 10;
    }
    if carry > 0 {
        out.push(b'0' + carry);
    }
    out.reverse();
    Ok(String::from_utf8(out).expect("ASCII digits"))
}

/// `left - right` over ASCII digit strings, `left >= right`, without leading
/// zeros.
fn subtract_digits<S: Admission + ?Sized>(
    left: &str,
    right: &str,
    memory: &mut Memory<'_, S>,
) -> Result<String, StorageError> {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut out = Vec::new();
    memory.reserve(&mut out, left.len())?;
    let mut borrow = 0_u8;
    for (index, digit) in left.iter().rev().enumerate() {
        let other = right
            .len()
            .checked_sub(index + 1)
            .map_or(0, |at| right[at] - b'0')
            + borrow;
        let minuend = digit - b'0';
        if minuend >= other {
            out.push(b'0' + minuend - other);
            borrow = 0;
        } else {
            out.push(b'0' + minuend + 10 - other);
            borrow = 1;
        }
    }
    while out.last() == Some(&b'0') {
        out.pop();
    }
    out.reverse();
    Ok(String::from_utf8(out).expect("ASCII digits"))
}

/// The exact value of a JSON number: `±coefficient × 10^exponent`, the
/// coefficient's leading and trailing zeros removed, or zero (`"0"`, exponent
/// `0`, not negative).
///
/// The form is canonical, so two lexemes that denote one number (`1`, `1.0`,
/// `10e-1`, `-0` and `0`) give equal values that hash alike. This is the
/// workspace's one reading of a JSON number's value: [`Number`] equality and
/// hashing, and the exact-arithmetic type built on it in `purrdf-xsd`.
///
/// ```rust
/// use purrdf_lex::json::Number;
///
/// let decimal = |text: &str| Number::from_lexeme(text).unwrap().decimal();
/// assert_eq!(decimal("1.0"), decimal("10e-1"));
/// assert_eq!(decimal("-0"), decimal("0"));
/// assert_eq!(decimal("1500").coefficient(), "15");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    negative: bool,
    coefficient: String,
    exponent: Exponent,
}

impl Decimal {
    /// Whether the value is negative (zero is not).
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// The significant ASCII digits, without leading or trailing zeros; `"0"`
    /// for zero.
    pub fn coefficient(&self) -> &str {
        &self.coefficient
    }

    /// The base-ten exponent of the coefficient; `0` for zero.
    pub const fn exponent(&self) -> &Exponent {
        &self.exponent
    }

    fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_string(self.coefficient)?;
        if let Exponent::Large { digits, .. } = self.exponent {
            memory.release_string(digits)?;
        }
        Ok(())
    }
}

/// The byte after the number that starts at `at`, or the offset and name of the
/// first byte the grammar refuses.
#[inline]
pub(crate) fn number_end(bytes: &[u8], at: usize) -> Result<usize, Error> {
    number_end_observed(bytes, at, &mut |_| Ok(()))
}

pub(crate) fn number_end_observed(
    bytes: &[u8],
    at: usize,
    progress: &mut impl FnMut(usize) -> Result<(), Error>,
) -> Result<usize, Error> {
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    let mut digits = |mut at: usize| -> Result<usize, Error> {
        let mut chunk = 0;
        while digit(at) {
            at += 1;
            chunk += 1;
            if chunk == 256 {
                progress(at)?;
                chunk = 0;
            }
        }
        progress(at)?;
        Ok(at)
    };
    let mut pos = at;
    if bytes.get(pos) == Some(&b'-') {
        pos += 1;
    }
    match bytes.get(pos) {
        Some(b'0') => {
            pos += 1;
            if digit(pos) {
                return Err(Error::new(
                    ErrorKind::Expected("no digit after a leading zero"),
                    pos,
                ));
            }
        }
        Some(b'1'..=b'9') => pos = digits(pos + 1)?,
        _ => return Err(Error::new(ErrorKind::Expected("a digit"), pos)),
    }
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        if !digit(pos) {
            return Err(Error::new(ErrorKind::Expected("a fraction digit"), pos));
        }
        pos = digits(pos)?;
    }
    if matches!(bytes.get(pos), Some(b'e' | b'E')) {
        pos += 1;
        if matches!(bytes.get(pos), Some(b'+' | b'-')) {
            pos += 1;
        }
        if !digit(pos) {
            return Err(Error::new(ErrorKind::Expected("an exponent digit"), pos));
        }
        pos = digits(pos)?;
    }
    Ok(pos)
}

impl Number {
    /// A number from a lexeme the caller vouches for; the reader's path, whose
    /// scan has already matched the grammar.
    pub(crate) const fn from_valid(lexeme: String) -> Self {
        Self(lexeme)
    }

    /// A number from its lexeme, refused unless the whole text matches the
    /// RFC 8259 §6 grammar.
    ///
    /// ```rust
    /// use purrdf_lex::json::Number;
    ///
    /// assert_eq!(Number::from_lexeme("-0.50e+3").unwrap().lexeme(), "-0.50e+3");
    /// assert!(Number::from_lexeme("01").is_err());
    /// assert!(Number::from_lexeme("0").is_ok());
    /// assert!(Number::from_lexeme("1.").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error`] at the first byte the grammar refuses.
    pub fn from_lexeme(lexeme: impl Into<String>) -> Result<Self, Error> {
        let lexeme = lexeme.into();
        let end = number_end(lexeme.as_bytes(), 0)?;
        if end != lexeme.len() {
            return Err(Error::new(ErrorKind::Trailing, end));
        }
        Ok(Self(lexeme))
    }

    /// The exact value the lexeme denotes; the reading `==` and [`Hash`]
    /// compare.
    ///
    /// ```rust
    /// use purrdf_lex::json::Number;
    ///
    /// let number = |text: &str| Number::from_lexeme(text).unwrap();
    /// assert_eq!(number("1e2"), number("100"));
    /// assert_eq!(number("1.50"), number("15e-1"));
    /// assert_ne!(number("1.5"), number("15"));
    /// assert!(!number("1e2").same_text(&number("100")));
    /// ```
    pub fn decimal(&self) -> Decimal {
        self.decimal_with_memory(&mut Memory::new(&mut Resident))
            .expect("resident JSON decimal allocation")
    }

    fn decimal_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Decimal, StorageError> {
        let body = self.0.strip_prefix('-');
        let negative = body.is_some();
        let body = body.unwrap_or(&self.0);
        let (mantissa, written) = body
            .split_once(['e', 'E'])
            .map_or((body, None), |(mantissa, written)| {
                (mantissa, Some(written))
            });
        let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let mut coefficient = String::new();
        memory.reserve_string(
            &mut coefficient,
            int.len()
                .checked_add(frac.len())
                .ok_or(StorageError::SizeOverflow)?,
        )?;
        coefficient.push_str(int);
        coefficient.push_str(frac);
        let leading = coefficient.len() - coefficient.trim_start_matches('0').len();
        if leading == coefficient.len() {
            memory.release_string(coefficient)?;
            return Ok(Decimal {
                negative: false,
                coefficient: memory.string("0")?,
                exponent: Exponent::Small(0),
            });
        }
        coefficient.drain(..leading);
        let significant = coefficient.trim_end_matches('0').len();
        let trailing = coefficient.len() - significant;
        coefficient.truncate(significant);
        // The value is `int frac × 10^(exponent − |frac|)`; each term of the
        // offset is a slice length, far inside `i128`.
        let shift = i128::try_from(trailing).unwrap_or(i128::MAX)
            - i128::try_from(frac.len()).unwrap_or(i128::MAX);
        Ok(Decimal {
            negative,
            coefficient,
            exponent: Exponent::from_written(written.unwrap_or("0"), memory)?
                .offset(shift, memory)?,
        })
    }

    pub(super) fn eq_with_memory<S: Admission + ?Sized>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<bool, StorageError> {
        if self.same_text(other) {
            return Ok(true);
        }
        memory.scope(|memory| {
            let left = self.decimal_with_memory(memory)?;
            let right = other.decimal_with_memory(memory)?;
            let equal = left == right;
            left.release(memory)?;
            right.release(memory)?;
            Ok(equal)
        })
    }

    pub(super) fn hash_with_memory<H: Hasher, S: Admission + ?Sized>(
        &self,
        state: &mut H,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.scope(|memory| {
            let decimal = self.decimal_with_memory(memory)?;
            decimal.hash(state);
            decimal.release(memory)
        })
    }

    /// Whether both numbers are spelled identically (`1.0` and `1` are not),
    /// for a caller that needs byte-stable identity rather than the value
    /// `==` compares.
    pub fn same_text(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// The lexeme, exactly as written.
    pub fn lexeme(&self) -> &str {
        &self.0
    }

    /// The lexeme, by value.
    pub fn into_lexeme(self) -> String {
        self.0
    }

    /// Whether the lexeme has neither a fraction nor an exponent.
    pub fn is_integer(&self) -> bool {
        !self.0.bytes().any(|b| matches!(b, b'.' | b'e' | b'E'))
    }

    /// The magnitude and sign of an integer lexeme, or `None` when the lexeme
    /// has a fraction or an exponent or does not fit `u128`.
    fn integer_parts(&self) -> Option<(bool, u128)> {
        if !self.is_integer() {
            return None;
        }
        let (negative, digits) = match self.0.strip_prefix('-') {
            Some(digits) => (true, digits),
            None => (false, self.0.as_str()),
        };
        let mut magnitude = 0_u128;
        for byte in digits.bytes() {
            magnitude = magnitude
                .checked_mul(10)?
                .checked_add(u128::from(byte - b'0'))?;
        }
        Some((negative, magnitude))
    }

    /// The value of an integer lexeme without a sign, when it fits `u64`.
    ///
    /// `None` for a fraction or exponent (`1.0`, `1e2`), for a negative lexeme
    /// (`-0` included), and past `u64::MAX`.
    pub fn as_u64(&self) -> Option<u64> {
        match self.integer_parts()? {
            (false, magnitude) => u64::try_from(magnitude).ok(),
            (true, _) => None,
        }
    }

    /// The value of an integer lexeme, when it fits `i64`. `-0` is `0`.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_i128().and_then(|value| i64::try_from(value).ok())
    }

    /// The value of an integer lexeme, when it fits `i128`. `-0` is `0`.
    pub fn as_i128(&self) -> Option<i128> {
        let (negative, magnitude) = self.integer_parts()?;
        if negative {
            if magnitude == 1_u128 << 127 {
                return Some(i128::MIN);
            }
            i128::try_from(magnitude).ok().map(|value| -value)
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// The nearest binary64 to the lexeme's decimal value, correctly rounded
    /// (round half to even). A lexeme beyond the binary64 range is `±inf`, and
    /// one below its smallest subnormal is `±0.0`; the lexeme is never refused,
    /// since every JSON number is a decimal the conversion defines.
    pub fn as_f64(&self) -> f64 {
        // The JSON number grammar is a subset of the grammar `f64::from_str`
        // accepts, and that conversion is correctly rounded.
        self.0
            .parse()
            .expect("a JSON number lexeme is a decimal floating-point literal")
    }

    /// The shortest lexeme that reads back as `value`, or `None` for NaN and
    /// the infinities, which JSON cannot spell.
    ///
    /// The digits are the shortest that round-trip through binary64. They are
    /// laid out as `serde_json` lays them out: positional notation, with `.0`
    /// after an integer, when the decimal exponent is in `-5..=15`, and
    /// otherwise one digit, the rest after a point, and a signed exponent.
    ///
    /// ```rust
    /// use purrdf_lex::json::Number;
    ///
    /// let spell = |value: f64| Number::from_f64(value).unwrap().into_lexeme();
    /// assert_eq!(spell(1.0), "1.0");
    /// assert_eq!(spell(-0.0), "-0.0");
    /// assert_eq!(spell(0.1), "0.1");
    /// assert_eq!(spell(1.5e-7), "1.5e-7");
    /// assert_eq!(spell(1e20), "1e+20");
    /// assert_eq!(spell(123_456.0), "123456.0");
    /// assert!(Number::from_f64(f64::NAN).is_none());
    /// ```
    pub fn from_f64(value: f64) -> Option<Self> {
        Self::from_shortest(value.is_finite(), value, -5..=15)
    }

    /// Construct the same shortest representation with every temporary and retained
    /// string admitted before allocation through the caller's account.
    /// # Errors
    /// Returns the original layout, admission or physical allocation refusal.
    pub fn from_f64_with_memory<S: Admission + ?Sized>(
        value: f64,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self>, StorageError> {
        Self::from_shortest_with_memory(value.is_finite(), value, -5..=15, memory)
    }

    /// [`Number::from_f64`] for a binary32: the shortest digits that round-trip
    /// through binary32, positional when the decimal exponent is in `-6..=12`.
    pub fn from_f32(value: f32) -> Option<Self> {
        Self::from_shortest(value.is_finite(), value, -6..=12)
    }

    /// The number of a finite binary float, from Rust's shortest round-trip
    /// digits (`{:e}`), positional when the decimal exponent is in `positional`;
    /// `None` when the float is not `finite`. Both widths spell through here, so
    /// they differ only in their digits and their window.
    fn from_shortest(
        finite: bool,
        value: impl fmt::LowerExp,
        positional: RangeInclusive<i32>,
    ) -> Option<Self> {
        Self::from_shortest_with_memory(finite, value, positional, &mut Memory::new(&mut Resident))
            .expect("resident JSON number formatting allocation")
    }

    fn from_shortest_with_memory<S: Admission + ?Sized>(
        finite: bool,
        value: impl fmt::LowerExp,
        positional: RangeInclusive<i32>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self>, StorageError> {
        if !finite {
            return Ok(None);
        }
        memory.scope(|memory| {
            let scientific = memory.format(&format_args!("{value:e}"))?;
            let result = layout(&scientific, positional, memory)?;
            memory.release_string(scientific)?;
            Ok(Some(Self(result)))
        })
    }
}

/// Lay out the shortest round-trip digits in `scientific` (Rust's `{:e}`
/// spelling: `[-]d[.ddd]e[-]x`), positionally when the decimal exponent is in
/// `positional`.
fn layout<S: Admission + ?Sized>(
    scientific: &str,
    positional: RangeInclusive<i32>,
    memory: &mut Memory<'_, S>,
) -> Result<String, StorageError> {
    let (negative, body) = match scientific.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, scientific),
    };
    let (mantissa, exponent) = body
        .split_once('e')
        .expect("the scientific spelling always writes an exponent");
    let exponent: i32 = exponent
        .parse()
        .expect("the scientific spelling writes a decimal exponent");
    let mut digits = String::new();
    for ch in mantissa.chars().filter(|&ch| ch != '.') {
        memory.push_char(&mut digits, ch)?;
    }
    let mut out = String::new();
    if negative {
        memory.push_char(&mut out, '-')?;
    }
    if digits == "0" {
        memory.push_str(&mut out, "0.0")?;
        memory.release_string(digits)?;
        return Ok(out);
    }
    let length = digits.len() as i32;
    if positional.contains(&exponent) {
        if length - 1 <= exponent {
            // 1234e7 -> 12340000000.0
            memory.push_str(&mut out, &digits)?;
            for _ in length..=exponent {
                memory.push_char(&mut out, '0')?;
            }
            memory.push_str(&mut out, ".0")?;
        } else if exponent >= 0 {
            // 1234e-2 -> 12.34
            let point = (exponent + 1) as usize;
            memory.push_str(&mut out, &digits[..point])?;
            memory.push_char(&mut out, '.')?;
            memory.push_str(&mut out, &digits[point..])?;
        } else {
            // 1234e-6 -> 0.001234
            memory.push_str(&mut out, "0.")?;
            for _ in 0..(-exponent - 1) {
                memory.push_char(&mut out, '0')?;
            }
            memory.push_str(&mut out, &digits)?;
        }
    } else {
        memory.push_str(&mut out, &digits[..1])?;
        if length > 1 {
            memory.push_char(&mut out, '.')?;
            memory.push_str(&mut out, &digits[1..])?;
        }
        memory.push_char(&mut out, 'e')?;
        memory.push_char(&mut out, if exponent < 0 { '-' } else { '+' })?;
        let exponent = memory.format(&exponent.unsigned_abs())?;
        memory.push_str(&mut out, &exponent)?;
        memory.release_string(exponent)?;
    }
    memory.release_string(digits)?;
    Ok(out)
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.eq_with_memory(other, &mut Memory::new(&mut Resident))
            .expect("resident JSON number equality allocation")
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The decimal form is canonical, so equal numbers hash alike.
        self.hash_with_memory(state, &mut Memory::new(&mut Resident))
            .expect("resident JSON number hashing allocation");
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Number({})", self.0)
    }
}

impl FromStr for Number {
    type Err = Error;

    fn from_str(lexeme: &str) -> Result<Self, Error> {
        Self::from_lexeme(lexeme)
    }
}

macro_rules! number_from_integer {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for Number {
            fn from(value: $t) -> Self {
                Self(value.to_string())
            }
        }
    )*};
}

number_from_integer!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
