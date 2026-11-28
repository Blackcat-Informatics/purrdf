// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The x87 control word and the correctly rounded operations.
//!
//! Only on a build whose binary floating-point unit is the x87. Every operation is one
//! `asm!` block with memory operands: it loads binary64 (binary32) values, performs one
//! rounding operation (bracketed by exact scalings for `×` and `÷`), and stores a binary64
//! (binary32) value. Each block declares all eight x87 registers clobbered, so the stack
//! is empty on entry, and leaves it empty on exit. None is `pure`: each is ordered after
//! the `fldcw` that set its precision, which is itself a side-effecting block the compiler
//! may not move them across. The operations are correct only inside the matching scope
//! ([`Binary64Scope`](super::Binary64Scope), [`Binary32Scope`](super::Binary32Scope)),
//! which is why they are reached through its token and not exported.

use core::arch::asm;

/// The precision-control field of the x87 control word (bits 8–9).
pub const PRECISION_CONTROL: u16 = 0b11 << 8;
/// The precision-control value for a 24-bit significand (binary32's).
pub const PRECISION_24: u16 = 0b00 << 8;
/// The precision-control value for a 53-bit significand (binary64's).
pub const PRECISION_53: u16 = 0b10 << 8;
/// The precision-control value for a 64-bit significand (the register's own; Linux's
/// initial value).
pub const PRECISION_64: u16 = 0b11 << 8;
/// The rounding-control field (bits 10–11); zero is round-to-nearest-even.
pub const ROUNDING_CONTROL: u16 = 0b11 << 10;

/// The x87 extended format's exponent bias.
const X87_BIAS: i32 = 16383;
/// The x87 extended format's smallest normal exponent, −16382.
const X87_EMIN: i32 = 1 - X87_BIAS;
/// Binary64's smallest normal exponent.
const BINARY64_EMIN: i32 = -1022;
/// Binary32's smallest normal exponent.
const BINARY32_EMIN: i32 = -126;

/// `2^exp` as an 80-bit extended value, little-endian: the significand
/// `0x8000_0000_0000_0000` (the explicit integer bit alone), then the biased exponent.
const fn power_of_two(exp: i32) -> [u8; 10] {
    let biased = (X87_BIAS + exp) as u16;
    [0, 0, 0, 0, 0, 0, 0, 0x80, biased as u8, (biased >> 8) as u8]
}

/// 2⁻¹⁵³⁶⁰ (biased exponent `0x03ff`): the distance from binary64's smallest normal
/// exponent down to the x87's.
static SCALE_DOWN_64: [u8; 10] = power_of_two(X87_EMIN - BINARY64_EMIN);
/// 2¹⁵³⁶⁰ (biased exponent `0x7bff`).
static SCALE_UP_64: [u8; 10] = power_of_two(BINARY64_EMIN - X87_EMIN);
/// 2⁻¹⁶²⁵⁶ (biased exponent `0x007f`): the distance from binary32's smallest normal
/// exponent down to the x87's.
static SCALE_DOWN_32: [u8; 10] = power_of_two(X87_EMIN - BINARY32_EMIN);
/// 2¹⁶²⁵⁶ (biased exponent `0x7f7f`).
static SCALE_UP_32: [u8; 10] = power_of_two(BINARY32_EMIN - X87_EMIN);

/// The current thread's x87 control word.
#[must_use]
pub fn control_word() -> u16 {
    let mut word: u16 = 0;
    // SAFETY: `fnstcw` stores the 16-bit control word, and nothing else, to the address it
    // is given: a live, aligned, writable `u16` on this frame. It changes no register,
    // flag or x87 state.
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
/// # Safety
///
/// Every floating-point operation this thread performs afterwards runs under `word`. The
/// caller must keep every exception masked that was masked (an unmasked exception traps
/// on the next operation that raises it), and must restore the word it replaced before
/// returning control to code that assumes the environment it found -- the scopes in this
/// module change only the precision-control field, and restore the saved word on drop.
pub unsafe fn load_control_word(word: u16) {
    // SAFETY: `fldcw` reads a 16-bit control word from a live, aligned `u16`; what the
    // word may contain is the caller's contract above. The control word is per-thread
    // state no other thread observes.
    unsafe {
        asm!(
            "fldcw word ptr [{word}]",
            word = in(reg) &raw const word,
            options(nostack, preserves_flags),
        );
    }
}

/// Set this thread's precision-control field to `precision`, changing no other field,
/// and return the word it replaced, for [`restore`].
pub(super) fn set_precision(precision: u16) -> u16 {
    let saved = control_word();
    let wanted = (saved & !PRECISION_CONTROL) | precision;
    if wanted != saved {
        // SAFETY: `wanted` is this thread's word with only the precision field changed, so
        // every exception stays masked as it was; the caller restores `saved`.
        unsafe { load_control_word(wanted) };
    }
    saved
}

/// Give this thread back the word [`set_precision`] returned.
pub(super) fn restore(saved: u16) {
    if control_word() != saved {
        // SAFETY: `saved` is the word this thread had before `set_precision`.
        unsafe { load_control_word(saved) };
    }
}

/// Emit one operation as an `asm!` block over memory operands of type `$ty`; `scaled`
/// operations also receive the two 80-bit scale constants as `{down}` and `{up}`.
macro_rules! x87_op {
    (@block $ty:ty ; $($operand:ident),+ ; [$($line:literal),+] ; $($scale:tt)*) => {{
        let mut result: $ty = 0.0;
        // SAFETY: the block reads the operands (and, when scaled, the two 80-bit scale
        // constants) through pointers to live values, and writes only `result` on this
        // frame, at the operands' width. All eight x87 registers are declared clobbered, so
        // the register stack is empty on entry, and the block pops everything it pushes,
        // so it is empty on exit. It touches no flag and no stack memory.
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
    (
        $(#[$attr:meta])*
        plain $name:ident($($operand:ident),+): $ty:ty => [$($line:literal),+ $(,)?]
    ) => {
        $(#[$attr])*
        #[inline(always)]
        #[allow(
            clippy::inline_always,
            reason = "one operation inside a kernel's inner loop"
        )]
        pub(super) fn $name($($operand: $ty),+) -> $ty {
            x87_op!(@block $ty ; $($operand),+ ; [$($line),+] ;)
        }
    };
    (
        $(#[$attr:meta])*
        scaled($down:ident, $up:ident) $name:ident($($operand:ident),+): $ty:ty
            => [$($line:literal),+ $(,)?]
    ) => {
        $(#[$attr])*
        #[inline(always)]
        #[allow(
            clippy::inline_always,
            reason = "one operation inside a kernel's inner loop"
        )]
        pub(super) fn $name($($operand: $ty),+) -> $ty {
            x87_op!(
                @block $ty ; $($operand),+ ; [$($line),+] ;
                down = in(reg) $down.as_ptr(),
                up = in(reg) $up.as_ptr(),
            )
        }
    };
}

// ---- binary64, under precision control at 53 bits -------------------------------------

x87_op! {
    /// `a + b`: one rounding to 53 bits, then an exact (or overflowing) store.
    plain add64(a, b): f64 => [
        "fld qword ptr [{a}]",
        "fadd qword ptr [{b}]",
        "fstp qword ptr [{result}]",
    ]
}

x87_op! {
    /// `a − b`: one rounding to 53 bits, then an exact (or overflowing) store.
    plain sub64(a, b): f64 => [
        "fld qword ptr [{a}]",
        "fsub qword ptr [{b}]",
        "fstp qword ptr [{result}]",
    ]
}

x87_op! {
    /// `a × b`: `a` scaled down by 2¹⁵³⁶⁰ (exact), one rounding on binary64's grid scaled
    /// likewise, scaled back up (exact), stored (exact or overflowing).
    scaled(SCALE_DOWN_64, SCALE_UP_64) mul64(a, b): f64 => [
        "fld tbyte ptr [{down}]",
        "fmul qword ptr [{a}]",
        "fmul qword ptr [{b}]",
        "fld tbyte ptr [{up}]",
        "fmulp st(1), st",
        "fstp qword ptr [{result}]",
    ]
}

x87_op! {
    /// `a ÷ b`: the dividend scaled down by 2¹⁵³⁶⁰ (exact), one rounding on binary64's
    /// grid scaled likewise, scaled back up (exact), stored.
    scaled(SCALE_DOWN_64, SCALE_UP_64) div64(a, b): f64 => [
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
    plain sqrt64(a): f64 => [
        "fld qword ptr [{a}]",
        "fsqrt",
        "fstp qword ptr [{result}]",
    ]
}

// ---- binary32, under precision control at 24 bits -------------------------------------

x87_op! {
    /// `a + b`: one rounding to 24 bits, then an exact (or overflowing) store.
    plain add32(a, b): f32 => [
        "fld dword ptr [{a}]",
        "fadd dword ptr [{b}]",
        "fstp dword ptr [{result}]",
    ]
}

x87_op! {
    /// `a − b`: one rounding to 24 bits, then an exact (or overflowing) store.
    plain sub32(a, b): f32 => [
        "fld dword ptr [{a}]",
        "fsub dword ptr [{b}]",
        "fstp dword ptr [{result}]",
    ]
}

x87_op! {
    /// `a × b`: `a` scaled down by 2¹⁶²⁵⁶ (exact), one rounding on binary32's grid scaled
    /// likewise, scaled back up (exact), stored (exact or overflowing).
    scaled(SCALE_DOWN_32, SCALE_UP_32) mul32(a, b): f32 => [
        "fld tbyte ptr [{down}]",
        "fmul dword ptr [{a}]",
        "fmul dword ptr [{b}]",
        "fld tbyte ptr [{up}]",
        "fmulp st(1), st",
        "fstp dword ptr [{result}]",
    ]
}

x87_op! {
    /// `a ÷ b`: the dividend scaled down by 2¹⁶²⁵⁶ (exact), one rounding on binary32's
    /// grid scaled likewise, scaled back up (exact), stored.
    scaled(SCALE_DOWN_32, SCALE_UP_32) div32(a, b): f32 => [
        "fld tbyte ptr [{down}]",
        "fmul dword ptr [{a}]",
        "fdiv dword ptr [{b}]",
        "fld tbyte ptr [{up}]",
        "fmulp st(1), st",
        "fstp dword ptr [{result}]",
    ]
}

x87_op! {
    /// `√a`: one rounding to 24 bits of a result inside binary32's normal range (or a
    /// zero, an infinity or a NaN, which are exact), then an exact store.
    plain sqrt32(a): f32 => [
        "fld dword ptr [{a}]",
        "fsqrt",
        "fstp dword ptr [{result}]",
    ]
}

// ---- the unscaled sequences, for the tests that show the scaling is needed -------------

x87_op! {
    /// `a × b` in binary64 with no scaling: the double rounding in the subnormal range
    /// that [`mul64`] closes.
    #[cfg(test)]
    plain mul64_unscaled(a, b): f64 => [
        "fld qword ptr [{a}]",
        "fmul qword ptr [{b}]",
        "fstp qword ptr [{result}]",
    ]
}

x87_op! {
    /// `a ÷ b` in binary64 with no scaling, as [`mul64_unscaled`].
    #[cfg(test)]
    plain div64_unscaled(a, b): f64 => [
        "fld qword ptr [{a}]",
        "fdiv qword ptr [{b}]",
        "fstp qword ptr [{result}]",
    ]
}

x87_op! {
    /// `a × b` in binary32 with no scaling: the double rounding in the subnormal range
    /// that [`mul32`] closes.
    #[cfg(test)]
    plain mul32_unscaled(a, b): f32 => [
        "fld dword ptr [{a}]",
        "fmul dword ptr [{b}]",
        "fstp dword ptr [{result}]",
    ]
}

x87_op! {
    /// `a ÷ b` in binary32 with no scaling, as [`mul32_unscaled`].
    #[cfg(test)]
    plain div32_unscaled(a, b): f32 => [
        "fld dword ptr [{a}]",
        "fdiv dword ptr [{b}]",
        "fstp dword ptr [{result}]",
    ]
}

#[cfg(test)]
mod tests {
    use super::{SCALE_DOWN_32, SCALE_DOWN_64, SCALE_UP_32, SCALE_UP_64};

    #[test]
    fn the_scale_constants_are_the_powers_the_derivation_names() {
        // 16383 − 15360 = 0x03ff, 16383 + 15360 = 0x7bff; 16383 ∓ 16256 = 0x007f, 0x7f7f.
        let power = |low: u8, high: u8| [0, 0, 0, 0, 0, 0, 0, 0x80, low, high];
        assert_eq!(SCALE_DOWN_64, power(0xff, 0x03));
        assert_eq!(SCALE_UP_64, power(0xff, 0x7b));
        assert_eq!(SCALE_DOWN_32, power(0x7f, 0x00));
        assert_eq!(SCALE_UP_32, power(0x7f, 0x7f));
    }
}
