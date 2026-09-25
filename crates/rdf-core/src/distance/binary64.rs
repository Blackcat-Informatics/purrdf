// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Binary64 operations that return the IEEE-754 result on every target, the x87 included.
//!
//! Every arithmetic in this module's parent computes in binary64: `+`, `−`, `×`, `÷` and
//! `√`, each correctly rounded, round-to-nearest-even, subnormals preserved. On every
//! target but one that is what the language's `f64` operators already do, and
//! [`Binary64`]'s methods are those operators, inlined, compiling to the instruction the
//! operator would.
//!
//! # The x87
//!
//! The exception is a 32-bit `x86` build without SSE2 (`i586-unknown-linux-gnu`, or
//! `i686` built with `-C target-feature=-sse2`), where `f64` arithmetic runs on the x87
//! floating-point unit. The x87 computes in registers of 80 bits -- a 64-bit significand
//! and a 15-bit exponent -- and rounds to the significand width its control word's
//! precision-control field (PC, bits 8–9) selects; Linux starts every thread at 64 bits.
//! A result is rounded to 64 bits in the register and rounded again, to 53, when it is
//! stored as an `f64`. Rounding twice is not rounding once: `1 + (2⁻⁵³ + 2⁻⁷⁸)` is just
//! above the midpoint between `1` and its successor, rounds to 64 bits exactly onto that
//! midpoint, and the tie then goes to the even neighbour `1`, where IEEE-754 gives the
//! successor. And a value kept in a register has the 15-bit exponent: it neither
//! overflows nor becomes subnormal where binary64 would. So the same source computes
//! other bits there, and nothing in the result says so.
//!
//! This module closes both gaps, by the technique `HotSpot` used to give Java's `strictfp`
//! its binary64 semantics on the x87:
//!
//! 1. **Precision.** [`Precision::enter`] saves the thread's control word (`fnstcw`) and
//!    loads it with PC set to 53 bits (`fldcw`), changing no other field; dropping the
//!    guard restores the saved word, on unwinding too. Under PC = 53 each `+`, `−`, `×`,
//!    `÷` and `√` rounds the exact result once, to 53 bits -- binary64's significand --
//!    over the x87's wider exponent range.
//! 2. **Range.** Each operation is one `asm!` block that loads its binary64 operands from
//!    memory, performs the one instruction, and stores the result to memory as binary64
//!    (`fstp qword`). No result is ever left in a register, so every value the next
//!    operation sees is a binary64 value. For `+`, `−` and `√` that is enough. A result
//!    in binary64's normal range already has 53 bits, so the store is exact. One above it
//!    (`≥ 2¹⁰²⁴` after rounding to 53 bits) is stored as the infinity IEEE-754 gives,
//!    since the correctly rounded result overflows exactly when its 53-bit rounding over
//!    an unbounded exponent is at least 2¹⁰²⁴. And one below it cannot arise with bits to
//!    lose: the exact sum or difference of two binary64 values is a multiple of 2⁻¹⁰⁷⁴,
//!    so one smaller than 2⁻¹⁰²² is a subnormal exactly and nothing rounds, while a
//!    square root of a binary64 value lies in `[2⁻⁵³⁷, 2⁵¹²)`.
//! 3. **The subnormal range of `×` and `÷`.** Here PC = 53 and a store are not enough,
//!    and that is the known result (Monniaux, *The pitfalls of verifying floating-point
//!    computations*, ACM TOPLAS 30(3), 2008, §3.1.2): a product or quotient smaller
//!    than 2⁻¹⁰²² is rounded to 53 bits in the register and rounded again, to the
//!    subnormal's fewer bits, by the store. So one operand is first scaled by 2⁻¹⁵³⁶⁰,
//!    the distance between the x87's smallest normal exponent (−16382) and binary64's
//!    (−1022). The scaled result is then below the x87's normal range exactly when the
//!    true one is below binary64's, and the x87 denormalizes it to the same 53-bit
//!    grid binary64 would -- one rounding, at binary64's subnormal spacing scaled by
//!    2⁻¹⁵³⁶⁰. Multiplying back by 2¹⁵³⁶⁰ is exact, and the store is then exact (or the
//!    overflow of item 2). Both scale factors are powers of two held as 80-bit constants,
//!    and scaling a binary64 operand by 2⁻¹⁵³⁶⁰ is exact: its smallest possible result,
//!    2⁻¹⁰⁷⁴⁻¹⁵³⁶⁰, is the 53-bit grid's own spacing there.
//!
//! What is left is the part no law here exposes. The x87 propagates the larger-payload
//! NaN of two where SSE2 propagates the first, so a NaN's payload may differ; every
//! distance is `None` when it is not finite, so no NaN's bits reach a caller.
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
//! runs under the same guard. Its `algebraic_*` operations license the compiler to
//! reassociate and contract; the licence permits that and requires nothing, so on the
//! x87 they are these same correctly rounded operations, in written order, and every
//! value the fold holds is a binary64 value. On every other target they are the
//! `algebraic_*` operations unchanged.

use core::marker::PhantomData;

/// The scope inside which [`Binary64`] operations return the IEEE-754 result on this
/// thread.
///
/// On the x87 it is a control-word guard: entering sets the precision-control field to
/// 53 bits, and dropping it restores the word it found, including when a panic unwinds
/// through it. Everywhere else it is empty and costs nothing. It is neither `Send` nor
/// `Sync`: the control word is the entering thread's own.
pub(crate) struct Precision {
    /// The control word found on entry, restored on drop.
    #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
    saved: u16,
    /// Neither `Send` nor `Sync`: the precision set is this thread's control state.
    thread: PhantomData<*const ()>,
}

/// The binary64 operations, available only while a [`Precision`] scope is alive.
///
/// `Copy` and zero-sized, so passing it costs nothing; it borrows the scope, so no
/// operation can outlive the precision it relies on.
#[derive(Clone, Copy)]
pub(crate) struct Binary64<'scope> {
    scope: PhantomData<&'scope Precision>,
}

impl Precision {
    /// Enter the scope: on the x87, set the precision-control field to 53 bits.
    #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
    #[inline(always)]
    #[allow(
        clippy::inline_always,
        reason = "entered around every kernel call, including per-pair ones; on every \
                  target but the x87 it must vanish entirely"
    )]
    pub(crate) const fn enter() -> Self {
        Self {
            thread: PhantomData,
        }
    }

    /// Enter the scope: on the x87, set the precision-control field to 53 bits.
    #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
    pub(crate) fn enter() -> Self {
        let saved = x87::control_word();
        #[cfg(test)]
        if bypass::active() {
            return Self {
                saved,
                thread: PhantomData,
            };
        }
        let wanted = (saved & !x87::PRECISION_CONTROL) | x87::PRECISION_53;
        if wanted != saved {
            x87::load_control_word(wanted);
        }
        Self {
            saved,
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
    pub(crate) const fn binary64(&self) -> Binary64<'_> {
        Binary64 { scope: PhantomData }
    }
}

#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
impl Drop for Precision {
    fn drop(&mut self) {
        if x87::control_word() != self.saved {
            x87::load_control_word(self.saved);
        }
    }
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
    reason = "the receiver is the proof that a `Precision` scope is alive"
)]
impl Binary64<'_> {
    /// `a + b`, correctly rounded.
    #[inline(always)]
    pub(crate) fn add(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::add(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a + b
        }
    }

    /// `a − b`, correctly rounded.
    #[inline(always)]
    pub(crate) fn sub(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::sub(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a - b
        }
    }

    /// `a × b`, correctly rounded.
    #[inline(always)]
    pub(crate) fn mul(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::mul(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a * b
        }
    }

    /// `a ÷ b`, correctly rounded.
    #[inline(always)]
    pub(crate) fn div(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::div(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a / b
        }
    }

    /// `√a`, correctly rounded.
    #[inline(always)]
    pub(crate) fn sqrt(self, a: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::sqrt(a)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a.sqrt()
        }
    }

    /// `a + b` under the reassociation licence; on the x87, [`Binary64::add`].
    #[inline(always)]
    pub(crate) fn algebraic_add(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::add(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a.algebraic_add(b)
        }
    }

    /// `a − b` under the reassociation licence; on the x87, [`Binary64::sub`].
    #[inline(always)]
    pub(crate) fn algebraic_sub(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::sub(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a.algebraic_sub(b)
        }
    }

    /// `a × b` under the reassociation licence; on the x87, [`Binary64::mul`].
    #[inline(always)]
    pub(crate) fn algebraic_mul(self, a: f64, b: f64) -> f64 {
        #[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
        {
            x87::mul(a, b)
        }
        #[cfg(not(all(target_arch = "x86", not(target_feature = "sse2"))))]
        {
            a.algebraic_mul(b)
        }
    }
}

/// The x87 control word and the correctly rounded operations.
///
/// Every operation is one `asm!` block with memory operands: it loads binary64 values,
/// performs one rounding operation (bracketed by exact scalings for `×` and `÷`), and
/// stores a binary64 value. Each block declares all eight x87 registers clobbered, so
/// the stack is empty on entry, and leaves it empty on exit. None is `pure`: each is
/// ordered after the `fldcw` that set its precision, which is itself a side-effecting
/// block the compiler may not move them across.
#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
pub(crate) mod x87 {
    use core::arch::asm;

    /// The precision-control field of the x87 control word (bits 8–9).
    pub(crate) const PRECISION_CONTROL: u16 = 0b11 << 8;
    /// The field's value for a 53-bit significand.
    pub(crate) const PRECISION_53: u16 = 0b10 << 8;
    /// The rounding-control field (bits 10–11); zero is round-to-nearest-even.
    pub(crate) const ROUNDING_CONTROL: u16 = 0b11 << 10;

    /// 2⁻¹⁵³⁶⁰ as an 80-bit extended value: significand `0x8000_0000_0000_0000` (the
    /// explicit integer bit alone), biased exponent `16383 − 15360 = 0x03ff`.
    static SCALE_DOWN: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x03];
    /// 2¹⁵³⁶⁰ as an 80-bit extended value: biased exponent `16383 + 15360 = 0x7bff`.
    static SCALE_UP: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x7b];

    /// The current thread's x87 control word.
    pub(crate) fn control_word() -> u16 {
        let mut word: u16 = 0;
        // SAFETY: `fnstcw` stores the 16-bit control word, and nothing else, to the
        // address it is given: a live, aligned, writable `u16` on this frame. It changes
        // no register, flag or x87 state.
        unsafe {
            asm!(
                "fnstcw word ptr [{word}]",
                word = in(reg) &raw mut word,
                options(nostack, preserves_flags),
            );
        }
        word
    }

    /// Load `word` into the current thread's x87 control word.
    ///
    /// Callers change only the precision-control field, or restore a word read from
    /// this thread, so every exception stays masked as it was.
    pub(crate) fn load_control_word(word: u16) {
        // SAFETY: `fldcw` reads a 16-bit control word from a live, aligned `u16`. The
        // word differs from the thread's only in fields the callers own (see above), and
        // the control word is per-thread state no other thread observes.
        unsafe {
            asm!(
                "fldcw word ptr [{word}]",
                word = in(reg) &raw const word,
                options(nostack, preserves_flags),
            );
        }
    }

    /// Emit one binary64 operation as an `asm!` block over memory operands; `scaled`
    /// operations also receive the two 80-bit scale constants as `{down}` and `{up}`.
    macro_rules! x87_op {
        (@block $($operand:ident),+ ; [$($line:literal),+] ; $($scale:tt)*) => {{
            let mut result = 0.0_f64;
            // SAFETY: the block reads the binary64 operands (and, when scaled, the two
            // 80-bit scale constants) through pointers to live values, and writes only
            // the binary64 `result` on this frame. All eight x87 registers are declared
            // clobbered, so the register stack is empty on entry, and the block pops
            // everything it pushes, so it is empty on exit. It touches no flag and no
            // stack memory.
            unsafe {
                asm!(
                    $($line),+,
                    $($operand = in(reg) &raw const $operand,)+
                    $($scale)*
                    result = in(reg) &raw mut result,
                    out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                    out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                    options(nostack, preserves_flags),
                );
            }
            result
        }};
        ($(#[$attr:meta])* plain $name:ident($($operand:ident),+) => [$($line:literal),+ $(,)?]) => {
            $(#[$attr])*
            #[inline(always)]
            #[allow(
                clippy::inline_always,
                reason = "one operation inside a kernel's inner loop"
            )]
            pub(crate) fn $name($($operand: f64),+) -> f64 {
                x87_op!(@block $($operand),+ ; [$($line),+] ;)
            }
        };
        ($(#[$attr:meta])* scaled $name:ident($($operand:ident),+) => [$($line:literal),+ $(,)?]) => {
            $(#[$attr])*
            #[inline(always)]
            #[allow(
                clippy::inline_always,
                reason = "one operation inside a kernel's inner loop"
            )]
            pub(crate) fn $name($($operand: f64),+) -> f64 {
                x87_op!(
                    @block $($operand),+ ; [$($line),+] ;
                    down = in(reg) SCALE_DOWN.as_ptr(),
                    up = in(reg) SCALE_UP.as_ptr(),
                )
            }
        };
    }

    x87_op! {
        /// `a + b`: one rounding to 53 bits, then an exact (or overflowing) store.
        plain add(a, b) => [
            "fld qword ptr [{a}]",
            "fadd qword ptr [{b}]",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `a − b`: one rounding to 53 bits, then an exact (or overflowing) store.
        plain sub(a, b) => [
            "fld qword ptr [{a}]",
            "fsub qword ptr [{b}]",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `a × b`: `a` scaled down by 2¹⁵³⁶⁰ (exact), one rounding at binary64's grid
        /// scaled likewise, scaled back up (exact), stored (exact or overflowing).
        scaled mul(a, b) => [
            "fld tbyte ptr [{down}]",
            "fmul qword ptr [{a}]",
            "fmul qword ptr [{b}]",
            "fld tbyte ptr [{up}]",
            "fmulp st(1), st",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `a ÷ b`: the dividend scaled down by 2¹⁵³⁶⁰ (exact), one rounding at
        /// binary64's grid scaled likewise, scaled back up (exact), stored.
        scaled div(a, b) => [
            "fld tbyte ptr [{down}]",
            "fmul qword ptr [{a}]",
            "fdiv qword ptr [{b}]",
            "fld tbyte ptr [{up}]",
            "fmulp st(1), st",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `√a`: one rounding to 53 bits of a result inside binary64's normal range (or a
        /// zero, an infinity or a NaN, which are exact), then an exact store.
        plain sqrt(a) => [
            "fld qword ptr [{a}]",
            "fsqrt",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `a × b` with no scaling: the double rounding in the subnormal range that the
        /// scaled [`mul`] closes, for the test that shows the probe observes it.
        #[cfg(test)]
        plain mul_unscaled(a, b) => [
            "fld qword ptr [{a}]",
            "fmul qword ptr [{b}]",
            "fstp qword ptr [{result}]",
        ]
    }

    x87_op! {
        /// `a ÷ b` with no scaling, as [`mul_unscaled`].
        #[cfg(test)]
        plain div_unscaled(a, b) => [
            "fld qword ptr [{a}]",
            "fdiv qword ptr [{b}]",
            "fstp qword ptr [{result}]",
        ]
    }
}

/// The test hook that stops [`Precision::enter`] from setting the precision field on the
/// calling thread, standing in for a thread whose guard did not take hold.
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
