// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`any`]: the canonical strategy for a type.

use std::fmt;
use std::marker::PhantomData;

use super::choices::{Choices, Invalid};
use super::sample::{Index, IndexStrategy};
use super::strategy::Strategy;
use super::string::{RegexStrategy, regex};

/// A type with a canonical strategy.
pub trait Arbitrary: Sized + fmt::Debug {
    /// The canonical strategy's type.
    type Strategy: Strategy<Value = Self>;

    /// The canonical strategy.
    fn arbitrary() -> Self::Strategy;
}

/// The canonical strategy for `T`:
///
/// * `bool` — either value, shrinking to `false`;
/// * unsigned integers — uniform over the whole type, shrinking to 0;
/// * signed integers — uniform over the whole type, shrinking to 0 (the
///   choice is zig-zag decoded, so `0, -1, 1, -2, …` are its simplest values);
/// * `char` — uniform over the Unicode scalar values, shrinking to NUL;
/// * `String` — the regex `\PC*`: any run of characters that are not in a
///   Unicode "Other" category (so no control characters), shrinking to empty;
/// * [`Index`] — a position in a collection chosen later.
pub fn any<T: Arbitrary>() -> T::Strategy {
    T::arbitrary()
}

/// Either `bool`, shrinking to `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnyBool;

impl Strategy for AnyBool {
    type Value = bool;

    fn generate(&self, choices: &mut Choices) -> Result<bool, Invalid> {
        Ok(choices.draw(1)? == 1)
    }
}

impl Arbitrary for bool {
    type Strategy = AnyBool;

    fn arbitrary() -> AnyBool {
        AnyBool
    }
}

/// Any value of an integer type; see [`any`].
pub struct AnyInt<T>(PhantomData<fn() -> T>);

impl<T> Clone for AnyInt<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for AnyInt<T> {}

impl<T> fmt::Debug for AnyInt<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyInt<{}>", std::any::type_name::<T>())
    }
}

/// An unsigned maximum as a draw bound; every integer type here is at most 64
/// bits wide.
fn widest(max: impl TryInto<u64>) -> u64 {
    max.try_into()
        .unwrap_or_else(|_| unreachable!("integer types here are at most 64 bits"))
}

macro_rules! any_unsigned {
    ($($t:ty)+) => {$(
        impl Strategy for AnyInt<$t> {
            type Value = $t;

            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                Ok(choices.draw(widest(<$t>::MAX))? as $t)
            }
        }

        impl Arbitrary for $t {
            type Strategy = AnyInt<$t>;

            fn arbitrary() -> AnyInt<$t> {
                AnyInt(PhantomData)
            }
        }
    )+};
}

any_unsigned!(u8 u16 u32 u64 usize);

macro_rules! any_signed {
    ($($t:ty => $u:ty)+) => {$(
        impl Strategy for AnyInt<$t> {
            type Value = $t;

            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                let zigzag = choices.draw(widest(<$u>::MAX))? as $u;
                Ok(((zigzag >> 1) as $t) ^ -((zigzag & 1) as $t))
            }
        }

        impl Arbitrary for $t {
            type Strategy = AnyInt<$t>;

            fn arbitrary() -> AnyInt<$t> {
                AnyInt(PhantomData)
            }
        }
    )+};
}

any_signed!(i8 => u8 i16 => u16 i32 => u32 i64 => u64 isize => usize);

/// Any `char`; see [`any`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnyChar;

/// The number of Unicode scalar values: every code point but the surrogates.
const SCALAR_VALUES: u64 = 0x11_0000 - 0x800;

impl Strategy for AnyChar {
    type Value = char;

    fn generate(&self, choices: &mut Choices) -> Result<char, Invalid> {
        let index = choices.draw(SCALAR_VALUES - 1)? as u32;
        let code = if index >= 0xD800 {
            index + 0x800
        } else {
            index
        };
        Ok(char::from_u32(code).expect("the index skips the surrogates"))
    }
}

impl Arbitrary for char {
    type Strategy = AnyChar;

    fn arbitrary() -> AnyChar {
        AnyChar
    }
}

impl Arbitrary for String {
    type Strategy = RegexStrategy;

    fn arbitrary() -> RegexStrategy {
        regex("\\PC*")
    }
}

impl Arbitrary for Index {
    type Strategy = IndexStrategy;

    fn arbitrary() -> IndexStrategy {
        IndexStrategy
    }
}
