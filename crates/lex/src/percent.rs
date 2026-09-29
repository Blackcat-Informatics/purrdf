// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Percent-encoding (RFC 3986 §2.1): the encoder over the sets the
//! specifications define, the strict decoder, the
//! `application/x-www-form-urlencoded` decoder, and RFC 3986 §6.2.2
//! normalization.
//!
//! An encoder writes every byte of the text's UTF-8 that its set does not
//! keep as `%` and two uppercase hex digits (§2.1: "should use uppercase
//! hexadecimal digits"). A set is an [`EncodeSet`]: the scan for the first
//! byte it encodes, one chunked [`ByteClass`] kernel per set, so a clean run
//! is copied whole and the scan runs sixteen bytes at a time. The sets the
//! specifications name are defined here; a caller whose law keeps some other
//! set declares its own class and scan the same way.
//!
//! | Set | Keeps | Specification |
//! |---|---|---|
//! | [`UNRESERVED`] | `ALPHA DIGIT - . _ ~` | RFC 3986 §2.3 `unreserved`; SPARQL `ENCODE_FOR_URI`; RFC 6570 simple expansion |
//! | [`UNRESERVED_SLASH`] | `unreserved` and `/` | a path of `unreserved` segments |
//! | [`REG_NAME`] | `unreserved` and `sub-delims` | RFC 3986 §3.2.2 `reg-name` |
//! | [`PATH`] | `pchar` and `/` | RFC 3986 §3.3 `path-abempty` |
//! | [`FRAGMENT`] | `pchar`, `/` and `?` | RFC 3986 §3.5 `fragment` |
//! | [`URI_TEMPLATE_RESERVED`] | `unreserved`, `reserved` and `%` | RFC 6570 §3.2.3 reserved expansion |
//! | [`NON_ASCII`] | every ASCII byte | RFC 3987 §3.1 IRI-to-URI mapping |
//!
//! ```rust
//! use purrdf_lex::percent::{self, FRAGMENT, UNRESERVED};
//!
//! assert_eq!(percent::encode("a b/c", UNRESERVED), "a%20b%2Fc");
//! assert_eq!(percent::encode("/a b/c?d", FRAGMENT), "/a%20b/c?d");
//! assert_eq!(percent::decode("caf%C3%A9").unwrap(), "café");
//! assert_eq!(percent::decode_form("a+b%21").unwrap(), "a b!");
//! ```

use std::borrow::Cow;

use crate::scan::{ByteClass, byte_run_count};

/// A percent-encoding set: the offset of the first byte of its input the set
/// encodes, or `None`.
pub type EncodeSet = fn(&[u8]) -> Option<usize>;

/// Whether `byte` is RFC 3986 §2.3 `unreserved`: `ALPHA / DIGIT / "-" / "." /
/// "_" / "~"`.
const fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// Whether `byte` is RFC 3986 §2.2 `sub-delims`: `! $ & ' ( ) * + , ; =`.
const fn is_sub_delim(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// Whether `byte` is RFC 3986 §2.2 `gen-delims`: `: / ? # [ ] @`.
const fn is_gen_delim(byte: u8) -> bool {
    matches!(byte, b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@')
}

/// The class table of the bytes a set encodes: every byte `keep` does not
/// hold. `keep` is written as a `match` over the set's own grammar, one per
/// set, so a table is derived from its specification at compile time.
macro_rules! encode_table {
    (|$byte:ident| $keep:expr) => {{
        let mut table = [0_u8; 256];
        let mut index = 0;
        while index < 256 {
            #[allow(clippy::cast_possible_truncation, reason = "index < 256")]
            let $byte = index as u8;
            if !$keep {
                table[index] = 1;
            }
            index += 1;
        }
        table
    }};
}

/// One set: its class table, its [`ByteClass`], and the out-of-line scan that
/// is the set's kernel.
macro_rules! encode_set {
    ($(#[$doc:meta])* $name:ident, $scan:ident, $table:ident, |$byte:ident| $keep:expr) => {
        const $table: [u8; 256] = encode_table!(|$byte| $keep);

        /// The first byte this set encodes: one chunked [`ByteClass`] kernel,
        /// out of line so the class's runs fold in as constants.
        #[inline(never)]
        fn $scan(bytes: &[u8]) -> Option<usize> {
            const CLASS: ByteClass<{ byte_run_count(&$table) }> = ByteClass::from_table($table);
            CLASS.find_first(bytes)
        }

        $(#[$doc])*
        pub const $name: EncodeSet = $scan;
    };
}

encode_set!(
    /// Keeps RFC 3986 §2.3 `unreserved` (`ALPHA DIGIT - . _ ~`) and encodes
    /// every other byte: the SPARQL `ENCODE_FOR_URI` set, and RFC 6570's
    /// simple string expansion (`{var}`).
    UNRESERVED,
    unreserved_escape,
    UNRESERVED_TABLE,
    |byte| is_unreserved(byte)
);

encode_set!(
    /// Keeps `unreserved` and `/`: a path whose segments are `unreserved`
    /// text, its separators kept.
    UNRESERVED_SLASH,
    unreserved_slash_escape,
    UNRESERVED_SLASH_TABLE,
    |byte| is_unreserved(byte) || byte == b'/'
);

encode_set!(
    /// Keeps RFC 3986 §3.2.2 `reg-name` without `pct-encoded`: `unreserved`
    /// and `sub-delims`.
    REG_NAME,
    reg_name_escape,
    REG_NAME_TABLE,
    |byte| is_unreserved(byte) || is_sub_delim(byte)
);

encode_set!(
    /// Keeps RFC 3986 §3.3 `pchar` and `/` (`unreserved`, `sub-delims`, `:`,
    /// `@`, `/`): a path.
    PATH,
    path_escape,
    PATH_TABLE,
    |byte| is_unreserved(byte) || is_sub_delim(byte) || matches!(byte, b':' | b'@' | b'/')
);

encode_set!(
    /// Keeps RFC 3986 §3.5 `fragment` (`pchar`, `/`, `?`).
    FRAGMENT,
    fragment_escape,
    FRAGMENT_TABLE,
    |byte| is_unreserved(byte) || is_sub_delim(byte) || matches!(byte, b':' | b'@' | b'/' | b'?')
);

encode_set!(
    /// Keeps RFC 6570 §3.2.3's reserved-expansion set (`{+var}`): `unreserved`,
    /// `reserved` (`gen-delims` and `sub-delims`) and `%`, so a percent-encoded
    /// triplet already in the value is not encoded again.
    URI_TEMPLATE_RESERVED,
    uri_template_reserved_escape,
    URI_TEMPLATE_RESERVED_TABLE,
    |byte| is_unreserved(byte) || is_sub_delim(byte) || is_gen_delim(byte) || byte == b'%'
);

encode_set!(
    /// Keeps every ASCII byte and encodes every byte of a non-ASCII scalar:
    /// RFC 3987 §3.1's mapping of an IRI to a URI.
    NON_ASCII,
    non_ascii_escape,
    NON_ASCII_TABLE,
    |byte| byte.is_ascii()
);

/// Append `%` and `byte`'s two uppercase hex digits to `out`.
///
/// ```rust
/// use purrdf_lex::percent::push_triplet;
///
/// let mut out = String::new();
/// push_triplet(&mut out, b'/');
/// assert_eq!(out, "%2F");
/// ```
pub fn push_triplet(out: &mut String, byte: u8) {
    out.push('%');
    purrdf_hash::hex::encode_upper_into(&[byte], out);
}

/// `text` with every byte `set` encodes written as a [triplet](push_triplet);
/// borrowed when there is none.
///
/// ```rust
/// use purrdf_lex::percent::{self, NON_ASCII, REG_NAME};
///
/// assert_eq!(percent::encode("é", NON_ASCII), "%C3%A9");
/// assert_eq!(percent::encode("a:b", REG_NAME), "a%3Ab");
/// assert!(matches!(percent::encode("abc", REG_NAME), std::borrow::Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn encode(text: &str, set: EncodeSet) -> Cow<'_, str> {
    let Some(first) = set(text.as_bytes()) else {
        return Cow::Borrowed(text);
    };
    let mut out = String::with_capacity(text.len() + 8);
    push_encoded_from(&mut out, text, set, first);
    Cow::Owned(out)
}

/// Append `text` to `out` with every byte `set` encodes written as a
/// [triplet](push_triplet).
pub fn push_encoded(out: &mut String, text: &str, set: EncodeSet) {
    match set(text.as_bytes()) {
        Some(first) => push_encoded_from(out, text, set, first),
        None => out.push_str(text),
    }
}

/// Append `text` to `out`, encoding from the stop at `first`.
///
/// A run between two stops is copied whole. Every set here encodes every
/// non-ASCII byte, or (for [`NON_ASCII`]) exactly those, so a run always
/// begins and ends on a `char` boundary: the position after a stop may fall
/// inside a multi-byte scalar only when the next byte is a stop too, and the
/// empty run there is never sliced.
fn push_encoded_from(out: &mut String, text: &str, set: EncodeSet, first: usize) {
    let bytes = text.as_bytes();
    let mut run_start = 0;
    let mut hit = first;
    loop {
        if hit > run_start {
            out.push_str(&text[run_start..hit]);
        }
        push_triplet(out, bytes[hit]);
        run_start = hit + 1;
        match set(&bytes[run_start..]) {
            Some(offset) => hit = run_start + offset,
            None => break,
        }
    }
    out.push_str(&text[run_start..]);
}

/// Why percent-decoding refused its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PercentError {
    /// A `%` at this byte offset is not followed by two hex digits.
    Malformed {
        /// The offset of the `%`.
        offset: usize,
    },
    /// The decoded bytes are not UTF-8; those before this offset are.
    NotUtf8 {
        /// The length of the decoded bytes' longest UTF-8 prefix.
        valid_up_to: usize,
    },
}

impl core::fmt::Display for PercentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Malformed { offset } => {
                write!(
                    f,
                    "`%` at byte {offset} is not followed by two hexadecimal digits"
                )
            }
            Self::NotUtf8 { valid_up_to } => write!(
                f,
                "the decoded bytes are not UTF-8 after their first {valid_up_to} bytes"
            ),
        }
    }
}

impl std::error::Error for PercentError {}

/// Decode `text`, with `+` as a space when `plus_is_space`.
fn decode_with(text: &str, plus_is_space: bool) -> Result<Cow<'_, str>, PercentError> {
    let bytes = text.as_bytes();
    if !bytes
        .iter()
        .any(|&b| b == b'%' || (plus_is_space && b == b'+'))
    {
        return Ok(Cow::Borrowed(text));
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let digit = |k: usize| {
                    bytes
                        .get(at + k)
                        .copied()
                        .and_then(purrdf_hash::hex::nibble)
                };
                let (Some(high), Some(low)) = (digit(1), digit(2)) else {
                    return Err(PercentError::Malformed { offset: at });
                };
                out.push((high << 4) | low);
                at += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out)
        .map(Cow::Owned)
        .map_err(|error| PercentError::NotUtf8 {
            valid_up_to: error.utf8_error().valid_up_to(),
        })
}

/// `text` with every `%XX` triplet (either case) decoded to its byte, the
/// result read as UTF-8; borrowed when there is no `%`.
///
/// Strict: a `%` not followed by two hex digits is refused rather than kept,
/// and so is a result that is not UTF-8. `+` is itself.
///
/// # Errors
///
/// [`PercentError`] names the malformed triplet or the invalid UTF-8.
///
/// ```rust
/// use purrdf_lex::percent::{PercentError, decode};
///
/// assert_eq!(decode("a%2Fb+c").unwrap(), "a/b+c");
/// assert_eq!(decode("%4"), Err(PercentError::Malformed { offset: 0 }));
/// assert_eq!(decode("%+1"), Err(PercentError::Malformed { offset: 0 }));
/// assert_eq!(decode("a%FF"), Err(PercentError::NotUtf8 { valid_up_to: 1 }));
/// ```
pub fn decode(text: &str) -> Result<Cow<'_, str>, PercentError> {
    decode_with(text, false)
}

/// [`decode`] for an `application/x-www-form-urlencoded` name or value
/// (WHATWG URL §5.1): `+` is a space.
///
/// # Errors
///
/// [`PercentError`] names the malformed triplet or the invalid UTF-8.
///
/// ```rust
/// use purrdf_lex::percent::decode_form;
///
/// assert_eq!(decode_form("SELECT+%3Fs").unwrap(), "SELECT ?s");
/// assert_eq!(decode_form("a%2Bb").unwrap(), "a+b");
/// ```
pub fn decode_form(text: &str) -> Result<Cow<'_, str>, PercentError> {
    decode_with(text, true)
}

/// RFC 3986 §6.2.2 percent-encoding normalization: a triplet whose octet is
/// `unreserved` is decoded (§6.2.2.2), every other triplet has its hex digits
/// uppercased (§6.2.2.1), and everything else — a `%` that opens no triplet
/// included — is copied; borrowed when there is no `%`.
///
/// ```rust
/// use purrdf_lex::percent::normalize;
///
/// assert_eq!(normalize("%7euser/%2f%41"), "~user/%2FA");
/// assert_eq!(normalize("100%"), "100%");
/// ```
#[must_use]
pub fn normalize(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let Some(first) = bytes.iter().position(|&b| b == b'%') else {
        return Cow::Borrowed(text);
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..first]);
    let mut copied = first;
    let mut at = first;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && let (Some(&high), Some(&low)) = (bytes.get(at + 1), bytes.get(at + 2))
            && let (Some(h), Some(l)) = (
                purrdf_hash::hex::nibble(high),
                purrdf_hash::hex::nibble(low),
            )
        {
            out.push_str(&text[copied..at]);
            let octet = (h << 4) | l;
            if is_unreserved(octet) {
                out.push(char::from(octet));
            } else {
                push_triplet(&mut out, octet);
            }
            at += 3;
            copied = at;
        } else {
            at += 1;
        }
    }
    // Every stop is an ASCII `%` or the end of an ASCII triplet, so each slice
    // starts and ends on a `char` boundary.
    out.push_str(&text[copied..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{
        EncodeSet, FRAGMENT, NON_ASCII, PATH, PercentError, REG_NAME, UNRESERVED, UNRESERVED_SLASH,
        URI_TEMPLATE_RESERVED, decode, decode_form, encode, is_gen_delim, is_sub_delim,
        is_unreserved, normalize, push_encoded,
    };

    /// A set's name, its scan, and the per-byte keep predicate its specification states.
    type SetCase = (&'static str, EncodeSet, fn(u8) -> bool);

    /// Every set with the per-byte keep predicate its specification states.
    fn sets() -> [SetCase; 7] {
        [
            ("unreserved", UNRESERVED, is_unreserved),
            ("unreserved-slash", UNRESERVED_SLASH, |b| {
                is_unreserved(b) || b == b'/'
            }),
            ("reg-name", REG_NAME, |b| {
                is_unreserved(b) || is_sub_delim(b)
            }),
            ("path", PATH, |b| {
                is_unreserved(b) || is_sub_delim(b) || matches!(b, b':' | b'@' | b'/')
            }),
            ("fragment", FRAGMENT, |b| {
                is_unreserved(b) || is_sub_delim(b) || matches!(b, b':' | b'@' | b'/' | b'?')
            }),
            ("uri-template-reserved", URI_TEMPLATE_RESERVED, |b| {
                is_unreserved(b) || is_sub_delim(b) || is_gen_delim(b) || b == b'%'
            }),
            ("non-ascii", NON_ASCII, |b| b.is_ascii()),
        ]
    }

    /// The per-byte encoder each set's scan must equal.
    fn reference(value: &str, keep: fn(u8) -> bool) -> String {
        let mut out = String::with_capacity(value.len());
        for byte in value.bytes() {
            if keep(byte) {
                out.push(char::from(byte));
            } else {
                out.push('%');
                purrdf_hash::hex::encode_upper_into(&[byte], &mut out);
            }
        }
        out
    }

    /// Every ASCII scalar and non-ASCII scalars of each UTF-8 width, each
    /// placed after every prefix length 0 to 40 of a plain run (so across the
    /// sixteen-byte chunks and into the tail), alone, doubled, and followed by
    /// plain text, other stops and a second scalar.
    fn samples() -> Vec<String> {
        let mut scalars: Vec<char> = (0_u8..=0x7F).map(char::from).collect();
        scalars.extend([
            '\u{85}',
            '\u{a0}',
            '\u{e9}',
            '\u{2028}',
            '\u{feff}',
            '\u{1f600}',
        ]);
        let mut out = vec![String::new()];
        for &c in &scalars {
            for prefix in 0..=40 {
                let plain = "a/b".repeat(prefix / 3 + 1);
                let head = &plain[..prefix.min(plain.len())];
                out.push(format!("{head}{c}"));
                out.push(format!("{head}{c}{c}"));
                out.push(format!("{head}{c}tail-0123456789-ABCDEFGHIJ"));
                out.push(format!("{head}{c} %{{}}\u{e9}{head}\u{1f600}"));
            }
        }
        out
    }

    #[test]
    fn every_set_scan_equals_its_per_byte_encoder() {
        let samples = samples();
        for (name, set, keep) in sets() {
            for byte in 0_u8..=u8::MAX {
                assert_eq!(set(&[byte]).is_some(), !keep(byte), "{name}: {byte:#04x}");
            }
            for value in &samples {
                let expected = reference(value, keep);
                let actual = encode(value, set);
                assert_eq!(actual, expected, "{name}: {value:?}");
                assert_eq!(
                    matches!(actual, Cow::Borrowed(_)),
                    expected == *value,
                    "{name}: {value:?}"
                );
                let mut pushed = String::from("prefix:");
                push_encoded(&mut pushed, value, set);
                assert_eq!(pushed, format!("prefix:{expected}"), "{name}: {value:?}");
            }
        }
    }

    #[test]
    fn a_reserved_expansion_borrows_an_unchanged_iri_and_encodes_only_on_demand() {
        let unchanged = "https://example.org/a/b?x=y#fragment";
        assert!(matches!(
            encode(unchanged, URI_TEMPLATE_RESERVED),
            Cow::Borrowed(value) if value == unchanged
        ));
        assert_eq!(
            encode("https://example.org/na\u{ef}ve path", URI_TEMPLATE_RESERVED),
            "https://example.org/na%C3%AFve%20path"
        );
    }

    #[test]
    fn a_malformed_or_signed_triplet_is_refused() {
        for text in ["%", "%4", "%4g", "%+1", "%-1", "% 41", "a%", "%%41"] {
            assert!(
                matches!(decode(text), Err(PercentError::Malformed { .. })),
                "{text:?}"
            );
            assert!(
                matches!(decode_form(text), Err(PercentError::Malformed { .. })),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_well_formed_triplet_of_either_case_still_decodes() {
        for text in ["%41", "%4a", "%4A", "a%2fb", "%C3%A9", "%c3%a9"] {
            assert!(decode(text).is_ok(), "{text:?}");
            assert!(decode_form(text).is_ok(), "{text:?}");
        }
        assert_eq!(decode("%C3%A9").unwrap(), "\u{e9}");
    }

    #[test]
    fn decoded_bytes_that_are_not_utf8_are_refused_and_utf8_neighbours_decode() {
        assert_eq!(
            decode("ab%FF"),
            Err(PercentError::NotUtf8 { valid_up_to: 2 })
        );
        assert_eq!(decode("%C3"), Err(PercentError::NotUtf8 { valid_up_to: 0 }));
        assert_eq!(decode("%C3%A9").unwrap(), "\u{e9}");
    }

    #[test]
    fn only_the_form_decoder_reads_plus_as_a_space() {
        assert_eq!(decode("a+b").unwrap(), "a+b");
        assert_eq!(decode_form("a+b").unwrap(), "a b");
        assert_eq!(decode_form("a%2Bb").unwrap(), "a+b");
        assert!(matches!(decode("plain").unwrap(), Cow::Borrowed("plain")));
        assert!(matches!(
            decode_form("plain").unwrap(),
            Cow::Borrowed("plain")
        ));
    }

    #[test]
    fn normalization_decodes_unreserved_octets_and_uppercases_the_rest() {
        assert_eq!(normalize("%7e%2f%41%4a%zz%4"), "~%2FAJ%zz%4");
        assert_eq!(normalize("caf\u{e9}%c3%a9"), "caf\u{e9}%C3%A9");
        assert!(matches!(normalize("plain"), Cow::Borrowed("plain")));
    }
}
