// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The floating-point environment every arithmetic assumes, read from the processor.
//!
//! IEEE-754 fixes one result for each correctly rounded operation, but only under the
//! environment it assumes: round-to-nearest, ties-to-even, with subnormals preserved.
//! A thread can leave that environment. On `x86_64` the MXCSR register's flush-to-zero
//! (FTZ, bit 15) and denormals-are-zero (DAZ, bit 6) bits replace subnormal results and
//! operands with zero, and its rounding-control field (bits 13–14) selects another
//! direction; on `aarch64` the FPCR's flush-to-zero bit (FZ, bit 24), its
//! input-flush bit (FIZ, bit 0, where implemented) and its rounding-mode field
//! (bits 22–23) do the same. A shared library loaded with fast-math startup code sets
//! them for the thread that loads it and every thread it spawns afterwards.
//!
//! Under such an environment the same source computes different bits, and nothing in the
//! result says so. So the environment is read here, from the register itself, and a
//! flushing or re-rounding one is a named error rather than a silent divergence.
//! WebAssembly has no such register: its numeric instructions are specified to preserve
//! subnormals and round to nearest.

use core::fmt;

/// Why the current thread's floating-point environment cannot run an arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FloatEnvironmentError {
    /// The control register flushes subnormal results or operands to zero.
    FlushToZero {
        /// The register read: `"MXCSR"` or `"FPCR"`.
        register: &'static str,
        /// The register's full value, as read.
        bits: u64,
    },
    /// The control register selects a rounding direction other than to-nearest.
    RoundingMode {
        /// The register read: `"MXCSR"` or `"FPCR"`.
        register: &'static str,
        /// The register's full value, as read.
        bits: u64,
    },
    /// This target's floating-point control register is not one this build can read,
    /// so the environment cannot be proven to be the one the arithmetic assumes.
    Uninspectable {
        /// The target architecture, as `std::env::consts::ARCH` names it.
        target_arch: &'static str,
    },
}

impl fmt::Display for FloatEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlushToZero { register, bits } => write!(
                f,
                "the thread's {register} ({bits:#x}) flushes subnormals to zero; binary64 \
                 distances computed under it differ from the IEEE-754 results the arithmetic \
                 defines, so they are refused rather than ranked"
            ),
            Self::RoundingMode { register, bits } => write!(
                f,
                "the thread's {register} ({bits:#x}) rounds other than to nearest; binary64 \
                 distances computed under it differ from the IEEE-754 results the arithmetic \
                 defines, so they are refused rather than ranked"
            ),
            Self::Uninspectable { target_arch } => write!(
                f,
                "the floating-point control register of {target_arch} cannot be read by this \
                 build, so a flush-to-zero or re-rounding environment cannot be ruled out"
            ),
        }
    }
}

impl std::error::Error for FloatEnvironmentError {}

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

/// Refuse a floating-point environment the arithmetics do not define results for.
#[cfg(target_arch = "x86_64")]
pub(crate) fn check() -> Result<(), FloatEnvironmentError> {
    let bits = mxcsr();
    if bits & (MXCSR_FTZ | MXCSR_DAZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero {
            register: "MXCSR",
            bits: u64::from(bits),
        });
    }
    if bits & MXCSR_RC != 0 {
        return Err(FloatEnvironmentError::RoundingMode {
            register: "MXCSR",
            bits: u64::from(bits),
        });
    }
    Ok(())
}

/// Refuse a floating-point environment the arithmetics do not define results for.
#[cfg(target_arch = "aarch64")]
pub(crate) fn check() -> Result<(), FloatEnvironmentError> {
    let bits = fpcr();
    if bits & (FPCR_FZ | FPCR_FIZ) != 0 {
        return Err(FloatEnvironmentError::FlushToZero {
            register: "FPCR",
            bits,
        });
    }
    if bits & FPCR_RMODE != 0 {
        return Err(FloatEnvironmentError::RoundingMode {
            register: "FPCR",
            bits,
        });
    }
    Ok(())
}

/// WebAssembly's numeric instructions preserve subnormals and round to nearest by
/// specification; there is no register to set otherwise.
#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
pub(crate) const fn check() -> Result<(), FloatEnvironmentError> {
    Ok(())
}

/// A target whose control register this build does not read: the environment cannot be
/// proven, so it is refused by name.
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "wasm64"
)))]
pub(crate) const fn check() -> Result<(), FloatEnvironmentError> {
    Err(FloatEnvironmentError::Uninspectable {
        target_arch: std::env::consts::ARCH,
    })
}
