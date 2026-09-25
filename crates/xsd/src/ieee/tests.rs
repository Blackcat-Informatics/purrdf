// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The correctly rounded operations, held to the integer software reference bit for bit,
//! at both widths, on adversarial operands: every exponent, subnormals, ties, and
//! double-rounding witnesses whose results differ when rounded through a wider format.

use core::hint::black_box;

use super::reference as soft;
use super::{
    Binary32Scope, Binary64Scope, f32_add, f32_div, f32_mul, f32_sqrt, f32_sub, f64_add, f64_div,
    f64_mul, f64_sqrt, f64_sub,
};

/// A SplitMix64 stream: deterministic operands, no clock and no RNG.
struct Stream(u64);

impl Stream {
    const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

// ---- the witnesses -----------------------------------------------------------------------

/// Which operation a witness performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Mul,
    Div,
}

/// A binary64 double-rounding witness: an exact result within a 64-bit (or, in the
/// subnormal range, a 53-bit) rounding of a binary64 midpoint without being one.
struct Witness64 {
    name: &'static str,
    op: Op,
    lhs: u64,
    rhs: u64,
    expected: u64,
}

/// `1`.
const ONE: u64 = 0x3ff0_0000_0000_0000;
/// 2⁻⁵¹² and 2⁻¹⁰²³ + 2⁻¹⁰⁷⁴, the subnormal just above half the smallest normal.
const TWO_POW_MINUS_512: u64 = (1023 - 512) << 52;
const HALF_MIN_NORMAL_PLUS_ONE: u64 = 0x0008_0000_0000_0001;

const WITNESSES_64: [Witness64; 5] = [
    Witness64 {
        name: "1 + (2^-53 + 2^-78)",
        op: Op::Add,
        lhs: ONE,
        rhs: 0x3ca0_0000_0800_0000,
        expected: ONE + 1,
    },
    Witness64 {
        name: "(1 + ulp) + (2^-53 - 2^-78)",
        op: Op::Add,
        lhs: ONE + 1,
        rhs: 0x3c9f_ffff_f000_0000,
        expected: ONE + 1,
    },
    Witness64 {
        name: "(1 - 0.5 ulp) * (1 + 2 ulp)",
        op: Op::Mul,
        lhs: ONE - 1,
        rhs: ONE + 2,
        expected: ONE + 1,
    },
    Witness64 {
        name: "2^-512 (1 + 4 ulp) * 2^-511 (1 - ulp)",
        op: Op::Mul,
        lhs: TWO_POW_MINUS_512 + 4,
        rhs: ((1023 - 511) << 52) - 2,
        expected: HALF_MIN_NORMAL_PLUS_ONE,
    },
    Witness64 {
        name: "2^-512 / (2^511 (1 - ulp))",
        op: Op::Div,
        lhs: TWO_POW_MINUS_512,
        rhs: ((1023 + 511) << 52) - 2,
        expected: HALF_MIN_NORMAL_PLUS_ONE,
    },
];

impl Witness64 {
    const fn operands(&self) -> (f64, f64) {
        (f64::from_bits(self.lhs), f64::from_bits(self.rhs))
    }

    fn via(&self, first: soft::Format) -> f64 {
        let (a, b) = self.operands();
        match self.op {
            Op::Add => soft::add_via(a, b, first),
            Op::Mul => soft::mul_via(a, b, first),
            Op::Div => soft::div_via(a, b, first),
        }
    }

    fn observe(&self) -> f64 {
        let (a, b) = self.operands();
        let (a, b) = (black_box(a), black_box(b));
        black_box(match self.op {
            Op::Add => f64_add(a, b),
            Op::Mul => f64_mul(a, b),
            Op::Div => f64_div(a, b),
        })
    }
}

/// A binary32 double-rounding witness: a subnormal-range product or quotient within a
/// 24-bit rounding of a binary32 midpoint without being one.
struct Witness32 {
    name: &'static str,
    op: Op,
    lhs: u32,
    rhs: u32,
    expected: u32,
}

/// 2⁻⁶⁴, and 2⁻¹²⁷ + 2⁻¹⁴⁹: the subnormal just above half binary32's smallest normal.
const F32_TWO_POW_MINUS_64: u32 = (127 - 64) << 23;
const F32_HALF_MIN_NORMAL_PLUS_ONE: u32 = 0x0040_0001;

const WITNESSES_32: [Witness32; 2] = [
    Witness32 {
        // 2⁻¹²⁷ (1 + 3u − 4u²), u = 2⁻²³: below the midpoint 2⁻¹²⁷ + 1.5 · 2⁻¹⁴⁹.
        name: "2^-64 (1 + 4 ulp) * 2^-63 (1 - ulp)",
        op: Op::Mul,
        lhs: F32_TWO_POW_MINUS_64 + 4,
        rhs: ((127 - 63) << 23) - 2,
        expected: F32_HALF_MIN_NORMAL_PLUS_ONE,
    },
    Witness32 {
        // 2⁻¹²⁷ (1 + u + u² + …): above the midpoint 2⁻¹²⁷ + 0.5 · 2⁻¹⁴⁹.
        name: "2^-64 / (2^63 (1 - ulp))",
        op: Op::Div,
        lhs: F32_TWO_POW_MINUS_64,
        rhs: ((127 + 63) << 23) - 2,
        expected: F32_HALF_MIN_NORMAL_PLUS_ONE,
    },
];

impl Witness32 {
    const fn operands(&self) -> (f32, f32) {
        (f32::from_bits(self.lhs), f32::from_bits(self.rhs))
    }

    fn via(&self, first: soft::Format) -> f32 {
        let (a, b) = self.operands();
        match self.op {
            Op::Add => soft::add32_via(a, b, first),
            Op::Mul => soft::mul32_via(a, b, first),
            Op::Div => soft::div32_via(a, b, first),
        }
    }

    fn observe(&self) -> f32 {
        let (a, b) = self.operands();
        let (a, b) = (black_box(a), black_box(b));
        black_box(match self.op {
            Op::Add => f32_add(a, b),
            Op::Mul => f32_mul(a, b),
            Op::Div => f32_div(a, b),
        })
    }
}

#[test]
fn the_reference_rounds_twice_the_way_the_x87_does() {
    // The observing oracle for every witness below: the reference's two-step rounding is
    // not the one-step one. `1 + (2⁻⁵³ + 2⁻⁷⁸)` rounds up once and down twice.
    let little = f64::from_bits(0x3ca0_0000_0800_0000);
    assert_eq!(
        little,
        f64::EPSILON / 2.0 + f64::EPSILON / (1_u64 << 26) as f64
    );
    assert_eq!(soft::add(1.0, little), 1.0 + f64::EPSILON);
    assert_eq!(soft::add_via(1.0, little, soft::X87_EXTENDED), 1.0);
    assert_eq!(
        soft::add_via(1.0, little, soft::X87_DOUBLE),
        1.0 + f64::EPSILON
    );
}

#[test]
fn every_binary64_witness_rounds_twice_through_the_register_and_once_under_the_scope() {
    for witness in &WITNESSES_64 {
        let once = witness.via(soft::BINARY64);
        assert_eq!(once.to_bits(), witness.expected, "{}", witness.name);
        // Rounded to the x87's 64 bits and then to binary64, the witness gives other bits.
        assert_ne!(
            witness.via(soft::X87_EXTENDED).to_bits(),
            witness.expected,
            "{}: rounded twice through 64 bits",
            witness.name
        );
        // At 53-bit precision control the normal-range witnesses are closed and the
        // subnormal-range ones are not: only the scaling closes those.
        let subnormal = !once.is_normal();
        assert_eq!(
            witness.via(soft::X87_DOUBLE).to_bits() == witness.expected,
            !subnormal,
            "{}: precision control at 53 bits closes exactly the normal-range rows",
            witness.name
        );
        // And this unit, through the layer, gives the IEEE result.
        assert_eq!(
            witness.observe().to_bits(),
            witness.expected,
            "{}",
            witness.name
        );
    }
}

#[test]
fn every_binary32_witness_rounds_twice_at_24_bits_and_once_under_the_scope() {
    for witness in &WITNESSES_32 {
        let once = witness.via(soft::BINARY32);
        assert_eq!(once.to_bits(), witness.expected, "{}", witness.name);
        assert!(!once.is_normal(), "{}: a subnormal result", witness.name);
        // Rounded to 24 bits over the x87's exponent and then stored, the result differs:
        // this is what the x87 at 24-bit precision does without the scaling.
        assert_ne!(
            witness.via(soft::X87_SINGLE).to_bits(),
            witness.expected,
            "{}: rounded twice through 24 bits",
            witness.name
        );
        // Through 53 or 64 bits it does not: a product of two binary32 values is exact in
        // 48 bits, and a quotient rounded to 53 or more bits cannot land on a binary32
        // midpoint it is not on. The scope's 24 bits are what need the scaling.
        for first in [soft::X87_DOUBLE, soft::X87_EXTENDED] {
            assert_eq!(
                witness.via(first).to_bits(),
                witness.expected,
                "{}",
                witness.name
            );
        }
        assert_eq!(
            witness.observe().to_bits(),
            witness.expected,
            "{}",
            witness.name
        );
    }
}

// ---- the operand fixtures ----------------------------------------------------------------

/// A binary64 of the adversarial class `class`: every exponent, subnormals, values near
/// one (where ties are dense), and powers of two.
fn operand64(stream: &mut Stream, class: u64) -> f64 {
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

/// A binary32 of the adversarial class `class`, as [`operand64`].
fn operand32(stream: &mut Stream, class: u64) -> f32 {
    let bits = stream.next_u64();
    let sign = u32::from(bits >> 63 == 1) << 31;
    let fraction = u32::try_from(bits & ((1 << 23) - 1)).expect("23 bits");
    let biased = u32::try_from(match class {
        0 => (bits >> 40) % 0xff,
        1 => 0,
        2 => 127 + (bits >> 40) % 2,
        3 => 127 - 63 - (bits >> 40) % 16,
        4 => 127 + 63 - (bits >> 40) % 16,
        _ => 127 + (bits >> 40) % 24,
    })
    .expect("eight bits");
    let fraction = if class == 2 || class == 5 {
        fraction & 0x78_0007
    } else {
        fraction
    };
    f32::from_bits(sign | (biased << 23) | fraction)
}

/// Bits, with every NaN collapsed to one pattern: no result exposes a NaN's payload.
fn canonical64(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

fn canonical32(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

#[test]
fn binary64_operations_equal_the_software_reference() {
    // On an IEEE binary64 unit this is the reference proven against the hardware; on the
    // x87 it is the scope and the scaling proven against the reference.
    let mut stream = Stream(0x5eed_b64f);
    let mut pairs: Vec<(f64, f64)> = (0..60_000_u64)
        .map(|index| {
            (
                operand64(&mut stream, index % 6),
                operand64(&mut stream, (index / 6) % 6),
            )
        })
        .collect();
    pairs.extend(WITNESSES_64.iter().map(Witness64::operands));
    pairs.extend([
        (f64::MAX, 2.0),
        (f64::MAX, f64::MAX),
        (f64::MIN_POSITIVE, 0.5),
        (f64::from_bits(1), 0.5),
        (f64::from_bits(1), f64::from_bits(1)),
        (1.0, 0.0),
        (0.0, 0.0),
        (-0.0, 0.0),
        (f64::INFINITY, -1.0),
        (f64::NAN, 1.0),
    ]);
    let scope = Binary64Scope::enter();
    let ops = scope.ops();
    let mut compared = 0_usize;
    for (a, b) in pairs {
        let (x, y) = (black_box(a), black_box(b));
        let cases = [
            ("+", ops.add(x, y), soft::add(a, b)),
            ("-", ops.sub(x, y), soft::sub(a, b)),
            ("*", ops.mul(x, y), soft::mul(a, b)),
            ("/", ops.div(x, y), soft::div(a, b)),
            ("sqrt", ops.sqrt(x.abs()), soft::sqrt(a.abs())),
            ("algebraic +", ops.algebraic_add(x, y), soft::add(a, b)),
            ("algebraic -", ops.algebraic_sub(x, y), soft::sub(a, b)),
            ("algebraic *", ops.algebraic_mul(x, y), soft::mul(a, b)),
            // The one-off functions enter a nested scope of their own.
            ("f64_add", f64_add(x, y), soft::add(a, b)),
            ("f64_sub", f64_sub(x, y), soft::sub(a, b)),
            ("f64_mul", f64_mul(x, y), soft::mul(a, b)),
            ("f64_div", f64_div(x, y), soft::div(a, b)),
            ("f64_sqrt", f64_sqrt(x.abs()), soft::sqrt(a.abs())),
        ];
        for (name, got, want) in cases {
            let (got, want) = (canonical64(got), canonical64(want));
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
    assert!(compared >= 780_000, "{compared} comparisons");
}

#[test]
fn binary32_operations_equal_the_software_reference() {
    let mut stream = Stream(0x5eed_b32f);
    let mut pairs: Vec<(f32, f32)> = (0..60_000_u64)
        .map(|index| {
            (
                operand32(&mut stream, index % 6),
                operand32(&mut stream, (index / 6) % 6),
            )
        })
        .collect();
    pairs.extend(WITNESSES_32.iter().map(Witness32::operands));
    pairs.extend([
        (f32::MAX, 2.0),
        (f32::MAX, f32::MAX),
        (f32::MIN_POSITIVE, 0.5),
        (f32::from_bits(1), 0.5),
        (f32::from_bits(1), f32::from_bits(1)),
        (1.0, 0.0),
        (0.0, 0.0),
        (-0.0, 0.0),
        (f32::INFINITY, -1.0),
        (f32::NAN, 1.0),
        // `1 + (2⁻²⁴ + 2⁻⁴⁷)`: just above a binary32 midpoint.
        (1.0, f32::from_bits(0x3380_0001)),
    ]);
    let scope = Binary32Scope::enter();
    let ops = scope.ops();
    let mut compared = 0_usize;
    for (a, b) in pairs {
        let (x, y) = (black_box(a), black_box(b));
        let cases = [
            ("+", ops.add(x, y), soft::add32(a, b)),
            ("-", ops.sub(x, y), soft::sub32(a, b)),
            ("*", ops.mul(x, y), soft::mul32(a, b)),
            ("/", ops.div(x, y), soft::div32(a, b)),
            ("sqrt", ops.sqrt(x.abs()), soft::sqrt32(a.abs())),
            ("f32_add", f32_add(x, y), soft::add32(a, b)),
            ("f32_sub", f32_sub(x, y), soft::sub32(a, b)),
            ("f32_mul", f32_mul(x, y), soft::mul32(a, b)),
            ("f32_div", f32_div(x, y), soft::div32(a, b)),
            ("f32_sqrt", f32_sqrt(x.abs()), soft::sqrt32(a.abs())),
        ];
        for (name, got, want) in cases {
            let (got, want) = (canonical32(got), canonical32(want));
            assert_eq!(
                got,
                want,
                "{a:e} {name} {b:e} ({:#010x}, {:#010x}): {got:#010x} where the reference \
                 gives {want:#010x}",
                a.to_bits(),
                b.to_bits()
            );
            compared += 1;
        }
    }
    assert!(compared >= 600_000, "{compared} comparisons");
}

#[test]
fn binary32_subnormal_products_and_quotients_equal_the_software_reference() {
    // Dense in the range the scaling exists for: operands whose product or quotient lands
    // within a few binades of binary32's smallest normal, both signs, all significands.
    let mut stream = Stream(0x5eed_5b32);
    let scope = Binary32Scope::enter();
    let ops = scope.ops();
    let mut subnormal = 0_usize;
    for _ in 0..200_000 {
        let bits = stream.next_u64();
        let fraction = |shift: u32| u32::try_from((bits >> shift) & 0x7f_ffff).expect("23 bits");
        // `a` = 2^(−40 + e)·m, `b` = 2^(−100 − e + k)·m′, `c` = 2^(100 + e − k)·m″: the
        // product and the quotient both have exponent −140 + k, from 29 binades below the
        // smallest normal to 15 above it.
        let e = u32::try_from((bits >> 50) % 20).expect("small");
        let k = u32::try_from((bits >> 56) % 30).expect("small");
        let sign = u32::from(bits >> 63 == 1) << 31;
        let a = f32::from_bits(sign | ((127 - 40 + e) << 23) | fraction(0));
        let b = f32::from_bits(((27 - e + k) << 23) | fraction(23));
        let c = f32::from_bits(((227 + e - k) << 23) | fraction(23));
        let (x, y, z) = (black_box(a), black_box(b), black_box(c));
        let product = ops.mul(x, y);
        assert_eq!(
            product.to_bits(),
            soft::mul32(a, b).to_bits(),
            "{a:e} * {b:e}"
        );
        let quotient = ops.div(x, z);
        assert_eq!(
            quotient.to_bits(),
            soft::div32(a, c).to_bits(),
            "{a:e} / {c:e}"
        );
        subnormal += usize::from(product.is_subnormal()) + usize::from(quotient.is_subnormal());
    }
    assert!(subnormal > 100_000, "{subnormal} subnormal results");
}

#[test]
fn square_roots_that_round_twice_through_64_bits_round_once_here() {
    // About one binary64 in four thousand has a square root whose 64-bit rounding lands on
    // a binary64 midpoint; at both widths, every such root is the reference's.
    let mut stream = Stream(0x5eed_5a47);
    let mut witnesses = 0_usize;
    for _ in 0..200_000 {
        let bits = stream.next_u64();
        let x = f64::from_bits(
            (bits & 0x000f_ffff_ffff_ffff) | (0x3ff0_0000_0000_0000 + ((bits >> 63) << 52)),
        );
        let once = soft::sqrt(x);
        if soft::sqrt_via(x, soft::X87_EXTENDED).to_bits() != once.to_bits() {
            witnesses += 1;
            assert_eq!(
                f64_sqrt(black_box(x)).to_bits(),
                once.to_bits(),
                "sqrt {x:e}"
            );
        }
        let y = f32::from_bits(u32::try_from(bits >> 41).expect("23 bits") | 0x3f80_0000);
        assert_eq!(
            f32_sqrt(black_box(y)).to_bits(),
            soft::sqrt32(y).to_bits(),
            "sqrt {y:e}"
        );
    }
    assert!(witnesses > 10, "{witnesses} square-root witnesses");
}

// ---- the x87 -------------------------------------------------------------------------------

#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
#[allow(
    unsafe_code,
    reason = "the tests load a caller's control word, as a caller that changed it would"
)]
mod x87 {
    use core::hint::black_box;

    use super::super::x87::{
        self, PRECISION_24, PRECISION_53, PRECISION_64, PRECISION_CONTROL, div32_unscaled,
        div64_unscaled, mul32_unscaled, mul64_unscaled,
    };
    use super::super::{Binary32Scope, Binary64Scope, f32_div, f64_add};
    use super::{Op, WITNESSES_32, WITNESSES_64, soft};

    /// The control word loaded for as long as the guard lives, restored when it drops.
    struct Loaded(u16);

    impl Loaded {
        fn load(word: u16) -> Self {
            let saved = x87::control_word();
            // SAFETY: only the precision or rounding field differs from this thread's
            // word, and `Drop` restores it.
            unsafe { x87::load_control_word(word) };
            Self(saved)
        }
    }

    impl Drop for Loaded {
        fn drop(&mut self) {
            // SAFETY: restores the word this thread had.
            unsafe { x87::load_control_word(self.0) };
        }
    }

    fn precision() -> u16 {
        x87::control_word() & PRECISION_CONTROL
    }

    #[test]
    fn each_scope_sets_its_precision_and_restores_the_callers() {
        let saved = x87::control_word();
        assert_eq!(
            saved & PRECISION_CONTROL,
            PRECISION_64,
            "Linux starts at 64 bits"
        );
        for caller in [PRECISION_24, PRECISION_53, PRECISION_64] {
            let _loaded = Loaded::load((saved & !PRECISION_CONTROL) | caller);
            let before = x87::control_word();
            {
                let _outer = Binary64Scope::enter();
                assert_eq!(precision(), PRECISION_53);
                assert_eq!(
                    x87::control_word() & !PRECISION_CONTROL,
                    before & !PRECISION_CONTROL,
                    "no other field changes"
                );
                {
                    let _inner = Binary32Scope::enter();
                    assert_eq!(precision(), PRECISION_24);
                    {
                        let _innermost = Binary64Scope::enter();
                        assert_eq!(precision(), PRECISION_53);
                    }
                    assert_eq!(precision(), PRECISION_24, "nested scopes restore");
                }
                assert_eq!(precision(), PRECISION_53, "nested scopes restore");
            }
            assert_eq!(x87::control_word(), before, "the caller's word is restored");
            for unwinding in [true, false] {
                let unwound = std::panic::catch_unwind(|| {
                    if unwinding {
                        let _scope = Binary64Scope::enter();
                        panic!("unwinding through the scope");
                    }
                    let _scope = Binary32Scope::enter();
                    panic!("unwinding through the scope");
                });
                assert!(unwound.is_err());
                assert_eq!(x87::control_word(), before, "restored when a panic unwinds");
            }
        }
        assert_eq!(x87::control_word(), saved);
    }

    #[test]
    fn without_the_scope_the_x87_rounds_the_binary64_witnesses_twice() {
        // The refused case: the plain operators at the thread's 64-bit precision give the
        // doubly rounded bits the reference predicts for every witness; through the layer
        // (the valid neighbour, same thread, same operands) each gives the IEEE bits.
        for witness in &WITNESSES_64 {
            let (a, b) = witness.operands();
            let (x, y) = (black_box(a), black_box(b));
            let plain = black_box(match witness.op {
                Op::Add => x + y,
                Op::Mul => x * y,
                Op::Div => x / y,
            });
            assert_eq!(
                plain.to_bits(),
                witness.via(soft::X87_EXTENDED).to_bits(),
                "{}: the unguarded x87 is the reference's double rounding",
                witness.name
            );
            assert_ne!(plain.to_bits(), witness.expected, "{}", witness.name);
            assert_eq!(
                witness.observe().to_bits(),
                witness.expected,
                "{}",
                witness.name
            );
        }
    }

    #[test]
    fn without_the_scaling_the_subnormal_witnesses_round_twice() {
        // Precision control alone closes the normal range and leaves the subnormal one:
        // the unscaled product and quotient round twice, exactly as the reference models
        // the register format; the scaled operations do not.
        {
            let _scope = Binary64Scope::enter();
            for (witness, unscaled) in [
                (&WITNESSES_64[3], mul64_unscaled as fn(f64, f64) -> f64),
                (&WITNESSES_64[4], div64_unscaled),
            ] {
                let (a, b) = witness.operands();
                let twice = unscaled(black_box(a), black_box(b));
                assert_ne!(twice.to_bits(), witness.expected, "{}", witness.name);
                assert_eq!(
                    twice.to_bits(),
                    witness.via(soft::X87_DOUBLE).to_bits(),
                    "{}",
                    witness.name
                );
            }
        }
        {
            let _scope = Binary32Scope::enter();
            for (witness, unscaled) in [
                (&WITNESSES_32[0], mul32_unscaled as fn(f32, f32) -> f32),
                (&WITNESSES_32[1], div32_unscaled),
            ] {
                let (a, b) = witness.operands();
                let twice = unscaled(black_box(a), black_box(b));
                assert_ne!(twice.to_bits(), witness.expected, "{}", witness.name);
                assert_eq!(
                    twice.to_bits(),
                    witness.via(soft::X87_SINGLE).to_bits(),
                    "{}",
                    witness.name
                );
            }
        }
        for witness in &WITNESSES_64 {
            assert_eq!(
                witness.observe().to_bits(),
                witness.expected,
                "{}",
                witness.name
            );
        }
        for witness in &WITNESSES_32 {
            assert_eq!(
                witness.observe().to_bits(),
                witness.expected,
                "{}",
                witness.name
            );
        }
    }

    #[test]
    fn a_callers_precision_is_overridden_not_inherited() {
        // A caller that left the x87 at 24 bits (as some graphics runtimes do) is served
        // correctly rounded binary64, and one at 64 or 53 bits correctly rounded binary32:
        // each operation sets the precision its format needs, and gives the caller's back.
        let saved = x87::control_word();
        for caller in [PRECISION_24, PRECISION_53, PRECISION_64] {
            let word = (saved & !PRECISION_CONTROL) | caller;
            let _loaded = Loaded::load(word);
            for witness in &WITNESSES_64 {
                assert_eq!(
                    witness.observe().to_bits(),
                    witness.expected,
                    "{}",
                    witness.name
                );
            }
            for witness in &WITNESSES_32 {
                assert_eq!(
                    witness.observe().to_bits(),
                    witness.expected,
                    "{}",
                    witness.name
                );
            }
            let little = f64::from_bits(0x3ca0_0000_0800_0000);
            assert_eq!(f64_add(black_box(1.0), little), 1.0 + f64::EPSILON);
            let (a, b) = WITNESSES_32[1].operands();
            assert_eq!(f32_div(black_box(a), b).to_bits(), WITNESSES_32[1].expected);
            assert_eq!(
                x87::control_word(),
                word,
                "the caller's word is theirs again"
            );
        }
    }
}
