// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 3986 §2.1 percent-encoding, and its inverse, spelled once.
//!
//! ```text
//! pct-encoded = "%" HEXDIG HEXDIG
//! unreserved  = ALPHA / DIGIT / "-" / "." / "_" / "~"
//! sub-delims  = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
//! pchar       = unreserved / pct-encoded / sub-delims / ":" / "@"
//! fragment    = *( pchar / "/" / "?" )
//! ```
//!
//! The workspace's minted IRIs — a rule IRI, a CSVW URI-template expansion, a
//! `file://` path segment, a JSON Pointer travelling in a `$ref` fragment —
//! each percent-encode with a different *kept* set and the same escape
//! spelling. The encoder here takes the kept set as a predicate over bytes,
//! so a writer names its set (one of the RFC productions below, or its own)
//! and never retypes the `%XX` loop; the named sets are the three the
//! workspace's writers actually keep.
//!
//! Encoding is byte-wise over UTF-8 and always spells a triplet with
//! **uppercase** hex, the RFC 3986 §2.1 normal form (§6.2.2.1: "uppercase
//! hexadecimal digits for all percent-encodings"), so two writers that keep
//! the same set emit the same bytes.
//!
//! Decoding is strict: a `%` must be followed by exactly two `HEXDIG`s (a
//! sign, a space or a third digit is not one), and the result is the decoded
//! **bytes** — a `%FF` is a lawful triplet whose byte is not UTF-8, and
//! whether that is an error is the caller's question, not this module's.
//! [`decode_form`] adds the `application/x-www-form-urlencoded` rule that a
//! `+` is a space.
//!
//! ```rust
//! use purrdf_iri::percent::{decode, decode_form, encode, is_unreserved};
//!
//! assert_eq!(encode("a b/€", is_unreserved), "a%20b%2F%E2%82%AC");
//! // Nothing to encode: the input is borrowed, not copied.
//! assert!(matches!(encode("plain-name", is_unreserved), std::borrow::Cow::Borrowed(_)));
//! assert_eq!(decode("a%20b%2f").unwrap(), b"a b/");
//! assert_eq!(decode_form("a+b%2B").unwrap(), b"a b+");
//! assert!(decode("100%").is_err());
//! ```

use core::fmt;
use std::borrow::Cow;

use crate::terminals::hex_value;

/// Uppercase `HEXDIG`, one byte per nibble — the RFC 3986 §2.1 normal form.
const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// RFC 3986 §2.3 `unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"`.
///
/// The set a writer keeps when everything else must be encoded: Python's
/// `urllib.parse.quote(value, safe="")` keeps exactly this, which is what the
/// workspace's rule-IRI minting was pinned to.
#[inline]
#[must_use]
pub const fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// RFC 3986 §2.2 `sub-delims`:
/// `"!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="`.
#[inline]
#[must_use]
pub const fn is_sub_delims(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// [`is_unreserved`] plus the path separator `/`: the set a writer keeps
/// when encoding a whole path whose segment boundaries are already the
/// caller's own (a document path minted under a base, `unreserved / "/"`).
#[inline]
#[must_use]
pub const fn is_unreserved_or_slash(b: u8) -> bool {
    is_unreserved(b) || b == b'/'
}

/// RFC 3986 §3.5 `fragment = *( pchar / "/" / "?" )` less the `pct-encoded`
/// alternative, which is a triplet rather than a byte:
/// `unreserved / sub-delims / ":" / "@" / "/" / "?"`.
///
/// The set a writer keeps when it places text into a fragment — a JSON
/// Pointer after the `#` of a JSON Schema `$ref` — so that the result is a
/// well-formed URI fragment. `%` is not in the set, so a literal `%` is
/// encoded as `%25` rather than left to be read as the start of a triplet.
#[inline]
#[must_use]
pub const fn is_pchar_or_slash_or_question(b: u8) -> bool {
    is_unreserved(b) || is_sub_delims(b) || matches!(b, b':' | b'@' | b'/' | b'?')
}

/// Append one `%XX` triplet, uppercase hex, for `b`.
#[inline]
fn push_triplet(out: &mut String, b: u8) {
    out.push('%');
    out.push(char::from(HEX_UPPER[usize::from(b >> 4)]));
    out.push(char::from(HEX_UPPER[usize::from(b & 0x0F)]));
}

/// Whether every byte of `ch`'s UTF-8 encoding is kept by `keep`.
#[inline]
fn scalar_kept(ch: char, keep: fn(u8) -> bool) -> bool {
    let mut buf = [0_u8; 4];
    ch.encode_utf8(&mut buf).bytes().all(keep)
}

/// Append `input` to `out`, percent-encoding every byte `keep` does not
/// admit as an uppercase `%XX` triplet (RFC 3986 §2.1).
///
/// The walk is byte-wise over the UTF-8 encoding of `input`: `keep` is asked
/// about every byte, ASCII or not, so a non-ASCII scalar is written as the
/// triplets of its UTF-8 bytes (`€` as `%E2%82%AC`) under any of the ASCII
/// sets above. A scalar is never half-encoded: a non-ASCII scalar is written
/// raw only when `keep` admits **every** byte of its encoding, and as
/// triplets otherwise, so `out` always stays a valid `String` and the
/// result decodes to the input under any UTF-8-aware reader.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::percent::{encode_with, is_unreserved, is_unreserved_or_slash};
///
/// let mut out = String::from("http://example.org/rule/");
/// encode_with(&mut out, "has value?", is_unreserved);
/// assert_eq!(out, "http://example.org/rule/has%20value%3F");
///
/// let mut path = String::new();
/// encode_with(&mut path, "a/b c.ttl", is_unreserved_or_slash);
/// assert_eq!(path, "a/b%20c.ttl");
/// ```
pub fn encode_with(out: &mut String, input: &str, keep: fn(u8) -> bool) {
    out.reserve(input.len());
    for ch in input.chars() {
        if scalar_kept(ch, keep) {
            out.push(ch);
        } else {
            let mut buf = [0_u8; 4];
            for b in ch.encode_utf8(&mut buf).bytes() {
                push_triplet(out, b);
            }
        }
    }
}

/// `input` with every byte `keep` does not admit percent-encoded — see
/// [`encode_with`] — borrowing `input` unchanged when nothing needs encoding.
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
///
/// use purrdf_iri::percent::{encode, is_pchar_or_slash_or_question, is_unreserved};
///
/// assert_eq!(encode("a b", is_unreserved), "a%20b");
/// assert!(matches!(encode("a-b_c.d~e", is_unreserved), Cow::Borrowed(_)));
/// // The fragment set keeps `/`, `?`, `:` and `@` but never `#` or `%`.
/// assert_eq!(
///     encode("/definitions/a%b#c", is_pchar_or_slash_or_question),
///     "/definitions/a%25b%23c"
/// );
/// ```
#[must_use]
pub fn encode(input: &str, keep: fn(u8) -> bool) -> Cow<'_, str> {
    let Some((first, _)) = input.char_indices().find(|&(_, ch)| !scalar_kept(ch, keep)) else {
        return Cow::Borrowed(input);
    };
    let mut out = String::with_capacity(input.len() + 8);
    out.push_str(&input[..first]);
    encode_with(&mut out, &input[first..], keep);
    Cow::Owned(out)
}

/// Why a percent-encoded string could not be decoded.
///
/// Both variants carry the byte offset of the `%` that opened the malformed
/// triplet, so a diagnostic can point at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PercentError {
    /// A `%` is followed by fewer than two bytes: the input ends inside the
    /// triplet.
    Truncated {
        /// Byte offset of the `%`.
        at: usize,
    },
    /// A `%` is followed by a byte that is not a `HEXDIG` in one of its two
    /// digit positions (`%2G`, `%+1`, `% 1`, `%%`).
    NotHex {
        /// Byte offset of the `%`.
        at: usize,
    },
}

impl PercentError {
    /// Byte offset of the `%` that opened the malformed triplet.
    #[must_use]
    pub const fn at(self) -> usize {
        match self {
            Self::Truncated { at } | Self::NotHex { at } => at,
        }
    }

    /// The human-readable reason, as a `&'static str`; [`fmt::Display`]
    /// renders this and the offset.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Truncated { .. } => "percent-encoding ends inside a `%XX` triplet",
            Self::NotHex { .. } => "`%` is not followed by two hexadecimal digits",
        }
    }
}

impl fmt::Display for PercentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message(), self.at())
    }
}

impl core::error::Error for PercentError {}

/// Decode `input`, `plus_is_space` selecting the form-encoding rule.
fn decode_inner(input: &str, plus_is_space: bool) -> Result<Vec<u8>, PercentError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let (Some(&hi), Some(&lo)) = (bytes.get(at + 1), bytes.get(at + 2)) else {
                    return Err(PercentError::Truncated { at });
                };
                let (Some(hi), Some(lo)) = (hex_value(hi), hex_value(lo)) else {
                    return Err(PercentError::NotHex { at });
                };
                out.push((hi << 4) | lo);
                at += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                at += 1;
            }
            b => {
                out.push(b);
                at += 1;
            }
        }
    }
    Ok(out)
}

/// The bytes `input` percent-encodes (RFC 3986 §2.1), every `%XX` triplet
/// replaced by its byte and every other byte copied as it is.
///
/// Strict: a `%` must be followed by exactly two `HEXDIG`s, either case. A
/// sign, a space, a third digit or the end of the input is a typed
/// [`PercentError`] naming the `%`'s offset, never a `%` passed through.
/// A `+` is an ordinary byte here — see [`decode_form`] for the form rule.
///
/// The result is bytes, not text: `%FF` is a lawful triplet whose byte is
/// not UTF-8, and a caller that needs a `String` decides what to do with
/// that (`String::from_utf8` on the result).
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::percent::{PercentError, decode};
///
/// assert_eq!(decode("a%20b").unwrap(), b"a b");
/// assert_eq!(decode("%e2%82%AC").unwrap(), "€".as_bytes());
/// assert_eq!(decode("a+b").unwrap(), b"a+b");
/// assert_eq!(decode("100%").unwrap_err(), PercentError::Truncated { at: 3 });
/// assert_eq!(decode("%2G").unwrap_err(), PercentError::NotHex { at: 0 });
/// assert_eq!(decode("%+1").unwrap_err(), PercentError::NotHex { at: 0 });
/// ```
pub fn decode(input: &str) -> Result<Vec<u8>, PercentError> {
    decode_inner(input, false)
}

/// [`decode`] under the `application/x-www-form-urlencoded` rule that a `+`
/// stands for a space (WHATWG URL §5.1; HTML 4.01 §17.13.4): the decoding a
/// SPARQL Protocol form body or query string needs.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::percent::{PercentError, decode_form};
///
/// assert_eq!(decode_form("query=SELECT+*+WHERE+%7B%7D").unwrap(), b"query=SELECT * WHERE {}");
/// // A literal `+` arrives as `%2B`.
/// assert_eq!(decode_form("1%2B1").unwrap(), b"1+1");
/// assert_eq!(decode_form("a%").unwrap_err(), PercentError::Truncated { at: 1 });
/// ```
pub fn decode_form(input: &str) -> Result<Vec<u8>, PercentError> {
    decode_inner(input, true)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{
        PercentError, decode, decode_form, encode, encode_with, is_pchar_or_slash_or_question,
        is_sub_delims, is_unreserved, is_unreserved_or_slash,
    };

    /// RFC 3986 §2.3 `unreserved`, transcribed independently as a string.
    const UNRESERVED: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    /// RFC 3986 §2.2 `sub-delims`, transcribed independently.
    const SUB_DELIMS: &str = "!$&'()*+,;=";

    #[test]
    fn named_sets_are_exactly_the_rfc_3986_productions_on_every_byte() {
        for b in 0..=u8::MAX {
            let unreserved = UNRESERVED.as_bytes().contains(&b);
            let sub_delims = SUB_DELIMS.as_bytes().contains(&b);
            assert_eq!(is_unreserved(b), unreserved, "{b:#04X}");
            assert_eq!(is_sub_delims(b), sub_delims, "{b:#04X}");
            assert_eq!(
                is_unreserved_or_slash(b),
                unreserved || b == b'/',
                "{b:#04X}"
            );
            assert_eq!(
                is_pchar_or_slash_or_question(b),
                unreserved || sub_delims || b":@/?".contains(&b),
                "{b:#04X}"
            );
        }
        // Cardinalities, so a stray member cannot hide in a passing sweep.
        assert_eq!((0..=u8::MAX).filter(|&b| is_unreserved(b)).count(), 66);
        assert_eq!((0..=u8::MAX).filter(|&b| is_sub_delims(b)).count(), 11);
        assert_eq!(
            (0..=u8::MAX)
                .filter(|&b| is_pchar_or_slash_or_question(b))
                .count(),
            81
        );
        // The sets are usable in const context.
        const { assert!(is_unreserved(b'~')) }
        const { assert!(!is_unreserved(b'%')) }
        const { assert!(is_pchar_or_slash_or_question(b'?')) }
        const { assert!(!is_pchar_or_slash_or_question(b'#')) }
    }

    #[test]
    fn encode_pins_uppercase_triplets_and_utf8_bytes() {
        assert_eq!(encode("a b", is_unreserved), "a%20b");
        assert_eq!(encode("100%", is_unreserved), "100%25");
        assert_eq!(encode("€", is_unreserved), "%E2%82%AC");
        assert_eq!(encode("é", is_unreserved), "%C3%A9");
        assert_eq!(encode("\u{1F600}", is_unreserved), "%F0%9F%98%80");
        assert_eq!(encode("\0\u{7F}", is_unreserved), "%00%7F");
        assert_eq!(encode("a/b?c#d", is_unreserved), "a%2Fb%3Fc%23d");
        assert_eq!(encode("a/b?c#d", is_unreserved_or_slash), "a/b%3Fc%23d");
        assert_eq!(
            encode("a/b?c#d:e@f%g", is_pchar_or_slash_or_question),
            "a/b?c%23d:e@f%25g"
        );
        assert_eq!(encode("", is_unreserved), "");
    }

    #[test]
    fn encode_borrows_only_when_nothing_is_encoded() {
        assert!(matches!(
            encode("plain-Name_1.~", is_unreserved),
            Cow::Borrowed("plain-Name_1.~")
        ));
        assert!(matches!(encode("", is_unreserved), Cow::Borrowed("")));
        assert!(matches!(encode("a b", is_unreserved), Cow::Owned(_)));
        // The neighbouring byte: `/` is kept by one set and encoded by the
        // other, and only the second copies.
        assert!(matches!(
            encode("a/b", is_unreserved_or_slash),
            Cow::Borrowed(_)
        ));
        assert!(matches!(encode("a/b", is_unreserved), Cow::Owned(_)));
    }

    #[test]
    fn encode_with_appends_and_matches_encode() {
        for input in ["", "a", "a b", "€/x", "%", "\u{1F600}#"] {
            for keep in [
                is_unreserved as fn(u8) -> bool,
                is_unreserved_or_slash,
                is_pchar_or_slash_or_question,
            ] {
                let mut out = String::from("prefix:");
                encode_with(&mut out, input, keep);
                assert_eq!(out, format!("prefix:{}", encode(input, keep)), "{input:?}");
            }
        }
    }

    /// A kept set that admits some but not all bytes of a scalar's UTF-8
    /// encoding: the scalar must be encoded whole, never half.
    fn keeps_continuation_bytes_only(b: u8) -> bool {
        is_unreserved(b) || (0x80..0xC0).contains(&b)
    }

    /// A kept set that admits every non-ASCII byte: the scalar is written raw.
    fn keeps_non_ascii(b: u8) -> bool {
        is_unreserved(b) || b >= 0x80
    }

    #[test]
    fn a_scalar_is_never_half_encoded() {
        assert_eq!(encode("é", keeps_continuation_bytes_only), "%C3%A9");
        assert_eq!(encode("é", keeps_non_ascii), "é");
        assert!(matches!(encode("é", keeps_non_ascii), Cow::Borrowed(_)));
        assert_eq!(encode("é ", keeps_non_ascii), "é%20");
        assert!(
            std::str::from_utf8(encode("é€", keeps_continuation_bytes_only).as_bytes()).is_ok()
        );
    }

    #[test]
    fn encode_then_decode_is_the_identity_on_bytes() {
        for input in [
            "",
            "a b",
            "€ and é",
            "100%",
            "a+b",
            "/x?y#z",
            "\0\u{7F}\u{80}",
        ] {
            for keep in [
                is_unreserved as fn(u8) -> bool,
                is_unreserved_or_slash,
                is_pchar_or_slash_or_question,
            ] {
                let encoded = encode(input, keep);
                assert!(encoded.is_ascii(), "{input:?}");
                assert_eq!(decode(&encoded).unwrap(), input.as_bytes(), "{input:?}");
            }
        }
    }

    #[test]
    fn decode_pins_bytes_and_accepts_either_hex_case() {
        assert_eq!(decode("").unwrap(), b"");
        assert_eq!(decode("abc").unwrap(), b"abc");
        assert_eq!(decode("%41%62%63").unwrap(), b"Abc");
        assert_eq!(decode("%e2%82%ac").unwrap(), "€".as_bytes());
        assert_eq!(decode("%E2%82%AC").unwrap(), "€".as_bytes());
        assert_eq!(decode("%00%ff%FF").unwrap(), [0x00, 0xFF, 0xFF]);
        // `+` is an ordinary byte outside the form rule.
        assert_eq!(decode("a+b").unwrap(), b"a+b");
        // Raw non-ASCII passes through as its UTF-8 bytes.
        assert_eq!(decode("é%20").unwrap(), "é ".as_bytes());
        // A decoded byte that is not UTF-8 is still a decoded byte.
        assert_eq!(decode("%FF").unwrap(), [0xFF]);
    }

    #[test]
    fn decode_refusals_each_have_an_accepted_neighbour() {
        for (refused, error, accepted) in [
            ("%", PercentError::Truncated { at: 0 }, "%25"),
            ("%4", PercentError::Truncated { at: 0 }, "%41"),
            ("ab%4", PercentError::Truncated { at: 2 }, "ab%41"),
            ("%4G", PercentError::NotHex { at: 0 }, "%4F"),
            ("%G4", PercentError::NotHex { at: 0 }, "%F4"),
            ("%+1", PercentError::NotHex { at: 0 }, "%21"),
            ("%-1", PercentError::NotHex { at: 0 }, "%21"),
            ("% 1", PercentError::NotHex { at: 0 }, "%01"),
            ("%%41", PercentError::NotHex { at: 0 }, "%25%41"),
            ("%4%1", PercentError::NotHex { at: 0 }, "%41"),
            ("a%2Gb", PercentError::NotHex { at: 1 }, "a%2Fb"),
            ("%é1", PercentError::NotHex { at: 0 }, "%E9"),
        ] {
            assert_eq!(decode(refused).unwrap_err(), error, "{refused:?}");
            assert!(decode(accepted).is_ok(), "{accepted:?}");
            assert_eq!(decode_form(refused).unwrap_err(), error, "{refused:?}");
            assert!(decode_form(accepted).is_ok(), "{accepted:?}");
        }
        assert_eq!(PercentError::NotHex { at: 7 }.at(), 7);
        assert_eq!(
            PercentError::Truncated { at: 3 }.to_string(),
            "percent-encoding ends inside a `%XX` triplet at byte 3"
        );
        assert_eq!(
            PercentError::NotHex { at: 0 }.to_string(),
            "`%` is not followed by two hexadecimal digits at byte 0"
        );
    }

    #[test]
    fn decode_form_turns_plus_into_space_and_nothing_else() {
        assert_eq!(decode_form("a+b").unwrap(), b"a b");
        assert_eq!(decode_form("+").unwrap(), b" ");
        assert_eq!(decode_form("a%2Bb").unwrap(), b"a+b");
        assert_eq!(decode_form("a%20b").unwrap(), b"a b");
        assert_eq!(
            decode_form("query=SELECT+%2A+WHERE+%7B%7D").unwrap(),
            b"query=SELECT * WHERE {}"
        );
        // The two decoders agree on everything but `+`.
        for input in ["", "abc", "%41", "a%20b", "é"] {
            assert_eq!(
                decode(input).unwrap(),
                decode_form(input).unwrap(),
                "{input:?}"
            );
        }
        assert_ne!(decode("+").unwrap(), decode_form("+").unwrap());
    }
}
