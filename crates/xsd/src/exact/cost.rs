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
//!
//! # How the evaluator charges it
//!
//! The SPARQL governor (`purrdf_core::governor`) meters compute as `Fuel` and
//! value-constructing allocation as `ScratchBytes`. When the exact tower
//! becomes the evaluator's numeric representation, each arithmetic or cast
//! site computes the operation's [`Cost`], charges `work` to `Fuel` and `bytes`
//! to `ScratchBytes` — refusing with the governor's typed stop before the
//! operation runs if either ceiling would be crossed — and only then computes.
//! An inline value's costs are a handful of units, so a query that never leaves
//! the `i128` range is charged what it is charged today within a constant.

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

/// `a × b`.
#[must_use]
pub(crate) const fn mul(la: u64, lb: u64) -> Cost {
    let result = la.saturating_add(lb);
    let work = la
        .saturating_mul(lb)
        .saturating_add(result.saturating_mul(8))
        .saturating_add(1);
    Cost::new(work, limb_bytes(result))
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
