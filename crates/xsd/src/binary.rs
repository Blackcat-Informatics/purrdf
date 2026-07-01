// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hand-rolled, zero-dependency codecs for `xsd:hexBinary` and `xsd:base64Binary`.
//!
//! Both are genuine **EXTEND** — not present in `oxsdatatypes`. The value space for
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
pub fn parse_hex(lexical: &str) -> Result<Vec<u8>, XsdError> {
    let err = |reason| XsdError::InvalidLexical {
        datatype: XsdDatatype::HexBinary,
        lexical: lexical.to_string(),
        reason,
    };

    if !lexical.len().is_multiple_of(2) {
        return Err(err("hexBinary lexical must have an even number of digits"));
    }

    let bytes_len = lexical.len() / 2;
    let mut out = Vec::with_capacity(bytes_len);
    let chars: &[u8] = lexical.as_bytes();

    let mut i = 0;
    while i < chars.len() {
        let hi = hex_digit(chars[i])
            .ok_or_else(|| err("non-hexadecimal character in hexBinary lexical"))?;
        let lo = hex_digit(chars[i + 1])
            .ok_or_else(|| err("non-hexadecimal character in hexBinary lexical"))?;
        out.push((hi << 4) | lo);
        i += 2;
    }

    Ok(out)
}

/// Decode a single ASCII hex character `[0-9A-Fa-f]` to its 4-bit nibble value,
/// or `None` if the character is not a valid hex digit.
#[inline]
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Encode a byte slice to XSD canonical hexBinary form (UPPERCASE hex, two chars per byte).
pub fn canonical_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        // Both indices are masked to 0..15 so the table lookup is always in-range.
        out.push(char::from(HEX[(b >> 4) as usize]));
        out.push(char::from(HEX[(b & 0x0F) as usize]));
    }
    out
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
/// - The empty string (after stripping whitespace) is valid and decodes to empty `Vec<u8>`.
pub fn parse_base64(lexical: &str) -> Result<Vec<u8>, XsdError> {
    let err = |reason| XsdError::InvalidLexical {
        datatype: XsdDatatype::Base64Binary,
        lexical: lexical.to_string(),
        reason,
    };

    // Strip ASCII whitespace first (XSD base64Binary lexical space permits it).
    let stripped: Vec<u8> = lexical
        .bytes()
        .filter(|&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .collect();

    if stripped.is_empty() {
        return Ok(Vec::new());
    }

    // After stripping, length must be a multiple of 4.
    if !stripped.len().is_multiple_of(4) {
        return Err(err(
            "base64Binary lexical length (after stripping whitespace) must be a multiple of 4",
        ));
    }

    // Count and validate trailing `=` padding — at most 2.
    let pad_count = stripped.iter().rev().take_while(|&&b| b == b'=').count();
    if pad_count > 2 {
        return Err(err(
            "base64Binary lexical has more than 2 padding characters",
        ));
    }

    // Ensure `=` only appears at the end (no internal `=`).
    let data_len = stripped.len() - pad_count;
    for &b in &stripped[..data_len] {
        if b == b'=' {
            return Err(err(
                "internal padding character '=' in base64Binary lexical",
            ));
        }
    }

    // Decode groups of 4 characters.
    let mut out = Vec::with_capacity((stripped.len() / 4) * 3);
    let groups = stripped.len() / 4;

    for g in 0..groups {
        let base = g * 4;
        let is_last = g == groups - 1;

        let (a_raw, b_raw, c_raw, d_raw) = (
            stripped[base],
            stripped[base + 1],
            stripped[base + 2],
            stripped[base + 3],
        );

        if is_last && pad_count > 0 {
            // Final group with padding. pad_count is 1 or 2 (0 takes the else branch;
            // >2 was rejected above). Both arms are explicit; no wildcard needed.
            if pad_count == 1 {
                // Three base64 chars + one `=` → 2 output bytes.
                let a = decode_b64_char(a_raw)
                    .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
                let b = decode_b64_char(b_raw)
                    .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
                let c = decode_b64_char(c_raw)
                    .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
                if d_raw != b'=' {
                    return Err(err(
                        "expected padding character '=' in base64Binary lexical",
                    ));
                }
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
            } else {
                // pad_count == 2: Two base64 chars + two `=` → 1 output byte.
                let a = decode_b64_char(a_raw)
                    .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
                let b = decode_b64_char(b_raw)
                    .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
                if c_raw != b'=' || d_raw != b'=' {
                    return Err(err(
                        "expected padding characters '==' in base64Binary lexical",
                    ));
                }
                out.push((a << 2) | (b >> 4));
            }
        } else {
            // Full group (no padding needed).
            let a = decode_b64_char(a_raw)
                .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
            let b = decode_b64_char(b_raw)
                .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
            let c = decode_b64_char(c_raw)
                .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
            let d = decode_b64_char(d_raw)
                .ok_or_else(|| err("invalid character in base64Binary lexical"))?;
            out.push((a << 2) | (b >> 4));
            out.push((b << 4) | (c >> 2));
            out.push((c << 6) | d);
        }
    }

    Ok(out)
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
    if bytes.is_empty() {
        return String::new();
    }

    let full_groups = bytes.len() / 3;
    let remainder = bytes.len() % 3;
    let out_len = (full_groups + usize::from(remainder > 0)) * 4;
    // Build directly as a String — all output bytes are valid ASCII (BASE64_ALPHABET
    // + '='), so char::from is safe and no from_utf8 conversion is needed.
    let mut out = String::with_capacity(out_len);

    for g in 0..full_groups {
        let base = g * 3;
        let (a, b, c) = (bytes[base], bytes[base + 1], bytes[base + 2]);
        out.push(char::from(BASE64_ALPHABET[(a >> 2) as usize]));
        out.push(char::from(
            BASE64_ALPHABET[((a & 0x03) << 4 | b >> 4) as usize],
        ));
        out.push(char::from(
            BASE64_ALPHABET[((b & 0x0F) << 2 | c >> 6) as usize],
        ));
        out.push(char::from(BASE64_ALPHABET[(c & 0x3F) as usize]));
    }

    if remainder > 0 {
        let base = full_groups * 3;
        // remainder is the result of `% 3`, so only 1 or 2 can reach here (0 skips
        // the entire `if` block). Both arms are explicit; no wildcard is needed.
        if remainder == 1 {
            let a = bytes[base];
            out.push(char::from(BASE64_ALPHABET[(a >> 2) as usize]));
            out.push(char::from(BASE64_ALPHABET[((a & 0x03) << 4) as usize]));
            out.push('=');
        } else {
            // remainder == 2
            let (a, b) = (bytes[base], bytes[base + 1]);
            out.push(char::from(BASE64_ALPHABET[(a >> 2) as usize]));
            out.push(char::from(
                BASE64_ALPHABET[((a & 0x03) << 4 | b >> 4) as usize],
            ));
            out.push(char::from(BASE64_ALPHABET[((b & 0x0F) << 2) as usize]));
        }
        // Both remainder arms end with one shared padding '='; remainder == 1 adds
        // its second '=' above, keeping the emitted bytes identical.
        out.push('=');
    }

    out
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
        _ => Err(XsdError::InvalidLexical {
            datatype,
            lexical: lexical.to_string(),
            reason: "parse_binary called with non-binary datatype",
        }),
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
