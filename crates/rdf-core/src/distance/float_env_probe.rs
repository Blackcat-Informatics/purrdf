// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The MXCSR probes the float-environment tests set and restore: reading and loading
//! the register, and a guard that flushes subnormals to zero for as long as it lives.
//!
//! This module exists for THIS crate's own tests, as `purremb_fixture` does: the
//! `distance` unit tests and the `tests/purremb_float_environment.rs` integration
//! target need the same two `asm!` blocks, and a `tests/` target can only share code
//! with a `src/` unit test through the library. Two transcriptions of an `asm!`
//! block are two places an `options(...)` clause can drift. Native x86 only, where
//! MXCSR governs binary64: `x86_64`, and 32-bit `x86` with SSE2.
//!
//! It is `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. Nothing here is
//! part of any stability promise, and no shipping code path calls it — the kernels'
//! own environment check ([`super::env`]) only READS the register.

/// MXCSR flush-to-zero (bit 15).
pub const MXCSR_FTZ: u32 = 1 << 15;

/// The current thread's MXCSR.
#[must_use]
pub fn read_mxcsr() -> u32 {
    let mut value: u32 = 0;
    // SAFETY: `stmxcsr` stores the 32-bit MXCSR, and nothing else, to a live, aligned,
    // writable `u32` on this stack frame. It touches no flag and no stack memory beyond
    // that store.
    unsafe {
        core::arch::asm!(
            "stmxcsr [{ptr}]",
            ptr = in(reg) &raw mut value,
            options(nostack, preserves_flags),
        );
    }
    value
}

/// Load `value` into the current thread's MXCSR.
///
/// For a test thread only. The register is per-thread, so no other thread observes
/// the load, but every kernel in this crate assumes the IEEE environment: a caller
/// loads a value that differs from the one it read only in the FTZ, DAZ and
/// rounding-control fields, and restores the read value before returning — on every
/// exit, a panic included — which is what [`FlushToZero`] does for the FTZ case.
pub fn write_mxcsr(value: u32) {
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`. Every value loaded here
    // differs from the saved register only in the FTZ, DAZ and rounding-control fields
    // (the caller contract above), the caller restores the saved value on every exit
    // including a panic, and the register is per-thread, so no other test observes it.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// FTZ set on this thread for as long as the guard lives; the saved register is
/// restored when it drops, including by unwinding.
#[derive(Debug)]
pub struct FlushToZero(u32);

impl FlushToZero {
    /// Set FTZ on the current thread, remembering the register as it was.
    #[must_use]
    pub fn engage() -> Self {
        let saved = read_mxcsr();
        write_mxcsr(saved | MXCSR_FTZ);
        Self(saved)
    }

    /// The register as it was before FTZ was set.
    #[must_use]
    pub const fn saved(&self) -> u32 {
        self.0
    }
}

impl Drop for FlushToZero {
    fn drop(&mut self) {
        write_mxcsr(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard sets exactly FTZ and restores the register on drop; the register
    /// reads back what was written.
    #[test]
    fn the_guard_sets_ftz_and_restores_the_register() {
        let before = read_mxcsr();
        {
            let guard = FlushToZero::engage();
            assert_eq!(guard.saved(), before);
            assert_eq!(read_mxcsr(), before | MXCSR_FTZ);
        }
        assert_eq!(read_mxcsr(), before, "the register was restored");
        // The valid neighbour: writing the read value back changes nothing.
        write_mxcsr(before);
        assert_eq!(read_mxcsr(), before);
    }
}
