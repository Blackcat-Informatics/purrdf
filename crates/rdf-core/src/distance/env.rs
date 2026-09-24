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
//! spawns afterwards.
//!
//! Under such an environment the same source computes different bits, and nothing in the
//! result says so. So the environment is established by what it DOES: a probe of eight
//! binary64 operations whose correctly rounded results under the assumed environment are
//! known constants, each chosen so that one departure from it changes the bits. Operands
//! pass through [`core::hint::black_box`], so the compiler cannot fold an operation into
//! the constant it would give under its own (default) environment; the multiply and add
//! run on the thread's floating-point unit, under whatever that thread has set.
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
//!
//! Every directed mode fails at least one row of each sign, and a nearest mode that
//! breaks ties other than to even fails one of the last two, so the table distinguishes
//! round-to-nearest-even from every other rounding rule. The probe runs in binary64
//! because that is the only precision the arithmetics execute in: every operand is
//! widened to binary64 per component ([`Scalar`](super::Scalar)) before any operation,
//! so no binary32 operation is ever performed and none needs probing. On 32-bit Arm this
//! also settles which unit is proven: NEON has no binary64 lanes, so binary64 runs on the
//! VFP, which honours the FPSCR the probe exercises.
//!
//! Where the control register CAN be read — `x86_64` and `aarch64` — it is read first,
//! so a refusal names the register and its value, and the probe then runs as well: a
//! register read proves only the bits this build knows the meaning of, and the probe
//! proves the environment itself. On WebAssembly the numeric instructions are specified
//! to round to nearest-even and preserve subnormals, and there is no register to set
//! otherwise; the probe runs there too, because it costs eight operations and an engine
//! that departed from the specification would be refused by name rather than trusted.

use core::fmt;
use core::hint::black_box;

/// What a refusal of the floating-point environment observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FloatEnvironmentEvidence {
    /// The thread's floating-point control register, read directly.
    Register {
        /// The register read: `"MXCSR"` or `"FPCR"`.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FloatEnvironmentError {
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
}

impl fmt::Display for FloatEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlushToZero { evidence } => write!(
                f,
                "the thread flushes subnormals to zero ({evidence}); binary64 distances \
                 computed under it differ from the IEEE-754 results the arithmetic defines, \
                 so they are refused rather than ranked"
            ),
            Self::RoundingMode { evidence } => write!(
                f,
                "the thread rounds other than to nearest, ties to even ({evidence}); binary64 \
                 distances computed under it differ from the IEEE-754 results the arithmetic \
                 defines, so they are refused rather than ranked"
            ),
        }
    }
}

impl std::error::Error for FloatEnvironmentError {}

/// Which refusal a probe row's failure is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Departure {
    /// A subnormal result or operand became zero.
    Flush,
    /// A rounding rule other than round-to-nearest-even.
    Rounding,
}

/// The binary64 operation a probe row performs.
#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Mul,
}

/// One probe row: an operation on two operands and the bits IEEE-754 defines for it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Probe {
    /// The operation, in words.
    pub(crate) operation: &'static str,
    /// Which refusal a mismatch is.
    pub(crate) departure: Departure,
    op: Op,
    lhs: u64,
    rhs: u64,
    /// The correctly rounded result under the assumed environment.
    pub(crate) expected: u64,
}

impl Probe {
    /// Perform the operation on this thread's floating-point unit and return its bits.
    ///
    /// Both operands pass through [`black_box`], so the operation is executed at run time
    /// under the thread's environment rather than folded at compile time under the
    /// compiler's.
    #[inline]
    pub(crate) fn observe(&self) -> u64 {
        let lhs = black_box(f64::from_bits(self.lhs));
        let rhs = black_box(f64::from_bits(self.rhs));
        let result = match self.op {
            Op::Add => lhs + rhs,
            Op::Mul => lhs * rhs,
        };
        black_box(result).to_bits()
    }

    /// This row's refusal, if the thread's result differs from the defined one.
    fn refusal(&self) -> Option<FloatEnvironmentError> {
        let observed = self.observe();
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

// Each constant is the value its name says, checked at compile time against the
// definition written as arithmetic. Constant evaluation is IEEE round-to-nearest-even
// with subnormals preserved, and these operations are exact in any case.
const _: () = {
    assert!(f64::from_bits(ONE) == 1.0);
    assert!(f64::from_bits(ONE_PLUS_ULP) == 1.0 + f64::EPSILON);
    assert!(f64::from_bits(ONE_PLUS_TWO_ULP) == 1.0 + 2.0 * f64::EPSILON);
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
};

/// The probe, in the order it is run: flushing first, then rounding.
pub(crate) const PROBES: [Probe; 8] = [
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
];

/// Refuse a thread whose binary64 operations do not return the IEEE-754 results: the
/// first probe row whose bits differ names the departure and what it saw.
pub(crate) fn probe() -> Result<(), FloatEnvironmentError> {
    PROBES.iter().find_map(Probe::refusal).map_or(Ok(()), Err)
}

/// MXCSR flush-to-zero.
#[cfg(target_arch = "x86_64")]
const MXCSR_FTZ: u32 = 1 << 15;
/// MXCSR denormals-are-zero.
#[cfg(target_arch = "x86_64")]
const MXCSR_DAZ: u32 = 1 << 6;
/// MXCSR rounding control; zero is round-to-nearest.
#[cfg(target_arch = "x86_64")]
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

/// The current thread's MXCSR.
#[cfg(target_arch = "x86_64")]
pub(crate) fn mxcsr() -> u32 {
    let mut value: u32 = 0;
    // SAFETY: `stmxcsr` stores the 32-bit MXCSR, and nothing else, to the address it is
    // given, which is a live, aligned, writable `u32` on this stack frame. It touches
    // no flag and no stack memory beyond that store.
    unsafe {
        core::arch::asm!(
            "stmxcsr [{ptr}]",
            ptr = in(reg) &raw mut value,
            options(nostack, preserves_flags),
        );
    }
    value
}

/// The current thread's FPCR.
#[cfg(target_arch = "aarch64")]
pub(crate) fn fpcr() -> u64 {
    let value: u64;
    // SAFETY: `mrs` of FPCR is readable at EL0 and only copies the register into a
    // general-purpose register; it reads no memory and changes no state.
    unsafe {
        core::arch::asm!(
            "mrs {value}, fpcr",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Refuse an MXCSR whose flush or rounding fields are set, naming the register.
#[cfg(target_arch = "x86_64")]
fn register() -> Result<(), FloatEnvironmentError> {
    let bits = mxcsr();
    let evidence = FloatEnvironmentEvidence::Register {
        name: "MXCSR",
        bits: u64::from(bits),
    };
    if bits & (MXCSR_FTZ | MXCSR_DAZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero { evidence });
    }
    if bits & MXCSR_RC != 0 {
        return Err(FloatEnvironmentError::RoundingMode { evidence });
    }
    Ok(())
}

/// Refuse an FPCR whose flush or rounding fields are set, naming the register.
#[cfg(target_arch = "aarch64")]
fn register() -> Result<(), FloatEnvironmentError> {
    let bits = fpcr();
    let evidence = FloatEnvironmentEvidence::Register { name: "FPCR", bits };
    if bits & (FPCR_FZ | FPCR_FIZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero { evidence });
    }
    if bits & FPCR_RMODE != 0 {
        return Err(FloatEnvironmentError::RoundingMode { evidence });
    }
    Ok(())
}

/// A target with no control register this build reads: the probe alone decides.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "one signature across targets, so `check` is written once"
)]
const fn register() -> Result<(), FloatEnvironmentError> {
    Ok(())
}

/// Refuse a floating-point environment the arithmetics do not define results for: the
/// control register where this build reads one, then the behavioural probe everywhere.
pub(crate) fn check() -> Result<(), FloatEnvironmentError> {
    register()?;
    probe()
}
