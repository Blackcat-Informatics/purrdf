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

    /// The cost of doing `self` and then `next`, one after the other: the work adds,
    /// and the bytes are the larger working set, since the first's is released
    /// before the second's is needed.
    #[must_use]
    pub const fn then(self, next: Self) -> Self {
        Self {
            work: self.work.saturating_add(next.work),
            bytes: if self.bytes > next.bytes {
                self.bytes
            } else {
                next.bytes
            },
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

/// The size of an `xsd:integer`/`xsd:decimal` value — its coefficient's length in
/// limbs and digits, its scale and its sign — which is all every estimate here
/// reads.
///
/// A shape is obtained in constant time from a value ([`Self::of_value`]), or in one
/// pass over a lexical form without parsing it ([`Self::of_lexical`]), and shapes
/// combine into upper bounds on the shapes of results ([`Self::sum`],
/// [`Self::product`], [`Self::quotient`]). So a governor can price a whole chain —
/// a `SUM` over a group, the moments of a `VARIANCE` — before running any of it,
/// by walking the shapes in the chain's order and adding up the step costs
/// ([`Self::add_cost`] and its siblings).
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::exact::cost::Shape;
///
/// let tiny = Shape::of_lexical(&format!("0.{}1", "0".repeat(99_999)), XsdDatatype::Decimal)
///     .expect("a decimal lexical");
/// let one = Shape::of_lexical("1", XsdDatatype::Integer).expect("an integer lexical");
/// assert!(!tiny.is_bounded() && one.is_bounded());
/// // Adding them aligns the integer to 100 000 fractional digits, and the sum's text
/// // is as long.
/// let sum = tiny.sum(one);
/// assert!(tiny.add_cost(one).bytes() > 40_000);
/// assert!(sum.render_cost().bytes() > 100_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
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

    /// The shape of an `xsd:integer`/`xsd:decimal` value of any size, in constant
    /// time; `None` for every other value.
    #[must_use]
    pub fn of_value(value: &crate::XsdValue) -> Option<Self> {
        crate::numeric::exact_path::shape_of(value)
    }

    /// The shape of the value `lexical` denotes under `datatype`, an integer-family
    /// datatype or `xsd:decimal`, read in one pass without parsing it; `None` for any
    /// other datatype or a lexical form outside its lexical space (whose parse fails
    /// and so computes nothing).
    #[must_use]
    pub fn of_lexical(lexical: &str, datatype: crate::XsdDatatype) -> Option<Self> {
        let valid = if datatype == crate::XsdDatatype::Decimal {
            crate::numeric::is_decimal_lexical(lexical)
        } else {
            datatype.is_integer_family() && crate::numeric::is_integer_lexical(lexical)
        };
        if !valid {
            return None;
        }
        let negative = lexical.starts_with('-');
        let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
        let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        let digits = if whole.is_empty() {
            fraction.trim_start_matches('0').len()
        } else {
            whole.len() + fraction.len()
        } as u64;
        if digits == 0 {
            return Some(Self::of_i128(0, 0));
        }
        Some(Self {
            limbs: digits.div_ceil(9),
            digits,
            scale: fraction.len() as u64,
            sign: if negative { -1 } else { 1 },
        })
    }

    /// Whether a value of this shape fits the machine-word variants (an `i128`
    /// coefficient, at most eighteen fractional digits), where every operation is
    /// machine arithmetic and costs nothing here.
    #[must_use]
    pub const fn is_bounded(self) -> bool {
        self.digits <= 38 && self.scale <= 18
    }

    /// The coefficient's length in base-`1e9` limbs.
    #[must_use]
    pub const fn limbs(self) -> u64 {
        self.limbs
    }

    /// The number of fractional digits.
    #[must_use]
    pub const fn scale(self) -> u64 {
        self.scale
    }

    /// An upper bound on the shape of `self ± other`: the larger scale, and one
    /// more integer digit than the longer integer part.
    #[must_use]
    pub const fn sum(self, other: Self) -> Self {
        let scale = if self.scale > other.scale {
            self.scale
        } else {
            other.scale
        };
        let whole_a = self.digits.saturating_sub(self.scale);
        let whole_b = other.digits.saturating_sub(other.scale);
        let whole = if whole_a > whole_b { whole_a } else { whole_b };
        Self::bound(whole.saturating_add(1), scale)
    }

    /// An upper bound on the shape of `self × other`: the scales and the digits add.
    #[must_use]
    pub const fn product(self, other: Self) -> Self {
        let digits = self.digits.saturating_add(other.digits);
        let scale = self.scale.saturating_add(other.scale);
        Self::bound(digits.saturating_sub(scale), scale)
    }

    /// An upper bound on the shape of `self ÷ other` under `policy`: the policy's
    /// scale (for [`super::DivisionPolicy::Exact`], the dividend's scale and four
    /// digits per divisor digit, which every terminating expansion fits in), and the
    /// dividend's integer digits plus the divisor's fractional ones.
    #[must_use]
    pub const fn quotient(self, other: Self, policy: super::DivisionPolicy) -> Self {
        let scale = match policy {
            super::DivisionPolicy::Scale { scale, .. } => scale as u64,
            super::DivisionPolicy::Exact => {
                self.scale.saturating_add(other.digits.saturating_mul(4))
            }
        };
        let whole = self
            .digits
            .saturating_sub(self.scale)
            .saturating_add(other.scale)
            .saturating_add(1);
        Self::bound(whole, scale)
    }

    /// The shape of a nonzero value with `whole` integer digits and `scale`
    /// fractional ones.
    const fn bound(whole: u64, scale: u64) -> Self {
        let digits = whole.saturating_add(scale);
        Self {
            limbs: digits.div_ceil(9),
            digits,
            scale,
            sign: -1,
        }
    }

    /// The cost of `self + other` or `self − other`.
    #[must_use]
    pub const fn add_cost(self, other: Self) -> Cost {
        decimal_add(self, other)
    }

    /// The cost of `self × other`.
    #[must_use]
    pub const fn mul_cost(self, other: Self) -> Cost {
        decimal_mul(self, other)
    }

    /// The cost of `self ÷ other` under `policy`.
    #[must_use]
    pub const fn div_cost(self, other: Self, policy: super::DivisionPolicy) -> Cost {
        decimal_div(self, other, policy)
    }

    /// An upper bound on the cost of comparing a value of this shape with one of
    /// `other`'s, whatever their leading positions: aligning the two coefficients.
    /// [`crate::numeric::numeric_cost`] prices one known pair more tightly.
    #[must_use]
    pub const fn cmp_cost(self, other: Self) -> Cost {
        decimal_add(self, other)
    }

    /// A coefficient/scale bound covering either operand of a running comparison.
    #[must_use]
    pub fn comparison_bound(self, other: Self) -> Self {
        Self::bound(
            self.digits
                .saturating_sub(self.scale)
                .max(other.digits.saturating_sub(other.scale)),
            self.scale.max(other.scale),
        )
    }

    /// The cost of rendering a value of this shape as its canonical lexical form.
    #[must_use]
    pub const fn render_cost(self) -> Cost {
        render_shape(self)
    }

    /// The cost of converting a value of this shape to the nearest `f64`/`f32`, or
    /// of comparing it exactly with one, whichever is dearer.
    #[must_use]
    pub const fn ieee_cost(self) -> Cost {
        decimal_to_float(self).max(decimal_cmp_f64(self))
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

/// What `values` cost to fold into one running total in this order — `SUM`, the
/// first moment of a `VARIANCE` — and an upper bound on the total's shape (`None`
/// for no value): the step costs of a chain whose accumulator takes each sum's
/// bounded shape, done one after another ([`Cost::then`]: the work adds, the bytes
/// are the largest step's). Zero while every step stays inside the bounded
/// variants, where the fold is machine arithmetic.
///
/// The accumulator's shape is bounded by the values seen so far, not grown by a
/// digit per step: `k` values each below `10^W` (with `W` the longest integer part
/// among them) sum to less than `k · 10^W ≤ 10^(W + ⌈log10 k⌉)`, at the largest
/// scale among them. So `n` values of `d` digits cost `n` additions of about
/// `d + log10 n` digits — linear in `n` — where a digit per step would charge
/// `n²/2` and refuse a fold the fold itself would finish.
#[must_use]
pub fn sum_chain(values: impl IntoIterator<Item = Shape>) -> (Cost, Option<Shape>) {
    let mut chain = SumChain::default();
    for value in values {
        chain.push(value);
    }
    chain.finish()
}

/// Incremental form of [`sum_chain`], pricing each addition before it runs.
#[derive(Debug, Default)]
pub struct SumChain {
    total: Cost,
    running: Option<Shape>,
    longest_whole: u64,
    largest_scale: u64,
    count: u64,
}

impl SumChain {
    /// Add one operand shape and return this addition's incremental cost.
    // Keep the extracted recurrence in its caller, without per-operand state/ABI traffic.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn push(&mut self, value: Shape) -> Cost {
        self.count = self.count.saturating_add(1);
        let whole = value.digits.saturating_sub(value.scale);
        if whole > self.longest_whole {
            self.longest_whole = whole;
        }
        if value.scale > self.largest_scale {
            self.largest_scale = value.scale;
        }
        let mut step = Cost::ZERO;
        self.running = Some(match self.running {
            None => value,
            Some(acc) => {
                // ⌈log10 count⌉ for count ≥ 2: the digits of count − 1.
                let carry = u64::from((self.count - 1).ilog10()) + 1;
                let next =
                    Shape::bound(self.longest_whole.saturating_add(carry), self.largest_scale);
                if !(acc.is_bounded() && value.is_bounded() && next.is_bounded()) {
                    step = acc.add_cost(value);
                    self.total = self.total.then(step);
                }
                next
            }
        });
        step
    }

    /// Return the complete chain cost and its resulting shape bound.
    #[inline]
    #[must_use]
    pub const fn finish(&self) -> (Cost, Option<Shape>) {
        (self.total, self.running)
    }
}

/// An upper bound on what comparing `values` costs when each takes part in at
/// most `rounds` comparisons as the side that moves — `⌈log2 n⌉` rounds for a
/// sort, one for a running `MIN`/`MAX`.
///
/// A comparison costs more than a constant only between two values of one sign
/// and one leading-digit position, where it aligns their coefficients; so the
/// values are grouped by that key, and a group holding a value past the bounded
/// variants is charged its size × `rounds` alignments against its largest
/// member. Every other group compares in machine words, or decides by the key
/// alone, and is free.
#[must_use]
pub fn compare_chain(values: &[Shape], rounds: u64) -> Cost {
    let mut keyed: Vec<(i32, i64, Shape)> = values
        .iter()
        .map(|shape| (shape.sign, shape.magnitude_exponent(), *shape))
        .collect();
    keyed.sort_unstable_by_key(|&(sign, exponent, _)| (sign, exponent));
    let mut total = Cost::ZERO;
    for group in keyed.chunk_by(|a, b| (a.0, a.1) == (b.0, b.1)) {
        if group.iter().all(|(_, _, shape)| shape.is_bounded()) {
            continue;
        }
        let largest = group
            .iter()
            .map(|&(_, _, shape)| shape)
            .max_by_key(|shape| (shape.limbs, shape.scale))
            .expect("a group is nonempty");
        let one = largest.cmp_cost(largest);
        let count = (group.len() as u64).saturating_mul(rounds);
        total = total.saturating_add(Cost::new(one.work().saturating_mul(count), one.bytes()));
    }
    total
}

/// The rounds of [`compare_chain`] a sort of `n` values makes each value take part
/// in as the moving side: `⌈log2 n⌉`, and at least one.
#[must_use]
pub const fn sort_rounds(n: usize) -> u64 {
    let rounds = n.saturating_sub(1).bit_width() as u64;
    if rounds == 0 { 1 } else { rounds }
}
