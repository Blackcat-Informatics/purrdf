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
