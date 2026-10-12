// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one place the thread's floating-point control register is read and written.
//!
//! The float-environment probe reads the register to name it in a refusal (as
//! [`super::environment::FloatEnvironmentEvidence::Register`]); the float
//! environment tests write it to prove the refusal. Both go through these functions, so
//! the inline assembly, its `options(...)` and its safety argument exist once. The
//! register is per-thread state; a caller that changes it restores the value it read.
//!
//! The x86 pair exists where MXCSR governs binary64 (`x86_64`, and 32-bit `x86` with
//! SSE2), including a runtime-selected SSE2 path in an x87 baseline; the
//! `aarch64` pair reads and writes FPCR.

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86"))]
use core::marker::PhantomData;

/// The bits of MXCSR that are defined on every SSE2 processor: the exception flags
/// (0-5), denormals-are-zero (6), the exception masks (7-12), the rounding control
/// (13-14) and flush-to-zero (15). Loading a value with any other bit set raises a
/// general-protection fault, so [`set_mxcsr`] clears them.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const MXCSR_DEFINED: u32 = 0xFFFF;

/// Whether this processor can read and use MXCSR. The runtime case lets an
/// x87-baseline executable safely admit its independently selected SSE2 path.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[must_use]
pub fn mxcsr_available() -> bool {
    #[cfg(any(target_arch = "x86_64", target_feature = "sse2"))]
    {
        true
    }
    #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
    {
        std::is_x86_feature_detected!("sse2")
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn require_mxcsr() {
    assert!(
        mxcsr_available(),
        "MXCSR requires an SSE2-capable processor"
    );
}

/// The current thread's MXCSR.
///
/// # Panics
/// Refuses a processor without SSE2 before executing a register instruction.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[must_use]
pub fn mxcsr() -> u32 {
    require_mxcsr();
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
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub fn set_mxcsr(value: u32) {
    require_mxcsr();
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
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub const MXCSR_FTZ: u32 = 1 << 15;

/// MXCSR loaded with a value for as long as the guard lives; the value it replaced is
/// restored when it drops, including by unwinding. The guard is neither `Send`
/// nor `Sync`, because restoration belongs to the entering thread.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[derive(Debug)]
/// ```compile_fail
/// fn requires_send<T: Send>() {}
/// requires_send::<purrdf_xsd::ieee::control::Mxcsr>();
/// ```
/// ```compile_fail
/// fn requires_sync<T: Sync>() {}
/// requires_sync::<purrdf_xsd::ieee::control::Mxcsr>();
/// ```
pub struct Mxcsr {
    saved: u32,
    thread: PhantomData<*const ()>,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
impl Mxcsr {
    /// Load `value` into this thread's MXCSR, remembering the register it replaces.
    #[must_use]
    pub fn load(value: u32) -> Self {
        let saved = mxcsr();
        set_mxcsr(value);
        Self {
            saved,
            thread: PhantomData,
        }
    }

    /// The register as it is, with flush-to-zero added.
    #[must_use]
    pub fn flush_to_zero() -> Self {
        Self::load(mxcsr() | MXCSR_FTZ)
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
impl Drop for Mxcsr {
    fn drop(&mut self) {
        set_mxcsr(self.saved);
    }
}

/// FPCR loaded with a value for as long as the guard lives; the value it replaced is
/// restored when it drops, including by unwinding. The guard is neither `Send`
/// nor `Sync`, because restoration belongs to the entering thread.
#[cfg(target_arch = "aarch64")]
#[derive(Debug)]
/// ```compile_fail
/// fn requires_send<T: Send>() {}
/// requires_send::<purrdf_xsd::ieee::control::Fpcr>();
/// ```
/// ```compile_fail
/// fn requires_sync<T: Sync>() {}
/// requires_sync::<purrdf_xsd::ieee::control::Fpcr>();
/// ```
pub struct Fpcr {
    saved: u64,
    thread: PhantomData<*const ()>,
}

#[cfg(target_arch = "aarch64")]
impl Fpcr {
    /// Load `value` into this thread's FPCR, remembering the register it replaces.
    #[must_use]
    pub fn load(value: u64) -> Self {
        let saved = fpcr();
        set_fpcr(value);
        Self {
            saved,
            thread: PhantomData,
        }
    }
}

#[cfg(target_arch = "aarch64")]
impl Drop for Fpcr {
    fn drop(&mut self) {
        set_fpcr(self.saved);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    #[test]
    fn mxcsr_restores_nested_scopes_and_unwinding() {
        use super::{Mxcsr, mxcsr};
        // An x87 baseline may run on a pre-SSE2 processor. Its portable
        // arithmetic does not require this optional vector control register.
        if !super::mxcsr_available() {
            return;
        }
        let saved = mxcsr();
        {
            let _outer = Mxcsr::load(saved | (1 << 15));
            let outer = mxcsr();
            {
                let _inner = Mxcsr::load(saved);
                assert_eq!(mxcsr(), saved);
            }
            assert_eq!(mxcsr(), outer);
        }
        assert_eq!(mxcsr(), saved);
        let unwound = std::panic::catch_unwind(|| {
            let _loaded = Mxcsr::load(saved | (1 << 15));
            panic!("unwind the register guard");
        });
        assert!(unwound.is_err());
        assert_eq!(mxcsr(), saved);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn fpcr_restores_nested_scopes_and_unwinding() {
        use super::{Fpcr, fpcr};
        let saved = fpcr();
        {
            let _outer = Fpcr::load(saved | (1 << 24));
            let outer = fpcr();
            {
                let _inner = Fpcr::load(saved);
                assert_eq!(fpcr(), saved);
            }
            assert_eq!(fpcr(), outer);
        }
        assert_eq!(fpcr(), saved);
        let unwound = std::panic::catch_unwind(|| {
            let _loaded = Fpcr::load(saved | (1 << 24));
            panic!("unwind the register guard");
        });
        assert!(unwound.is_err());
        assert_eq!(fpcr(), saved);
    }
}
