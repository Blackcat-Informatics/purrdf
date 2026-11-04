// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked unsigned digit conversion and borrowed integer-word operations.

use core::fmt;
use core::num::NonZeroU64;

/// Why [`parse_u128`] refused an unsigned digit sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadixError {
    /// The radix is outside the supported range `2..=16`.
    InvalidRadix {
        /// The unsupported radix.
        radix: u8,
    },
    /// No digits were supplied; an empty sequence is not zero.
    Empty,
    /// A byte is not an ASCII digit of the selected radix.
    InvalidDigit {
        /// The byte offset of the first invalid digit.
        offset: usize,
        /// The offending byte.
        byte: u8,
    },
    /// Every digit is valid, but the value exceeds [`u128::MAX`].
    Overflow,
}

impl purrdf_hash::error::StructuredError for RadixError {
    purrdf_hash::error_visit! {
        InvalidRadix { radix } => "integer-radix" {
            "radix" => Unsigned(u128::from(*radix)),
        },
        Empty => "integer-empty" {},
        InvalidDigit { offset, byte } => "integer-digit" {
            "offset" => ByteCount(*offset as u128),
            "byte" => Unsigned(u128::from(*byte)),
        },
        Overflow => "integer-overflow" {},
    }
}

impl fmt::Display for RadixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidRadix { radix } => {
                write!(f, "unsigned integer radix {radix} is outside 2..=16")
            }
            Self::Empty => f.write_str("an unsigned integer requires at least one digit"),
            Self::InvalidDigit { offset, byte } => {
                write!(
                    f,
                    "invalid unsigned integer digit {byte:#04x} at offset {offset}"
                )
            }
            Self::Overflow => f.write_str("the unsigned integer exceeds u128::MAX"),
        }
    }
}

impl core::error::Error for RadixError {}

/// Read an unsigned integer from ASCII digits in radix `2..=16`.
///
/// Digits `a` through `f` accept either ASCII letter case. Prefixes, signs,
/// separators, suffixes and whitespace are not accepted: their policy belongs
/// to the caller. Leading zeros do not affect the value or cause overflow.
/// The reader allocates nothing and uses constant storage, with work linear
/// in the number of bytes examined.
///
/// # Errors
///
/// An unsupported radix yields [`RadixError::InvalidRadix`] before inspecting
/// the input. For a supported radix, empty input yields [`RadixError::Empty`].
/// Otherwise, the first invalid byte yields [`RadixError::InvalidDigit`],
/// even after the preceding digits overflow. Only a fully valid sequence
/// whose value exceeds [`u128::MAX`] yields [`RadixError::Overflow`].
pub fn parse_u128(digits: &[u8], radix: u8) -> Result<u128, RadixError> {
    if !(2..=16).contains(&radix) {
        return Err(RadixError::InvalidRadix { radix });
    }
    if digits.is_empty() {
        return Err(RadixError::Empty);
    }
    let mut value = Some(0_u128);
    for (offset, &byte) in digits.iter().enumerate() {
        let digit = purrdf_hash::hex::nibble(byte)
            .filter(|&digit| digit < radix)
            .ok_or(RadixError::InvalidDigit { offset, byte })?;
        value = value.and_then(|value| {
            value
                .checked_mul(u128::from(radix))?
                .checked_add(u128::from(digit))
        });
    }
    value.ok_or(RadixError::Overflow)
}

/// Divide little-endian unsigned words in place by one nonzero word.
///
/// Returns the significant quotient length and remainder. Leading zero words
/// are accepted; all words outside the returned length are zero. The caller
/// owns storage and work admission. This operation allocates nothing.
pub fn div_rem_word(words: &mut [u64], divisor: NonZeroU64) -> (usize, u64) {
    let divisor = u128::from(divisor.get());
    let mut remainder = 0_u128;
    for word in words.iter_mut().rev() {
        let current = (remainder << 64) | u128::from(*word);
        *word = (current / divisor) as u64;
        remainder = current % divisor;
    }
    let length = words
        .iter()
        .rposition(|&word| word != 0)
        .map_or(0, |index| index + 1);
    (length, remainder as u64)
}
