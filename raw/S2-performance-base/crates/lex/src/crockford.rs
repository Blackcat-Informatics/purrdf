// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Crockford Base32 for 128-bit values: the text form of a ULID.
//!
//! The alphabet is Douglas Crockford's Base32 — the ten decimal digits, then
//! the letters without I, L, O and U — and a 128-bit value is written as
//! exactly [`U128_DIGITS`] digits, most significant first, the first digit
//! carrying the top three bits (so it is at most `7`). That is the ULID
//! specification's canonical text form, and the one rendering every
//! deterministic identifier in the workspace (GTS ULIDs, the codecs' minted
//! blank-node labels) is written in; a label written by one and read by the
//! other must agree digit for digit, which is why there is one renderer.
//!
//! Reading accepts the canonical form in either letter case and nothing else:
//! no hyphens, and none of Crockford's decode-time aliases (`I`/`L` for `1`,
//! `O` for `0`), because a ULID's text form is canonical and an alias would
//! give one value two spellings.

use core::fmt;

/// The Crockford Base32 alphabet, indexed by digit value.
pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The number of digits a 128-bit value renders in: `ceil(128 / 5)`.
pub const U128_DIGITS: usize = 26;

/// Why [`parse_u128`] refused a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrockfordError {
    /// The text is not exactly [`U128_DIGITS`] bytes long.
    Length {
        /// The byte length found.
        found: usize,
    },
    /// The byte at `offset` is not a Crockford Base32 digit.
    Digit {
        /// The byte offset of the offending byte.
        offset: usize,
        /// The offending byte.
        byte: u8,
    },
    /// The first digit is greater than `7`, so the value needs more than 128
    /// bits.
    Overflow,
}

impl fmt::Display for CrockfordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Length { found } => write!(
                f,
                "a 128-bit Crockford Base32 value is {U128_DIGITS} characters, found {found}"
            ),
            Self::Digit { offset, byte } => write!(
                f,
                "invalid Crockford Base32 character {:?} at offset {offset}",
                char::from(byte)
            ),
            Self::Overflow => {
                f.write_str("the value exceeds 128 bits because the first digit is greater than 7")
            }
        }
    }
}

impl std::error::Error for CrockfordError {}

/// The [`U128_DIGITS`] Crockford Base32 digits of `value`, most significant
/// first; digit `i` is `(value >> (125 - 5 i)) & 0x1F`.
///
/// ```
/// use purrdf_lex::crockford::encode_u128;
///
/// assert_eq!(&encode_u128(0), b"00000000000000000000000000");
/// assert_eq!(&encode_u128(31), b"0000000000000000000000000Z");
/// assert_eq!(&encode_u128(u128::MAX), b"7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
/// ```
#[must_use]
pub fn encode_u128(value: u128) -> [u8; U128_DIGITS] {
    let mut digits = [0_u8; U128_DIGITS];
    for (index, byte) in digits.iter_mut().enumerate() {
        let shift = 125 - index * 5;
        *byte = ALPHABET[((value >> shift) & 0x1F) as usize];
    }
    digits
}

/// Append the [`U128_DIGITS`] Crockford Base32 digits of `value` to `out`.
///
/// # Errors
///
/// Only an error `out` itself reports.
///
/// ```
/// use purrdf_lex::crockford::write_u128;
///
/// let mut label = String::from("gts_");
/// write_u128(5, &mut label).unwrap();
/// assert_eq!(label, "gts_00000000000000000000000005");
/// ```
pub fn write_u128<W: fmt::Write + ?Sized>(value: u128, out: &mut W) -> fmt::Result {
    let digits = encode_u128(value);
    // Every byte comes from the ASCII alphabet.
    out.write_str(core::str::from_utf8(&digits).map_err(|_| fmt::Error)?)
}

/// The value of the Crockford Base32 digit `byte`, in either letter case, or
/// `None` when `byte` is not a digit of the alphabet.
///
/// ```
/// use purrdf_lex::crockford::decode_digit;
///
/// assert_eq!(decode_digit(b'Z'), Some(31));
/// assert_eq!(decode_digit(b'z'), Some(31));
/// assert_eq!(decode_digit(b'U'), None);
/// assert_eq!(decode_digit(b'I'), None);
/// ```
#[must_use]
pub const fn decode_digit(byte: u8) -> Option<u8> {
    let value = DECODE[byte as usize];
    if value == INVALID { None } else { Some(value) }
}

const INVALID: u8 = 0xFF;

/// Byte → digit value, both letter cases, [`INVALID`] elsewhere; derived from
/// [`ALPHABET`] so the reader cannot disagree with the writer.
const DECODE: [u8; 256] = {
    let mut table = [INVALID; 256];
    let mut digit = 0;
    while digit < 32 {
        let upper = ALPHABET[digit];
        table[upper as usize] = digit as u8;
        table[upper.to_ascii_lowercase() as usize] = digit as u8;
        digit += 1;
    }
    table
};

/// Read a 128-bit value from its [`U128_DIGITS`]-digit Crockford Base32 text.
///
/// # Errors
///
/// [`CrockfordError`] when the text is not exactly [`U128_DIGITS`] digits of
/// the alphabet, or its first digit is greater than `7`.
///
/// ```
/// use purrdf_lex::crockford::{CrockfordError, parse_u128};
///
/// assert_eq!(parse_u128("0000000000000000000000000z"), Ok(31));
/// assert_eq!(parse_u128("80000000000000000000000000"), Err(CrockfordError::Overflow));
/// ```
pub fn parse_u128(text: &str) -> Result<u128, CrockfordError> {
    if text.len() != U128_DIGITS {
        return Err(CrockfordError::Length { found: text.len() });
    }
    let mut value = 0_u128;
    for (offset, byte) in text.bytes().enumerate() {
        let digit = decode_digit(byte).ok_or(CrockfordError::Digit { offset, byte })?;
        if offset == 0 && digit > 7 {
            return Err(CrockfordError::Overflow);
        }
        value = (value << 5) | u128::from(digit);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        ALPHABET, CrockfordError, U128_DIGITS, decode_digit, encode_u128, parse_u128, write_u128,
    };

    fn rendered(value: u128) -> String {
        let mut out = String::new();
        write_u128(value, &mut out).expect("a String never fails");
        out
    }

    #[test]
    fn the_alphabet_omits_i_l_o_u() {
        for excluded in *b"ILOUilou" {
            assert!(!ALPHABET.contains(&excluded.to_ascii_uppercase()));
            assert_eq!(decode_digit(excluded), None);
        }
        for (value, &byte) in ALPHABET.iter().enumerate() {
            assert_eq!(decode_digit(byte), Some(value as u8));
            assert_eq!(decode_digit(byte.to_ascii_lowercase()), Some(value as u8));
        }
    }

    #[test]
    fn a_counter_below_two_to_the_eighty_renders_as_a_zero_timestamp_ulid() {
        // The minted blank-node labels: prefix plus the counter's digits.
        assert_eq!(rendered(0), "00000000000000000000000000");
        assert_eq!(rendered(1), "00000000000000000000000001");
        assert_eq!(rendered(32), "00000000000000000000000010");
        assert_eq!(rendered(1_000_000), "0000000000000000000000YGJ0");
    }

    #[test]
    fn values_round_trip_through_both_cases() {
        let mut state = 0x0DDB_A11C_0FFE_E000_u64;
        for _ in 0..2000 {
            let high = purrdf_testkit::rng::splitmix64_next(&mut state);
            let low = purrdf_testkit::rng::splitmix64_next(&mut state);
            let value = (u128::from(high) << 64) | u128::from(low);
            let text = rendered(value);
            assert_eq!(text.len(), U128_DIGITS);
            assert_eq!(parse_u128(&text), Ok(value));
            assert_eq!(parse_u128(&text.to_ascii_lowercase()), Ok(value));
        }
        assert_eq!(parse_u128(&rendered(u128::MAX)), Ok(u128::MAX));
    }

    #[test]
    fn a_first_digit_above_seven_is_refused_and_seven_is_read() {
        assert_eq!(
            parse_u128("80000000000000000000000000"),
            Err(CrockfordError::Overflow)
        );
        assert_eq!(parse_u128("70000000000000000000000000"), Ok(7_u128 << 125));
    }

    #[test]
    fn a_wrong_length_or_foreign_byte_is_refused_and_the_neighbour_is_read() {
        assert_eq!(
            parse_u128("0000000000000000000000000"),
            Err(CrockfordError::Length { found: 25 })
        );
        assert_eq!(
            parse_u128("000000000000000000000000000"),
            Err(CrockfordError::Length { found: 27 })
        );
        assert_eq!(
            parse_u128("0000000000000000000000000U"),
            Err(CrockfordError::Digit {
                offset: 25,
                byte: b'U'
            })
        );
        assert_eq!(
            parse_u128("000000000000-0000000000000"),
            Err(CrockfordError::Digit {
                offset: 12,
                byte: b'-'
            })
        );
        assert_eq!(parse_u128("0000000000000000000000000V"), Ok(27));
        assert_eq!(&encode_u128(27)[25..], b"V");
    }
}
