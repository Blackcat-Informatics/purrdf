// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Numeric order over XSD lexical forms, as JSON Schema can state it.
//!
//! A numeric range bound (`sh:minInclusive`, …) compares a value's number with
//! the bound's (SHACL 1.2 Core §7.3, by the SPARQL operators and their numeric
//! type promotion). The projection (see [`crate::instance`]) carries an
//! `xsd:integer` in canonical form as a bare JSON integer — judged by
//! `minimum`/`maximum` — and every other numeric literal as a typed-literal
//! object whose `@value` is the lexical form, which only a `pattern` can judge.
//!
//! The set of `xsd:integer` or `xsd:decimal` lexical forms on one side of a
//! rational threshold is a regular language: strip leading zeros from the
//! integer part and trailing zeros from the fraction, then compare digit by
//! digit. [`order_pattern`] writes it as an anchored pattern in the grammar
//! Unicode ECMA-262 and the Rust engine share, around the `whiteSpace`
//! `collapse` trim every numeric datatype fixes.
//!
//! A bound of `xsd:double` or `xsd:float` promotes an integer or decimal value
//! to that type before comparing, rounding to nearest with ties to even; the
//! values whose rounding lies on one side of the bound are those on one side of
//! the midpoint between the bound and its neighbour, which is an exact binary —
//! so exactly decimal — fraction ([`Threshold::for_bound`]).

use std::cmp::Ordering;
use std::fmt::Write as _;

/// The `whiteSpace` `collapse` facet's trim (the four code points XSD names).
pub(super) const WS: &str = "[\\t\\n\\r ]*";

/// A pattern matching no string at all.
pub(super) const NOTHING: &str = "[^\\u{0}-\\u{10ffff}]";

/// A relation of a value to a threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rel {
    /// value > threshold
    Gt,
    /// value ≥ threshold
    Ge,
    /// value < threshold
    Lt,
    /// value ≤ threshold
    Le,
}

/// An exact decimal number: a sign, the integer digits without leading zeros
/// (empty for zero) and the fraction digits without trailing zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Decimal {
    negative: bool,
    int: String,
    frac: String,
}

impl Decimal {
    fn normalized(negative: bool, int: &str, frac: &str) -> Self {
        let int = int.trim_start_matches('0').to_owned();
        let frac = frac.trim_end_matches('0').to_owned();
        let zero = int.is_empty() && frac.is_empty();
        Self {
            negative: negative && !zero,
            int,
            frac,
        }
    }

    /// Parse an `xsd:decimal` lexical form (after the `collapse` trim), or an
    /// `xsd:integer` one when `integer` is set; `None` when it is not one.
    pub(super) fn parse(lexical: &str, integer: bool) -> Option<Self> {
        let lexical = lexical.trim_matches(['\t', '\n', '\r', ' ']);
        let (negative, body) = match lexical.as_bytes().first() {
            Some(b'-') => (true, &lexical[1..]),
            Some(b'+') => (false, &lexical[1..]),
            _ => (false, lexical),
        };
        let (int, frac) = body.split_once('.').unwrap_or((body, ""));
        let digits = |text: &str| text.bytes().all(|b| b.is_ascii_digit());
        let valid = digits(int)
            && digits(frac)
            && !(int.is_empty() && frac.is_empty())
            && (!integer || !body.contains('.'));
        valid.then(|| Self::normalized(negative, int, frac))
    }

    /// `numerator × 2^exponent`, exactly.
    fn from_binary(numerator: i128, exponent: i32) -> Self {
        let negative = numerator < 0;
        let mut digits = Digits::from_u128(numerator.unsigned_abs());
        if exponent >= 0 {
            for _ in 0..exponent {
                digits.mul_small(2);
            }
            Self::normalized(negative, &digits.to_string(), "")
        } else {
            let places = exponent.unsigned_abs() as usize;
            for _ in 0..places {
                digits.mul_small(5);
            }
            let text = digits.to_string();
            let text = format!("{}{text}", "0".repeat(places.saturating_sub(text.len())));
            let (int, frac) = text.split_at(text.len() - places);
            Self::normalized(negative, int, frac)
        }
    }

    /// The nearest `f64` (the decimal promoted to double).
    pub(super) fn to_f64(&self) -> f64 {
        let text = format!(
            "{}{}.{}",
            if self.negative { "-" } else { "" },
            if self.int.is_empty() { "0" } else { &self.int },
            if self.frac.is_empty() {
                "0"
            } else {
                &self.frac
            }
        );
        text.parse().unwrap_or(f64::NAN)
    }

    fn is_zero(&self) -> bool {
        self.int.is_empty() && self.frac.is_empty()
    }

    /// The magnitude with the sign cleared.
    fn magnitude(&self) -> Self {
        Self {
            negative: false,
            ..self.clone()
        }
    }

    /// The least integer ≥ self (`ceil`) or the greatest ≤ self, clamped
    /// well inside the `i128` range.
    fn to_integer(&self, ceil: bool) -> i128 {
        const LIMIT: i128 = i128::MAX / 4;
        let int: i128 = if self.int.len() > 36 {
            LIMIT
        } else if self.int.is_empty() {
            0
        } else {
            self.int.parse().unwrap_or(LIMIT)
        };
        let fractional = i128::from(!self.frac.is_empty());
        let value = match (self.negative, ceil) {
            (false, false) => int,
            (false, true) => int + fractional,
            (true, false) => -int - fractional,
            (true, true) => -int,
        };
        value.clamp(-LIMIT, LIMIT)
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let magnitude = |a: &Self, b: &Self| {
            a.int
                .len()
                .cmp(&b.int.len())
                .then_with(|| a.int.cmp(&b.int))
                .then_with(|| a.frac.cmp(&b.frac))
        };
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => magnitude(self, other),
            (true, true) => magnitude(other, self),
        }
    }
}

/// A non-negative big integer as little-endian base-10⁹ limbs.
struct Digits(Vec<u32>);

impl Digits {
    const BASE: u64 = 1_000_000_000;

    fn from_u128(mut value: u128) -> Self {
        let mut limbs = Vec::new();
        while value > 0 {
            limbs.push(u32::try_from(value % u128::from(Self::BASE)).expect("limb < 10^9"));
            value /= u128::from(Self::BASE);
        }
        Self(limbs)
    }

    fn mul_small(&mut self, factor: u64) {
        let mut carry = 0_u64;
        for limb in &mut self.0 {
            let product = u64::from(*limb) * factor + carry;
            *limb = u32::try_from(product % Self::BASE).expect("limb < 10^9");
            carry = product / Self::BASE;
        }
        while carry > 0 {
            self.0
                .push(u32::try_from(carry % Self::BASE).expect("limb < 10^9"));
            carry /= Self::BASE;
        }
    }
}

impl std::fmt::Display for Digits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some((last, rest)) = self.0.split_last() else {
            return f.write_str("0");
        };
        write!(f, "{last}")?;
        for limb in rest.iter().rev() {
            write!(f, "{limb:09}")?;
        }
        Ok(())
    }
}

/// Which values of an integer or decimal datatype satisfy one range bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Threshold {
    /// Every value.
    All,
    /// No value.
    None,
    /// The values in `Rel` to the decimal.
    Cmp(Rel, Decimal),
}

/// A range bound's number, as SPARQL compares it with an integer or decimal
/// value.
#[derive(Clone, Copy, Debug)]
pub(super) enum BoundNumber<'a> {
    /// An `xsd:integer`-family or `xsd:decimal` bound: exact.
    Exact(&'a Decimal),
    /// An `xsd:double` bound: the value is promoted to double.
    Double(f64),
    /// An `xsd:float` bound: the value is promoted to float.
    Float(f32),
}

/// The four range components, by the relation a conforming value bears to the
/// bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Facet {
    /// `sh:minInclusive`
    MinInclusive,
    /// `sh:minExclusive`
    MinExclusive,
    /// `sh:maxInclusive`
    MaxInclusive,
    /// `sh:maxExclusive`
    MaxExclusive,
}

impl Threshold {
    /// The integer and decimal values satisfying `facet` against `bound`.
    ///
    /// Against a double or float bound `B` a value `v` is compared as its
    /// rounding `r(v)` (XPath F&O 3.1 §19.1.2.1, round to nearest, ties to
    /// even). `r` is monotone, so `r(v) ≥ B` holds exactly for `v` above the
    /// midpoint `m` of `B` and its predecessor — and at `m` itself when the tie
    /// rounds to `B`, i.e. when `B`'s significand is even. The other facets
    /// follow the same way: `r(v) > B` is `r(v) ≥ succ(B)`, `r(v) ≤ B` is `v`
    /// below the midpoint of `B` and its successor (or at it, when `B` is even),
    /// `r(v) < B` is `r(v) ≤ pred(B)`. Infinities round in from the midpoint of
    /// the greatest finite value and the next power of two; a NaN bound
    /// compares with nothing.
    pub(super) fn for_bound(facet: Facet, bound: BoundNumber<'_>) -> Self {
        match bound {
            BoundNumber::Exact(decimal) => {
                let rel = match facet {
                    Facet::MinInclusive => Rel::Ge,
                    Facet::MinExclusive => Rel::Gt,
                    Facet::MaxInclusive => Rel::Le,
                    Facet::MaxExclusive => Rel::Lt,
                };
                Self::Cmp(rel, decimal.clone())
            }
            BoundNumber::Double(value) => Self::ieee(facet, &Ieee::double(value)),
            BoundNumber::Float(value) => Self::ieee(facet, &Ieee::float(value)),
        }
    }

    fn ieee(facet: Facet, format: &Ieee) -> Self {
        let Some(bound) = format.point else {
            return Self::None;
        };
        // `r(v) ≥ x`, for a representable (or infinite) `x`.
        let at_least = |x: Point| -> Self {
            if x == format.negative_infinity() {
                return Self::All;
            }
            let pred = format.pred(x);
            let rel = if format.even(x) { Rel::Ge } else { Rel::Gt };
            Self::Cmp(rel, format.midpoint(pred, x))
        };
        // `r(v) ≤ x`.
        let at_most = |x: Point| -> Self {
            if x == format.positive_infinity() {
                return Self::All;
            }
            let succ = format.succ(x);
            let rel = if format.even(x) { Rel::Le } else { Rel::Lt };
            Self::Cmp(rel, format.midpoint(x, succ))
        };
        match facet {
            Facet::MinInclusive => at_least(bound),
            Facet::MaxInclusive => at_most(bound),
            Facet::MinExclusive if bound == format.positive_infinity() => Self::None,
            Facet::MinExclusive => at_least(format.succ(bound)),
            Facet::MaxExclusive if bound == format.negative_infinity() => Self::None,
            Facet::MaxExclusive => at_most(format.pred(bound)),
        }
    }

    /// The same set, over the integers alone: `(least, greatest)`, each `None`
    /// when unbounded; `None` overall when no integer is in it.
    pub(super) fn integer_range(&self) -> Option<(Option<i128>, Option<i128>)> {
        match self {
            Self::All => Some((None, None)),
            Self::None => None,
            Self::Cmp(rel, decimal) => Some(match rel {
                Rel::Ge => (Some(decimal.to_integer(true)), None),
                Rel::Gt => (Some(decimal.to_integer(false) + 1), None),
                Rel::Le => (None, Some(decimal.to_integer(false))),
                Rel::Lt => (None, Some(decimal.to_integer(true) - 1)),
            }),
        }
    }
}

/// A point of an IEEE binary format: the ordinal of a value in the ordered
/// sequence of representable values (−∞ … −0 … +∞, with −0 and +0 one
/// point), so neighbours differ by one.
type Point = i64;

/// An IEEE binary interchange format and one bound in it.
struct Ieee {
    /// Significand bits, the implicit one included.
    precision: u32,
    /// The exponent of the least subnormal, `2^min_exponent`.
    min_exponent: i32,
    /// The ordinal of the greatest finite value.
    max_ordinal: Point,
    /// The bound's ordinal, `None` for NaN.
    point: Option<Point>,
}

impl Ieee {
    fn double(value: f64) -> Self {
        Self::new(53, -1074, value.is_nan(), || {
            let bits = value.abs().to_bits();
            i64::try_from(bits).expect("an absolute value's bits fit i64")
                * if value.is_sign_negative() && value != 0.0 {
                    -1
                } else {
                    1
                }
        })
    }

    fn float(value: f32) -> Self {
        Self::new(24, -149, value.is_nan(), || {
            i64::from(value.abs().to_bits())
                * if value.is_sign_negative() && value != 0.0 {
                    -1
                } else {
                    1
                }
        })
    }

    fn new(precision: u32, min_exponent: i32, nan: bool, ordinal: impl FnOnce() -> Point) -> Self {
        // The bits of +∞ are one past those of the greatest finite value; the
        // magnitude's bit pattern is its ordinal among non-negative values.
        let exponent_bits = if precision == 53 { 11 } else { 8 };
        let infinity: Point = ((1_i64 << exponent_bits) - 1) << (precision - 1);
        Self {
            precision,
            min_exponent,
            max_ordinal: infinity - 1,
            point: (!nan).then(ordinal),
        }
    }

    const fn positive_infinity(&self) -> Point {
        self.max_ordinal + 1
    }

    const fn negative_infinity(&self) -> Point {
        -(self.max_ordinal + 1)
    }

    const fn succ(&self, x: Point) -> Point {
        x + 1
    }

    const fn pred(&self, x: Point) -> Point {
        x - 1
    }

    /// Whether the significand of `x` is even (zero and the infinities count
    /// as even: a tie beside them rounds to them).
    fn even(&self, x: Point) -> bool {
        x.unsigned_abs() > self.max_ordinal.unsigned_abs() || x.unsigned_abs().is_multiple_of(2)
    }

    /// `x` as `numerator × 2^exponent`; the infinities are the next power of
    /// two past the greatest finite value.
    fn exact(&self, x: Point) -> (i128, i32) {
        let magnitude = x.unsigned_abs();
        let fraction_bits = self.precision - 1;
        let biased = magnitude >> fraction_bits;
        let fraction = magnitude & ((1 << fraction_bits) - 1);
        let (significand, exponent) = if biased == 0 {
            (fraction, self.min_exponent)
        } else {
            (
                fraction | (1 << fraction_bits),
                self.min_exponent + i32::try_from(biased).expect("small exponent") - 1,
            )
        };
        let significand = i128::from(significand);
        (if x < 0 { -significand } else { significand }, exponent)
    }

    /// The exact midpoint of two adjacent points.
    fn midpoint(&self, low: Point, high: Point) -> Decimal {
        let (a, ea) = self.exact(low);
        let (b, eb) = self.exact(high);
        let exponent = ea.min(eb);
        let sum = (a << (ea - exponent)) + (b << (eb - exponent));
        Decimal::from_binary(sum, exponent - 1)
    }
}

/// An anchored pattern matching exactly the `xsd:decimal` lexical forms (or,
/// with `integer`, the `xsd:integer` ones) — around the `collapse` trim —
/// whose value is in `threshold`.
pub(super) fn order_pattern(threshold: &Threshold, integer: bool) -> String {
    match order_body(threshold, integer) {
        Some(body) => format!("^{WS}(?:{body}){WS}$"),
        None => NOTHING.to_owned(),
    }
}

/// The unanchored alternatives of [`order_pattern`], without the trim; `None`
/// when no lexical form is in `threshold`.
pub(super) fn order_body(threshold: &Threshold, integer: bool) -> Option<String> {
    match threshold {
        Threshold::All => signed(&[(Sign::Any, Magnitude::Any)], integer),
        Threshold::None => None,
        Threshold::Cmp(rel, bound) => {
            let b = bound.magnitude();
            let arms: Vec<(Sign, Magnitude)> = match (rel, bound.negative, bound.is_zero()) {
                (Rel::Gt, false, _) => vec![(Sign::Plus, Magnitude::Cmp(Rel::Gt, b))],
                (Rel::Gt, true, _) => vec![
                    (Sign::Plus, Magnitude::Any),
                    (Sign::Minus, Magnitude::Cmp(Rel::Lt, b)),
                ],
                (Rel::Ge, false, false) => vec![(Sign::Plus, Magnitude::Cmp(Rel::Ge, b))],
                (Rel::Ge, false, true) => {
                    vec![(Sign::Plus, Magnitude::Any), (Sign::Minus, Magnitude::Zero)]
                }
                (Rel::Ge, true, _) => vec![
                    (Sign::Plus, Magnitude::Any),
                    (Sign::Minus, Magnitude::Cmp(Rel::Le, b)),
                ],
                (Rel::Lt, false, false) => vec![
                    (Sign::Minus, Magnitude::Any),
                    (Sign::Plus, Magnitude::Cmp(Rel::Lt, b)),
                ],
                (Rel::Lt, _, _) => vec![(Sign::Minus, Magnitude::Cmp(Rel::Gt, b))],
                (Rel::Le, false, false) => vec![
                    (Sign::Minus, Magnitude::Any),
                    (Sign::Plus, Magnitude::Cmp(Rel::Le, b)),
                ],
                (Rel::Le, false, true) => {
                    vec![(Sign::Minus, Magnitude::Any), (Sign::Plus, Magnitude::Zero)]
                }
                (Rel::Le, true, _) => vec![(Sign::Minus, Magnitude::Cmp(Rel::Ge, b))],
            };
            signed(&arms, integer)
        }
    }
}

#[derive(Clone, Copy)]
enum Sign {
    /// No sign or `+`.
    Plus,
    /// `-`.
    Minus,
    /// Any sign.
    Any,
}

enum Magnitude {
    Any,
    Zero,
    Cmp(Rel, Decimal),
}

fn signed(arms: &[(Sign, Magnitude)], integer: bool) -> Option<String> {
    let alternatives: Vec<String> = arms
        .iter()
        .filter_map(|(sign, magnitude)| {
            let magnitude = magnitude_pattern(magnitude, integer)?;
            Some(match sign {
                Sign::Plus => format!("\\+?(?:{magnitude})"),
                Sign::Minus => format!("-(?:{magnitude})"),
                Sign::Any => format!("[+\\-]?(?:{magnitude})"),
            })
        })
        .collect();
    (!alternatives.is_empty()).then(|| alternatives.join("|"))
}

/// Which stripped integer parts `i` (no leading zeros; empty for zero) a
/// condition admits: zero, and the non-empty ones as pattern alternatives.
struct IntPart {
    zero: bool,
    positive: Vec<String>,
}

/// Which raw fraction digit strings (trailing zeros included, possibly empty)
/// a condition admits, as alternatives; `""` stands for the empty fraction.
pub(super) type FracPart = Vec<String>;

const ANY_POSITIVE: &str = "[1-9][0-9]*";

fn digit_class(low: u8, high: u8) -> Option<String> {
    match low.cmp(&high) {
        Ordering::Greater => None,
        Ordering::Equal => Some(char::from(b'0' + low).to_string()),
        Ordering::Less => Some(format!(
            "[{}-{}]",
            char::from(b'0' + low),
            char::from(b'0' + high)
        )),
    }
}

fn digit(c: u8) -> u8 {
    c - b'0'
}

/// `i > I`, `i == I`, `i < I` over stripped integer parts.
fn int_part(rel: Option<Rel>, bound: &str) -> IntPart {
    let len = bound.len();
    let bytes = bound.as_bytes();
    match rel {
        None => IntPart {
            zero: bound.is_empty(),
            positive: if bound.is_empty() {
                Vec::new()
            } else {
                vec![bound.to_owned()]
            },
        },
        Some(Rel::Gt) => {
            let mut positive = vec![if len == 0 {
                ANY_POSITIVE.to_owned()
            } else {
                format!("[1-9][0-9]{{{len},}}")
            }];
            for k in 0..len {
                if let Some(class) = digit_class(digit(bytes[k]) + 1, 9) {
                    positive.push(format!("{}{class}[0-9]{{{}}}", &bound[..k], len - k - 1));
                }
            }
            IntPart {
                zero: false,
                positive,
            }
        }
        Some(Rel::Lt) => {
            if len == 0 {
                return IntPart {
                    zero: false,
                    positive: Vec::new(),
                };
            }
            let mut positive = Vec::new();
            if len >= 2 {
                positive.push(format!("[1-9][0-9]{{0,{}}}", len - 2));
            }
            for k in 0..len {
                let low = u8::from(k == 0);
                if digit(bytes[k]) > 0
                    && let Some(class) = digit_class(low, digit(bytes[k]) - 1)
                {
                    positive.push(format!("{}{class}[0-9]{{{}}}", &bound[..k], len - k - 1));
                }
            }
            IntPart {
                zero: true,
                positive,
            }
        }
        Some(Rel::Ge | Rel::Le) => unreachable!("strict or equal only"),
    }
}

/// `R > F`, `R == F`, `R < F` over raw fraction digit strings `R`, against the
/// trailing-zero-free `F`.
pub(super) fn frac_part(rel: Option<Rel>, bound: &str) -> FracPart {
    let bytes = bound.as_bytes();
    match rel {
        None => {
            if bound.is_empty() {
                vec![String::new(), "0+".to_owned()]
            } else {
                vec![format!("{bound}0*")]
            }
        }
        Some(Rel::Gt) => {
            let mut alternatives = vec![format!("{bound}0*[1-9][0-9]*")];
            for (k, &byte) in bytes.iter().enumerate() {
                if let Some(class) = digit_class(digit(byte) + 1, 9) {
                    alternatives.push(format!("{}{class}[0-9]*", &bound[..k]));
                }
            }
            alternatives
        }
        Some(Rel::Lt) => {
            let mut alternatives = Vec::new();
            for (k, &byte) in bytes.iter().enumerate() {
                alternatives.push(bound[..k].to_owned());
                if digit(byte) > 0
                    && let Some(class) = digit_class(0, digit(byte) - 1)
                {
                    alternatives.push(format!("{}{class}[0-9]*", &bound[..k]));
                }
            }
            alternatives
        }
        Some(Rel::Ge | Rel::Le) => unreachable!("strict or equal only"),
    }
}

fn any_frac() -> FracPart {
    vec![String::new(), "[0-9]+".to_owned()]
}

fn any_int() -> IntPart {
    IntPart {
        zero: true,
        positive: vec![ANY_POSITIVE.to_owned()],
    }
}

fn magnitude_pattern(magnitude: &Magnitude, integer: bool) -> Option<String> {
    let pairs: Vec<(IntPart, FracPart)> = match magnitude {
        Magnitude::Any => vec![(any_int(), any_frac())],
        Magnitude::Zero => vec![(int_part(None, ""), frac_part(None, ""))],
        Magnitude::Cmp(rel, bound) => {
            let (i, f) = (bound.int.as_str(), bound.frac.as_str());
            let strict = match rel {
                Rel::Gt | Rel::Ge => Rel::Gt,
                Rel::Lt | Rel::Le => Rel::Lt,
            };
            let mut pairs = vec![
                (int_part(Some(strict), i), any_frac()),
                (int_part(None, i), frac_part(Some(strict), f)),
            ];
            if matches!(rel, Rel::Ge | Rel::Le) {
                pairs.push((int_part(None, i), frac_part(None, f)));
            }
            pairs
        }
    };
    let mut alternatives: Vec<String> = Vec::new();
    for (int, frac) in &pairs {
        render_pair(int, frac, integer, &mut alternatives);
    }
    alternatives.dedup();
    (!alternatives.is_empty()).then(|| alternatives.join("|"))
}

fn group(alternatives: &[&String]) -> String {
    let mut out = String::from("(?:");
    for (index, alternative) in alternatives.iter().enumerate() {
        if index > 0 {
            out.push('|');
        }
        out.push_str(alternative);
    }
    out.push(')');
    out
}

/// The lexical forms `[0-9]+`, `[0-9]+\.[0-9]*` and `\.[0-9]+` whose integer
/// and fraction parts meet `int` and `frac`.
fn render_pair(int: &IntPart, frac: &FracPart, integer: bool, out: &mut Vec<String>) {
    let empty_fraction = frac.iter().any(String::is_empty);
    let fractions: Vec<&String> = frac.iter().collect();
    let nonempty: Vec<&String> = frac.iter().filter(|f| !f.is_empty()).collect();
    let positive: Vec<&String> = int.positive.iter().collect();
    if int.zero {
        if empty_fraction {
            out.push("0+".to_owned());
        }
        if !integer && !fractions.is_empty() {
            out.push(format!("0+\\.{}", group(&fractions)));
            if !nonempty.is_empty() {
                out.push(format!("\\.{}", group(&nonempty)));
            }
        }
    }
    if !positive.is_empty() {
        let positive = group(&positive);
        if empty_fraction {
            out.push(format!("0*{positive}"));
        }
        if !integer && !fractions.is_empty() {
            let mut line = String::new();
            write!(line, "0*{positive}\\.{}", group(&fractions)).expect("String write");
            out.push(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, input: &str) -> bool {
        assert!(
            purrdf_core::xsd_regex::ecma_262_rust_compatible(pattern),
            "{pattern}"
        );
        regex::Regex::new(pattern)
            .expect("pattern compiles")
            .is_match(input)
    }

    /// Every decimal lexical form of up to four characters over a small
    /// alphabet, against thresholds on both sides of zero, agrees with the
    /// exact comparison.
    #[test]
    fn order_patterns_agree_with_exact_comparison() {
        let alphabet = ['0', '1', '5', '9', '.', '-', '+'];
        let mut inputs = vec![String::new()];
        let mut frontier = vec![String::new()];
        for _ in 0..4 {
            let mut next = Vec::new();
            for prefix in &frontier {
                for c in alphabet {
                    next.push(format!("{prefix}{c}"));
                }
            }
            inputs.extend(next.iter().cloned());
            frontier = next;
        }
        inputs.extend([" 15 ".to_owned(), "\t-0.5\n".to_owned()]);
        let bounds = [
            "0", "-0", "1", "-1", "0.5", "-0.5", "15", "9.9", "-15.05", "100", "0.05",
        ];
        for bound in bounds {
            let bound = Decimal::parse(bound, false).expect("bound");
            for rel in [Rel::Gt, Rel::Ge, Rel::Lt, Rel::Le] {
                for integer in [false, true] {
                    let pattern = order_pattern(&Threshold::Cmp(rel, bound.clone()), integer);
                    for input in &inputs {
                        let expected = Decimal::parse(input, integer).is_some_and(|value| {
                            let ordering = value.cmp(&bound);
                            match rel {
                                Rel::Gt => ordering.is_gt(),
                                Rel::Ge => ordering.is_ge(),
                                Rel::Lt => ordering.is_lt(),
                                Rel::Le => ordering.is_le(),
                            }
                        });
                        assert_eq!(
                            matches(&pattern, input),
                            expected,
                            "{input:?} {rel:?} {bound:?} integer={integer}: {pattern}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn double_bounds_compare_through_rounding() {
        // 0.1 is not a double: the double nearest it is slightly larger, so an
        // exact decimal 0.1 promoted to double equals it, and ≥ holds.
        let threshold = Threshold::for_bound(Facet::MinInclusive, BoundNumber::Double(0.1));
        let pattern = order_pattern(&threshold, false);
        assert!(matches(&pattern, "0.1"));
        assert!(!matches(&pattern, "0.09999999999999999"));
        // Integers beyond 2^53 round: 2^53 + 1 rounds (to even) to 2^53.
        let threshold = Threshold::for_bound(
            Facet::MaxInclusive,
            BoundNumber::Double(9_007_199_254_740_992.0),
        );
        let pattern = order_pattern(&threshold, true);
        assert!(matches(&pattern, "9007199254740993"));
        // 2^53 + 2 is representable, and larger.
        assert!(!matches(&pattern, "9007199254740994"));
        assert_eq!(
            threshold.integer_range(),
            Some((None, Some(9_007_199_254_740_993)))
        );
        // Infinities and NaN.
        assert_eq!(
            Threshold::for_bound(Facet::MaxInclusive, BoundNumber::Double(f64::INFINITY)),
            Threshold::All
        );
        assert_eq!(
            Threshold::for_bound(Facet::MinExclusive, BoundNumber::Double(f64::INFINITY)),
            Threshold::None
        );
        assert_eq!(
            Threshold::for_bound(Facet::MinInclusive, BoundNumber::Double(f64::NAN)),
            Threshold::None
        );
        let huge = order_pattern(
            &Threshold::for_bound(Facet::MinInclusive, BoundNumber::Double(f64::INFINITY)),
            true,
        );
        assert!(!matches(&huge, &"9".repeat(308)));
        assert!(matches(&huge, &"9".repeat(310)));
        // A float bound promotes to float: below 2^24, a value rounds under it
        // exactly when it lies under the midpoint 16777215.5.
        let threshold = Threshold::for_bound(Facet::MaxExclusive, BoundNumber::Float(16_777_216.0));
        assert_eq!(threshold.integer_range(), Some((None, Some(16_777_215))));
    }

    #[test]
    fn integer_ranges_round_toward_the_admitted_side() {
        let decimal = |text| Decimal::parse(text, false).expect("decimal");
        assert_eq!(
            Threshold::Cmp(Rel::Ge, decimal("2.5")).integer_range(),
            Some((Some(3), None))
        );
        assert_eq!(
            Threshold::Cmp(Rel::Gt, decimal("2")).integer_range(),
            Some((Some(3), None))
        );
        assert_eq!(
            Threshold::Cmp(Rel::Gt, decimal("-2.5")).integer_range(),
            Some((Some(-2), None))
        );
        assert_eq!(
            Threshold::Cmp(Rel::Le, decimal("-2.5")).integer_range(),
            Some((None, Some(-3)))
        );
        assert_eq!(
            Threshold::Cmp(Rel::Lt, decimal("-2")).integer_range(),
            Some((None, Some(-3)))
        );
        assert_eq!(
            Threshold::Cmp(Rel::Lt, decimal("2.5")).integer_range(),
            Some((None, Some(2)))
        );
    }
}
