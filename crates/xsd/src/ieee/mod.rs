// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Binary64 and binary32 operations that return the IEEE-754 result on every target, the
//! x87 included.
//!
//! This is the one implementation of binary floating-point arithmetic the workspace's
//! results are computed with: `xsd:double` and `xsd:float` arithmetic in this crate
//! ([`numeric_add`](crate::numeric_add) and its siblings, `fn:round`, the decimal
//! conversions' exact fast path), the SPARQL evaluator's cost model and `STDDEV`, the
//! distance kernels and the PURREMB norm in `purrdf-core`, and every other `f64`/`f32`
//! operation whose bits reach a query result, serialized bytes or an identity. It lives
//! here because this crate is the lowest one all of those depend on.
//!
//! # What is correctly rounded
//!
//! `+`, `−`, `×`, `÷` and `√` at both widths: each returns the exact result rounded once,
//! to nearest, ties to even, into the operation's format, subnormals preserved, overflow
//! to the correctly signed infinity. On every target but one that is what the language's
//! operators already do, and every method and function here is that operator, inlined,
//! compiling to the instruction the operator would -- a zero-cost identity wrapper.
//!
//! Nothing else is. The transcendental functions (`ln`, `exp`, `pow`, the trigonometric
//! family) are not required by IEEE-754 to be correctly rounded, so every math library
//! is entitled to its own last bit, and no result in this workspace is computed with one.
//! Exact operations -- negation, `abs`, `floor`, `ceil`, `round`, `trunc`, comparison,
//! scaling by a power of two that neither overflows nor leaves the normal range, and
//! the widening `f32 → f64` -- have one result on every unit and need no layer.
//!
//! # The x87
//!
//! The exception is a 32-bit `x86` build without SSE2 (`i586-unknown-linux-gnu`, or
//! `i686` built with `-C target-feature=-sse2`), where `f64` and `f32` arithmetic run on
//! the x87 floating-point unit. The x87 computes in registers of 80 bits -- a 64-bit
//! significand and a 15-bit exponent -- and rounds to the significand width its control
//! word's precision-control field (PC, bits 8–9) selects; Linux starts every thread at
//! 64 bits. A result is rounded to 64 bits in the register and rounded again, to 53 or
//! 24, when it is stored as an `f64` or an `f32`. Rounding twice is not rounding once:
//! `1 + (2⁻⁵³ + 2⁻⁷⁸)` is just above the midpoint between `1` and its binary64 successor,
//! rounds to 64 bits exactly onto that midpoint, and the tie then goes to the even
//! neighbour `1`, where IEEE-754 gives the successor. And a value kept in a register has
//! the 15-bit exponent: it neither overflows nor becomes subnormal where binary64 or
//! binary32 would. So the same source computes other bits there, and nothing in the
//! result says so.
//!
//! This module closes both gaps, by the technique `HotSpot` used to give Java's `strictfp`
//! its IEEE semantics on the x87:
//!
//! 1. **Precision.** [`Binary64Scope::enter`] saves the thread's control word (`fnstcw`)
//!    and loads it with PC set to 53 bits (`fldcw`), [`Binary32Scope::enter`] with PC set
//!    to 24, changing no other field; dropping the scope restores the saved word, on
//!    unwinding too. Under PC = 53 (24) each `+`, `−`, `×`, `÷` and `√` rounds the exact
//!    result once, to binary64's (binary32's) significand, over the x87's wider exponent
//!    range.
//! 2. **Range.** Each operation is one `asm!` block that loads its operands from memory,
//!    performs the one instruction, and stores the result to memory in the operation's
//!    format (`fstp qword` or `fstp dword`). No result is ever left in a register, so every
//!    value the next operation sees is a binary64 (binary32) value. For `+`, `−` and `√`
//!    that is enough. A result in the format's normal range already has the format's
//!    significand width, so the store is exact. One above it (`≥ 2¹⁰²⁴`, or `≥ 2¹²⁸`, after
//!    rounding to the significand width) is stored as the infinity IEEE-754 gives, since
//!    the correctly rounded result overflows exactly when its rounding over an unbounded
//!    exponent is at least that power. And one below it cannot arise with bits to lose:
//!    the exact sum or difference of two binary64 (binary32) values is a multiple of
//!    2⁻¹⁰⁷⁴ (2⁻¹⁴⁹), so one smaller than the smallest normal is a subnormal exactly and
//!    nothing rounds, while a square root lies in `[2⁻⁵³⁷, 2⁵¹²)` (`[2⁻⁷⁵, 2⁶⁴)`).
//! 3. **The subnormal range of `×` and `÷`.** Here the precision control and a store are
//!    not enough, and that is the known result (Monniaux, *The pitfalls of verifying
//!    floating-point computations*, ACM TOPLAS 30(3), 2008, §3.1.2): a product or quotient
//!    smaller than the format's smallest normal is rounded to the full significand width
//!    in the register and rounded again, to the subnormal's fewer bits, by the store. So
//!    one operand is first scaled down by the distance between the x87's smallest normal
//!    exponent (−16382) and the format's: 2⁻¹⁵³⁶⁰ for binary64 (−1022), 2⁻¹⁶²⁵⁶ for
//!    binary32 (−126). The scaled result is then below the x87's normal range exactly when
//!    the true one is below the format's, and the x87 denormalizes it to the same grid the
//!    format would -- the PC-width significand below 2⁻¹⁶³⁸², spaced 2⁻¹⁶³⁸²⁻⁵² =
//!    2⁻¹⁰⁷⁴·2⁻¹⁵³⁶⁰ (2⁻¹⁶³⁸²⁻²³ = 2⁻¹⁴⁹·2⁻¹⁶²⁵⁶), binary64's (binary32's) subnormal
//!    spacing scaled -- with one rounding. Multiplying back up by the same power is exact,
//!    and the store is then exact (or the overflow of item 2). Both scale factors are
//!    powers of two held as 80-bit constants, and scaling an operand down is exact: its
//!    smallest possible result, 2⁻¹⁰⁷⁴⁻¹⁵³⁶⁰ (2⁻¹⁴⁹⁻¹⁶²⁵⁶), lies on the scaled grid, and
//!    every other is a multiple of it with no more significant bits than the operand.
//!    The one rounding is the only one at the top of the range too: the largest scaled
//!    product or quotient, below 2²¹⁰⁰⁻¹⁵³⁶⁰ (2²⁸⁰⁻¹⁶²⁵⁶), is far inside the x87's normal
//!    range, as is its unscaled value, below 2²¹⁰⁰ (2²⁸⁰), so nothing overflows before
//!    the store.
//!
//! What is left is the part no law here exposes. The x87 propagates the larger-payload
//! NaN of two where SSE2 propagates the first, so a NaN's payload may differ; no result
//! in this workspace exposes a NaN's payload (every NaN is written `NaN`).
//!
//! The x87 rounding-control field is not set by the scopes: a thread that loaded a
//! directed rounding mode computes directed results here exactly as a thread that set
//! the MXCSR's rounding field would on `x86_64`. `purrdf-core`'s distance arithmetic
//! reads that field and refuses such a thread by name.
//!
//! # Using it
//!
//! A kernel enters a scope once and passes the zero-sized, `Copy` operation token it
//! hands out ([`Binary64Scope::ops`], [`Binary32Scope::ops`]) to its inner loop: the
//! token borrows the scope, so no operation can outlive the precision it relies on, and
//! a scope is neither `Send` nor `Sync`, since the control word is the entering thread's
//! own. A one-off operation calls the free functions ([`f64_add`], [`f32_div`], ...),
//! each of which enters and leaves its own scope. On every target but the x87 both are
//! the bare operator.
//!
//! ```rust
//! use purrdf_xsd::ieee::{Binary64Scope, f64_add};
//!
//! // `1 + (2⁻⁵³ + 2⁻⁷⁸)`: the successor of 1, on every target.
//! let little = f64::from_bits(0x3ca0_0000_0800_0000);
//! assert_eq!(f64_add(1.0, little), 1.0 + f64::EPSILON);
//!
//! let scope = Binary64Scope::enter();
//! let ops = scope.ops();
//! assert_eq!(ops.mul(ops.add(1.0, 2.0), 0.5), 1.5);
//! ```

use core::fmt;
use core::marker::PhantomData;

#[doc(hidden)]
pub mod reference;
#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
#[allow(
    unsafe_code,
    reason = "the x87 control word and the correctly rounded x87 sequences are inline \
              assembly; this is the crate's only unsafe code, each block with its own \
              safety argument"
)]
pub mod x87;

#[cfg(test)]
mod tests;

/// Whether this build's binary floating-point unit is the x87.
///
/// `true` only on a 32-bit `x86` build without SSE2; there every operation in this module
/// runs the correctly rounded x87 sequence, and everywhere else it is the language's
/// operator.
pub const X87: bool = cfg!(all(target_arch = "x86", not(target_feature = "sse2")));

/// Defines one precision scope and its operation token.
macro_rules! scope {
    (
        $(#[$scope_attr:meta])*
        scope $scope:ident,
        $(#[$token_attr:meta])*
        token $token:ident,
        precision $precision:ident,
        width $width:literal
    ) => {
        $(#[$scope_attr])*
        pub struct $scope {
            /// The control word found on entry, restored on drop.
            #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
            saved: u16,
            /// Neither `Send` nor `Sync`: the precision set is this thread's control state.
            thread: PhantomData<*const ()>,
        }

        $(#[$token_attr])*
        #[derive(Clone, Copy)]
        pub struct $token<'scope> {
            scope: PhantomData<&'scope $scope>,
        }

        impl $scope {
            #[doc = concat!(
                "Enter the scope: on the x87, set the precision-control field to ",
                $width,
                " bits."
            )]
            #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
            #[inline(always)]
            #[allow(
                clippy::inline_always,
                reason = "entered around every kernel call, including per-pair ones; on \
                          every target but the x87 it must vanish entirely"
            )]
            #[must_use]
            pub const fn enter() -> Self {
                Self {
                    thread: PhantomData,
                }
            }

            #[doc = concat!(
                "Enter the scope: on the x87, set the precision-control field to ",
                $width,
                " bits."
            )]
            #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
            #[must_use]
            pub fn enter() -> Self {
                Self {
                    saved: x87::set_precision(x87::$precision),
                    thread: PhantomData,
                }
            }

            /// The operations this scope makes correct.
            #[inline(always)]
            #[allow(
                clippy::inline_always,
                reason = "a zero-sized token; it must never be a call"
            )]
            #[allow(
                clippy::unused_self,
                reason = "the receiver is the proof: the token borrows the scope"
            )]
            #[must_use]
            pub const fn ops(&self) -> $token<'_> {
                $token { scope: PhantomData }
            }
        }

        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        impl Drop for $scope {
            fn drop(&mut self) {
                x87::restore(self.saved);
            }
        }

        impl fmt::Debug for $scope {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($scope)).finish_non_exhaustive()
            }
        }

        impl fmt::Debug for $token<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(stringify!($token))
            }
        }
    };
}

scope! {
    /// The scope inside which [`Binary64`] operations return the IEEE-754 result on this
    /// thread.
    ///
    /// On the x87 it is a control-word guard: entering sets the precision-control field
    /// to 53 bits, and dropping it restores the word it found, including when a panic
    /// unwinds through it. Everywhere else it is empty and costs nothing. It is neither
    /// `Send` nor `Sync`: the control word is the entering thread's own.
    scope Binary64Scope,
    /// The binary64 operations, available only while a [`Binary64Scope`] is alive.
    ///
    /// `Copy` and zero-sized, so passing it costs nothing; it borrows the scope, so no
    /// operation can outlive the precision it relies on.
    token Binary64,
    precision PRECISION_53,
    width "53"
}

scope! {
    /// The scope inside which [`Binary32`] operations return the IEEE-754 result on this
    /// thread.
    ///
    /// On the x87 it is a control-word guard: entering sets the precision-control field
    /// to 24 bits, and dropping it restores the word it found, including when a panic
    /// unwinds through it. Everywhere else it is empty and costs nothing. It is neither
    /// `Send` nor `Sync`: the control word is the entering thread's own.
    scope Binary32Scope,
    /// The binary32 operations, available only while a [`Binary32Scope`] is alive.
    ///
    /// `Copy` and zero-sized, so passing it costs nothing; it borrows the scope, so no
    /// operation can outlive the precision it relies on.
    token Binary32,
    precision PRECISION_24,
    width "24"
}

/// Defines one correctly rounded operation on a token: the operator everywhere, the x87
/// sequence on the x87.
macro_rules! op {
    ($(#[$attr:meta])* $name:ident($($operand:ident),+) : $ty:ty => $x87:path, $portable:expr) => {
        $(#[$attr])*
        #[inline(always)]
        #[must_use]
        pub fn $name(self, $($operand: $ty),+) -> $ty {
            #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
            {
                $x87($($operand),+)
            }
            #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
            {
                $portable
            }
        }
    };
}

/// Each method is the language operator on every target but the x87, where it is the
/// correctly rounded x87 sequence described in the [module documentation](self).
#[allow(
    clippy::inline_always,
    reason = "each method is one arithmetic instruction inside a kernel's inner loop, and \
              must compile into every dispatch wrapper under that wrapper's features"
)]
#[allow(
    clippy::unused_self,
    reason = "the receiver is the proof that a `Binary64Scope` is alive"
)]
impl Binary64<'_> {
    op! {
        /// `a + b`, correctly rounded.
        add(a, b): f64 => x87::add64, a + b
    }
    op! {
        /// `a − b`, correctly rounded.
        sub(a, b): f64 => x87::sub64, a - b
    }
    op! {
        /// `a × b`, correctly rounded.
        mul(a, b): f64 => x87::mul64, a * b
    }
    op! {
        /// `a ÷ b`, correctly rounded.
        div(a, b): f64 => x87::div64, a / b
    }
    op! {
        /// `√a`, correctly rounded.
        sqrt(a): f64 => x87::sqrt64, a.sqrt()
    }
    op! {
        /// `a + b` under the reassociation licence (`f64::algebraic_add`), which permits
        /// the compiler to reassociate and contract and requires neither; on the x87,
        /// [`Binary64::add`], in written order.
        algebraic_add(a, b): f64 => x87::add64, a.algebraic_add(b)
    }
    op! {
        /// `a − b` under the reassociation licence; on the x87, [`Binary64::sub`].
        algebraic_sub(a, b): f64 => x87::sub64, a.algebraic_sub(b)
    }
    op! {
        /// `a × b` under the reassociation licence; on the x87, [`Binary64::mul`].
        algebraic_mul(a, b): f64 => x87::mul64, a.algebraic_mul(b)
    }
}

/// Each method is the language operator on every target but the x87, where it is the
/// correctly rounded x87 sequence described in the [module documentation](self).
#[allow(
    clippy::inline_always,
    reason = "each method is one arithmetic instruction, and must compile to it"
)]
#[allow(
    clippy::unused_self,
    reason = "the receiver is the proof that a `Binary32Scope` is alive"
)]
impl Binary32<'_> {
    op! {
        /// `a + b`, correctly rounded.
        add(a, b): f32 => x87::add32, a + b
    }
    op! {
        /// `a − b`, correctly rounded.
        sub(a, b): f32 => x87::sub32, a - b
    }
    op! {
        /// `a × b`, correctly rounded.
        mul(a, b): f32 => x87::mul32, a * b
    }
    op! {
        /// `a ÷ b`, correctly rounded.
        div(a, b): f32 => x87::div32, a / b
    }
    op! {
        /// `√a`, correctly rounded.
        sqrt(a): f32 => x87::sqrt32, a.sqrt()
    }
}

/// Defines a one-off operation: enter a scope, perform the operation, leave the scope.
macro_rules! once {
    ($(#[$attr:meta])* $name:ident = $scope:ident::$op:ident($($operand:ident),+) : $ty:ty) => {
        $(#[$attr])*
        #[inline(always)]
        #[allow(
            clippy::inline_always,
            reason = "the bare operator on every target but the x87"
        )]
        #[must_use]
        pub fn $name($($operand: $ty),+) -> $ty {
            let scope = $scope::enter();
            scope.ops().$op($($operand),+)
        }
    };
}

once! {
    /// `a + b` in binary64, correctly rounded, in a scope of its own.
    f64_add = Binary64Scope::add(a, b): f64
}
once! {
    /// `a − b` in binary64, correctly rounded, in a scope of its own.
    f64_sub = Binary64Scope::sub(a, b): f64
}
once! {
    /// `a × b` in binary64, correctly rounded, in a scope of its own.
    f64_mul = Binary64Scope::mul(a, b): f64
}
once! {
    /// `a ÷ b` in binary64, correctly rounded, in a scope of its own.
    f64_div = Binary64Scope::div(a, b): f64
}
once! {
    /// `√a` in binary64, correctly rounded, in a scope of its own.
    f64_sqrt = Binary64Scope::sqrt(a): f64
}
once! {
    /// `a + b` in binary32, correctly rounded, in a scope of its own.
    f32_add = Binary32Scope::add(a, b): f32
}
once! {
    /// `a − b` in binary32, correctly rounded, in a scope of its own.
    f32_sub = Binary32Scope::sub(a, b): f32
}
once! {
    /// `a × b` in binary32, correctly rounded, in a scope of its own.
    f32_mul = Binary32Scope::mul(a, b): f32
}
once! {
    /// `a ÷ b` in binary32, correctly rounded, in a scope of its own.
    f32_div = Binary32Scope::div(a, b): f32
}
once! {
    /// `√a` in binary32, correctly rounded, in a scope of its own.
    f32_sqrt = Binary32Scope::sqrt(a): f32
}
