// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The binary64 operations every arithmetic computes with, held to a software reference
//! that uses no floating-point unit at all.
//!
//! [`soft`] computes each operation's exact result in integers and rounds it once, to
//! nearest-even, into binary64's format, subnormals included; the same code rounds into
//! any other format, so it also models what the x87 does when it rounds twice. The
//! [`Binary64`](super::binary64::Binary64) operations, the probe rows' constants, and the
//! exact kernels are all compared with it bit for bit. On a target whose unit is IEEE
//! binary64 the comparison shows the reference is right; on the x87 it shows the guard
//! and the scaling make the unit right.

use core::hint::black_box;

use super::binary64::Precision;
use super::tests::{Stream, reference_distance};
use super::*;

/// Binary64 arithmetic in integers: an exact result, rounded once.
pub(super) mod soft {
    /// An exact finite value, `(−1)^neg · sig · 2^exp`.
    #[derive(Debug, Clone, Copy)]
    struct Exact {
        neg: bool,
        sig: u128,
        exp: i32,
    }

    /// A binary floating-point format with round-to-nearest-even: its significand width
    /// and the exponent of its smallest normal (the lowest exponent a leading bit keeps
    /// full precision at). Its exponent range is unbounded above; overflow is decided when
    /// the value is converted to binary64.
    #[derive(Debug, Clone, Copy)]
    pub(in super::super) struct Format {
        precision: u32,
        emin: i32,
    }

    /// IEEE-754 binary64.
    pub(in super::super) const BINARY64: Format = Format {
        precision: 53,
        emin: -1022,
    };
    /// The x87's register format at its power-on precision: a 64-bit significand, a
    /// 15-bit exponent.
    pub(in super::super) const X87_EXTENDED: Format = Format {
        precision: 64,
        emin: -16382,
    };
    /// The x87's register format with precision control at 53 bits: binary64's
    /// significand over the 15-bit exponent.
    pub(in super::super) const X87_DOUBLE: Format = Format {
        precision: 53,
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
            core::cmp::Ordering::Greater => 0,
            // The half-unit is 2^127; only a significand above it rounds up to one unit.
            core::cmp::Ordering::Equal => u128::from(value.sig > 1 << 127),
            core::cmp::Ordering::Less => {
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

    /// A value already on binary64's grid, as binary64 (an infinity past its range).
    fn to_f64(value: Exact) -> f64 {
        let sign = u64::from(value.neg) << 63;
        if value.sig == 0 {
            return f64::from_bits(sign);
        }
        let leading = value.exp + bit_length(value.sig) - 1;
        if leading > 1023 {
            return f64::from_bits(sign | f64::INFINITY.to_bits());
        }
        let (unit, biased) = if leading >= -1022 {
            (
                leading - 52,
                u64::try_from(leading + 1023).expect("positive"),
            )
        } else {
            (-1074, 0)
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
        let fraction = if biased == 0 { sig } else { sig - (1 << 52) };
        f64::from_bits(sign | (biased << 52) | fraction)
    }

    /// The exact sum of two finite values, with the sticky bit of anything lost far
    /// below every rounding position this module uses.
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
            core::cmp::Ordering::Equal => Exact {
                neg: false,
                sig: 0,
                exp,
            },
            core::cmp::Ordering::Greater => Exact {
                neg: high.neg,
                sig: high_sig - low_sig,
                exp,
            },
            core::cmp::Ordering::Less => Exact {
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

    /// The square root of a positive value to 62 bits, with a sticky bit.
    fn exact_root(a: f64) -> Exact {
        let x = decompose(a);
        let (mut sig, mut exp) = (x.sig, x.exp);
        if exp % 2 != 0 {
            sig <<= 1;
            exp -= 1;
        }
        let radicand = sig << 70;
        let root = radicand.isqrt();
        let sticky = u128::from(root * root != radicand);
        Exact {
            neg: false,
            sig: (root << 1) | sticky,
            exp: (exp - 70) / 2 - 1,
        }
    }

    /// Whether the IEEE result is fixed without rounding: an operand is a NaN or an
    /// infinity, for which every unit computes the same special value.
    fn special(a: f64, b: f64) -> bool {
        !a.is_finite() || !b.is_finite()
    }

    /// `exact` rounded into `first`, then into binary64: one rounding when `first` is
    /// binary64, and the x87's two when it is one of the register formats.
    fn through(exact: Exact, first: Format) -> f64 {
        to_f64(round(round(exact, first), BINARY64))
    }

    /// `a + b` rounded into `first`, then binary64.
    pub(in super::super) fn add_via(a: f64, b: f64, first: Format) -> f64 {
        if special(a, b) {
            return a + b;
        }
        through(exact_sum(a, b), first)
    }

    /// `a × b` rounded into `first`, then binary64.
    pub(in super::super) fn mul_via(a: f64, b: f64, first: Format) -> f64 {
        if special(a, b) {
            return a * b;
        }
        through(exact_product(a, b), first)
    }

    /// `a ÷ b` rounded into `first`, then binary64.
    pub(in super::super) fn div_via(a: f64, b: f64, first: Format) -> f64 {
        if special(a, b) || b == 0.0 {
            return a / b;
        }
        through(exact_quotient(a, b), first)
    }

    /// `a + b`, correctly rounded.
    pub(in super::super) fn add(a: f64, b: f64) -> f64 {
        add_via(a, b, BINARY64)
    }

    /// `a − b`, correctly rounded.
    pub(in super::super) fn sub(a: f64, b: f64) -> f64 {
        add_via(a, -b, BINARY64)
    }

    /// `a × b`, correctly rounded.
    pub(in super::super) fn mul(a: f64, b: f64) -> f64 {
        mul_via(a, b, BINARY64)
    }

    /// `a ÷ b`, correctly rounded.
    pub(in super::super) fn div(a: f64, b: f64) -> f64 {
        div_via(a, b, BINARY64)
    }

    /// `√a`, correctly rounded.
    pub(in super::super) fn sqrt(a: f64) -> f64 {
        if !a.is_finite() || a <= 0.0 {
            return a.sqrt();
        }
        through(exact_root(a), BINARY64)
    }
}

// ---- the operand fixture -------------------------------------------------------------

/// A binary64 of the adversarial class `class`: every exponent, subnormals, values near
/// one (where ties are dense), and powers of two.
fn operand(stream: &mut Stream, class: u64) -> f64 {
    let bits = stream.next_u64();
    let sign = bits & (1 << 63);
    let fraction = bits & ((1 << 52) - 1);
    let biased = match class {
        // Any finite exponent, subnormals included.
        0 => (bits >> 52) % 0x7ff,
        // Subnormal.
        1 => 0,
        // Near one: short significands make exact results that sit on or next to ties.
        2 => 1023 + (bits >> 52) % 2,
        // A product or quotient of two of these lands in or near the subnormal range.
        3 => 1023 - 511 - (bits >> 52) % 32,
        4 => 1023 + 511 - (bits >> 52) % 32,
        _ => 1023 + (bits >> 52) % 64,
    };
    let fraction = if class == 2 || class == 5 {
        // Few significant bits, at either end of the significand.
        fraction & 0xf_0000_0000_000f
    } else {
        fraction
    };
    f64::from_bits(sign | (biased << 52) | fraction)
}

/// Operand pairs covering every class against every class.
fn pairs(count: usize) -> Vec<(f64, f64)> {
    let mut stream = Stream(0x5eed_b64f);
    let mut pairs = Vec::with_capacity(count);
    for index in 0..count {
        let index = u64::try_from(index).expect("small");
        let left = operand(&mut stream, index % 6);
        let right = operand(&mut stream, (index / 6) % 6);
        pairs.push((left, right));
    }
    // Every probe row, the double-rounding witnesses included.
    for row in &env::PROBES {
        pairs.push(row.operands());
    }
    pairs
}

/// Bits, with every NaN collapsed to one pattern: no law exposes a NaN's payload.
fn canonical(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

#[test]
fn binary64_operations_equal_the_software_reference() {
    // On an IEEE binary64 unit this is the reference proven against the hardware; on the
    // x87 it is the guard and the scaling proven against the reference.
    let precision = Precision::enter();
    let ops = precision.binary64();
    let mut compared = 0_usize;
    for (a, b) in pairs(60_000) {
        let (x, y) = (black_box(a), black_box(b));
        let cases = [
            ("+", canonical(ops.add(x, y)), canonical(soft::add(a, b))),
            ("-", canonical(ops.sub(x, y)), canonical(soft::sub(a, b))),
            ("*", canonical(ops.mul(x, y)), canonical(soft::mul(a, b))),
            ("/", canonical(ops.div(x, y)), canonical(soft::div(a, b))),
            (
                "sqrt",
                canonical(ops.sqrt(x.abs())),
                canonical(soft::sqrt(a.abs())),
            ),
            (
                "algebraic +",
                canonical(ops.algebraic_add(x, y)),
                canonical(soft::add(a, b)),
            ),
            (
                "algebraic -",
                canonical(ops.algebraic_sub(x, y)),
                canonical(soft::sub(a, b)),
            ),
            (
                "algebraic *",
                canonical(ops.algebraic_mul(x, y)),
                canonical(soft::mul(a, b)),
            ),
        ];
        for (name, got, want) in cases {
            assert_eq!(
                got,
                want,
                "{a:e} {name} {b:e} ({:#018x}, {:#018x}): {got:#018x} where the reference \
                 gives {want:#018x}",
                a.to_bits(),
                b.to_bits()
            );
            compared += 1;
        }
    }
    assert!(compared >= 480_000, "{compared} comparisons");
}

#[test]
fn the_reference_rounds_twice_the_way_the_x87_does() {
    // The observing oracle for the rows below: the reference's two-step rounding is not
    // the one-step one. `1 + (2⁻⁵³ + 2⁻⁷⁸)` rounds up once and down twice.
    let one = 1.0_f64;
    let little = f64::EPSILON / 2.0 + f64::EPSILON / (1_u64 << 26) as f64;
    assert_eq!(soft::add(one, little), 1.0 + f64::EPSILON);
    assert_eq!(soft::add_via(one, little, soft::X87_EXTENDED), 1.0);
    assert_eq!(
        soft::add_via(one, little, soft::X87_DOUBLE),
        1.0 + f64::EPSILON
    );
}

#[test]
fn every_double_rounding_row_is_a_witness_and_no_other_row_is() {
    for row in &env::PROBES {
        let (a, b) = row.operands();
        let via = |first| match row.symbol() {
            '+' => soft::add_via(a, b, first),
            '*' => soft::mul_via(a, b, first),
            '/' => soft::div_via(a, b, first),
            other => unreachable!("no probe row performs {other}"),
        };
        let once = via(soft::BINARY64);
        assert_eq!(
            once.to_bits(),
            row.expected,
            "{}: the reference computes the row's constant",
            row.operation
        );
        let extended = via(soft::X87_EXTENDED);
        let double = via(soft::X87_DOUBLE);
        let subnormal = !f64::from_bits(row.expected).is_normal();
        if row.departure == env::Departure::DoubleRounding {
            // Rounded to the x87's 64 bits and then to binary64, the row gives other bits.
            assert_ne!(
                extended.to_bits(),
                row.expected,
                "{}: rounded twice through 64 bits, the row must differ",
                row.operation
            );
            // At 53-bit precision control, the normal-range rows are closed and the
            // subnormal-range rows are not: only the scaling closes those.
            if subnormal {
                assert_ne!(
                    double.to_bits(),
                    row.expected,
                    "{}: a subnormal result rounded at 53 bits and again when stored",
                    row.operation
                );
            } else {
                assert_eq!(
                    double.to_bits(),
                    row.expected,
                    "{}: precision control at 53 bits rounds a normal result once",
                    row.operation
                );
            }
        } else {
            // The valid neighbours: no earlier row can tell double rounding apart, which
            // is why the witnesses are there.
            assert_eq!(
                extended.to_bits(),
                row.expected,
                "{}: not a double-rounding witness",
                row.operation
            );
        }
    }
    let witnesses: Vec<(&str, bool)> = env::PROBES
        .iter()
        .filter(|row| row.departure == env::Departure::DoubleRounding)
        .map(|row| (row.operation, f64::from_bits(row.expected).is_normal()))
        .collect();
    assert_eq!(
        witnesses,
        [
            ("1 + (2^-53 + 2^-78)", true),
            ("(1 + ulp) + (2^-53 - 2^-78)", true),
            ("(1 - 0.5 ulp) * (1 + 2 ulp)", true),
            ("2^-512 (1 + 4 ulp) * 2^-511 (1 - ulp)", false),
            ("2^-512 / (2^511 (1 - ulp))", false),
        ],
        "sums and a product in the normal range, a product and a quotient in the \
         subnormal range"
    );
}

// ---- the kernels against the reference ------------------------------------------------

/// The PURREMB §13.2 norm, over the reference's operations.
fn reference_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0_f64;
    let mut ssq = 1.0_f64;
    for &value in values {
        let value = value.abs();
        if value == 0.0 {
            continue;
        }
        if scale < value {
            let ratio = soft::div(scale, value);
            ssq = soft::add(1.0, soft::mul(ssq, soft::mul(ratio, ratio)));
            scale = value;
        } else {
            let ratio = soft::div(value, scale);
            ssq = soft::add(ssq, soft::mul(ratio, ratio));
        }
    }
    soft::mul(scale, soft::sqrt(ssq))
}

/// Vectors whose every term is a double-rounding witness or lands in the subnormal range,
/// at every length that exercises the lanes, the tree and the tail.
fn witness_vectors() -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut stream = Stream(0x0dd_7135);
    let mut vectors = Vec::new();
    for len in (1..=40).chain([63, 64, 65, 129]) {
        let mut a = Vec::with_capacity(len);
        let mut b = Vec::with_capacity(len);
        for index in 0..len {
            let row = &env::PROBES[8 + (index + len) % 5];
            let (x, y) = row.operands();
            match stream.next_u64() % 4 {
                // The row's own operands: a product that rounds twice on the x87.
                0 | 1 if row.symbol() == '*' => {
                    a.push(x);
                    b.push(y);
                }
                // Differences whose squares are subnormal or tie-adjacent.
                0 | 1 => {
                    a.push(x);
                    b.push(x - y);
                }
                // Random subnormal-range products.
                2 => {
                    a.push(operand(&mut stream, 3));
                    b.push(operand(&mut stream, 3));
                }
                _ => {
                    a.push(operand(&mut stream, 2));
                    b.push(operand(&mut stream, 5));
                }
            }
        }
        vectors.push((a, b));
    }
    vectors
}

#[test]
fn exact_kernels_and_the_norm_equal_the_software_reference_on_witnesses() {
    // On the x87 these inputs round twice without the guard: every term is a witness row,
    // a subnormal-range product, or a tie-adjacent sum.
    let exact = Exact::resolve().expect("the default environment is the IEEE one");
    let mut compared = 0_usize;
    for (a, b) in witness_vectors() {
        let a_norm = exact.norm(&a);
        let b_norm = exact.norm(&b);
        assert_eq!(
            a_norm.to_bits(),
            reference_norm(&a).to_bits(),
            "norm of {a:?}"
        );
        assert_eq!(
            b_norm.to_bits(),
            reference_norm(&b).to_bits(),
            "norm of {b:?}"
        );
        for measure in [
            Measure::SquaredEuclidean,
            Measure::NegativeDot,
            Measure::Cosine,
        ] {
            let want = reference_distance(measure, &a, a_norm, &b, b_norm).map(f64::to_bits);
            let pair = exact.distance(measure, &a, a_norm, &b, b_norm);
            assert_eq!(
                pair.map(f64::to_bits),
                want,
                "{measure:?} at len {}",
                a.len()
            );
            let norms = [b_norm];
            let rows = RowsRef::new(&b, 1, b.len(), &norms).expect("one row");
            let mut out = [None];
            exact.distances(measure, &a, a_norm, rows, &mut out);
            assert_eq!(out[0].map(f64::to_bits), want, "{measure:?} batch");
            let bounded = match exact.distance_bounded(
                measure,
                &a,
                a_norm,
                &b,
                b_norm,
                Bound::Above(f64::INFINITY),
            ) {
                Bounded::Below(value) => Some(value.to_bits()),
                Bounded::NonFinite => None,
                Bounded::Beyond => panic!("an infinite bound is never met"),
            };
            assert_eq!(bounded, want, "{measure:?} bounded");
            compared += 3;
        }
    }
    assert!(compared > 100, "{compared} comparisons");
}

// ---- the x87 ----------------------------------------------------------------------------

#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
mod x87 {
    use super::super::binary64::{Precision, bypass, x87};
    use super::super::{
        Arithmetic, Bound, Bounded, Exact, FloatEnvironmentError, FloatEnvironmentEvidence,
        Measure, Reassociated, env,
    };
    use super::{reference_norm, soft, witness_vectors};

    /// The precision-control values: 24, 53 and 64 bits.
    const PC_24: u16 = 0b00 << 8;
    const PC_53: u16 = 0b10 << 8;
    const PC_64: u16 = 0b11 << 8;

    /// The control word loaded for as long as the guard lives, restored when it drops.
    struct Loaded(u16);

    impl Loaded {
        fn load(word: u16) -> Self {
            let saved = x87::control_word();
            x87::load_control_word(word);
            Self(saved)
        }
    }

    impl Drop for Loaded {
        fn drop(&mut self) {
            x87::load_control_word(self.0);
        }
    }

    /// Which probe rows fail on this thread, observed through a guard.
    fn failing_rows() -> Vec<usize> {
        let precision = Precision::enter();
        let ops = precision.binary64();
        env::PROBES
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.observe(ops) != row.expected).then_some(index))
            .collect()
    }

    fn refusal(row: usize, observed: f64) -> FloatEnvironmentError {
        let probe = env::PROBES[row];
        let evidence = FloatEnvironmentEvidence::Probe {
            operation: probe.operation,
            expected: probe.expected,
            observed: observed.to_bits(),
        };
        match probe.departure {
            env::Departure::Flush => FloatEnvironmentError::FlushToZero { evidence },
            env::Departure::Rounding => FloatEnvironmentError::RoundingMode { evidence },
            env::Departure::DoubleRounding => FloatEnvironmentError::DoubleRounding { evidence },
        }
    }

    #[test]
    fn the_guard_sets_binary64_precision_and_restores_the_callers() {
        let saved = x87::control_word();
        assert_eq!(
            saved & x87::PRECISION_CONTROL,
            PC_64,
            "Linux starts at 64 bits"
        );
        for caller in [PC_24, PC_53, PC_64] {
            let _loaded = Loaded::load((saved & !x87::PRECISION_CONTROL) | caller);
            let before = x87::control_word();
            {
                let _outer = Precision::enter();
                assert_eq!(x87::control_word() & x87::PRECISION_CONTROL, PC_53);
                assert_eq!(
                    x87::control_word() & !x87::PRECISION_CONTROL,
                    before & !x87::PRECISION_CONTROL,
                    "no other field changes"
                );
                {
                    let _inner = Precision::enter();
                    assert_eq!(x87::control_word() & x87::PRECISION_CONTROL, PC_53);
                }
                assert_eq!(
                    x87::control_word() & x87::PRECISION_CONTROL,
                    PC_53,
                    "a nested guard restores the outer one's precision"
                );
            }
            assert_eq!(x87::control_word(), before, "the caller's word is restored");
            let unwound = std::panic::catch_unwind(|| {
                let _guard = Precision::enter();
                panic!("unwinding through the guard");
            });
            assert!(unwound.is_err());
            assert_eq!(x87::control_word(), before, "restored when a panic unwinds");
        }
        assert_eq!(x87::control_word(), saved);
    }

    #[test]
    fn a_thread_the_guard_did_not_take_hold_on_is_refused_by_name() {
        // The refused case: the guard bypassed, the thread at Linux's 64-bit precision.
        // Every double-rounding row fails and nothing else does, and the first names the
        // departure.
        let (failing, probed, resolved, reassociated) = {
            let _bypassed = bypass::bypass();
            (
                failing_rows(),
                env::probe(),
                Exact::resolve(),
                Reassociated::resolve(),
            )
        };
        assert_eq!(failing, vec![8, 9, 10, 11, 12]);
        let refused = refusal(8, 1.0);
        assert_eq!(probed, Err(refused));
        assert_eq!(resolved, Err(refused));
        assert_eq!(reassociated.map(|_| ()), Err(refused));
        assert!(
            refused
                .to_string()
                .contains("rounds binary64 results twice")
                && refused.to_string().contains("1 + (2^-53 + 2^-78)"),
            "{refused}"
        );
        // The valid neighbour: the same thread, the guard in force.
        assert_eq!(failing_rows(), Vec::<usize>::new());
        assert_eq!(env::probe(), Ok(()));
        assert!(Exact::resolve().is_ok());
        assert!(Reassociated::resolve().is_ok());
    }

    #[test]
    fn without_the_scaling_the_subnormal_rows_round_twice() {
        // Precision control at 53 bits closes the normal-range rows and leaves the
        // subnormal ones: the unscaled product and quotient round twice, and the probe
        // row observes it; the scaled operations do not.
        let precision = Precision::enter();
        let ops = precision.binary64();
        for (row, unscaled) in [
            (11, x87::mul_unscaled as fn(f64, f64) -> f64),
            (12, x87::div_unscaled),
        ] {
            let probe = env::PROBES[row];
            let (a, b) = probe.operands();
            let twice = unscaled(a, b);
            assert_ne!(twice.to_bits(), probe.expected, "{}", probe.operation);
            let modelled = if probe.symbol() == '*' {
                soft::mul_via(a, b, soft::X87_DOUBLE)
            } else {
                soft::div_via(a, b, soft::X87_DOUBLE)
            };
            assert_eq!(twice.to_bits(), modelled.to_bits(), "{}", probe.operation);
            assert_eq!(probe.observe(ops), probe.expected, "{}", probe.operation);
        }
    }

    #[test]
    fn a_directed_rounding_control_is_refused_by_register_and_by_probe() {
        let saved = x87::control_word();
        let departures = [
            ("down", 0b01 << 10, vec![2, 5, 7, 8, 12], refusal(2, 1.0)),
            (
                "up",
                0b10 << 10,
                vec![3, 4, 6, 9, 10, 11],
                refusal(3, 1.0 + f64::EPSILON),
            ),
            (
                "toward zero",
                0b11 << 10,
                vec![2, 4, 7, 8, 12],
                refusal(2, 1.0),
            ),
        ];
        for (name, rounding, rows, probe_refusal) in departures {
            let word = (saved & !x87::ROUNDING_CONTROL) | rounding;
            let (failing, probed, resolved) = {
                let _loaded = Loaded::load(word);
                (failing_rows(), env::probe(), Exact::resolve())
            };
            assert_eq!(x87::control_word(), saved, "{name}: restored");
            assert_eq!(failing, rows, "{name}: the rows that must fail");
            assert_eq!(probed, Err(probe_refusal), "{name}: the probe's refusal");
            assert_eq!(
                resolved,
                Err(FloatEnvironmentError::RoundingMode {
                    evidence: FloatEnvironmentEvidence::Register {
                        name: "x87 control word",
                        bits: u64::from(word),
                    },
                }),
                "{name}: resolve names the register"
            );
        }
        // The valid neighbour: round-to-nearest, loaded through the same guard.
        let _loaded = Loaded::load(saved & !x87::ROUNDING_CONTROL);
        assert_eq!(failing_rows(), Vec::<usize>::new());
        assert!(Exact::resolve().is_ok());
    }

    #[test]
    fn a_callers_single_precision_is_overridden_not_inherited() {
        // A caller that left the x87 at 24 bits (as some graphics runtimes do) is not
        // refused: the guard sets binary64's precision for every kernel, so the results
        // are the reference's.
        let saved = x87::control_word();
        let _loaded = Loaded::load((saved & !x87::PRECISION_CONTROL) | PC_24);
        let exact = Exact::resolve().expect("the guard sets the precision it needs");
        for (a, b) in witness_vectors() {
            assert_eq!(exact.norm(&a).to_bits(), reference_norm(&a).to_bits());
            let want = super::reference_distance(Measure::NegativeDot, &a, 0.0, &b, 0.0);
            assert_eq!(
                exact
                    .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
                    .map(f64::to_bits),
                want.map(f64::to_bits)
            );
        }
        assert_eq!(
            x87::control_word() & x87::PRECISION_CONTROL,
            PC_24,
            "the caller's precision is theirs again after every call"
        );
    }

    /// The reassociated law as the x87 computes it: 64-element blocks, each summed in
    /// written order, the block sums added in ascending order, every operation
    /// correctly rounded.
    fn reassociated_reference(a: &[f64], b: &[f64], squares: bool) -> f64 {
        let mut total = 0.0_f64;
        for (x, y) in a.chunks(64).zip(b.chunks(64)) {
            let mut block = 0.0_f64;
            for (&p, &q) in x.iter().zip(y) {
                let term = if squares {
                    let delta = soft::sub(p, q);
                    soft::mul(delta, delta)
                } else {
                    soft::mul(p, q)
                };
                block = soft::add(block, term);
            }
            total = soft::add(total, block);
        }
        total
    }

    #[test]
    fn the_reassociated_arithmetic_is_binary64_in_written_order() {
        let resolved = Reassociated::resolve().expect("the default environment");
        for (a, b) in witness_vectors() {
            let dot = reassociated_reference(&a, &b, false);
            let want = dot.is_finite().then_some((-dot).to_bits());
            assert_eq!(
                resolved
                    .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
                    .map(f64::to_bits),
                want,
                "dot at len {}",
                a.len()
            );
            let squares = reassociated_reference(&a, &b, true);
            let want = squares.is_finite().then_some(squares.to_bits());
            let bounded = match resolved.distance_bounded(
                Measure::SquaredEuclidean,
                &a,
                0.0,
                &b,
                0.0,
                Bound::Above(f64::INFINITY),
            ) {
                Bounded::Below(value) => Some(value.to_bits()),
                Bounded::NonFinite => None,
                Bounded::Beyond => panic!("an infinite bound is never met"),
            };
            assert_eq!(bounded, want, "squares at len {}", a.len());
        }
    }
}
