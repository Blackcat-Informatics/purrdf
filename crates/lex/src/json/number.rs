// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A JSON number as the text the document wrote (RFC 8259 §6).

use core::fmt;
use core::ops::RangeInclusive;
use core::str::FromStr;

use super::{Error, ErrorKind};

/// A JSON number, held as its lexeme.
///
/// The lexeme always matches the RFC 8259 §6 grammar
///
/// ```text
/// number = [ "-" ] ( "0" / %x31-39 *DIGIT ) [ "." 1*DIGIT ] [ ( "e" / "E" ) [ "+" / "-" ] 1*DIGIT ]
/// ```
///
/// and nothing has decided what it denotes: `1.5`, `1.50` and `15e-1` are three
/// numbers, and equality compares lexemes. A reader that must decide a value
/// exactly (an XSD decimal, a geometry coordinate) reads [`Number::lexeme`];
/// [`Number::as_u64`], [`Number::as_i64`], [`Number::as_i128`] and
/// [`Number::as_f64`] are the machine conversions.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Number(String);

/// The byte after the number that starts at `at`, or the offset and name of the
/// first byte the grammar refuses.
pub(crate) fn number_end(bytes: &[u8], at: usize) -> Result<usize, Error> {
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    let digits = |mut at: usize| {
        while digit(at) {
            at += 1;
        }
        at
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
        Some(b'1'..=b'9') => pos = digits(pos + 1),
        _ => return Err(Error::new(ErrorKind::Expected("a digit"), pos)),
    }
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        if !digit(pos) {
            return Err(Error::new(ErrorKind::Expected("a fraction digit"), pos));
        }
        pos = digits(pos);
    }
    if matches!(bytes.get(pos), Some(b'e' | b'E')) {
        pos += 1;
        if matches!(bytes.get(pos), Some(b'+' | b'-')) {
            pos += 1;
        }
        if !digit(pos) {
            return Err(Error::new(ErrorKind::Expected("an exponent digit"), pos));
        }
        pos = digits(pos);
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
        finite.then(|| Self(layout(&format!("{value:e}"), positional)))
    }
}

/// Lay out the shortest round-trip digits in `scientific` (Rust's `{:e}`
/// spelling: `[-]d[.ddd]e[-]x`), positionally when the decimal exponent is in
/// `positional`.
fn layout(scientific: &str, positional: RangeInclusive<i32>) -> String {
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
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();
    let mut out = String::with_capacity(digits.len() + 8);
    if negative {
        out.push('-');
    }
    if digits == "0" {
        out.push_str("0.0");
        return out;
    }
    let length = digits.len() as i32;
    if positional.contains(&exponent) {
        if length - 1 <= exponent {
            // 1234e7 -> 12340000000.0
            out.push_str(&digits);
            for _ in length..=exponent {
                out.push('0');
            }
            out.push_str(".0");
        } else if exponent >= 0 {
            // 1234e-2 -> 12.34
            let point = (exponent + 1) as usize;
            out.push_str(&digits[..point]);
            out.push('.');
            out.push_str(&digits[point..]);
        } else {
            // 1234e-6 -> 0.001234
            out.push_str("0.");
            for _ in 0..(-exponent - 1) {
                out.push('0');
            }
            out.push_str(&digits);
        }
    } else {
        out.push_str(&digits[..1]);
        if length > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exponent < 0 { '-' } else { '+' });
        out.push_str(&exponent.unsigned_abs().to_string());
    }
    out
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
