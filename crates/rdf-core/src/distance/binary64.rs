// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The binary64 operations every arithmetic in this module's parent computes with.
//!
//! Every arithmetic here computes in binary64: `+`, `−`, `×`, `÷` and `√`, each correctly
//! rounded, round-to-nearest-even, subnormals preserved. The operations are
//! [`purrdf_xsd::ieee`]'s -- the workspace's one implementation of binary floating-point
//! arithmetic, which on every target but the x87 is the language's `f64` operators,
//! inlined, and on the x87 (`i586`, or `i686` without SSE2) is a precision guard at 53
//! bits, one stored operation per `asm!` block, and the 2⁻¹⁵³⁶⁰ scaling of products and
//! quotients that makes a subnormal result round once. That module's documentation is
//! the derivation; this one only names the scope the kernels enter.
//!
//! The guard is not trusted to have worked: the behavioural probe (`env::PROBES`) runs
//! its operations through these same methods, under the same guard, and carries rows
//! whose correctly rounded result differs from the doubly rounded one -- in the normal
//! range for `+` and `×`, and in the subnormal range for `×` and `÷`. A thread where
//! the guard did not take hold is refused by name
//! ([`FloatEnvironmentError::DoubleRounding`](super::FloatEnvironmentError::DoubleRounding))
//! rather than handed doubly rounded distances.
//!
//! # The reassociated arithmetic
//!
//! [`Reassociated`](super::Reassociated)'s evidence claims binary64 arithmetic, and it
//! runs under the same guard. Its `algebraic_*` operations ([`Algebraic`]) license the
//! compiler to reassociate and contract; the licence permits that and requires nothing,
//! so on the x87 they are these same correctly rounded operations, in written order, and
//! every value the fold holds is a binary64 value. On every other target they are the
//! `algebraic_*` operations unchanged.
//!
//! The licence is defined here, not in [`purrdf_xsd::ieee`]: that layer is exact by
//! contract, and the licence to reassociate belongs to the ranked-distance modules alone.

pub(crate) use purrdf_xsd::ieee::Binary64;
use purrdf_xsd::ieee::Binary64Scope;
#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
pub(crate) use purrdf_xsd::ieee::x87;

/// The binary64 operations under the reassociation licence, on the [`Binary64`] token.
///
/// On every target but the x87 each is the `f64` method of the same name, inlined, which
/// permits the compiler to reassociate and contract and requires neither. On the x87 each
/// is the token's correctly rounded operation, in written order: one of the results the
/// licence admits, and a binary64 value rather than an 80-bit register's.
pub(crate) trait Algebraic: Copy {
    /// `a + b` under the reassociation licence; on the x87, [`Binary64::add`].
    fn algebraic_add(self, a: f64, b: f64) -> f64;
    /// `a − b` under the reassociation licence; on the x87, [`Binary64::sub`].
    fn algebraic_sub(self, a: f64, b: f64) -> f64;
    /// `a × b` under the reassociation licence; on the x87, [`Binary64::mul`].
    fn algebraic_mul(self, a: f64, b: f64) -> f64;
}

/// Defines one licensed operation: the `f64` method everywhere, the token's correctly
/// rounded operation on the x87.
macro_rules! algebraic {
    ($name:ident => $exact:ident) => {
        #[inline(always)]
        fn $name(self, a: f64, b: f64) -> f64 {
            #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
            {
                self.$exact(a, b)
            }
            #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
            {
                a.$name(b)
            }
        }
    };
}

#[allow(
    clippy::inline_always,
    reason = "each method is one arithmetic instruction inside a kernel's inner loop, and \
              must compile into every dispatch wrapper under that wrapper's features"
)]
impl Algebraic for Binary64<'_> {
    algebraic!(algebraic_add => add);
    algebraic!(algebraic_sub => sub);
    algebraic!(algebraic_mul => mul);
}

/// The scope inside which [`Binary64`] operations return the IEEE-754 result on this
/// thread: a [`Binary64Scope`].
///
/// On the x87 it is a control-word guard that sets the precision-control field to 53
/// bits and restores the caller's word on drop, unwinding included. Everywhere else it is
/// empty and costs nothing. It is neither `Send` nor `Sync`: the control word is the
/// entering thread's own.
pub(crate) struct Precision {
    scope: Binary64Scope,
}

impl Precision {
    /// Enter the scope: on the x87, set the precision-control field to 53 bits.
    #[inline(always)]
    #[allow(
        clippy::inline_always,
        reason = "entered around every kernel call, including per-pair ones; on every \
                  target but the x87 it must vanish entirely"
    )]
    pub(crate) fn enter() -> Self {
        #[cfg(all(test, target_arch = "x86", not(target_feature = "sse2")))]
        let found = x87::control_word();
        let scope = Binary64Scope::enter();
        #[cfg(all(test, target_arch = "x86", not(target_feature = "sse2")))]
        if bypass::active() {
            // SAFETY: `found` is this thread's own word from before the scope was
            // entered, which the scope restores (to the same value) when it drops.
            unsafe { x87::load_control_word(found) };
        }
        Self { scope }
    }

    /// The operations this scope makes correct.
    #[inline(always)]
    #[allow(
        clippy::inline_always,
        reason = "a zero-sized token; it must never be a call"
    )]
    pub(crate) const fn binary64(&self) -> Binary64<'_> {
        self.scope.ops()
    }
}

/// The test hook that undoes [`Precision::enter`]'s precision on the calling thread,
/// standing in for a thread whose guard did not take hold.
#[cfg(all(test, target_arch = "x86", not(target_feature = "sse2")))]
pub(crate) mod bypass {
    use std::cell::Cell;

    thread_local! {
        /// Whether the calling thread's guards leave the control word as they find it.
        static BYPASSED: Cell<bool> = const { Cell::new(false) };
    }

    /// While alive, the calling thread's [`Precision`](super::Precision) guards set
    /// nothing.
    pub(crate) struct Bypassed {
        previous: bool,
    }

    impl Drop for Bypassed {
        fn drop(&mut self) {
            BYPASSED.with(|bypassed| bypassed.set(self.previous));
        }
    }

    /// Bypass the guard on this thread until the returned value drops.
    pub(crate) fn bypass() -> Bypassed {
        let previous = BYPASSED.with(|bypassed| bypassed.replace(true));
        Bypassed { previous }
    }

    /// Whether a test bypassed the guard on this thread.
    pub(crate) fn active() -> bool {
        BYPASSED.with(Cell::get)
    }
}
