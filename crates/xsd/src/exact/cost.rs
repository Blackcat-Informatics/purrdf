// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resource governance: what an exact operation will cost, known before it runs.
//!
//! An unbounded number is a denial-of-service vector — `10^(10^9)` is a short
//! query and a gigabyte of digits — so every operation of the tower has a cost
//! method (`Integer::mul_cost`, `Decimal::div_cost`, …) returning a [`Cost`]
//! computed from the operands' sizes alone, in constant time, without touching
//! their digits. A governor charges the estimate and refuses the operation
//! before it allocates.
//!
//! # Units
//!
//! * [`Cost::work`] is an upper bound on the base-`1e9` limb operations (one
//!   machine multiply-add, compare or division step on a 32-bit digit group) the
//!   operation performs: linear for addition, subtraction, comparison, parsing,
//!   rendering and decimal shifts; at most the product of the operand lengths
//!   for multiplication (Karatsuba stays below it from
//!   [`crate::bigint::KARATSUBA_THRESHOLD`] limbs up); quotient length times
//!   divisor length for division; quadratic in the result for powers.
//! * [`Cost::bytes`] is an upper bound on the heap bytes of the value the
//!   operation produces — the memory it mints and hands back.
//!
//! Both are deterministic functions of the operand sizes, so the same query
//! over the same data charges the same amount on every run and every target.

/// The estimated cost of one exact operation; see the module docs for the units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cost {
    work: u64,
    bytes: u64,
}

impl Cost {
    /// No cost.
    pub const ZERO: Self = Self { work: 0, bytes: 0 };

    /// A cost of `work` limb operations producing `bytes` heap bytes.
    #[must_use]
    pub const fn new(work: u64, bytes: u64) -> Self {
        Self { work, bytes }
    }

    /// The upper bound on limb operations.
    #[must_use]
    pub const fn work(self) -> u64 {
        self.work
    }

    /// The upper bound on heap bytes the result holds.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// The cost of doing both, saturating at `u64::MAX`.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self {
            work: self.work.saturating_add(other.work),
            bytes: self.bytes.saturating_add(other.bytes),
        }
    }

    /// The larger of the two in each unit.
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        Self {
            work: if self.work > other.work {
                self.work
            } else {
                other.work
            },
            bytes: if self.bytes > other.bytes {
                self.bytes
            } else {
                other.bytes
            },
        }
    }
}

/// The cost of parsing a lexical form of `len` bytes into any exact value.
#[must_use]
pub const fn parse(len: u64) -> Cost {
    Cost::new(len.saturating_add(1), limb_bytes(len / 9 + 1))
}

/// The cost of rendering a value of `limbs` limbs (plus `scale` placed fractional
/// digits for a decimal) as its canonical lexical form.
#[must_use]
pub const fn render(limbs: u64, scale: u64) -> Cost {
    let digits = limbs
        .saturating_mul(9)
        .saturating_add(scale)
        .saturating_add(3);
    Cost::new(digits, digits)
}

/// Heap bytes of `limbs` base-`1e9` limbs.
#[must_use]
pub const fn limb_bytes(limbs: u64) -> u64 {
    limbs.saturating_mul(4)
}

/// Limbs holding `digits` decimal digits.
#[must_use]
pub(crate) const fn limbs_for_digits(digits: u64) -> u64 {
    digits / 9 + 1
}

/// `a ± b` over `la`- and `lb`-limb magnitudes.
#[must_use]
pub(crate) const fn add(la: u64, lb: u64) -> Cost {
    let result = if la > lb { la } else { lb }.saturating_add(1);
    Cost::new(result, limb_bytes(result))
}

/// `a × b`. Karatsuba's recursion holds its half-size sums and partial products
/// beside the result along one path of the recursion — a geometric series under
/// the top level's — so its working set is a constant multiple of the result's.
#[must_use]
pub(crate) const fn mul(la: u64, lb: u64) -> Cost {
    let result = la.saturating_add(lb);
    let work = la
        .saturating_mul(lb)
        .saturating_add(result.saturating_mul(8))
        .saturating_add(1);
    let shorter = if la < lb { la } else { lb };
    let working = if shorter >= crate::bigint::KARATSUBA_THRESHOLD as u64 {
        8
    } else {
        2
    };
    Cost::new(work, limb_bytes(result).saturating_mul(working))
}

/// `a ÷ b` with remainder.
#[must_use]
pub(crate) const fn div(la: u64, lb: u64) -> Cost {
    let quotient = la.saturating_sub(lb).saturating_add(1);
    let work = quotient
        .saturating_mul(lb.saturating_add(1))
        .saturating_mul(4)
        .saturating_add(la)
        .saturating_add(1);
    Cost::new(work, limb_bytes(la.saturating_add(lb).saturating_add(2)))
}

/// `log2(1e9)` in Q16 fixed point, rounded up and down.
pub(crate) const LOG2_LIMB_Q16_UP: u64 = 1_959_355;
const LOG2_LIMB_Q16_DOWN: u64 = 1_959_354;

/// An upper bound on `log2(m)`, `m ≥ 1`, in Q16 fixed point, at most two
/// units (`2^-15`) above the true logarithm, so a power's size estimate is
/// within a few bits of the result even at exponents in the millions.
///
/// The integer part is the leading bit; the sixteen fractional bits are read off
/// by repeated squaring of the normalized significand `x ∈ [1, 2)` (each square
/// that reaches 2 is a one bit, halved back), in Q62 fixed point rounded up at
/// every step, so the computed `x` never falls below the true one and the bits
/// never undershoot. One unit on top covers the truncation of the sixteenth bit.
pub(crate) const fn log2_upper_q16(m: u128) -> u64 {
    const ONE: u128 = 1 << 62;
    if m <= 1 {
        return 0;
    }
    let lead = m.ilog2();
    let mut x = if lead <= 62 {
        m << (62 - lead)
    } else {
        let shift = lead - 62;
        (m >> shift) + if m & ((1 << shift) - 1) != 0 { 1 } else { 0 }
    };
    let mut fraction = 0_u64;
    let mut bit = 0;
    while bit < 16 {
        // x < 2^63 + 2^-k slack, so x² < 2^127: inside u128.
        let square = x * x;
        x = (square >> 62) + if square & (ONE - 1) != 0 { 1 } else { 0 };
        fraction <<= 1;
        if x >= 2 * ONE {
            fraction |= 1;
            x = (x >> 1) + (x & 1);
        }
        bit += 1;
    }
    (lead as u64) * 65_536 + fraction + 1
}

/// `a^exp` for an `a` with `log2|a| ≤ log2_q16 / 65536`: the result has at most
/// `exp × log2|a| / log2(1e9) + 1` limbs, and square-and-multiply does at most
/// two thirds of the square of that in limb products (the squarings form a
/// geometric series under the final one).
#[must_use]
pub(crate) const fn pow(log2_q16: u64, exp: u32) -> Cost {
    let bits = (log2_q16 as u128) * (exp as u128);
    let limbs = bits.div_ceil(LOG2_LIMB_Q16_DOWN as u128) + 1;
    let result = if limbs > u64::MAX as u128 {
        u64::MAX
    } else {
        limbs as u64
    };
    let work = result
        .saturating_mul(result)
        .saturating_add(result.saturating_mul(8))
        .saturating_add(1);
    Cost::new(work, limb_bytes(result))
}

/// `gcd(a, b)` by Euclid's algorithm.
#[must_use]
pub(crate) const fn gcd(la: u64, lb: u64) -> Cost {
    let longer = if la > lb { la } else { lb };
    let work = la
        .saturating_add(1)
        .saturating_mul(lb.saturating_add(1))
        .saturating_mul(8)
        .saturating_add(longer.saturating_mul(64));
    Cost::new(work, limb_bytes(longer.saturating_add(1)))
}

/// `a × 10^digits` (a limb shift and one short multiply).
#[must_use]
pub(crate) const fn shift10(la: u64, digits: u64) -> Cost {
    let result = la.saturating_add(limbs_for_digits(digits));
    Cost::new(result, limb_bytes(result))
}

/// The size of one exact value, read in constant time without touching its digits:
/// what every cost estimate below is computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shape {
    /// The coefficient's size in base-`1e9` limbs (zero for zero).
    pub(crate) limbs: u64,
    /// The coefficient's decimal digits (`1` for zero).
    pub(crate) digits: u64,
    /// The number of fractional digits (`0` for an integer).
    pub(crate) scale: u64,
    /// `-1`, `0` or `1` by the sign of the value.
    pub(crate) sign: i32,
}

impl Shape {
    /// The shape of `mantissa × 10^-scale`, an inline coefficient.
    pub(crate) const fn of_i128(mantissa: i128, scale: u64) -> Self {
        let magnitude = mantissa.unsigned_abs();
        let (limbs, digits) = if magnitude == 0 {
            (0, 1)
        } else {
            let digits = magnitude.ilog10() as u64 + 1;
            (digits.div_ceil(9), digits)
        };
        Self {
            limbs,
            digits,
            scale,
            sign: if mantissa < 0 {
                -1
            } else if mantissa > 0 {
                1
            } else {
                0
            },
        }
    }

    /// The position of the leading digit: `|value| ∈ [10^(e−1), 10^e)` for the
    /// returned `e` (meaningless for zero).
    pub(crate) const fn magnitude_exponent(self) -> i64 {
        (self.digits as i64).saturating_sub(self.scale as i64)
    }

    /// The decimal digits of the canonical lexical form, sign and point included:
    /// the bytes rendering it produces.
    pub(crate) const fn rendered_len(self) -> u64 {
        let body = if self.scale >= self.digits {
            // `0.` and the leading zeros.
            self.scale.saturating_add(2)
        } else if self.scale == 0 {
            self.digits
        } else {
            self.digits.saturating_add(1)
        };
        body.saturating_add(if self.sign < 0 { 1 } else { 0 })
    }
}

/// Rendering a value of this shape as its canonical lexical form: one pass over
/// the coefficient's digits, then the bytes of the text (the leading zeros of a
/// small fraction included).
#[must_use]
pub(crate) const fn render_shape(value: Shape) -> Cost {
    let text = value.rendered_len();
    Cost::new(
        value
            .limbs
            .saturating_mul(9)
            .saturating_add(text)
            .saturating_add(1),
        text.saturating_add(value.limbs.saturating_mul(9)),
    )
}

/// `a + b` / `a − b` over decimals: the coefficient with the smaller scale is
/// shifted up to the larger one first.
#[must_use]
pub(crate) const fn decimal_add(a: Shape, b: Shape) -> Cost {
    let gap = a.scale.abs_diff(b.scale);
    let (la, lb) = if a.scale < b.scale {
        (a.limbs.saturating_add(limbs_for_digits(gap)), b.limbs)
    } else {
        (a.limbs, b.limbs.saturating_add(limbs_for_digits(gap)))
    };
    let shifted = if a.scale < b.scale { a.limbs } else { b.limbs };
    shift10(shifted, gap).saturating_add(add(la, lb))
}

/// Comparing two decimals: decided by the signs or the leading-digit positions
/// alone unless both agree, and then by aligning the coefficients, whose scale gap
/// equals their digit-count gap and so never exceeds the longer coefficient.
#[must_use]
pub(crate) const fn decimal_cmp(a: Shape, b: Shape) -> Cost {
    if a.sign != b.sign || a.sign == 0 || a.magnitude_exponent() != b.magnitude_exponent() {
        return Cost::new(1, 0);
    }
    decimal_add(a, b)
}

/// `a × b` over decimals: the coefficient product, then stripping the trailing
/// zeros the canonical form drops (one pass over the product).
#[must_use]
pub(crate) const fn decimal_mul(a: Shape, b: Shape) -> Cost {
    let product = a.limbs.saturating_add(b.limbs);
    mul(a.limbs, b.limbs).saturating_add(Cost::new(product.saturating_add(1), limb_bytes(product)))
}

/// Negation, the absolute value or a rounding to an integer: a few passes over the
/// coefficient, and a result no longer than it plus one limb.
#[must_use]
pub(crate) const fn decimal_unary(a: Shape) -> Cost {
    // The operand's copy, the kept digits, the discarded ones and the rounded
    // result can be live at once, with the canonical form's own copy.
    Cost::new(
        a.limbs.saturating_mul(4).saturating_add(2),
        limb_bytes(a.limbs.saturating_add(1)).saturating_mul(6),
    )
}

/// The decimal exponents past which a value of a binary format is an infinity or
/// rounds to a signed zero, decided before any digit is touched: `|v| ≥ 10^(e−1)`
/// with `e ≥ overflow` is past the largest finite value, and `|v| < 10^e` with
/// `e ≤ underflow` is below half the smallest subnormal.
pub(crate) const F64_DECIMAL_EXPONENTS: (i64, i64) = (310, -324);
/// [`F64_DECIMAL_EXPONENTS`] for binary32.
pub(crate) const F32_DECIMAL_EXPONENTS: (i64, i64) = (40, -46);

/// Converting a decimal to the nearest `f64`/`f32`: free past the format's range
/// (an infinity or a signed zero, decided from the shape), otherwise forming
/// `10^scale` and one division whose quotient is at most five limbs.
#[must_use]
pub(crate) const fn decimal_to_float(a: Shape) -> Cost {
    let exponent = a.magnitude_exponent();
    if a.sign == 0 || exponent >= F64_DECIMAL_EXPONENTS.0 || exponent <= F64_DECIMAL_EXPONENTS.1 {
        return Cost::new(1, 0);
    }
    let denominator = limbs_for_digits(a.scale);
    let la = a.limbs.saturating_add(5);
    shift10(0, a.scale).saturating_add(div(
        if la > denominator.saturating_add(5) {
            la
        } else {
            denominator.saturating_add(5)
        },
        denominator,
    ))
}

/// Comparing a decimal with a finite `f64` exactly: free when the signs or the
/// magnitudes' binades decide, otherwise scaling the coefficient by at most `2^1074`
/// and the binary significand by `10^scale` (whose scale is then within 330 digits
/// of the coefficient's length) and comparing the two.
#[must_use]
pub(crate) const fn decimal_cmp_f64(a: Shape) -> Cost {
    let exponent = a.magnitude_exponent();
    if a.sign == 0 || exponent >= F64_DECIMAL_EXPONENTS.0 || exponent <= F64_DECIMAL_EXPONENTS.1 {
        return Cost::new(1, 0);
    }
    // 1074 bits are 36 chunks of 29 bits, each a pass over the coefficient.
    let left = a.limbs.saturating_add(36);
    let right = limbs_for_digits(a.scale).saturating_add(3);
    Cost::new(
        left.saturating_mul(37)
            .saturating_add(right.saturating_mul(2))
            .saturating_add(1),
        limb_bytes(left.saturating_add(right)),
    )
}

/// `a ÷ b` over decimals under `policy`: a rounded quotient scales one operand by
/// the scale gap and divides; an exact one takes a gcd, two reductions, strips the
/// divisor's twos and fives (at most `log2(10^(9·lb)) < 30·lb` of each, one linear
/// pass apiece) and multiplies by a factor of at most `3·lb` limbs.
#[must_use]
pub(crate) const fn decimal_div(a: Shape, b: Shape, policy: super::DivisionPolicy) -> Cost {
    let (la, lb) = (a.limbs, b.limbs);
    match policy {
        super::DivisionPolicy::Scale { scale, .. } => {
            let shift = (scale as i64)
                .saturating_add(b.scale as i64)
                .saturating_sub(a.scale as i64);
            let digits = shift.unsigned_abs();
            let (numerator, denominator) = if shift >= 0 {
                (la.saturating_add(limbs_for_digits(digits)), lb)
            } else {
                (la, lb.saturating_add(limbs_for_digits(digits)))
            };
            let shifted = if la < lb { la } else { lb };
            shift10(shifted, digits).saturating_add(div(numerator, denominator))
        }
        super::DivisionPolicy::Exact => {
            let strip = lb.saturating_mul(60).saturating_mul(lb.saturating_add(1));
            gcd(la, lb)
                .saturating_add(div(la, 1))
                .saturating_add(div(lb, 1))
                .saturating_add(Cost::new(strip, 0))
                .saturating_add(mul(la, lb.saturating_mul(3)))
        }
    }
}
