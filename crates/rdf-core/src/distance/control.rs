// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one place the thread's floating-point control register is read and written.
//!
//! The probe in [`super::env`] reads the register to name it in a refusal; the float
//! environment tests write it to prove the refusal. Both go through these functions, so
//! the inline assembly, its `options(...)` and its safety argument exist once. The
//! register is per-thread state; a caller that changes it restores the value it read.
//!
//! The x86 pair exists where MXCSR governs binary64 (`x86_64`, and 32-bit `x86` with
//! SSE2); the `aarch64` pair reads and writes FPCR.

/// The bits of MXCSR that are defined on every SSE2 processor: the exception flags
/// (0-5), denormals-are-zero (6), the exception masks (7-12), the rounding control
/// (13-14) and flush-to-zero (15). Loading a value with any other bit set raises a
/// general-protection fault, so [`set_mxcsr`] clears them.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_DEFINED: u32 = 0xFFFF;

/// The current thread's MXCSR.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
#[must_use]
pub fn mxcsr() -> u32 {
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

/// Load `value` into the current thread's MXCSR. Bits outside the defined 0-15 range are
/// cleared, so no argument can fault; the caller restores the value it saved.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
pub fn set_mxcsr(value: u32) {
    let value = value & MXCSR_DEFINED;
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32` on this stack frame, with
    // every reserved bit cleared, so the load cannot fault. The register is per-thread
    // and the instruction touches no flag and no memory other than the read.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// The current thread's FPCR.
#[cfg(target_arch = "aarch64")]
#[must_use]
pub fn fpcr() -> u64 {
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

/// Load `value` into the current thread's FPCR. Reserved bits are RES0 and ignored on a
/// write; the caller restores the value it saved.
#[cfg(target_arch = "aarch64")]
pub fn set_fpcr(value: u64) {
    // SAFETY: FPCR is writable at EL0 and per-thread; `msr` changes
    // no flag. Every value written is a saved register or a saved register with
    // documented control bits changed.
    unsafe {
        core::arch::asm!(
            "msr fpcr, {value}",
            value = in(reg) value,
            options(nostack, preserves_flags),
        );
    }
}

/// MXCSR flush-to-zero.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
pub const MXCSR_FTZ: u32 = 1 << 15;

/// MXCSR loaded with a value for as long as the guard lives; the value it replaced is
/// restored when it drops, including by unwinding.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
#[derive(Debug)]
pub struct Mxcsr(u32);

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
impl Mxcsr {
    /// Load `value` into this thread's MXCSR, remembering the register it replaces.
    #[must_use]
    pub fn load(value: u32) -> Self {
        let saved = mxcsr();
        set_mxcsr(value);
        Self(saved)
    }

    /// The register as it is, with flush-to-zero added.
    #[must_use]
    pub fn flush_to_zero() -> Self {
        Self::load(mxcsr() | MXCSR_FTZ)
    }
}

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
impl Drop for Mxcsr {
    fn drop(&mut self) {
        set_mxcsr(self.0);
    }
}

/// FPCR loaded with a value for as long as the guard lives; the value it replaced is
/// restored when it drops, including by unwinding.
#[cfg(target_arch = "aarch64")]
#[derive(Debug)]
pub struct Fpcr(u64);

#[cfg(target_arch = "aarch64")]
impl Fpcr {
    /// Load `value` into this thread's FPCR, remembering the register it replaces.
    #[must_use]
    pub fn load(value: u64) -> Self {
        let saved = fpcr();
        set_fpcr(value);
        Self(saved)
    }
}

#[cfg(target_arch = "aarch64")]
impl Drop for Fpcr {
    fn drop(&mut self) {
        set_fpcr(self.0);
    }
}
