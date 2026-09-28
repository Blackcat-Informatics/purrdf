// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lowercase base16 (RFC 4648 §8) rendering of byte strings, and the strict
//! decoder that reads it back.
//!
//! [`Lower`] wraps a byte slice and renders it through [`Display`]: each
//! byte becomes two characters of `0123456789abcdef`, high nibble first, with
//! no separator and no prefix, so a digest renders exactly as its `{:02x}`
//! byte-by-byte form does. [`lower`] is the same text as a `String`, and
//! [`encode_into`] writes it into a caller-owned buffer without allocating.
//!
//! ```
//! use purrdf_hash::hex::{decode, encode_into, lower, Lower};
//!
//! assert_eq!(Lower(b"foobar").to_string(), "666f6f626172");
//! assert_eq!(format!("sha256:{}", Lower(&[0x00, 0xff])), "sha256:00ff");
//! assert_eq!(lower(b"foobar"), "666f6f626172");
//!
//! let mut buffer = [0u8; 12];
//! assert_eq!(encode_into(b"foobar", &mut buffer).unwrap(), "666f6f626172");
//! assert_eq!(decode("666f6f626172").unwrap(), b"foobar");
//! ```
//!
//! Rendering never allocates: the characters are produced in a stack buffer
//! and handed to the formatter one chunk at a time, one `write_str` per chunk
//! of up to 128 input bytes (every digest this crate computes is one chunk).
//! The chunk is encoded on SSSE3 (x86-64, detected at run time), NEON
//! (AArch64) or wasm `simd128` (when compiled in), otherwise by the portable
//! table; every path writes the same bytes. [`encode_into`] and [`lower`]
//! run the same selected kernel over the whole input at once.
//!
//! # Decoding is strict
//!
//! [`decode`], [`decode_into`] and [`decode_32`] accept exactly the digits
//! `0-9`, `a-f` and `A-F`, in pairs: no sign, no `0x` prefix, no whitespace,
//! no separators, and no odd trailing digit. Uppercase digits are accepted
//! because RFC 4648 §8 makes base16 case-insensitive; lowercase is the one
//! canonical output, so a text is canonical exactly when it round-trips
//! through `lower(&decode(text)?) == text`.

use core::fmt::{self, Alignment, Display, Formatter, Write as _};

use crate::backend::HexBackend;

/// The lowercase base16 alphabet: RFC 4648 §8's, with `a`–`f` for 10–15.
pub(crate) const ALPHABET: &[u8; 16] = b"0123456789abcdef";

/// Input bytes encoded per `write_str`.
const CHUNK: usize = 128;

/// A byte string rendered as lowercase hexadecimal by [`Display`].
///
/// Construct it with any byte slice, including a `&[u8; N]` or a digest
/// array, which coerce: `Lower(&digest)`. Formatter width, fill, alignment and
/// precision apply to the rendered characters as they do to a `str`.
#[derive(Clone, Copy, Debug)]
pub struct Lower<'a>(pub &'a [u8]);

impl Display for Lower<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let encode = HexBackend::selected().encode_fn();
        if f.width().is_none() && f.precision().is_none() {
            return write_chars(f, self.0, encode, usize::MAX);
        }
        // `str` semantics: precision caps the characters written, width pads
        // what remains, left-aligned unless the formatter says otherwise.
        let total = self.0.len().saturating_mul(2);
        let shown = f
            .precision()
            .map_or(total, |precision| total.min(precision));
        let padding = f.width().map_or(0, |width| width.saturating_sub(shown));
        let (before, after) = match f.align() {
            None | Some(Alignment::Left) => (0, padding),
            Some(Alignment::Right) => (padding, 0),
            Some(Alignment::Center) => (padding / 2, padding - padding / 2),
        };
        let fill = f.fill();
        for _ in 0..before {
            f.write_char(fill)?;
        }
        write_chars(f, self.0, encode, shown)?;
        for _ in 0..after {
            f.write_char(fill)?;
        }
        Ok(())
    }
}

/// Writes the first `limit` characters of the encoding of `bytes`.
fn write_chars(
    f: &mut Formatter<'_>,
    bytes: &[u8],
    encode: crate::arch::HexEncode,
    mut limit: usize,
) -> fmt::Result {
    let mut buffer = [0u8; 2 * CHUNK];
    for chunk in bytes.chunks(CHUNK) {
        if limit == 0 {
            break;
        }
        let out = &mut buffer[..2 * chunk.len()];
        encode(chunk, out);
        let take = out.len().min(limit);
        // Every byte the alphabet holds is ASCII, so this never fails.
        let text = core::str::from_utf8(&out[..take]).map_err(|_| fmt::Error)?;
        f.write_str(text)?;
        limit -= take;
    }
    Ok(())
}

/// The portable encoder: `output` must be exactly twice as long as `input`.
pub(crate) fn encode_portable(input: &[u8], output: &mut [u8]) {
    debug_assert_eq!(output.len(), 2 * input.len());
    let (pairs, _) = output.as_chunks_mut::<2>();
    for (&byte, pair) in input.iter().zip(pairs) {
        *pair = [
            ALPHABET[usize::from(byte >> 4)],
            ALPHABET[usize::from(byte & 0x0f)],
        ];
    }
}

// --- Allocation-free encoding and the String form ---------------------------

/// Why [`encode_into`] refused its output buffer: it is shorter than twice
/// the input, or twice the input does not fit a `usize`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexLenError {
    /// The input length in bytes.
    pub input: usize,
    /// The output buffer's length in bytes; at least `2 * input` is needed.
    pub output: usize,
}

impl Display for HexLenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a base16 encoding of {} bytes needs {} output bytes, found {}",
            self.input,
            self.input.saturating_mul(2),
            self.output
        )
    }
}

impl core::error::Error for HexLenError {}

/// Writes the lowercase base16 encoding of `bytes` into the first
/// `2 * bytes.len()` bytes of `out` and returns that prefix as text, without
/// allocating.
///
/// `out` may be longer than needed; the rest of it is left untouched. The
/// encoding runs on the same selected kernel [`Lower`] renders through, so
/// the text is byte-for-byte what `Lower(bytes)` would print.
///
/// # Errors
///
/// [`HexLenError`] when `out` is shorter than `2 * bytes.len()`; nothing is
/// written then.
#[inline]
pub fn encode_into<'a>(bytes: &[u8], out: &'a mut [u8]) -> Result<&'a str, HexLenError> {
    let needed = bytes
        .len()
        .checked_mul(2)
        .filter(|needed| *needed <= out.len())
        .ok_or(HexLenError {
            input: bytes.len(),
            output: out.len(),
        })?;
    let out = &mut out[..needed];
    HexBackend::selected().encode_fn()(bytes, out);
    // Every byte the alphabet holds is ASCII, so this never fails.
    Ok(core::str::from_utf8(out).expect("the base16 alphabet is ASCII"))
}

/// The lowercase base16 encoding of `bytes` as a `String`: the text
/// [`Lower`] renders, in one exactly-sized allocation.
#[must_use]
pub fn lower(bytes: &[u8]) -> String {
    let mut out = vec![0u8; bytes.len() * 2];
    HexBackend::selected().encode_fn()(bytes, &mut out);
    // Every byte the alphabet holds is ASCII, so this never fails.
    String::from_utf8(out).expect("the base16 alphabet is ASCII")
}

// --- Decoding ---------------------------------------------------------------

/// Why a decoder refused its text.
///
/// Positions and lengths count bytes of the text, which for well-formed
/// base16 are the same as its characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexError {
    /// The text has an odd number of bytes, so its last digit has no pair.
    OddLength {
        /// The text's length.
        len: usize,
    },
    /// The byte at `index` is not one of `0-9`, `a-f` or `A-F`.
    InvalidDigit {
        /// The byte offset of the offending byte.
        index: usize,
        /// The offending byte (the first byte of a non-ASCII character).
        byte: u8,
    },
    /// The text does not hold exactly `expected` digits.
    Length {
        /// The number of digits the output needs.
        expected: usize,
        /// The number of bytes the text has.
        actual: usize,
    },
}

impl Display for HexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OddLength { len } => write!(f, "base16 text of {len} bytes has an odd length"),
            Self::InvalidDigit { index, byte } => {
                write!(
                    f,
                    "byte 0x{byte:02x} at index {index} is not a base16 digit"
                )
            }
            Self::Length { expected, actual } => {
                write!(f, "expected {expected} base16 digits, found {actual}")
            }
        }
    }
}

impl core::error::Error for HexError {}

/// The value of one base16 digit: `0-9`, `a-f` or `A-F`, otherwise `None`.
#[must_use]
#[inline]
pub const fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decodes `digits`, which has exactly twice `out`'s length, pair by pair.
fn decode_pairs(digits: &[u8], out: &mut [u8]) -> Result<(), HexError> {
    debug_assert_eq!(digits.len(), 2 * out.len());
    let (pairs, _) = digits.as_chunks::<2>();
    for (index, (pair, byte)) in pairs.iter().zip(out).enumerate() {
        let high = hex_value(pair[0]).ok_or(HexError::InvalidDigit {
            index: 2 * index,
            byte: pair[0],
        })?;
        let low = hex_value(pair[1]).ok_or(HexError::InvalidDigit {
            index: 2 * index + 1,
            byte: pair[1],
        })?;
        *byte = (high << 4) | low;
    }
    Ok(())
}

/// Decodes base16 `text` into new bytes.
///
/// Strict: an even number of digits from `0-9`, `a-f` and `A-F` and nothing
/// else. A sign, a `0x` prefix, whitespace anywhere or a separator is an
/// [`HexError::InvalidDigit`] at its byte offset; an odd length is
/// [`HexError::OddLength`]. Uppercase digits are accepted; `lower` renders
/// the canonical lowercase form.
///
/// # Errors
///
/// As above; `out` is not returned on any error.
pub fn decode(text: &str) -> Result<Vec<u8>, HexError> {
    let digits = text.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return Err(HexError::OddLength { len: digits.len() });
    }
    let mut out = vec![0u8; digits.len() / 2];
    decode_pairs(digits, &mut out)?;
    Ok(out)
}

/// Decodes base16 `text` into `out`, which must be exactly half as long as
/// the text, without allocating.
///
/// The same strict grammar as [`decode`]; a text that is not exactly
/// `2 * out.len()` bytes long is [`HexError::Length`]. On an error `out` may
/// be partly overwritten.
///
/// # Errors
///
/// As above.
pub fn decode_into(text: &str, out: &mut [u8]) -> Result<(), HexError> {
    let digits = text.as_bytes();
    let expected = 2 * out.len();
    if digits.len() != expected {
        return Err(HexError::Length {
            expected,
            actual: digits.len(),
        });
    }
    decode_pairs(digits, out)
}

/// Decodes exactly 64 base16 digits into a 32-byte digest.
///
/// The same strict grammar as [`decode`]: uppercase digits are accepted (the
/// text `"AB…"` and `"ab…"` decode to the same bytes), and lowercase is the
/// canonical rendering [`Lower`] and [`lower`] produce. Any other length is
/// [`HexError::Length`] with `expected: 64`.
///
/// # Errors
///
/// As above.
pub fn decode_32(text: &str) -> Result<[u8; 32], HexError> {
    let mut out = [0u8; 32];
    decode_into(text, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_value_agrees_with_to_digit_on_every_byte() {
        for b in 0..=u8::MAX {
            let expected = char::from(b).to_digit(16).map(|v| v as u8);
            assert_eq!(hex_value(b), expected, "0x{b:02x}");
        }
        assert_eq!(hex_value(b'0'), Some(0));
        assert_eq!(hex_value(b'9'), Some(9));
        assert_eq!(hex_value(b'a'), Some(10));
        assert_eq!(hex_value(b'f'), Some(15));
        assert_eq!(hex_value(b'A'), Some(10));
        assert_eq!(hex_value(b'F'), Some(15));
        assert_eq!(hex_value(b'g'), None);
        assert_eq!(hex_value(b' '), None);
        assert_eq!(hex_value(b'+'), None);
    }

    #[test]
    fn lower_and_encode_into_pin_the_rfc_4648_vectors() {
        let vectors: [(&[u8], &str); 7] = [
            (b"", ""),
            (b"f", "66"),
            (b"fo", "666f"),
            (b"foo", "666f6f"),
            (b"foob", "666f6f62"),
            (b"fooba", "666f6f6261"),
            (b"foobar", "666f6f626172"),
        ];
        for (input, expected) in vectors {
            assert_eq!(lower(input), expected);
            assert_eq!(Lower(input).to_string(), expected);
            let mut buffer = [0u8; 12];
            assert_eq!(encode_into(input, &mut buffer).unwrap(), expected);
        }
        assert_eq!(lower(&[0x00, 0xff]), "00ff");
        assert_eq!(lower(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    /// Every byte value, at every length 0..64, through every spelling of
    /// the encoder and back through the decoder, in both digit cases.
    #[test]
    fn round_trips_every_byte_value_at_every_length_through_64() {
        let mut buffer = [0u8; 128];
        for len in 0..64usize {
            for start in 0..256usize {
                let bytes: Vec<u8> = (0..len).map(|i| ((start + i) % 256) as u8).collect();
                let text = lower(&bytes);
                assert_eq!(text.len(), 2 * len);
                assert_eq!(Lower(&bytes).to_string(), text);
                assert_eq!(encode_into(&bytes, &mut buffer).unwrap(), text);
                assert_eq!(decode(&text).unwrap(), bytes, "{text}");
                assert_eq!(decode(&text.to_ascii_uppercase()).unwrap(), bytes);
                let mut back = vec![0u8; len];
                decode_into(&text, &mut back).unwrap();
                assert_eq!(back, bytes);
            }
        }
        // All 256 values in one string, longer than the formatter's chunk.
        let all: Vec<u8> = (0..=u8::MAX).collect();
        let text = lower(&all);
        assert_eq!(text.len(), 512);
        assert!(text.starts_with("000102") && text.ends_with("fdfeff"));
        assert_eq!(decode(&text).unwrap(), all);
        assert_eq!(Lower(&all).to_string(), text);
    }

    #[test]
    fn encode_into_uses_a_prefix_and_refuses_a_short_buffer() {
        let mut buffer = [b'.'; 8];
        assert_eq!(encode_into(b"ab", &mut buffer).unwrap(), "6162");
        assert_eq!(&buffer, b"6162....");
        assert_eq!(
            encode_into(b"abcde", &mut buffer),
            Err(HexLenError {
                input: 5,
                output: 8
            })
        );
        assert_eq!(&buffer, b"6162....", "nothing is written on refusal");
        assert_eq!(encode_into(b"abcd", &mut buffer).unwrap(), "61626364");
        assert_eq!(encode_into(b"", &mut []).unwrap(), "");
        assert_eq!(
            HexLenError {
                input: 5,
                output: 8
            }
            .to_string(),
            "a base16 encoding of 5 bytes needs 10 output bytes, found 8"
        );
    }

    #[test]
    fn decode_accepts_both_digit_cases_and_nothing_else() {
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
        assert_eq!(decode("DeadBEEF").unwrap(), [0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(decode("deadbeef").unwrap(), [0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(decode("00Ff"), Ok(vec![0x00, 0xff]));

        let invalid = |index, byte| Err(HexError::InvalidDigit { index, byte });
        assert_eq!(decode("+0"), invalid(0, b'+'));
        assert_eq!(decode("-0"), invalid(0, b'-'));
        assert_eq!(decode("+000"), invalid(0, b'+'));
        assert_eq!(decode("0x00"), invalid(1, b'x'));
        assert_eq!(decode(" 0"), invalid(0, b' '));
        assert_eq!(decode("00  "), invalid(2, b' '));
        assert_eq!(decode("00 0"), invalid(2, b' '));
        assert_eq!(decode("\n00\n"), invalid(0, b'\n'));
        assert_eq!(decode("\t0"), invalid(0, b'\t'));
        assert_eq!(decode("00:0"), invalid(2, b':'));
        assert_eq!(decode("gg"), invalid(0, b'g'));
        assert_eq!(decode("0G"), invalid(1, b'G'));
        assert_eq!(decode("\u{e9}\u{e9}"), invalid(0, 0xc3));
        assert_eq!(decode("00\u{e9}\u{e9}"), invalid(2, 0xc3));

        assert_eq!(decode("0"), Err(HexError::OddLength { len: 1 }));
        assert_eq!(decode("abc"), Err(HexError::OddLength { len: 3 }));
        assert_eq!(decode("+00"), Err(HexError::OddLength { len: 3 }));
        assert_eq!(decode("\u{e9}0"), Err(HexError::OddLength { len: 3 }));

        assert_eq!(
            HexError::OddLength { len: 3 }.to_string(),
            "base16 text of 3 bytes has an odd length"
        );
        assert_eq!(
            HexError::InvalidDigit {
                index: 1,
                byte: b'x'
            }
            .to_string(),
            "byte 0x78 at index 1 is not a base16 digit"
        );
        assert_eq!(
            HexError::Length {
                expected: 64,
                actual: 63
            }
            .to_string(),
            "expected 64 base16 digits, found 63"
        );
    }

    #[test]
    fn decode_into_needs_exactly_twice_the_output() {
        let mut out = [0u8; 4];
        decode_into("00112233", &mut out).unwrap();
        assert_eq!(out, [0x00, 0x11, 0x22, 0x33]);
        decode_into("AaBbCcDd", &mut out).unwrap();
        assert_eq!(out, [0xaa, 0xbb, 0xcc, 0xdd]);
        let length = |actual| {
            Err(HexError::Length {
                expected: 8,
                actual,
            })
        };
        assert_eq!(decode_into("001122", &mut out), length(6));
        assert_eq!(decode_into("0011223", &mut out), length(7));
        assert_eq!(decode_into("001122334", &mut out), length(9));
        assert_eq!(decode_into("0011223344", &mut out), length(10));
        assert_eq!(
            decode_into("0011 233", &mut out),
            Err(HexError::InvalidDigit {
                index: 4,
                byte: b' '
            })
        );
        decode_into("", &mut []).unwrap();
    }

    #[test]
    fn decode_32_needs_exactly_64_digits_in_either_case() {
        let bytes: [u8; 32] = core::array::from_fn(|i| (i * 8 + 1) as u8);
        let text = lower(&bytes);
        assert_eq!(text.len(), 64);
        assert_eq!(
            text,
            "0109111921293139414951596169717981899199a1a9b1b9c1c9d1d9e1e9f1f9"
        );
        assert_eq!(decode_32(&text).unwrap(), bytes);
        assert_eq!(decode_32(&text.to_ascii_uppercase()).unwrap(), bytes);
        assert_eq!(decode_32(&lower(&[0u8; 32])).unwrap(), [0u8; 32]);
        assert_eq!(decode_32(&"f".repeat(64)).unwrap(), [0xff; 32]);
        assert_eq!(decode_32(&"F".repeat(64)).unwrap(), [0xff; 32]);

        let length = |actual| {
            Err(HexError::Length {
                expected: 64,
                actual,
            })
        };
        assert_eq!(decode_32(""), length(0));
        assert_eq!(decode_32(&text[..63]), length(63));
        assert_eq!(decode_32(&format!("{text}0")), length(65));
        assert_eq!(decode_32(&format!("{text}00")), length(66));
        assert_eq!(decode_32(&format!(" {text}")), length(65));

        let mut signed = text.clone();
        signed.replace_range(0..1, "+");
        assert_eq!(
            decode_32(&signed),
            Err(HexError::InvalidDigit {
                index: 0,
                byte: b'+'
            })
        );
        let mut spaced = text;
        spaced.replace_range(10..11, " ");
        assert_eq!(
            decode_32(&spaced),
            Err(HexError::InvalidDigit {
                index: 10,
                byte: b' '
            })
        );
    }
}
