// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hand-rolled, zero-dependency codecs for `xsd:hexBinary` and `xsd:base64Binary`.
//!
//! The value space for
//! both is a byte sequence; value-equality is byte equality. The two datatypes have
//! DIFFERENT value spaces, so a hexBinary byte sequence and a base64Binary byte
//! sequence with identical bytes are nonetheless INCOMPARABLE (different value spaces).
//!
//! # hexBinary
//!
//! XSD hexBinary lexical space: an even number of hexadecimal digits [0-9A-Fa-f].
//! The empty string is a valid lexical form for the zero-length byte sequence.
//! Canonical form uses UPPERCASE hex digits.
//!
//! # base64Binary
//!
//! XSD base64Binary lexical space: RFC 4648 base64 encoding, with the XSD extension
//! that ASCII whitespace is permitted between characters in the lexical form.
//! Canonical form has NO whitespace and uses standard padding (`=`).
//!
//! Both codecs hard-fail (`XsdError::InvalidLexical`) on any malformed input.

use crate::datatype::XsdDatatype;
use crate::value::XsdError;

// ── hex codec ────────────────────────────────────────────────────────────────────

/// Decode an XSD `hexBinary` lexical form to a byte vector.
///
/// Rules (XSD hexBinary):
/// - The string must contain only hex digits `[0-9A-Fa-f]`.
/// - The string length must be even (two hex digits per byte).
/// - The empty string is valid and decodes to an empty `Vec<u8>`.
/// - Whitespace and any other non-hex character is a hard failure.
///
/// The lexical space is base16's either-case one, read by
/// [`purrdf_hash::hex::decode`]; this function names the refusal in XSD terms.
pub fn parse_hex(lexical: &str) -> Result<Vec<u8>, XsdError> {
    purrdf_hash::hex::decode(lexical).map_err(|error| XsdError::InvalidLexical {
        datatype: XsdDatatype::HexBinary,
        lexical: lexical.to_string(),
        reason: match error {
            purrdf_hash::hex::HexError::OddLength { .. } => {
                "hexBinary lexical must have an even number of digits"
            }
            _ => "non-hexadecimal character in hexBinary lexical",
        },
    })
}

/// Encode a byte slice to XSD canonical hexBinary form (UPPERCASE hex, two chars per byte):
/// [`purrdf_hash::hex::encode_upper`].
pub fn canonical_hex(bytes: &[u8]) -> String {
    purrdf_hash::hex::encode_upper(bytes)
}

// ── base64 codec ──────────────────────────────────────────────────────────────────

/// The standard RFC 4648 base64 alphabet (A-Z a-z 0-9 + /).
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Decode an XSD `base64Binary` lexical form to a byte vector.
///
/// XSD rules:
/// - The alphabet is the standard base64 alphabet `A-Z a-z 0-9 + /`.
/// - ASCII whitespace (0x09 HT, 0x0A LF, 0x0D CR, 0x20 SP) between characters is
///   permitted and stripped before processing.
/// - After stripping whitespace the remaining string length must be a multiple of 4.
/// - Padding character `=` is only allowed at the end: 0, 1, or 2 trailing `=`.
/// - An internal `=` (before the final group) is a hard failure.
/// - Over-padding (`====`, `TQ===`, etc.) is a hard failure.
/// - Any character outside the alphabet is a hard failure.
/// - The character before the padding encodes no bits past the last byte (`B16`
///   before `=`, `B04` before `==`): `AQ==` and `AQI=` are valid, `AR==` and
///   `AQJ=` are not.
/// - The empty string (after stripping whitespace) is valid and decodes to empty `Vec<u8>`.
pub fn parse_base64(lexical: &str) -> Result<Vec<u8>, XsdError> {
    let plan = BinaryPlan::new(XsdDatatype::Base64Binary, lexical)
        .map_err(|error| error.into_xsd(lexical))?;
    let mut output = vec![0; plan.len()];
    plan.decode_into(&mut output)
        .map_err(|error| error.into_xsd(lexical))?;
    Ok(output)
}

/// Map a base64 alphabet character to its 6-bit value, or `None` for invalid chars.
#[inline]
fn decode_b64_char(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Encode a byte slice to XSD canonical base64Binary form (standard base64, `=` padding, no whitespace).
pub fn canonical_base64(bytes: &[u8]) -> String {
    let full_groups = bytes.len() / 3;
    let remainder = bytes.len() % 3;
    let length = (full_groups + usize::from(remainder > 0)) * 4;
    let mut output = String::with_capacity(length);
    write_base64(bytes, &mut output).expect("resident String formatting");
    output
}

/// Borrowed canonical RFC 4648 text through the existing native encoder.
#[derive(Debug, Clone, Copy)]
pub struct Base64<'a>(pub &'a [u8]);
impl core::fmt::Display for Base64<'_> {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write_base64(self.0, output)
    }
}

fn write_base64(bytes: &[u8], out: &mut impl core::fmt::Write) -> core::fmt::Result {
    if bytes.is_empty() {
        return Ok(());
    }

    let full_groups = bytes.len() / 3;
    let remainder = bytes.len() % 3;

    for g in 0..full_groups {
        let base = g * 3;
        let (a, b, c) = (bytes[base], bytes[base + 1], bytes[base + 2]);
        out.write_char(char::from(BASE64_ALPHABET[(a >> 2) as usize]))?;
        out.write_char(char::from(
            BASE64_ALPHABET[((a & 0x03) << 4 | b >> 4) as usize],
        ))?;
        out.write_char(char::from(
            BASE64_ALPHABET[((b & 0x0F) << 2 | c >> 6) as usize],
        ))?;
        out.write_char(char::from(BASE64_ALPHABET[(c & 0x3F) as usize]))?;
    }

    if remainder > 0 {
        let base = full_groups * 3;
        // remainder is the result of `% 3`, so only 1 or 2 can reach here (0 skips
        // the entire `if` block). Both arms are explicit; no wildcard is needed.
        if remainder == 1 {
            let a = bytes[base];
            out.write_char(char::from(BASE64_ALPHABET[(a >> 2) as usize]))?;
            out.write_char(char::from(BASE64_ALPHABET[((a & 0x03) << 4) as usize]))?;
            out.write_char('=')?;
        } else {
            // remainder == 2
            let (a, b) = (bytes[base], bytes[base + 1]);
            out.write_char(char::from(BASE64_ALPHABET[(a >> 2) as usize]))?;
            out.write_char(char::from(
                BASE64_ALPHABET[((a & 0x03) << 4 | b >> 4) as usize],
            ))?;
            out.write_char(char::from(BASE64_ALPHABET[((b & 0x0F) << 2) as usize]))?;
        }
        // Both remainder arms end with one shared padding '='; remainder == 1 adds
        // its second '=' above, keeping the emitted bytes identical.
        out.write_char('=')?;
    }

    Ok(())
}

/// Borrowed native parse failures. No lexical String is constructed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryReadError {
    /// The lexical is outside the datatype's binary value space.
    InvalidLexical {
        /// Datatype whose lexical rules were applied.
        datatype: XsdDatatype,
        /// Stable native diagnostic reason.
        reason: &'static str,
    },
    /// The destination layout exceeds addressable storage.
    LayoutOverflow,
    /// A preallocated destination cannot hold the decoded bytes.
    OutputTooShort {
        /// Required decoded byte length.
        needed: usize,
        /// Provided destination length.
        available: usize,
    },
}

impl BinaryReadError {
    fn base64(reason: &'static str) -> Self {
        Self::InvalidLexical {
            datatype: XsdDatatype::Base64Binary,
            reason,
        }
    }

    fn into_xsd(self, lexical: &str) -> XsdError {
        match self {
            Self::InvalidLexical { datatype, reason } => {
                XsdError::invalid(datatype, lexical, reason)
            }
            // These are unreachable in the resident exact-size adapter for a
            // real &str, but remain hard failures rather than decoded success.
            Self::LayoutOverflow => XsdError::invalid(
                XsdDatatype::Base64Binary,
                lexical,
                "binary output layout overflow",
            ),
            Self::OutputTooShort { .. } => XsdError::invalid(
                XsdDatatype::Base64Binary,
                lexical,
                "binary destination is too short",
            ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Base64Shape {
    groups: usize,
    padding: usize,
}

/// A checked native plan over immutable caller-owned lexical bytes.
#[derive(Clone, Copy, Debug)]
pub struct BinaryPlan<'a> {
    lexical: &'a str,
    datatype: XsdDatatype,
    output: std::alloc::Layout,
    base64: Option<Base64Shape>,
}

impl<'a> BinaryPlan<'a> {
    /// Validate the lexical without heap allocation and price its destination.
    pub fn new(datatype: XsdDatatype, lexical: &'a str) -> Result<Self, BinaryReadError> {
        let (length, base64) = match datatype {
            XsdDatatype::HexBinary => {
                let length = purrdf_hash::hex::decoded_len(lexical).map_err(|error| {
                    BinaryReadError::InvalidLexical {
                        datatype,
                        reason: match error {
                            purrdf_hash::hex::HexError::OddLength { .. } => {
                                "hexBinary lexical must have an even number of digits"
                            }
                            _ => "non-hexadecimal character in hexBinary lexical",
                        },
                    }
                })?;
                (length, None)
            }
            XsdDatatype::Base64Binary => {
                let shape = base64_shape(lexical)?;
                decode_base64_groups(lexical, shape, |_| {})?;
                let length = shape
                    .groups
                    .checked_mul(3)
                    .and_then(|n| n.checked_sub(shape.padding))
                    .ok_or(BinaryReadError::LayoutOverflow)?;
                (length, Some(shape))
            }
            _ => {
                return Err(BinaryReadError::InvalidLexical {
                    datatype,
                    reason: "parse_binary called with non-binary datatype",
                });
            }
        };
        let output =
            std::alloc::Layout::array::<u8>(length).map_err(|_| BinaryReadError::LayoutOverflow)?;
        Ok(Self {
            lexical,
            datatype,
            output,
            base64,
        })
    }

    #[must_use]
    /// The exact decoded byte destination layout.
    pub const fn output_layout(self) -> std::alloc::Layout {
        self.output
    }
    #[must_use]
    /// The decoded byte length.
    pub const fn len(self) -> usize {
        self.output.size()
    }
    #[must_use]
    /// Whether the decoded value contains no bytes.
    pub const fn is_empty(self) -> bool {
        self.output.size() == 0
    }

    /// Fill supplied bytes. No allocation/growth and no stripped lexical buffer.
    pub fn decode_into(self, output: &mut [u8]) -> Result<(), BinaryReadError> {
        if output.len() < self.len() {
            return Err(BinaryReadError::OutputTooShort {
                needed: self.len(),
                available: output.len(),
            });
        }
        if let Some(shape) = self.base64 {
            let mut cursor = 0;
            decode_base64_groups(self.lexical, shape, |bytes| {
                let end = cursor + bytes.len();
                output[cursor..end].copy_from_slice(bytes);
                cursor = end;
            })?;
            debug_assert_eq!(cursor, self.len());
        } else {
            purrdf_hash::hex::decode_to_slice(self.lexical, output).map_err(
                |error| match error {
                    purrdf_hash::hex::HexError::OutputTooShort { needed, available } => {
                        BinaryReadError::OutputTooShort { needed, available }
                    }
                    purrdf_hash::hex::HexError::OddLength { .. } => {
                        BinaryReadError::InvalidLexical {
                            datatype: self.datatype,
                            reason: "hexBinary lexical must have an even number of digits",
                        }
                    }
                    _ => BinaryReadError::InvalidLexical {
                        datatype: self.datatype,
                        reason: "non-hexadecimal character in hexBinary lexical",
                    },
                },
            )?;
        }
        Ok(())
    }
}

fn base64_bytes(lexical: &str) -> impl Iterator<Item = u8> + '_ {
    lexical
        .bytes()
        .filter(|&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

fn base64_shape(lexical: &str) -> Result<Base64Shape, BinaryReadError> {
    let (mut length, mut padding, mut internal) = (0_usize, 0_usize, false);
    for byte in base64_bytes(lexical) {
        length = length
            .checked_add(1)
            .ok_or(BinaryReadError::LayoutOverflow)?;
        if byte == b'=' {
            padding = padding
                .checked_add(1)
                .ok_or(BinaryReadError::LayoutOverflow)?;
        } else {
            internal |= padding != 0;
            padding = 0;
        }
    }
    if !length.is_multiple_of(4) {
        return Err(BinaryReadError::base64(
            "base64Binary lexical length (after stripping whitespace) must be a multiple of 4",
        ));
    }
    if padding > 2 {
        return Err(BinaryReadError::base64(
            "base64Binary lexical has more than 2 padding characters",
        ));
    }
    if internal {
        return Err(BinaryReadError::base64(
            "internal padding character '=' in base64Binary lexical",
        ));
    }
    Ok(Base64Shape {
        groups: length / 4,
        padding,
    })
}

/// The one native base64 group decoder; validation uses a zero-allocation sink.
fn decode_base64_groups(
    lexical: &str,
    shape: Base64Shape,
    mut emit: impl FnMut(&[u8]),
) -> Result<(), BinaryReadError> {
    let mut input = base64_bytes(lexical);
    for group in 0..shape.groups {
        let mut raw = [0_u8; 4];
        for byte in &mut raw {
            *byte = input
                .next()
                .expect("the checked shape counts all group bytes");
        }
        let [a_raw, b_raw, c_raw, d_raw] = raw;
        let invalid = || BinaryReadError::base64("invalid character in base64Binary lexical");
        let a = decode_b64_char(a_raw).ok_or_else(invalid)?;
        let b = decode_b64_char(b_raw).ok_or_else(invalid)?;
        let last = group == shape.groups - 1;
        let (c, d, length) = if last && shape.padding == 2 {
            if c_raw != b'=' || d_raw != b'=' {
                return Err(BinaryReadError::base64(
                    "expected padding characters '==' in base64Binary lexical",
                ));
            }
            if b & 0b1111 != 0 {
                return Err(BinaryReadError::base64(
                    "base64Binary character before '==' must be one of AQgw",
                ));
            }
            (0, 0, 1)
        } else if last && shape.padding == 1 {
            let c = decode_b64_char(c_raw).ok_or_else(invalid)?;
            if d_raw != b'=' {
                return Err(BinaryReadError::base64(
                    "expected padding character '=' in base64Binary lexical",
                ));
            }
            if c & 0b11 != 0 {
                return Err(BinaryReadError::base64(
                    "base64Binary character before '=' must be one of AEIMQUYcgkosw048",
                ));
            }
            (c, 0, 2)
        } else {
            (
                decode_b64_char(c_raw).ok_or_else(invalid)?,
                decode_b64_char(d_raw).ok_or_else(invalid)?,
                3,
            )
        };
        let output = [(a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d];
        emit(&output[..length]);
    }
    Ok(())
}

// ── dispatch ──────────────────────────────────────────────────────────────────────

/// Dispatch `hexBinary` or `base64Binary` lexical parsing to the appropriate codec.
///
/// The `datatype` argument must be [`XsdDatatype::HexBinary`] or [`XsdDatatype::Base64Binary`];
/// any other value triggers a hard `InvalidLexical` error.
pub fn parse_binary(datatype: XsdDatatype, lexical: &str) -> Result<Vec<u8>, XsdError> {
    match datatype {
        XsdDatatype::HexBinary => parse_hex(lexical),
        XsdDatatype::Base64Binary => parse_base64(lexical),
        _ => Err(XsdError::invalid(
            datatype,
            lexical,
            "parse_binary called with non-binary datatype",
        )),
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_base64_matches_the_rfc_4648_test_vectors() {
        // RFC 4648 §10, verbatim: the padding boundaries are where an encoder goes wrong,
        // and every one of them is exercised here.
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(
                canonical_base64(input.as_bytes()),
                expected,
                "input {input:?}"
            );
        }
        // A byte outside ASCII exercises the high bits of the 24-bit group.
        assert_eq!(canonical_base64(&[0xff, 0xef, 0xbf]), "/++/");
    }

    // ── hexBinary positive ────────────────────────────────────────────────────────

    #[test]
    fn hex_decode_basic() {
        assert_eq!(parse_hex("0FB7").unwrap(), vec![0x0F, 0xB7]);
    }

    #[test]
    fn hex_decode_empty() {
        assert_eq!(parse_hex("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn hex_decode_lowercase() {
        assert_eq!(parse_hex("0f").unwrap(), vec![0x0F]);
        assert_eq!(parse_hex("0fb7").unwrap(), vec![0x0F, 0xB7]);
    }

    #[test]
    fn hex_case_insensitive_value_equality() {
        let upper = parse_hex("0F").unwrap();
        let lower = parse_hex("0f").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn hex_canonical_is_uppercase() {
        assert_eq!(canonical_hex(&[0x0F, 0xB7]), "0FB7");
        assert_eq!(canonical_hex(&[0x00, 0xFF]), "00FF");
        assert_eq!(canonical_hex(&[]), "");
    }

    #[test]
    fn hex_canonical_of_lowercase_input() {
        let bytes = parse_hex("0fb7").unwrap();
        assert_eq!(canonical_hex(&bytes), "0FB7");
    }

    #[test]
    fn hex_all_byte_values_round_trip() {
        let all_bytes: Vec<u8> = (0u8..=255).collect();
        let encoded = canonical_hex(&all_bytes);
        let decoded = parse_hex(&encoded).unwrap();
        assert_eq!(decoded, all_bytes);
    }

    // ── hexBinary negative ────────────────────────────────────────────────────────

    #[test]
    fn hex_odd_length_is_error() {
        assert!(parse_hex("0F0").is_err(), "odd-length hex must be rejected");
        assert!(parse_hex("F").is_err());
        assert!(parse_hex("ABC").is_err());
    }

    #[test]
    fn hex_bad_char_is_error() {
        assert!(parse_hex("0G").is_err(), "non-hex char G must be rejected");
        assert!(parse_hex("GG").is_err());
    }

    #[test]
    fn hex_whitespace_is_error() {
        assert!(
            parse_hex("0 F").is_err(),
            "whitespace in hex must be rejected"
        );
        assert!(parse_hex("0\tF").is_err());
        assert!(parse_hex(" 0F").is_err());
        assert!(parse_hex("0F ").is_err());
    }

    #[test]
    fn hex_zz_is_error() {
        assert!(parse_hex("zz").is_err(), "'z' is not a hex digit");
    }

    // ── base64Binary positive ─────────────────────────────────────────────────────

    #[test]
    fn base64_man() {
        assert_eq!(parse_base64("TWFu").unwrap(), b"Man");
    }

    #[test]
    fn base64_ma_with_one_pad() {
        assert_eq!(parse_base64("TWE=").unwrap(), b"Ma");
    }

    #[test]
    fn base64_m_with_two_pads() {
        assert_eq!(parse_base64("TQ==").unwrap(), b"M");
    }

    #[test]
    fn base64_aaaa_is_zero_bytes() {
        assert_eq!(parse_base64("AAAA").unwrap(), vec![0x00u8, 0x00, 0x00]);
    }

    #[test]
    fn base64_empty() {
        assert_eq!(parse_base64("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn base64_whitespace_only_is_empty() {
        assert_eq!(parse_base64("   ").unwrap(), Vec::<u8>::new());
        assert_eq!(parse_base64("\t\n\r").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn base64_whitespace_tolerant_mid() {
        let with_space = parse_base64("TW Fu").unwrap();
        let without = parse_base64("TWFu").unwrap();
        assert_eq!(with_space, without);
    }

    #[test]
    fn base64_whitespace_tolerant_newline() {
        let with_newline = parse_base64("TWFu\n").unwrap();
        let without = parse_base64("TWFu").unwrap();
        assert_eq!(with_newline, without);
    }

    #[test]
    fn base64_canonical_man() {
        let bytes = b"Man";
        let encoded = canonical_base64(bytes);
        assert_eq!(encoded, "TWFu");
        let decoded = parse_base64(&encoded).unwrap();
        assert_eq!(&decoded, bytes.as_ref());
    }

    #[test]
    fn base64_canonical_one_pad() {
        let bytes = b"Ma";
        let encoded = canonical_base64(bytes);
        assert_eq!(encoded, "TWE=");
        let decoded = parse_base64(&encoded).unwrap();
        assert_eq!(&decoded, bytes.as_ref());
    }

    #[test]
    fn base64_canonical_two_pads() {
        let bytes = b"M";
        let encoded = canonical_base64(bytes);
        assert_eq!(encoded, "TQ==");
        let decoded = parse_base64(&encoded).unwrap();
        assert_eq!(&decoded, bytes.as_ref());
    }

    #[test]
    fn base64_canonical_empty() {
        assert_eq!(canonical_base64(&[]), "");
    }

    #[test]
    fn base64_full_alphabet_round_trip() {
        let all_bytes: Vec<u8> = (0u8..=255).collect();
        let encoded = canonical_base64(&all_bytes);
        let decoded = parse_base64(&encoded).unwrap();
        assert_eq!(decoded, all_bytes);
    }

    // ── base64Binary negative ─────────────────────────────────────────────────────

    #[test]
    fn base64_length_not_multiple_of_4_is_error() {
        assert!(
            parse_base64("AAA").is_err(),
            "length-3 base64 must be rejected"
        );
        assert!(parse_base64("A").is_err());
        assert!(parse_base64("AAAAA").is_err());
    }

    #[test]
    fn base64_four_equals_is_error() {
        assert!(
            parse_base64("====").is_err(),
            "four '=' chars must be rejected"
        );
    }

    #[test]
    fn base64_internal_pad_is_error() {
        assert!(
            parse_base64("AB=C").is_err(),
            "internal '=' must be rejected"
        );
    }

    #[test]
    fn base64_bad_char_at_sign_is_error() {
        assert!(
            parse_base64("@@@@").is_err(),
            "'@' is not a base64 character"
        );
    }

    #[test]
    fn base64_over_pad_is_error() {
        assert!(
            parse_base64("TQ===").is_err(),
            "triple-pad must be rejected"
        );
    }

    /// XSD 1.1 Part 2 §3.3.17: the character before the padding carries no bits
    /// past the last encoded byte (`B16` before `=`, `B04` before `==`).
    #[test]
    fn base64_padding_bits_must_be_zero() {
        for refused in ["AQJ=", "AR==", "Zh==", "Zm9=", "AQ J="] {
            assert!(parse_base64(refused).is_err(), "{refused:?}");
        }
        for (accepted, bytes) in [
            ("AQI=", &[0x01, 0x02][..]),
            ("AQ==", &[0x01][..]),
            ("Zg==", b"f"),
            ("Zm8=", b"fo"),
            ("Zm9v", b"foo"),
        ] {
            assert_eq!(parse_base64(accepted).as_deref(), Ok(bytes), "{accepted:?}");
        }
    }

    // ── dispatch ──────────────────────────────────────────────────────────────────

    #[test]
    fn parse_binary_dispatches_hex() {
        let result = parse_binary(XsdDatatype::HexBinary, "0FB7").unwrap();
        assert_eq!(result, vec![0x0F, 0xB7]);
    }

    #[test]
    fn parse_binary_dispatches_base64() {
        let result = parse_binary(XsdDatatype::Base64Binary, "TWFu").unwrap();
        assert_eq!(result, b"Man");
    }

    #[test]
    fn parse_binary_wrong_datatype_is_error() {
        assert!(parse_binary(XsdDatatype::Integer, "0F").is_err());
    }
}
