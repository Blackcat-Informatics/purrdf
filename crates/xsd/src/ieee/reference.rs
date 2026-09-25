// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Binary64 and binary32 arithmetic in integers: an exact result, rounded once.
//!
//! The oracle every correctly rounded operation in this workspace is tested against. It
//! uses no floating-point unit for any finite result: each operation's exact value (or
//! enough of it, with a sticky bit for the rest, far below every rounding position used
//! here) is computed in integers and rounded to nearest-even into the format, subnormals
//! included. The same code rounds into any other format, so it also models what the x87
//! does when it rounds twice -- into its register's format and then into the stored one.
//! On a target whose unit is IEEE the comparison shows the reference is right; on the x87
//! it shows the scopes make the unit right.
//!
//! `#[doc(hidden)]`: shipped, because the tests of the crates that compute with
//! [`super`]'s operations are separate compilation units that can only reach shared code
//! through this library, but not public API. No shipping code path calls it. Every
//! operand that is a NaN or an infinity, or a zero divisor, is answered by the unit
//! itself: those results are fixed by IEEE-754 without any rounding.

use core::cmp::Ordering;

/// An exact finite value, `(−1)^neg · sig · 2^exp`.
#[derive(Debug, Clone, Copy)]
struct Exact {
    neg: bool,
    sig: u128,
    exp: i32,
}

/// A binary floating-point format with round-to-nearest-even: its significand width and
/// the exponent of its smallest normal (the lowest exponent a leading bit keeps full
/// precision at). Its exponent range is unbounded above; overflow is decided when the
/// value is encoded as binary64 or binary32.
#[derive(Debug, Clone, Copy)]
pub struct Format {
    precision: u32,
    emin: i32,
}

/// IEEE-754 binary64.
pub const BINARY64: Format = Format {
    precision: 53,
    emin: -1022,
};
/// IEEE-754 binary32.
pub const BINARY32: Format = Format {
    precision: 24,
    emin: -126,
};
/// The x87's register format at its power-on precision: a 64-bit significand, a 15-bit
/// exponent.
pub const X87_EXTENDED: Format = Format {
    precision: 64,
    emin: -16382,
};
/// The x87's register format with precision control at 53 bits: binary64's significand
/// over the 15-bit exponent.
pub const X87_DOUBLE: Format = Format {
    precision: 53,
    emin: -16382,
};
/// The x87's register format with precision control at 24 bits: binary32's significand
/// over the 15-bit exponent.
pub const X87_SINGLE: Format = Format {
    precision: 24,
    emin: -16382,
};

/// `x`, finite, with a normalized 53-bit significand when it is not zero.
fn decompose(x: f64) -> Exact {
    let bits = x.to_bits();
    let neg = bits >> 63 == 1;
    let biased = i32::try_from((bits >> 52) & 0x7ff).expect("eleven bits");
    let fraction = u128::from(bits & ((1 << 52) - 1));
    let (mut sig, mut exp) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    };
    if sig != 0 {
        while sig < 1 << 52 {
            sig <<= 1;
            exp -= 1;
        }
    }
    Exact { neg, sig, exp }
}

fn bit_length(sig: u128) -> i32 {
    i32::try_from(128 - sig.leading_zeros()).expect("at most 128")
}

/// `value` rounded once, to nearest-even, into `format`.
fn round(value: Exact, format: Format) -> Exact {
    if value.sig == 0 {
        return value;
    }
    let leading = value.exp + bit_length(value.sig) - 1;
    let precision = i32::try_from(format.precision).expect("small");
    let lsb = leading.max(format.emin) - (precision - 1);
    if lsb <= value.exp {
        return value;
    }
    let shift = u32::try_from(lsb - value.exp).expect("positive");
    let kept = match shift.cmp(&128) {
        Ordering::Greater => 0,
        // The half-unit is 2^127; only a significand above it rounds up to one unit.
        Ordering::Equal => u128::from(value.sig > 1 << 127),
        Ordering::Less => {
            let kept = value.sig >> shift;
            let rest = value.sig & ((1 << shift) - 1);
            let half = 1 << (shift - 1);
            if rest > half || (rest == half && kept & 1 == 1) {
                kept + 1
            } else {
                kept
            }
        }
    };
    Exact {
        neg: value.neg,
        sig: kept,
        exp: lsb,
    }
}

/// The encoding of an IEEE interchange format: significand bits stored (without the
/// implicit bit), the smallest normal exponent, the largest finite one.
struct Encoding {
    fraction_bits: i32,
    emin: i32,
    emax: i32,
}

const ENCODE_64: Encoding = Encoding {
    fraction_bits: 52,
    emin: -1022,
    emax: 1023,
};
const ENCODE_32: Encoding = Encoding {
    fraction_bits: 23,
    emin: -126,
    emax: 127,
};

/// A value already on the format's grid, as the format's bits without the sign, or
/// `None` past its range.
fn encode(value: Exact, encoding: &Encoding) -> Option<u64> {
    if value.sig == 0 {
        return Some(0);
    }
    let leading = value.exp + bit_length(value.sig) - 1;
    if leading > encoding.emax {
        return None;
    }
    let (unit, biased) = if leading >= encoding.emin {
        (
            leading - encoding.fraction_bits,
            u64::try_from(leading - encoding.emin + 1).expect("positive"),
        )
    } else {
        (encoding.emin - encoding.fraction_bits, 0)
    };
    let shift = value.exp - unit;
    let sig = if shift >= 0 {
        value.sig << shift
    } else {
        let right = shift.unsigned_abs();
        assert_eq!(value.sig & ((1 << right) - 1), 0, "a value on the grid");
        value.sig >> right
    };
    let sig = u64::try_from(sig).expect("at most 53 bits");
    let implicit = 1_u64 << encoding.fraction_bits;
    let fraction = if biased == 0 { sig } else { sig - implicit };
    Some((biased << encoding.fraction_bits) | fraction)
}

/// A value on binary64's grid, as binary64 (an infinity past its range).
fn to_f64(value: Exact) -> f64 {
    let sign = u64::from(value.neg) << 63;
    let magnitude = encode(value, &ENCODE_64).unwrap_or_else(|| f64::INFINITY.to_bits());
    f64::from_bits(sign | magnitude)
}

/// A value on binary32's grid, as binary32 (an infinity past its range).
fn to_f32(value: Exact) -> f32 {
    let sign = u32::from(value.neg) << 31;
    let magnitude = encode(value, &ENCODE_32).map_or_else(
        || f32::INFINITY.to_bits(),
        |bits| u32::try_from(bits).expect("thirty-one bits"),
    );
    f32::from_bits(sign | magnitude)
}

/// The exact sum of two finite values, with the sticky bit of anything lost far below
/// every rounding position this module uses.
fn exact_sum(a: f64, b: f64) -> Exact {
    let (x, y) = (decompose(a), decompose(b));
    if x.sig == 0 && y.sig == 0 {
        return Exact {
            neg: x.neg && y.neg,
            sig: 0,
            exp: 0,
        };
    }
    if x.sig == 0 {
        return y;
    }
    if y.sig == 0 {
        return x;
    }
    let (high, low) = if x.exp >= y.exp { (x, y) } else { (y, x) };
    let gap = high.exp - low.exp;
    // 74 bits of headroom keeps a 53-bit significand within 127 bits.
    let (high_sig, low_sig, exp) = if gap <= 74 {
        (high.sig << gap, low.sig, low.exp)
    } else {
        let right = gap - 74;
        let low_sig = if right >= 128 {
            1
        } else {
            let right = right.unsigned_abs();
            (low.sig >> right) | u128::from(low.sig & ((1 << right) - 1) != 0)
        };
        (high.sig << 74, low_sig, high.exp - 74)
    };
    if high.neg == low.neg {
        return Exact {
            neg: high.neg,
            sig: high_sig + low_sig,
            exp,
        };
    }
    match high_sig.cmp(&low_sig) {
        Ordering::Equal => Exact {
            neg: false,
            sig: 0,
            exp,
        },
        Ordering::Greater => Exact {
            neg: high.neg,
            sig: high_sig - low_sig,
            exp,
        },
        Ordering::Less => Exact {
            neg: low.neg,
            sig: low_sig - high_sig,
            exp,
        },
    }
}

fn exact_product(a: f64, b: f64) -> Exact {
    let (x, y) = (decompose(a), decompose(b));
    Exact {
        neg: x.neg != y.neg,
        sig: x.sig * y.sig,
        exp: x.exp + y.exp,
    }
}

/// The quotient to 73 bits or more, with a sticky bit for a remainder.
fn exact_quotient(a: f64, b: f64) -> Exact {
    let (x, y) = (decompose(a), decompose(b));
    let neg = x.neg != y.neg;
    if x.sig == 0 {
        return Exact {
            neg,
            sig: 0,
            exp: 0,
        };
    }
    let dividend = x.sig << 74;
    let quotient = dividend / y.sig;
    let sticky = u128::from(dividend % y.sig != 0);
    Exact {
        neg,
        sig: (quotient << 1) | sticky,
        exp: x.exp - 74 - y.exp - 1,
    }
}

/// The square root of a positive value to 72 bits, with a sticky bit: enough to round
/// into the x87's 64-bit register format as well as into binary64.
fn exact_root(a: f64) -> Exact {
    /// Bits the digit recurrence adds to the 62 `isqrt` gives.
    const EXTRA: i32 = 10;
    let x = decompose(a);
    let (mut sig, mut exp) = (x.sig, x.exp);
    if exp % 2 != 0 {
        sig <<= 1;
        exp -= 1;
    }
    let radicand = sig << 70;
    let mut root = radicand.isqrt();
    let mut remainder = radicand - root * root;
    // The restoring square-root recurrence: with `root = ⌊√N⌋` and `remainder = N −
    // root²`, the root of `4N` is `2·root + 1` exactly when `(2·root + 1)² ≤ 4N`, that is
    // when `4·remainder ≥ 4·root + 1`. The remainder stays below `2·root + 1`.
    for _ in 0..EXTRA {
        let trial = (root << 2) | 1;
        let widened = remainder << 2;
        if widened >= trial {
            root = (root << 1) | 1;
            remainder = widened - trial;
        } else {
            root <<= 1;
            remainder = widened;
        }
    }
    let sticky = u128::from(remainder != 0);
    Exact {
        neg: false,
        sig: (root << 1) | sticky,
        exp: (exp - 70) / 2 - EXTRA - 1,
    }
}

/// `exact` rounded into `first`, then into binary64: one rounding when `first` is
/// binary64, and the x87's two when it is one of the register formats.
fn through(exact: Exact, first: Format) -> f64 {
    to_f64(round(round(exact, first), BINARY64))
}

/// `exact` rounded into `first`, then into binary32.
fn through32(exact: Exact, first: Format) -> f32 {
    to_f32(round(round(exact, first), BINARY32))
}

// ---- binary64 ----------------------------------------------------------------------------

/// `a + b` rounded into `first`, then binary64.
#[must_use]
pub fn add_via(a: f64, b: f64, first: Format) -> f64 {
    if !a.is_finite() || !b.is_finite() {
        return a + b;
    }
    through(exact_sum(a, b), first)
}

/// `a × b` rounded into `first`, then binary64.
#[must_use]
pub fn mul_via(a: f64, b: f64, first: Format) -> f64 {
    if !a.is_finite() || !b.is_finite() {
        return a * b;
    }
    through(exact_product(a, b), first)
}

/// `a ÷ b` rounded into `first`, then binary64.
#[must_use]
pub fn div_via(a: f64, b: f64, first: Format) -> f64 {
    if !a.is_finite() || !b.is_finite() || b == 0.0 {
        return a / b;
    }
    through(exact_quotient(a, b), first)
}

/// `√a` rounded into `first`, then binary64.
#[must_use]
pub fn sqrt_via(a: f64, first: Format) -> f64 {
    if !a.is_finite() || a <= 0.0 {
        return a.sqrt();
    }
    through(exact_root(a), first)
}

/// `a + b`, correctly rounded.
#[must_use]
pub fn add(a: f64, b: f64) -> f64 {
    add_via(a, b, BINARY64)
}

/// `a − b`, correctly rounded.
#[must_use]
pub fn sub(a: f64, b: f64) -> f64 {
    add_via(a, -b, BINARY64)
}

/// `a × b`, correctly rounded.
#[must_use]
pub fn mul(a: f64, b: f64) -> f64 {
    mul_via(a, b, BINARY64)
}

/// `a ÷ b`, correctly rounded.
#[must_use]
pub fn div(a: f64, b: f64) -> f64 {
    div_via(a, b, BINARY64)
}

/// `√a`, correctly rounded.
#[must_use]
pub fn sqrt(a: f64) -> f64 {
    sqrt_via(a, BINARY64)
}

// ---- binary32 ----------------------------------------------------------------------------
//
// Every binary32 value is a binary64 value exactly, so the exact results above serve
// binary32 operands unchanged; only the final rounding differs.

/// `a + b` rounded into `first`, then binary32.
#[must_use]
pub fn add32_via(a: f32, b: f32, first: Format) -> f32 {
    if !a.is_finite() || !b.is_finite() {
        return a + b;
    }
    through32(exact_sum(f64::from(a), f64::from(b)), first)
}

/// `a × b` rounded into `first`, then binary32.
#[must_use]
pub fn mul32_via(a: f32, b: f32, first: Format) -> f32 {
    if !a.is_finite() || !b.is_finite() {
        return a * b;
    }
    through32(exact_product(f64::from(a), f64::from(b)), first)
}

/// `a ÷ b` rounded into `first`, then binary32.
#[must_use]
pub fn div32_via(a: f32, b: f32, first: Format) -> f32 {
    if !a.is_finite() || !b.is_finite() || b == 0.0 {
        return a / b;
    }
    through32(exact_quotient(f64::from(a), f64::from(b)), first)
}

/// `√a` rounded into `first`, then binary32.
#[must_use]
pub fn sqrt32_via(a: f32, first: Format) -> f32 {
    if !a.is_finite() || a <= 0.0 {
        return a.sqrt();
    }
    through32(exact_root(f64::from(a)), first)
}

/// The binary64 quotient `a ÷ b` rounded into `first` and then narrowed to binary32: with
/// `first` binary64, the law of a binary64 division whose result is stored as binary32;
/// with the x87's register format, what the x87 computes when the quotient is narrowed
/// straight from its register.
#[must_use]
pub fn div64_to32_via(a: f64, b: f64, first: Format) -> f32 {
    if !a.is_finite() || !b.is_finite() || b == 0.0 {
        return (a / b) as f32;
    }
    through32(exact_quotient(a, b), first)
}

/// `a + b` in binary32, correctly rounded.
#[must_use]
pub fn add32(a: f32, b: f32) -> f32 {
    add32_via(a, b, BINARY32)
}

/// `a − b` in binary32, correctly rounded.
#[must_use]
pub fn sub32(a: f32, b: f32) -> f32 {
    add32_via(a, -b, BINARY32)
}

/// `a × b` in binary32, correctly rounded.
#[must_use]
pub fn mul32(a: f32, b: f32) -> f32 {
    mul32_via(a, b, BINARY32)
}

/// `a ÷ b` in binary32, correctly rounded.
#[must_use]
pub fn div32(a: f32, b: f32) -> f32 {
    div32_via(a, b, BINARY32)
}

/// `√a` in binary32, correctly rounded.
#[must_use]
pub fn sqrt32(a: f32) -> f32 {
    sqrt32_via(a, BINARY32)
}

// ---- decimal to binary64 -----------------------------------------------------------------
//
// The oracle for the readers that turn a JSON number into an `f64`: what a reader that
// scales a `u64` significand by a power of ten computes, and the exact decimal
// expansions of the midpoints a correctly rounded reader must decide by every digit.

/// `n` rounded once to binary64.
#[must_use]
pub fn u64_to_f64(n: u64) -> f64 {
    to_f64(round(
        Exact {
            neg: false,
            sig: u128::from(n),
            exp: 0,
        },
        BINARY64,
    ))
}

/// `10^k` rounded once to binary64, `k ≤ 308`: the literal `1e{k}`.
fn power_of_ten(k: u32) -> f64 {
    format!("1e{k}")
        .parse()
        .expect("a decimal power of ten parses")
}

/// `significand · 10 + digit`, or `None` when it overflows a `u64`.
fn push_digit(significand: u64, digit: u8) -> Option<u64> {
    significand
        .checked_mul(10)
        .and_then(|scaled| scaled.checked_add(u64::from(digit - b'0')))
}

/// A JSON number as a significand-times-power reader scans it.
struct Scanned {
    negative: bool,
    /// The leading digits, as many as fit a `u64`.
    significand: u64,
    /// The decimal exponent of the significand's last digit.
    exponent: i32,
    /// Whether a digit was left out of the significand.
    truncated: bool,
    /// Whether the written exponent overflowed an `i32`: `Some(positive)`.
    exponent_overflow: Option<bool>,
}

/// Scan `lexical` the way `serde_json` does, or `None` when it is not a JSON number.
fn scan(lexical: &str) -> Option<Scanned> {
    let bytes = lexical.as_bytes();
    let negative = bytes.first() == Some(&b'-');
    let mut at = usize::from(negative);
    let mut scanned = Scanned {
        negative,
        significand: 0,
        exponent: 0,
        truncated: false,
        exponent_overflow: None,
    };

    let integer_start = at;
    while let Some(&digit @ b'0'..=b'9') = bytes.get(at) {
        at += 1;
        if scanned.truncated {
            scanned.exponent += 1;
        } else if let Some(next) = push_digit(scanned.significand, digit) {
            scanned.significand = next;
        } else {
            scanned.truncated = true;
            scanned.exponent += 1;
        }
    }
    let integer_digits = at - integer_start;
    if integer_digits == 0 || (integer_digits > 1 && bytes[integer_start] == b'0') {
        return None;
    }

    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let fraction_start = at;
        // After the integer part overflowed, a fraction digit is still pushed when it
        // happens to fit: `serde_json` resumes its ordinary decimal loop there.
        let mut fraction_overflowed = false;
        while let Some(&digit @ b'0'..=b'9') = bytes.get(at) {
            at += 1;
            if fraction_overflowed {
                continue;
            }
            if let Some(next) = push_digit(scanned.significand, digit) {
                scanned.significand = next;
                scanned.exponent -= 1;
            } else {
                fraction_overflowed = true;
                scanned.truncated = true;
            }
        }
        if at == fraction_start {
            return None;
        }
    }

    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        let positive = match bytes.get(at) {
            Some(b'-') => {
                at += 1;
                false
            }
            Some(b'+') => {
                at += 1;
                true
            }
            _ => true,
        };
        let digits_start = at;
        let mut written = Some(0_i32);
        while let Some(&digit @ b'0'..=b'9') = bytes.get(at) {
            at += 1;
            written = written
                .and_then(|value| value.checked_mul(10))
                .and_then(|value| value.checked_add(i32::from(digit - b'0')));
        }
        if at == digits_start {
            return None;
        }
        match written {
            Some(written) if positive => {
                scanned.exponent = scanned.exponent.saturating_add(written);
            }
            Some(written) => scanned.exponent = scanned.exponent.saturating_sub(written),
            None => scanned.exponent_overflow = Some(positive),
        }
    }
    (at == bytes.len()).then_some(scanned)
}

/// The binary64 value a significand-times-power-of-ten reader gives the JSON number
/// `lexical`, or `None` for a malformed number or one that reader refuses as out of
/// range.
///
/// The reader accumulates the decimal digits into a `u64` until the next one would
/// overflow it; after that it drops every further fraction digit, and counts every
/// further integer digit into the decimal exponent. It converts the significand to
/// binary64, then multiplies or divides it by the binary64 nearest `10^|e|` (below
/// `10^-308` it first divides by `1e308` as often as it must), each step rounded once.
/// Every step is rounded, so the result is not the correctly rounded value of the
/// number: one decimal can land on a neighbour of the binary64 nearest it. This is
/// `serde_json`'s conversion without its `float_roundtrip` feature
/// (`Deserializer::f64_from_parts`, reached through `parse_decimal`, `parse_exponent`,
/// `parse_decimal_overflow` and `parse_long_integer`), computed here in integers so it
/// has one result on every target; the tests of the JSON readers search it for the
/// numbers on which such a reader and a correctly rounded one disagree.
#[must_use]
pub fn significand_power_decimal(lexical: &str) -> Option<f64> {
    let scanned = scan(lexical)?;
    let signed = |value: f64| if scanned.negative { -value } else { value };
    if let Some(positive) = scanned.exponent_overflow {
        // An exponent past `i32`: out of range above, zero below.
        return (scanned.significand == 0 || !positive).then_some(signed(0.0));
    }
    let mut exponent = scanned.exponent;
    let mut value = u64_to_f64(scanned.significand);
    loop {
        let magnitude = exponent.unsigned_abs();
        if magnitude <= 308 {
            let power = power_of_ten(magnitude);
            if exponent >= 0 {
                value = mul(value, power);
                if value.is_infinite() {
                    return None;
                }
            } else {
                value = div(value, power);
            }
            break;
        }
        if value == 0.0 {
            break;
        }
        if exponent >= 0 {
            return None;
        }
        value = div(value, power_of_ten(308));
        exponent += 308;
    }
    Some(signed(value))
}

/// Whether a correctly rounded reader's exact fast path, run on the x87 at its
/// power-on 64-bit precision, misreads `lexical`.
///
/// A number whose digits fit a significand below `2^53` and whose decimal exponent is at
/// most 22 in magnitude is the product or quotient of two binary64 values (the
/// significand and `10^|e|`), and a reader computes it with that one operation (for an
/// exponent up to 37, shifting the excess into the significand first when it still fits).
/// Rounded once that is the correctly rounded value; through the x87's register it is
/// rounded to 64 bits and again to 53, and this answers whether the two differ. That is
/// the fast path of `serde_json`'s `float_roundtrip` reader (`lexical::algorithm::
/// fast_path`) and of core's `dec2flt`, which sets the precision control around it.
#[must_use]
pub fn x87_fast_path_misreads(lexical: &str) -> bool {
    let Some(scanned) = scan(lexical) else {
        return false;
    };
    if scanned.truncated
        || scanned.exponent_overflow.is_some()
        || scanned.significand == 0
        || scanned.significand >> 53 != 0
    {
        return false;
    }
    let (significand, exponent) = if scanned.exponent > 22 {
        let Some(shifted) = u32::try_from(scanned.exponent - 22)
            .ok()
            .and_then(|shift| 10_u64.checked_pow(shift))
            .and_then(|scale| scanned.significand.checked_mul(scale))
            .filter(|shifted| shifted >> 53 == 0)
        else {
            return false;
        };
        (shifted, 22)
    } else {
        (scanned.significand, scanned.exponent)
    };
    if exponent == 0 || exponent < -22 {
        return false;
    }
    let (a, b) = (
        u64_to_f64(significand),
        power_of_ten(exponent.unsigned_abs()),
    );
    if exponent > 0 {
        mul(a, b).to_bits() != mul_via(a, b, X87_EXTENDED).to_bits()
    } else {
        div(a, b).to_bits() != div_via(a, b, X87_EXTENDED).to_bits()
    }
}

/// The exact decimal expansion of the midpoint between the positive finite binary64
/// `x` and its successor: the number a correctly rounded reader can only decide by
/// reading every one of its digits (up to 767 significant ones), and whose tie it breaks
/// to the even neighbour.
///
/// # Panics
///
/// When `x` is not positive and finite.
#[must_use]
pub fn successor_midpoint_decimal(x: f64) -> String {
    assert!(x.is_finite() && x > 0.0, "a positive finite binary64");
    let bits = x.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1 << 52) - 1);
    let (significand, exponent) = if biased == 0 {
        (fraction, -1074_i32)
    } else {
        (
            fraction | (1 << 52),
            i32::try_from(biased).expect("eleven bits") - 1075,
        )
    };
    // The midpoint is (2·significand + 1) · 2^(exponent − 1): an odd integer times a
    // power of two, which is an integer (a power of two ≥ 1) or an integer over 10^k
    // (2^−k = 5^k / 10^k).
    let mut digits: Vec<u8> = (2 * significand + 1)
        .to_string()
        .bytes()
        .rev()
        .map(|digit| digit - b'0')
        .collect();
    let scale = exponent - 1;
    let multiply = |digits: &mut Vec<u8>, factor: u8| {
        let mut carry = 0_u8;
        for digit in digits.iter_mut() {
            let product = *digit * factor + carry;
            *digit = product % 10;
            carry = product / 10;
        }
        if carry != 0 {
            digits.push(carry);
        }
    };
    let point = if scale >= 0 {
        for _ in 0..scale {
            multiply(&mut digits, 2);
        }
        0
    } else {
        for _ in 0..scale.unsigned_abs() {
            multiply(&mut digits, 5);
        }
        usize::try_from(scale.unsigned_abs()).expect("small")
    };
    while digits.len() <= point {
        digits.push(0);
    }
    let mut text = String::with_capacity(digits.len() + 1);
    for (index, digit) in digits.iter().enumerate().rev() {
        text.push(char::from(b'0' + digit));
        if index == point && point != 0 {
            text.push('.');
        }
    }
    text
}

/// A SplitMix64 stream: deterministic draws, no clock and no RNG.
struct Draws(u64);

impl Draws {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A positive finite binary64 with a decimal exponent in `[−300, 300]`.
    fn value(&mut self) -> f64 {
        let exponent = self.next() % 1_994 + 26; // biased 26..=2019: 2^−997 ..< 2^997
        f64::from_bits((exponent << 52) | (self.next() & ((1 << 52) - 1)))
    }
}

/// `count` pairs of a binary64 value and its spelling `spell(value)` that
/// [`significand_power_decimal`] misreads, drawn deterministically: the values whose
/// shortest, serialized or otherwise produced form a significand-times-power reader would
/// not give back. A draw `spell` answers `None` for is skipped, so a reader with a
/// narrower range than binary64's can ask only for values it accepts.
///
/// # Panics
///
/// When `spell` does not spell its value (a correctly rounded read of it is another
/// value), or the draws run out before `count` are found.
#[must_use]
pub fn misread_spellings(
    count: usize,
    spell: impl Fn(f64) -> Option<String>,
) -> Vec<(f64, String)> {
    let mut draws = Draws(0x6d69_7372_6561_6473);
    let mut found = Vec::with_capacity(count);
    for _ in 0..count.saturating_mul(1_000) {
        if found.len() == count {
            break;
        }
        let value = draws.value();
        let Some(lexical) = spell(value) else {
            continue;
        };
        assert_eq!(
            lexical.parse::<f64>().map(f64::to_bits),
            Ok(value.to_bits()),
            "`{lexical}` must spell {value:e}"
        );
        if significand_power_decimal(&lexical).map(f64::to_bits) != Some(value.to_bits()) {
            found.push((value, lexical));
        }
    }
    assert_eq!(found.len(), count, "misread spellings found");
    found
}

/// `count` pairs of a binary64 value and its spelling `spell(value)` that a correctly
/// rounded reader's exact fast path misreads on the x87 at 64-bit precision
/// ([`x87_fast_path_misreads`]), drawn deterministically from the values that path can
/// serve (`2^-80 ≤ value < 2^123`). A draw `spell` answers `None` for is skipped.
///
/// # Panics
///
/// When `spell` does not spell its value, or the draws run out before `count` are found.
#[must_use]
pub fn x87_misread_spellings(
    count: usize,
    spell: impl Fn(f64) -> Option<String>,
) -> Vec<(f64, String)> {
    let mut draws = Draws(0x7838_3773_7065_6c6c);
    let mut found = Vec::with_capacity(count);
    for _ in 0..count.saturating_mul(1_000_000) {
        if found.len() == count {
            break;
        }
        let exponent = draws.next() % 203 + 943; // biased 943..=1145: 2^-80 ..< 2^123
        let value = f64::from_bits((exponent << 52) | (draws.next() & ((1 << 52) - 1)));
        let Some(lexical) = spell(value) else {
            continue;
        };
        assert_eq!(
            lexical.parse::<f64>().map(f64::to_bits),
            Ok(value.to_bits()),
            "`{lexical}` must spell {value:e}"
        );
        if x87_fast_path_misreads(&lexical) {
            found.push((value, lexical));
        }
    }
    assert_eq!(found.len(), count, "x87 misread spellings found");
    found
}

/// `count` decimal numbers of 17 to 30 significant digits that
/// [`significand_power_decimal`] misreads, drawn deterministically: long mantissas, the
/// digits past the nineteenth among them, which a correctly rounded reader must weigh.
///
/// # Panics
///
/// When the draws run out before `count` are found.
#[must_use]
pub fn misread_decimals(count: usize) -> Vec<String> {
    let mut draws = Draws(0x6c6f_6e67_6469_6773);
    let mut found = Vec::with_capacity(count);
    for _ in 0..count.saturating_mul(1_000) {
        if found.len() == count {
            break;
        }
        let digits = usize::try_from(draws.next() % 14).expect("small") + 17;
        let mut mantissa = String::with_capacity(digits + 1);
        mantissa.push(char::from(
            b'1' + u8::try_from(draws.next() % 9).expect("digit"),
        ));
        mantissa.push('.');
        for _ in 1..digits {
            mantissa.push(char::from(
                b'0' + u8::try_from(draws.next() % 10).expect("digit"),
            ));
        }
        let exponent = i64::try_from(draws.next() % 601).expect("small") - 300;
        let lexical = format!("{mantissa}e{exponent}");
        let correct = lexical.parse::<f64>().expect("a decimal");
        if significand_power_decimal(&lexical).map(f64::to_bits) != Some(correct.to_bits()) {
            found.push(lexical);
        }
    }
    assert_eq!(found.len(), count, "misread decimals found");
    found
}

/// `count` decimal numbers `m·10^±k` with `m < 2^53` and `k ≤ 22` -- the ones whose value
/// is a single binary64 product or quotient of two exact binary64 values, which a reader's
/// exact fast path computes with one operation -- on which that operation rounded first
/// to the x87's 64-bit register significand and then to binary64 gives another value
/// than rounding once. They are read right on the x87 only under a 53-bit precision
/// control.
///
/// # Panics
///
/// When the draws run out before `count` are found.
#[must_use]
pub fn x87_fast_path_decimals(count: usize) -> Vec<String> {
    let mut draws = Draws(0x7838_375f_6661_7374);
    let mut found = Vec::with_capacity(count);
    for _ in 0..count.saturating_mul(100_000) {
        if found.len() == count {
            break;
        }
        let significand = draws.next() >> 11;
        let power = draws.next() % 22 + 1;
        let sign = if draws.next() & 1 == 1 { "-" } else { "" };
        let lexical = format!("{significand}e{sign}{power}");
        if x87_fast_path_misreads(&lexical) {
            found.push(lexical);
        }
    }
    assert_eq!(found.len(), count, "x87 fast-path decimals found");
    found
}
