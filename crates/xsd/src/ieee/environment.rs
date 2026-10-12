// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The floating-point environment every arithmetic assumes, proven by behaviour on every
//! target.
//!
//! IEEE-754 fixes one result for each correctly rounded operation, but only under the
//! environment it assumes: round-to-nearest, ties-to-even, with subnormals preserved.
//! A thread can leave that environment. On `x86_64` the MXCSR register's flush-to-zero
//! (FTZ, bit 15) and denormals-are-zero (DAZ, bit 6) bits replace subnormal results and
//! operands with zero, and its rounding-control field (bits 13–14) selects another
//! direction; on `aarch64` the FPCR's flush-to-zero bit (FZ, bit 24), its
//! input-flush bit (FIZ, bit 0, where implemented) and its rounding-mode field
//! (bits 22–23) do the same; other architectures have their own controls (the RISC-V
//! `frm` field, the 32-bit Arm FPSCR, the POWER FPSCR). A shared library loaded with
//! fast-math startup code sets them for the thread that loads it and every thread it
//! spawns afterwards. And one unit does not round binary64 once at all: the x87, which
//! runs `f64` arithmetic in a 32-bit `x86` build without SSE2, rounds each result to the
//! width its precision-control field selects and again when it is stored, which the
//! arithmetics close by computing through [`Binary64`] under a [`Binary64Scope`] guard
//! (see [`super`]).
//!
//! Under such an environment the same source computes different bits, and nothing in the
//! result says so. So the environment is established by what it DOES: a probe of
//! thirteen binary64 operations whose correctly rounded results under the assumed
//! environment are known constants, each chosen so that one departure from it changes
//! the bits. The probe computes through the same [`Binary64`] operations, under the same
//! [`Binary64Scope`] guard, as every kernel, so it proves the arithmetic the kernels run
//! rather than some other. Operands pass through [`core::hint::black_box`], so the
//! compiler cannot fold an operation into the constant it would give under its own
//! (default) environment; each operation runs on the thread's floating-point unit, under
//! whatever that thread has set.
//!
//! | operation (`ulp` = 2⁻⁵², the spacing above 1) | assumed result | departure it exposes |
//! |---|---|---|
//! | `f64::MIN_POSITIVE × 0.5` | the subnormal 2⁻¹⁰²³ | a subnormal result flushed to zero |
//! | `2⁻¹⁰⁷⁴ × 2¹⁰²³` | the normal 2⁻⁵¹ | a subnormal operand read as zero |
//! | `1 + 0.75 ulp` | `1 + ulp` | rounding toward zero or downward |
//! | `1 + 0.25 ulp` | `1` | rounding upward |
//! | `−1 − 0.75 ulp` | `−(1 + ulp)` | rounding toward zero or upward |
//! | `−1 − 0.25 ulp` | `−1` | rounding downward |
//! | `1 + 0.5 ulp` | `1` | a tie rounded away from zero, or upward |
//! | `(1 + ulp) + 0.5 ulp` | `1 + 2 ulp` | a tie rounded toward zero, or downward |
//! | `1 + (2⁻⁵³ + 2⁻⁷⁸)` | `1 + ulp` | a sum rounded twice, landing on a tie |
//! | `(1 + ulp) + (2⁻⁵³ − 2⁻⁷⁸)` | `1 + ulp` | a sum rounded twice, landing on a tie |
//! | `(1 − 0.5 ulp) × (1 + 2 ulp)` | `1 + ulp` | a product rounded twice, landing on a tie |
//! | `2⁻⁵¹²(1 + 4 ulp) × 2⁻⁵¹¹(1 − ulp)` | the subnormal `2⁻¹⁰²³ + 2⁻¹⁰⁷⁴` | a subnormal product rounded twice |
//! | `2⁻⁵¹² ÷ 2⁵¹¹(1 − ulp)` | the subnormal `2⁻¹⁰²³ + 2⁻¹⁰⁷⁴` | a subnormal quotient rounded twice |
//!
//! Every directed mode fails at least one row of each sign, and a nearest mode that
//! breaks ties other than to even fails one of the two tie rows, so the table
//! distinguishes round-to-nearest-even from every other rounding rule.
//!
//! The last five rows are double-rounding witnesses. The exact result of each lies
//! within a 64-bit rounding of a binary64 midpoint without being one: rounded once, to
//! 53 bits, it goes to the nearer neighbour; rounded first to the x87's 64 bits it
//! lands on the midpoint, and the tie then goes to the even neighbour, which is the other
//! one. The first three are in binary64's normal range, where precision control at 53
//! bits closes the gap; the last two are in its subnormal range, where only the scaling
//! [`Binary64`] performs on the x87 closes it, since a 53-bit result is rounded again by
//! the store. Every other unit rounds once and passes all five. The probe runs in binary64
//! because that is the only precision the arithmetics execute in: every operand is
//! widened to binary64 per component (the distance kernel’s scalar widening) before any operation,
//! so no binary32 operation is ever performed and none needs probing. On 32-bit Arm this
//! also settles which unit is proven: NEON has no binary64 lanes, so binary64 runs on the
//! VFP, which honours the FPSCR the probe exercises.
//!
//! Where the control register CAN be read — `x86_64`, SSE2 `x86`, `aarch64`, and the x87 control
//! word where the x87 is the binary64 unit — it is read first, so a refusal names the
//! register and its value, and the probe then runs as well: a register read proves only
//! the bits this build knows the meaning of, and the probe proves the environment
//! itself. The x87 control word is refused for its rounding-control field only; its
//! precision-control field is the [`Binary64Scope`] guard's to set, and the probe's
//! double-rounding rows prove that it did. On WebAssembly the numeric instructions are specified
//! to round to nearest-even and preserve subnormals, and there is no register to set
//! otherwise; the probe runs there too, because it costs thirteen operations and an engine
//! that departed from the specification would be refused by name rather than trusted.

use core::fmt;
use core::hint::black_box;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86"))]
use super::control;
use super::{Binary64, Binary64Scope};

/// What a refusal of the floating-point environment observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FloatEnvironmentEvidence {
    /// The thread's floating-point control register, read directly.
    Register {
        /// The register read: `"MXCSR"`, `"FPCR"` or `"x87 control word"`.
        name: &'static str,
        /// The register's full value, as read.
        bits: u64,
    },
    /// A binary64 operation whose correctly rounded result under IEEE-754
    /// round-to-nearest-even with subnormals preserved is a known constant returned
    /// other bits on this thread.
    Probe {
        /// The operation performed, in words (`ulp` is 2⁻⁵², the spacing above 1).
        operation: &'static str,
        /// The bits IEEE-754 defines for it.
        expected: u64,
        /// The bits this thread produced.
        observed: u64,
    },
}

impl fmt::Display for FloatEnvironmentEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Register { name, bits } => write!(f, "its {name} reads {bits:#x}"),
            Self::Probe {
                operation,
                expected,
                observed,
            } => write!(
                f,
                "`{operation}` gave {observed:#018x} where IEEE-754 gives {expected:#018x}"
            ),
        }
    }
}

/// Why the current thread's floating-point environment cannot run an arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FloatEnvironmentError {
    /// Hardware exceptions are enabled, so the required probes or arithmetic
    /// could trap instead of returning a value or typed error.
    TrapsEnabled {
        /// The control register whose exception enables were observed.
        evidence: FloatEnvironmentEvidence,
    },
    /// The thread flushes subnormal results, or reads subnormal operands, as zero.
    FlushToZero {
        /// What showed it.
        evidence: FloatEnvironmentEvidence,
    },
    /// The thread rounds by a rule other than round-to-nearest, ties-to-even.
    RoundingMode {
        /// What showed it.
        evidence: FloatEnvironmentEvidence,
    },
    /// The thread rounds a binary64 result twice, first to a wider format and then to
    /// binary64: the x87's register precision, where the guard that sets it to binary64's
    /// did not take hold.
    DoubleRounding {
        /// What showed it.
        evidence: FloatEnvironmentEvidence,
    },
}

impl fmt::Display for FloatEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrapsEnabled { evidence } => write!(
                f,
                "the thread enables floating-point exception traps ({evidence}); numerical \
                 evaluation is refused before the probes execute"
            ),
            Self::FlushToZero { evidence } => write!(
                f,
                "the thread flushes subnormals to zero ({evidence}); binary64 values \
                 computed under it differ from the IEEE-754 results the arithmetic defines, \
                 so they are refused rather than returned"
            ),
            Self::RoundingMode { evidence } => write!(
                f,
                "the thread rounds other than to nearest, ties to even ({evidence}); binary64 \
                 numerical values computed under it differ from the IEEE-754 results the arithmetic \
                 defines, so they are refused rather than returned"
            ),
            Self::DoubleRounding { evidence } => write!(
                f,
                "the thread rounds binary64 results twice, through a wider format \
                 ({evidence}); binary64 values computed under it differ from the IEEE-754 \
                 results the arithmetic defines, so they are refused rather than returned"
            ),
        }
    }
}

impl std::error::Error for FloatEnvironmentError {}

/// Which refusal a probe row's failure is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub enum Departure {
    /// A subnormal result or operand became zero.
    Flush,
    /// A rounding rule other than round-to-nearest-even.
    Rounding,
    /// A result rounded twice, through a wider format.
    DoubleRounding,
}

/// The binary64 operation a probe row performs.
#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Mul,
    Div,
}

/// One probe row: an operation on two operands and the bits IEEE-754 defines for it.
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct Probe {
    /// The operation, in words.
    pub operation: &'static str,
    /// Which refusal a mismatch is.
    pub departure: Departure,
    op: Op,
    lhs: u64,
    rhs: u64,
    /// The correctly rounded result under the assumed environment.
    pub expected: u64,
}

impl Probe {
    /// Perform the operation on this thread's floating-point unit, through the kernels'
    /// own `ops`, and return its bits.
    ///
    /// Both operands pass through [`black_box`], so the operation is executed at run time
    /// under the thread's environment rather than folded at compile time under the
    /// compiler's.
    #[inline]
    pub fn observe(&self, ops: Binary64<'_>) -> u64 {
        let lhs = black_box(f64::from_bits(self.lhs));
        let rhs = black_box(f64::from_bits(self.rhs));
        let result = match self.op {
            Op::Add => ops.add(lhs, rhs),
            Op::Mul => ops.mul(lhs, rhs),
            Op::Div => ops.div(lhs, rhs),
        };
        black_box(result).to_bits()
    }

    /// The operands, as binary64 values.
    pub const fn operands(&self) -> (f64, f64) {
        (f64::from_bits(self.lhs), f64::from_bits(self.rhs))
    }

    /// The operation's symbol, `'+'`, `'*'` or `'/'`.
    pub const fn symbol(&self) -> char {
        match self.op {
            Op::Add => '+',
            Op::Mul => '*',
            Op::Div => '/',
        }
    }

    /// This row's refusal, if the thread's result differs from the defined one.
    fn refusal(&self, ops: Binary64<'_>) -> Option<FloatEnvironmentError> {
        let observed = self.observe(ops);
        if observed == self.expected {
            return None;
        }
        let evidence = FloatEnvironmentEvidence::Probe {
            operation: self.operation,
            expected: self.expected,
            observed,
        };
        Some(match self.departure {
            Departure::Flush => FloatEnvironmentError::FlushToZero { evidence },
            Departure::Rounding => FloatEnvironmentError::RoundingMode { evidence },
            Departure::DoubleRounding => FloatEnvironmentError::DoubleRounding { evidence },
        })
    }
}

/// The sign bit of a binary64.
const SIGN: u64 = 1 << 63;
/// `1`.
const ONE: u64 = 0x3ff0_0000_0000_0000;
/// `1 + ulp`: the successor of 1.
const ONE_PLUS_ULP: u64 = ONE + 1;
/// `1 + 2 ulp`: the successor of `1 + ulp`, whose last significand bit is even.
const ONE_PLUS_TWO_ULP: u64 = ONE + 2;
/// `0.25 ulp` = 2⁻⁵⁴.
const QUARTER_ULP: u64 = 0x3c90_0000_0000_0000;
/// `0.5 ulp` = 2⁻⁵³.
const HALF_ULP: u64 = 0x3ca0_0000_0000_0000;
/// `0.75 ulp` = 3 · 2⁻⁵⁴.
const THREE_QUARTER_ULP: u64 = 0x3ca8_0000_0000_0000;
/// `0.5`.
const HALF: u64 = 0x3fe0_0000_0000_0000;
/// The smallest positive normal binary64, 2⁻¹⁰²².
const MIN_NORMAL: u64 = 0x0010_0000_0000_0000;
/// Half of it, 2⁻¹⁰²³: a subnormal.
const HALF_MIN_NORMAL: u64 = 0x0008_0000_0000_0000;
/// The smallest positive subnormal binary64, 2⁻¹⁰⁷⁴.
const MIN_SUBNORMAL: u64 = 0x0000_0000_0000_0001;
/// 2¹⁰²³, the largest power of two.
const MAX_POWER_OF_TWO: u64 = 0x7fe0_0000_0000_0000;
/// 2⁻⁵¹ = 2⁻¹⁰⁷⁴ · 2¹⁰²³: a normal.
const TWO_POW_MINUS_51: u64 = 0x3cc0_0000_0000_0000;
/// 2⁻⁷⁸ = 2⁻²⁶ ulp: far below a 64-bit significand's spacing above 1 (2⁻⁶³).
const TWO_POW_MINUS_78: u64 = (1023 - 78) << 52;
/// `2⁻⁵³ + 2⁻⁷⁸`: half an ulp and a little more.
const HALF_ULP_AND_A_LITTLE: u64 = 0x3ca0_0000_0800_0000;
/// `2⁻⁵³ − 2⁻⁷⁸`: half an ulp and a little less.
const HALF_ULP_LESS_A_LITTLE: u64 = 0x3c9f_ffff_f000_0000;
/// `1 − 0.5 ulp` = 1 − 2⁻⁵³: the predecessor of 1.
const ONE_LESS_HALF_ULP: u64 = ONE - 1;
/// 2⁻⁵¹²(1 + 4 ulp).
const SUBNORMAL_PRODUCT_LHS: u64 = ((1023 - 512) << 52) + 4;
/// 2⁻⁵¹¹(1 − ulp) = 2⁻⁵¹²(2 − 2 ulp): the largest binary64 below 2⁻⁵¹¹.
const SUBNORMAL_PRODUCT_RHS: u64 = ((1023 - 511) << 52) - 2;
/// 2⁻⁵¹².
const TWO_POW_MINUS_512: u64 = (1023 - 512) << 52;
/// 2⁵¹¹(1 − ulp) = 2⁵¹⁰(2 − 2 ulp).
const SUBNORMAL_QUOTIENT_DIVISOR: u64 = ((1023 + 511) << 52) - 2;
/// 2⁻¹⁰²³ + 2⁻¹⁰⁷⁴: the subnormal just above half the smallest normal, whose last
/// significand bit is odd.
const HALF_MIN_NORMAL_PLUS_ONE: u64 = HALF_MIN_NORMAL + 1;

// Each constant is the value its name says, checked at compile time against the
// definition written as arithmetic. Constant evaluation is IEEE round-to-nearest-even
// with subnormals preserved, and these operations are exact in any case.
const _: () = {
    assert!(f64::from_bits(ONE) == 1.0);
    assert!(f64::from_bits(ONE_PLUS_ULP) == 1.0 + f64::EPSILON);
    assert!(f64::from_bits(ONE_PLUS_TWO_ULP) == 1.0 + f64::EPSILON + f64::EPSILON);
    assert!(f64::from_bits(QUARTER_ULP) == f64::EPSILON / 4.0);
    assert!(f64::from_bits(HALF_ULP) == f64::EPSILON / 2.0);
    assert!(f64::from_bits(THREE_QUARTER_ULP) == 3.0 * f64::EPSILON / 4.0);
    assert!(f64::from_bits(HALF) == 0.5);
    assert!(MIN_NORMAL == f64::MIN_POSITIVE.to_bits());
    assert!(f64::from_bits(HALF_MIN_NORMAL) == f64::MIN_POSITIVE / 2.0);
    assert!(!f64::from_bits(HALF_MIN_NORMAL).is_normal());
    assert!(!f64::from_bits(MIN_SUBNORMAL).is_normal());
    // A zero significand under biased exponent `e` is exactly 2^(e - 1023).
    assert!(MAX_POWER_OF_TWO == (1023 + 1023) << 52);
    assert!(TWO_POW_MINUS_51 == (1023 - 51) << 52);
    assert!(f64::from_bits(TWO_POW_MINUS_51) == 2.0 * f64::EPSILON);
    assert!(f64::from_bits(TWO_POW_MINUS_51).is_normal());
    // The double-rounding operands, each the arithmetic its name says (every operation
    // below is exact).
    assert!(f64::from_bits(TWO_POW_MINUS_78) == f64::EPSILON / (1_u64 << 26) as f64);
    assert!(
        f64::from_bits(HALF_ULP_AND_A_LITTLE)
            == f64::from_bits(HALF_ULP) + f64::from_bits(TWO_POW_MINUS_78)
    );
    assert!(
        f64::from_bits(HALF_ULP_LESS_A_LITTLE)
            == f64::from_bits(HALF_ULP) - f64::from_bits(TWO_POW_MINUS_78)
    );
    assert!(f64::from_bits(ONE_LESS_HALF_ULP) == 1.0 - f64::EPSILON / 2.0);
    assert!(
        f64::from_bits(SUBNORMAL_PRODUCT_LHS)
            == (1.0 + (f64::EPSILON + f64::EPSILON + f64::EPSILON + f64::EPSILON))
                * f64::from_bits(TWO_POW_MINUS_512)
    );
    assert!(
        f64::from_bits(SUBNORMAL_PRODUCT_RHS)
            == (1.0 - f64::EPSILON) * (2.0 * f64::from_bits(TWO_POW_MINUS_512))
    );
    assert!(
        f64::from_bits(SUBNORMAL_QUOTIENT_DIVISOR)
            == (1.0 - f64::EPSILON) * f64::from_bits((1023 + 511) << 52)
    );
    assert!(
        f64::from_bits(HALF_MIN_NORMAL_PLUS_ONE)
            == f64::MIN_POSITIVE / 2.0 + f64::from_bits(MIN_SUBNORMAL)
    );
    assert!(!f64::from_bits(HALF_MIN_NORMAL_PLUS_ONE).is_normal());
    // Their correctly rounded results: constant evaluation rounds once, to nearest-even,
    // with subnormals preserved, which is the result each row expects.
    assert!(
        f64::from_bits(ONE) + f64::from_bits(HALF_ULP_AND_A_LITTLE) == f64::from_bits(ONE_PLUS_ULP)
    );
    assert!(
        f64::from_bits(ONE_PLUS_ULP) + f64::from_bits(HALF_ULP_LESS_A_LITTLE)
            == f64::from_bits(ONE_PLUS_ULP)
    );
    assert!(
        f64::from_bits(ONE_LESS_HALF_ULP) * f64::from_bits(ONE_PLUS_TWO_ULP)
            == f64::from_bits(ONE_PLUS_ULP)
    );
    assert!(
        f64::from_bits(SUBNORMAL_PRODUCT_LHS) * f64::from_bits(SUBNORMAL_PRODUCT_RHS)
            == f64::from_bits(HALF_MIN_NORMAL_PLUS_ONE)
    );
    assert!(
        f64::from_bits(TWO_POW_MINUS_512) / f64::from_bits(SUBNORMAL_QUOTIENT_DIVISOR)
            == f64::from_bits(HALF_MIN_NORMAL_PLUS_ONE)
    );
};

/// The probe, in the order it is run: flushing first, then rounding, then double rounding.
#[doc(hidden)]
pub const PROBES: [Probe; 13] = [
    Probe {
        operation: "f64::MIN_POSITIVE * 0.5",
        departure: Departure::Flush,
        op: Op::Mul,
        lhs: MIN_NORMAL,
        rhs: HALF,
        expected: HALF_MIN_NORMAL,
    },
    Probe {
        operation: "2^-1074 * 2^1023",
        departure: Departure::Flush,
        op: Op::Mul,
        lhs: MIN_SUBNORMAL,
        rhs: MAX_POWER_OF_TWO,
        expected: TWO_POW_MINUS_51,
    },
    Probe {
        operation: "1 + 0.75 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: ONE,
        rhs: THREE_QUARTER_ULP,
        expected: ONE_PLUS_ULP,
    },
    Probe {
        operation: "1 + 0.25 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: ONE,
        rhs: QUARTER_ULP,
        expected: ONE,
    },
    Probe {
        operation: "-1 - 0.75 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: SIGN | ONE,
        rhs: SIGN | THREE_QUARTER_ULP,
        expected: SIGN | ONE_PLUS_ULP,
    },
    Probe {
        operation: "-1 - 0.25 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: SIGN | ONE,
        rhs: SIGN | QUARTER_ULP,
        expected: SIGN | ONE,
    },
    Probe {
        operation: "1 + 0.5 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: ONE,
        rhs: HALF_ULP,
        expected: ONE,
    },
    Probe {
        operation: "(1 + ulp) + 0.5 ulp",
        departure: Departure::Rounding,
        op: Op::Add,
        lhs: ONE_PLUS_ULP,
        rhs: HALF_ULP,
        expected: ONE_PLUS_TWO_ULP,
    },
    Probe {
        operation: "1 + (2^-53 + 2^-78)",
        departure: Departure::DoubleRounding,
        op: Op::Add,
        lhs: ONE,
        rhs: HALF_ULP_AND_A_LITTLE,
        expected: ONE_PLUS_ULP,
    },
    Probe {
        operation: "(1 + ulp) + (2^-53 - 2^-78)",
        departure: Departure::DoubleRounding,
        op: Op::Add,
        lhs: ONE_PLUS_ULP,
        rhs: HALF_ULP_LESS_A_LITTLE,
        expected: ONE_PLUS_ULP,
    },
    Probe {
        operation: "(1 - 0.5 ulp) * (1 + 2 ulp)",
        departure: Departure::DoubleRounding,
        op: Op::Mul,
        lhs: ONE_LESS_HALF_ULP,
        rhs: ONE_PLUS_TWO_ULP,
        expected: ONE_PLUS_ULP,
    },
    Probe {
        operation: "2^-512 (1 + 4 ulp) * 2^-511 (1 - ulp)",
        departure: Departure::DoubleRounding,
        op: Op::Mul,
        lhs: SUBNORMAL_PRODUCT_LHS,
        rhs: SUBNORMAL_PRODUCT_RHS,
        expected: HALF_MIN_NORMAL_PLUS_ONE,
    },
    Probe {
        operation: "2^-512 / (2^511 (1 - ulp))",
        departure: Departure::DoubleRounding,
        op: Op::Div,
        lhs: TWO_POW_MINUS_512,
        rhs: SUBNORMAL_QUOTIENT_DIVISOR,
        expected: HALF_MIN_NORMAL_PLUS_ONE,
    },
];

/// Refuse a thread whose binary64 operations do not return the IEEE-754 results: the
/// first probe row whose bits differ names the departure and what it saw.
///
/// The rows run inside a [`Binary64Scope`] guard, through its [`Binary64`] operations, exactly
/// as every kernel does.
///
/// # Errors
///
/// Returns the first behavioral departure from the required IEEE environment.
pub fn probe() -> Result<(), FloatEnvironmentError> {
    let scope = Binary64Scope::enter();
    probe_with(scope.ops())
}

/// Test the supplied operation scope against every behavioral probe.
///
/// # Errors
///
/// Returns the first observed departure from round-to-nearest-even with gradual
/// underflow and one rounding to binary64.
#[doc(hidden)]
pub fn probe_with(ops: Binary64<'_>) -> Result<(), FloatEnvironmentError> {
    // Behavioral probes deliberately diagnose flush/rounding departures, but
    // may never execute with exception traps enabled. Reuse the register home
    // while retaining behavioral evidence for the other departures.
    if let Err(error @ FloatEnvironmentError::TrapsEnabled { .. }) = register() {
        return Err(error);
    }
    PROBES
        .iter()
        .find_map(|row| row.refusal(ops))
        .map_or(Ok(()), Err)
}

/// MXCSR flush-to-zero.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const MXCSR_FTZ: u32 = 1 << 15;
/// MXCSR denormals-are-zero.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const MXCSR_DAZ: u32 = 1 << 6;
/// MXCSR rounding control; zero is round-to-nearest.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const MXCSR_RC: u32 = 0b11 << 13;

/// FPCR flush-to-zero.
#[cfg(target_arch = "aarch64")]
const FPCR_FZ: u64 = 1 << 24;
/// FPCR flush-inputs-to-zero (FEAT_AFP; reads as zero where unimplemented).
#[cfg(target_arch = "aarch64")]
const FPCR_FIZ: u64 = 1 << 0;
/// FPCR rounding mode; zero is round-to-nearest.
#[cfg(target_arch = "aarch64")]
const FPCR_RMODE: u64 = 0b11 << 22;

/// Refuse an MXCSR whose flush or rounding fields are set, naming the register.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn mxcsr_register() -> Result<(), FloatEnvironmentError> {
    let bits = control::mxcsr();
    let evidence = FloatEnvironmentEvidence::Register {
        name: "MXCSR",
        bits: u64::from(bits),
    };
    if bits & (0x3f << 7) != (0x3f << 7) {
        return Err(FloatEnvironmentError::TrapsEnabled { evidence });
    }
    if bits & (MXCSR_FTZ | MXCSR_DAZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero { evidence });
    }
    if bits & MXCSR_RC != 0 {
        return Err(FloatEnvironmentError::RoundingMode { evidence });
    }
    Ok(())
}

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
fn register() -> Result<(), FloatEnvironmentError> {
    mxcsr_register()
}

/// Supplement the x87 primary-unit check when a worker admits SIMD products.
/// The backend availability check has already proved SSE2 instruction support.
pub(crate) fn check_vector() -> Result<(), FloatEnvironmentError> {
    #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
    {
        mxcsr_register()?;
    }
    Ok(())
}

/// Refuse an FPCR whose flush or rounding fields are set, naming the register.
#[cfg(target_arch = "aarch64")]
fn register() -> Result<(), FloatEnvironmentError> {
    let bits = control::fpcr();
    let evidence = FloatEnvironmentEvidence::Register { name: "FPCR", bits };
    if bits & ((0x1f << 8) | (1 << 15)) != 0 {
        return Err(FloatEnvironmentError::TrapsEnabled { evidence });
    }
    if bits & (FPCR_FZ | FPCR_FIZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero { evidence });
    }
    if bits & FPCR_RMODE != 0 {
        return Err(FloatEnvironmentError::RoundingMode { evidence });
    }
    Ok(())
}

/// Refuse an x87 control word whose rounding-control field is set, naming the register.
///
/// Only where the x87 is this build's binary64 unit. Its precision-control field is not
/// refused: the [`Binary64Scope`] guard sets it for every operation, and the probe's
/// double-rounding rows prove the guard took hold.
#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
fn register() -> Result<(), FloatEnvironmentError> {
    use super::x87;

    let bits = x87::control_word();
    if bits & 0x3f != 0x3f {
        return Err(FloatEnvironmentError::TrapsEnabled {
            evidence: FloatEnvironmentEvidence::Register {
                name: "x87 control word",
                bits: u64::from(bits),
            },
        });
    }
    if bits & x87::ROUNDING_CONTROL != 0 {
        return Err(FloatEnvironmentError::RoundingMode {
            evidence: FloatEnvironmentEvidence::Register {
                name: "x87 control word",
                bits: u64::from(bits),
            },
        });
    }
    Ok(())
}

/// A target with no control register this build reads: the probe alone decides.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "one signature across targets, so `check` is written once"
)]
const fn register() -> Result<(), FloatEnvironmentError> {
    Ok(())
}

/// Refuse a floating-point environment the arithmetics do not define results for: the
/// control register where this build reads one, then the behavioural probe everywhere.
///
/// # Errors
///
/// Returns structured evidence when the calling thread flushes subnormals, uses
/// a different rounding mode, or fails the single-rounding probes.
pub fn check() -> Result<(), FloatEnvironmentError> {
    register()?;
    probe()
}

#[cfg(test)]
mod tests {
    use super::{PROBES, check, probe};

    #[test]
    fn the_required_environment_passes_every_probe() {
        assert_eq!(check(), Ok(()));
        assert_eq!(probe(), Ok(()));
        let scope = super::Binary64Scope::enter();
        for row in &PROBES {
            assert_eq!(row.observe(scope.ops()), row.expected, "{}", row.operation);
        }
    }

    #[cfg(any(
        target_arch = "x86_64",
        all(target_arch = "x86", target_feature = "sse2")
    ))]
    #[test]
    fn mxcsr_departures_return_register_evidence_and_restore() {
        use super::{FloatEnvironmentError, FloatEnvironmentEvidence, control};
        let saved = control::mxcsr();
        for bits in [1 << 15, 1 << 6, 1 << 13, 2 << 13, 3 << 13] {
            let failure = {
                let _guard = control::Mxcsr::load(saved | bits);
                check().expect_err("non-IEEE environment")
            };
            let evidence = FloatEnvironmentEvidence::Register {
                name: "MXCSR",
                bits: u64::from(saved | bits),
            };
            let wanted = if bits == 1 << 15 || bits == 1 << 6 {
                FloatEnvironmentError::FlushToZero { evidence }
            } else {
                FloatEnvironmentError::RoundingMode { evidence }
            };
            assert_eq!(failure, wanted);
            assert_eq!(control::mxcsr(), saved);
        }
        assert_eq!(check(), Ok(()));
    }

    #[cfg(any(
        target_arch = "x86_64",
        all(target_arch = "x86", target_feature = "sse2")
    ))]
    #[test]
    fn unmasked_traps_are_refused_before_any_exception_probe() {
        use super::{FloatEnvironmentError, FloatEnvironmentEvidence, control};
        let saved = control::mxcsr();
        for mask_bit in 7..13 {
            // Clear accrued status bits before unmasking, avoiding a pending
            // exception. check() must inspect the masks before computing probes.
            let changed = (saved & !0x3f) & !(1 << mask_bit);
            let failure = {
                let _guard = control::Mxcsr::load(changed);
                let failure = check();
                assert_eq!(probe(), failure);
                let scope = super::Binary64Scope::enter();
                assert_eq!(super::probe_with(scope.ops()), failure);
                failure
            };
            assert_eq!(
                failure,
                Err(FloatEnvironmentError::TrapsEnabled {
                    evidence: FloatEnvironmentEvidence::Register {
                        name: "MXCSR",
                        bits: u64::from(changed)
                    }
                })
            );
            assert_eq!(control::mxcsr(), saved);
        }
    }

    #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
    #[allow(
        unsafe_code,
        reason = "the test alters only its own x87 precision and restores the saved word before either scope drops"
    )]
    #[test]
    fn a_scope_with_changed_precision_is_refused_and_reentry_restores_it() {
        use super::{FloatEnvironmentError, FloatEnvironmentEvidence, probe_with};
        use crate::ieee::x87;
        let saved = x87::control_word();
        let scope = super::Binary64Scope::enter();
        let guarded = x87::control_word();
        // SAFETY: changes only this thread's precision-control field, and restores
        // the guarded word before leaving the scope, including after the assertion.
        unsafe {
            x87::load_control_word((guarded & !x87::PRECISION_CONTROL) | x87::PRECISION_64);
        }
        let failure = probe_with(scope.ops());
        // SAFETY: restores the word this thread's still-live scope established.
        unsafe { x87::load_control_word(guarded) };
        assert_eq!(
            failure,
            Err(FloatEnvironmentError::DoubleRounding {
                evidence: FloatEnvironmentEvidence::Probe {
                    operation: PROBES[8].operation,
                    expected: PROBES[8].expected,
                    observed: 1.0_f64.to_bits(),
                },
            })
        );
        assert_eq!(probe(), Ok(()));
        drop(scope);
        assert_eq!(x87::control_word(), saved);
    }
}
