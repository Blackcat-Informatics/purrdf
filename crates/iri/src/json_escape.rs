// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one JSON string-body escape law (RFC 8259 §7).
//!
//! Every PurRDF writer that emits a JSON string escapes its body here. The law
//! lives in this zero-dependency leaf, beside the
//! [`find_first_json_string_special`] class it scans with, because it is the one
//! crate every JSON-emitting crate already reaches — including the GTS container
//! crate, which deliberately does not depend on `purrdf-core`.
//!
//! RFC 8259 admits more than one spelling of the same string, and the writers
//! that share this law already emit fixed bytes that their goldens and digests
//! pin. [`JsonEscapes`] names the four spellings in use; every one of them
//! writes `\"`, `\\`, `\n`, `\r` and `\t`, writes each other escaped scalar as
//! `\u` plus four lowercase hex digits (a UTF-16 surrogate pair above U+FFFF),
//! and never escapes `/`. They differ only in which further scalars are
//! escaped and how; every scalar a spelling does not escape — U+2028 and
//! U+2029 included — is written as itself.
//!
//! The body is written as a sequence of `&str` pieces to a caller-supplied
//! sink, so a writer whose output is not a `String` pays no intermediate
//! buffer. Clean runs are found by a chunked byte-class scan and handed over
//! whole; only a stop byte goes through the per-`char` table. Every stop byte
//! is ASCII or a UTF-8 lead byte, so each run ends on a `char` boundary, and
//! every rule maps one `char` independently, so escaping a string's fragments
//! one by one gives the same bytes as escaping the whole.
//!
//! The decoding direction lives here too: [`unescape`] resolves a string body
//! back to its text under the same §7 grammar every spelling above writes to,
//! and [`decode_u_escape`] is the four-hex-digit `\u` unit it is built on.
//! The same scanner finds the clean runs, so a body with no escape is a
//! borrow.

use crate::scan::{ByteClass, byte_run_count, find_first_json_string_special};
use core::fmt;
use std::borrow::Cow;

/// Which JSON string spelling a writer emits. All four are valid RFC 8259
/// string bodies and decode to the same text; the choice is fixed by the bytes
/// a writer has always produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonEscapes {
    /// Only what RFC 8259 requires: `\"`, `\\`, `\n`, `\r`, `\t`, and every
    /// other C0 control (U+0000–U+001F) as `\u00xx`. DEL and the C1 controls
    /// are written raw. The SPARQL results JSON writer's spelling.
    Minimal,
    /// [`Minimal`](Self::Minimal), with BACKSPACE and FORM FEED as the short
    /// escapes `\b` and `\f`: every short escape RFC 8259 names except `\/`.
    /// The spelling `serde_json` and Python's `json.dumps` use for controls.
    ShortForms,
    /// [`Minimal`](Self::Minimal), and every other Unicode `Cc` scalar — DEL
    /// (U+007F) and the C1 block (U+0080–U+009F) — as `\u00xx` as well, so the
    /// output carries no raw control scalar at all. The GTS proof and
    /// replication reports' spelling.
    Controls,
    /// [`ShortForms`](Self::ShortForms), and every scalar outside printable
    /// ASCII — DEL and all non-ASCII — as `\uxxxx`, astral scalars as a UTF-16
    /// surrogate pair, so the output is pure printable ASCII. Python's
    /// `json.dumps` with its default `ensure_ascii=True`, byte for byte.
    Ascii,
}

/// Lowercase hex digits, one `&str` per nibble.
const HEX_LOWER: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c", "d", "e", "f",
];

/// The stop bytes of [`JsonEscapes::Controls`]: the
/// [`find_first_json_string_special`] class plus DEL and `0xC2`, the lead byte
/// of every C1 control (U+0080–U+009F is `C2 80`–`C2 9F`). A `0xC2` stop that
/// leads a non-control scalar (U+00A0–U+00BF) is written as itself.
const CONTROLS_STOP_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'"' as usize] = 1;
    table[b'\\' as usize] = 1;
    table[0x7F] = 1;
    table[0xC2] = 1;
    table
};

const CONTROLS_STOPS: ByteClass<{ byte_run_count(&CONTROLS_STOP_TABLE) }> =
    ByteClass::from_table(CONTROLS_STOP_TABLE);

/// The stop bytes of [`JsonEscapes::Ascii`]: the
/// [`find_first_json_string_special`] class plus DEL and every byte of a
/// non-ASCII scalar (only a lead byte is ever reached, since a stop consumes
/// its whole scalar).
const ASCII_STOP_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'"' as usize] = 1;
    table[b'\\' as usize] = 1;
    let mut high = 0x7F;
    while high <= 0xFF {
        table[high] = 1;
        high += 1;
    }
    table
};

const ASCII_STOPS: ByteClass<{ byte_run_count(&ASCII_STOP_TABLE) }> =
    ByteClass::from_table(ASCII_STOP_TABLE);

/// The offset of the first [`JsonEscapes::Ascii`] stop byte, or `None`.
#[inline(never)]
fn find_first_json_ascii_stop(bytes: &[u8]) -> Option<usize> {
    ASCII_STOPS.find_first(bytes)
}

/// The offset of the first [`JsonEscapes::Controls`] stop byte, or `None`.
///
/// Out of line, so the class compiles to one kernel with its runs folded in as
/// constants whichever writer calls it.
#[inline(never)]
fn find_first_json_controls_stop(bytes: &[u8]) -> Option<usize> {
    CONTROLS_STOPS.find_first(bytes)
}

impl JsonEscapes {
    /// Whether BACKSPACE and FORM FEED take their short escapes.
    const fn short_forms(self) -> bool {
        matches!(self, Self::ShortForms | Self::Ascii)
    }
}

/// Emit `\u` and the four lowercase hex digits of one UTF-16 code unit.
#[inline]
fn emit_u_escape(unit: u16, emit: &mut impl FnMut(&str)) {
    emit("\\u");
    for shift in [12_u16, 8, 4, 0] {
        emit(HEX_LOWER[usize::from(unit >> shift & 0xF)]);
    }
}

/// Write the JSON string BODY of `value` — everything between the quotes — as
/// a sequence of pieces passed to `emit`, spelled per `escapes`.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::json_escape::{JsonEscapes, escape_body};
///
/// let mut out = String::new();
/// escape_body("a\"b\u{8}\u{7f}", JsonEscapes::Minimal, |piece| out.push_str(piece));
/// assert_eq!(out, "a\\\"b\\u0008\u{7f}");
/// ```
#[inline]
pub fn escape_body(value: &str, escapes: JsonEscapes, mut emit: impl FnMut(&str)) {
    let mut rest = value;
    while !rest.is_empty() {
        let run = match escapes {
            JsonEscapes::Controls => find_first_json_controls_stop(rest.as_bytes()),
            JsonEscapes::Ascii => find_first_json_ascii_stop(rest.as_bytes()),
            JsonEscapes::Minimal | JsonEscapes::ShortForms => {
                find_first_json_string_special(rest.as_bytes())
            }
        }
        .unwrap_or(rest.len());
        if run > 0 {
            emit(&rest[..run]);
            rest = &rest[run..];
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        let width = ch.len_utf8();
        match ch {
            '"' => emit("\\\""),
            '\\' => emit("\\\\"),
            '\n' => emit("\\n"),
            '\r' => emit("\\r"),
            '\t' => emit("\\t"),
            '\u{8}' if escapes.short_forms() => emit("\\b"),
            '\u{c}' if escapes.short_forms() => emit("\\f"),
            c if u32::from(c) < 0x20
                || (escapes == JsonEscapes::Controls && c.is_control())
                || (escapes == JsonEscapes::Ascii && u32::from(c) >= 0x7F) =>
            {
                let mut units = [0_u16; 2];
                for &unit in c.encode_utf16(&mut units).iter() {
                    emit_u_escape(unit, &mut emit);
                }
            }
            // A `0xC2` stop that is not a C1 control: the scalar is written raw.
            _ => emit(&rest[..width]),
        }
        rest = &rest[width..];
    }
}

/// Append the JSON string body of `value` to `out`, spelled per `escapes`.
///
/// ```rust
/// use purrdf_iri::json_escape::{JsonEscapes, push_body};
///
/// let mut out = String::from("x=");
/// push_body(&mut out, "tab\there\u{85}", JsonEscapes::Controls);
/// assert_eq!(out, "x=tab\\there\\u0085");
/// ```
#[inline]
pub fn push_body(out: &mut String, value: &str, escapes: JsonEscapes) {
    escape_body(value, escapes, |piece| out.push_str(piece));
}

/// Append `value` to `out` as a whole JSON string: the body spelled per
/// `escapes`, between double quotes.
///
/// ```rust
/// use purrdf_iri::json_escape::{JsonEscapes, push_string};
///
/// let mut out = String::new();
/// push_string(&mut out, "line\u{c}feed", JsonEscapes::ShortForms);
/// assert_eq!(out, "\"line\\ffeed\"");
/// ```
#[inline]
pub fn push_string(out: &mut String, value: &str, escapes: JsonEscapes) {
    out.push('"');
    push_body(out, value, escapes);
    out.push('"');
}

/// Why a JSON string body could not be decoded: the clause of RFC 8259 §7 it
/// breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonUnescapeErrorKind {
    /// A raw control character (U+0000–U+001F). §7: the control characters
    /// are among "the characters that MUST be escaped".
    RawControl,
    /// A raw `"`, which ends a string and so cannot stand inside its body.
    RawQuote,
    /// A `\` at the end of the body, with nothing after it to escape.
    TruncatedEscape,
    /// `\` followed by a byte that opens no escape: not one of
    /// `"`, `\`, `/`, `b`, `f`, `n`, `r`, `t` or `u`.
    UnknownEscape,
    /// `\u` not followed by exactly four hexadecimal digits.
    MalformedUnicodeEscape,
    /// A `\u` escape naming a UTF-16 surrogate that is not half of a complete
    /// pair: a low surrogate on its own, or a high surrogate not followed by
    /// `\uDC00`–`\uDFFF`. §8.2 leaves the behaviour of an unpaired surrogate
    /// unpredictable, and a replacement character would be a silent
    /// corruption, so it is refused.
    LoneSurrogate,
}

impl fmt::Display for JsonUnescapeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RawControl => "a raw control character, which RFC 8259 requires to be escaped",
            Self::RawQuote => "a raw `\"`, which ends a string rather than standing inside one",
            Self::TruncatedEscape => "a `\\` at the end of the text with nothing to escape",
            Self::UnknownEscape => {
                "an escape other than `\\\"`, `\\\\`, `\\/`, `\\b`, `\\f`, `\\n`, `\\r`, `\\t` or \
                 `\\u`"
            }
            Self::MalformedUnicodeEscape => "a `\\u` not followed by four hexadecimal digits",
            Self::LoneSurrogate => {
                "a `\\u` escape naming a UTF-16 surrogate that is not half of a complete pair"
            }
        })
    }
}

/// A JSON string body RFC 8259 §7 does not admit: where, and which clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonUnescapeError {
    /// The byte offset, in the body, of the raw byte refused or of the `\`
    /// that opens the refused escape.
    pub at: usize,
    /// Which clause was broken.
    pub kind: JsonUnescapeErrorKind,
}

impl fmt::Display for JsonUnescapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON string body at byte {}: {}", self.at, self.kind)
    }
}

impl std::error::Error for JsonUnescapeError {}

/// The value of one hexadecimal digit, in either case, or `None`.
const fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// The UTF-16 code unit named by `digits`, which must be exactly four
/// hexadecimal digits (`HEXDIG`, either case) — the `4HEXDIG` of RFC 8259 §7's
/// `\u` escape — or `None` for anything else: a sign, a fifth digit, three
/// digits, or a byte that is not a digit.
///
/// A unit, not a scalar: a surrogate (`D800`–`DFFF`) is a valid answer here,
/// and pairing it is [`unescape`]'s job.
///
/// ```
/// use purrdf_iri::json_escape::decode_u_escape;
///
/// assert_eq!(decode_u_escape(b"00e9"), Some(0xE9));
/// assert_eq!(decode_u_escape(b"00E9"), Some(0xE9));
/// assert_eq!(decode_u_escape(b"d83d"), Some(0xD83D));
/// assert_eq!(decode_u_escape(b"+041"), None);
/// assert_eq!(decode_u_escape(b"041"), None);
/// assert_eq!(decode_u_escape(b"00041"), None);
/// ```
#[must_use]
pub const fn decode_u_escape(digits: &[u8]) -> Option<u16> {
    let &[a, b, c, d] = digits else {
        return None;
    };
    let (Some(a), Some(b), Some(c), Some(d)) =
        (hex_nibble(a), hex_nibble(b), hex_nibble(c), hex_nibble(d))
    else {
        return None;
    };
    Some(u16::from_be_bytes([(a << 4) | b, (c << 4) | d]))
}

/// The first UTF-16 code unit of a surrogate pair's high half.
const HIGH_SURROGATE_LO: u16 = 0xD800;
/// The last high surrogate.
const HIGH_SURROGATE_HI: u16 = 0xDBFF;
/// The first low surrogate.
const LOW_SURROGATE_LO: u16 = 0xDC00;
/// The last low surrogate.
const LOW_SURROGATE_HI: u16 = 0xDFFF;

/// Decode the escape that opens at `bytes[backslash]` (which the caller has
/// established is `\`): the scalar it names and the offset just past it.
///
/// The one escape decoder the JSON reader and [`unescape`] share, so both
/// refuse exactly the same escapes. An error's `at` is `backslash` itself,
/// except for a malformed second half of a surrogate pair, where it is that
/// half's own `\`.
pub(crate) fn decode_escape(
    bytes: &[u8],
    backslash: usize,
) -> Result<(char, usize), JsonUnescapeError> {
    debug_assert_eq!(bytes.get(backslash), Some(&b'\\'));
    let refuse = |kind| JsonUnescapeError {
        at: backslash,
        kind,
    };
    let Some(&code) = bytes.get(backslash + 1) else {
        return Err(refuse(JsonUnescapeErrorKind::TruncatedEscape));
    };
    let short = match code {
        b'"' => '"',
        b'\\' => '\\',
        b'/' => '/',
        b'b' => '\u{8}',
        b'f' => '\u{c}',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'u' => return decode_unicode_escape(bytes, backslash),
        _ => return Err(refuse(JsonUnescapeErrorKind::UnknownEscape)),
    };
    Ok((short, backslash + 2))
}

/// The four hex digits of the `\u` escape at `bytes[backslash]`, as a unit,
/// or the malformed-escape refusal at `backslash`.
fn unicode_unit(bytes: &[u8], backslash: usize) -> Result<u16, JsonUnescapeError> {
    let digits_at = backslash + 2;
    bytes
        .get(digits_at..digits_at + 4)
        .and_then(decode_u_escape)
        .ok_or(JsonUnescapeError {
            at: backslash,
            kind: JsonUnescapeErrorKind::MalformedUnicodeEscape,
        })
}

/// [`decode_escape`] for a `\u` escape: one unit outside the surrogates is a
/// scalar; a high surrogate must be followed by `\u` and a low surrogate, and
/// the pair is one supplementary scalar; a low surrogate alone is refused.
fn decode_unicode_escape(
    bytes: &[u8],
    backslash: usize,
) -> Result<(char, usize), JsonUnescapeError> {
    let lone = JsonUnescapeError {
        at: backslash,
        kind: JsonUnescapeErrorKind::LoneSurrogate,
    };
    let unit = unicode_unit(bytes, backslash)?;
    let after = backslash + 6;
    match unit {
        HIGH_SURROGATE_LO..=HIGH_SURROGATE_HI => {
            if bytes.get(after..after + 2) != Some(b"\\u") {
                return Err(lone);
            }
            let low = unicode_unit(bytes, after)?;
            if !(LOW_SURROGATE_LO..=LOW_SURROGATE_HI).contains(&low) {
                return Err(lone);
            }
            let scalar = 0x1_0000
                + ((u32::from(unit) - u32::from(HIGH_SURROGATE_LO)) << 10)
                + (u32::from(low) - u32::from(LOW_SURROGATE_LO));
            let ch = char::from_u32(scalar).expect("a surrogate pair names a supplementary scalar");
            Ok((ch, after + 6))
        }
        LOW_SURROGATE_LO..=LOW_SURROGATE_HI => Err(lone),
        _ => {
            let ch = char::from_u32(u32::from(unit))
                .expect("a UTF-16 unit outside the surrogates is a scalar");
            Ok((ch, after))
        }
    }
}

/// Decode a JSON string BODY — the text between the quotes, quotes excluded —
/// under RFC 8259 §7, borrowing it when it holds no escape.
///
/// Resolved: the eight short escapes `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`,
/// `\t`, and `\u` with exactly four hexadecimal digits, a high surrogate taking
/// the `\uDC00`–`\uDFFF` that must follow it as one supplementary scalar.
/// Refused, as a [`JsonUnescapeError`] naming the byte and the clause: any
/// other escape, a `\` with nothing after it, `\u` with anything but four hex
/// digits (a sign included), a surrogate that is not half of a complete pair,
/// a raw control character below U+0020, and a raw `"`. DEL, the C1 controls
/// and every non-ASCII scalar are `unescaped` and pass through as themselves.
///
/// The inverse of every [`JsonEscapes`] spelling: for any `s`,
/// `unescape(&escaped(s)) == s`.
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_iri::json_escape::{JsonUnescapeErrorKind, unescape};
///
/// assert_eq!(unescape("caf\\u00e9 \\ud83d\\ude00")?, "caf\u{e9} \u{1f600}");
/// assert!(matches!(unescape("caf\u{e9}"), Ok(Cow::Borrowed(_))));
/// // A lone high surrogate names no scalar.
/// assert_eq!(
///     unescape("\\ud83d").map_err(|e| e.kind),
///     Err(JsonUnescapeErrorKind::LoneSurrogate)
/// );
/// # Ok::<(), purrdf_iri::json_escape::JsonUnescapeError>(())
/// ```
pub fn unescape(text: &str) -> Result<Cow<'_, str>, JsonUnescapeError> {
    let bytes = text.as_bytes();
    let Some(first) = find_first_json_string_special(bytes) else {
        return Ok(Cow::Borrowed(text));
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..first]);
    let mut at = first;
    loop {
        // `at` is a stop byte: `"`, `\` or a C0 control.
        match bytes[at] {
            b'\\' => {
                let (ch, next) = decode_escape(bytes, at)?;
                out.push(ch);
                at = next;
            }
            b'"' => {
                return Err(JsonUnescapeError {
                    at,
                    kind: JsonUnescapeErrorKind::RawQuote,
                });
            }
            _ => {
                return Err(JsonUnescapeError {
                    at,
                    kind: JsonUnescapeErrorKind::RawControl,
                });
            }
        }
        // Every escape is ASCII, so `at` is a char boundary; so is the next
        // stop, which is ASCII too.
        match find_first_json_string_special(&bytes[at..]) {
            Some(offset) => {
                out.push_str(&text[at..at + offset]);
                at += offset;
            }
            None => {
                out.push_str(&text[at..]);
                return Ok(Cow::Owned(out));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        JsonEscapes, JsonUnescapeError, JsonUnescapeErrorKind, decode_u_escape,
        find_first_json_ascii_stop, find_first_json_controls_stop, push_body, unescape,
    };
    use std::borrow::Cow;
    use std::fmt::Write as _;

    /// The per-`char` escapers the law replaced, one per spelling, kept verbatim
    /// as the oracle.
    fn reference(value: &str, escapes: JsonEscapes) -> String {
        let mut out = String::new();
        for ch in value.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{08}' if escapes != JsonEscapes::Minimal && escapes != JsonEscapes::Controls => {
                    out.push_str("\\b");
                }
                '\u{0c}' if escapes != JsonEscapes::Minimal && escapes != JsonEscapes::Controls => {
                    out.push_str("\\f");
                }
                c if escapes == JsonEscapes::Ascii && (c as u32) >= 0x7F => {
                    let mut units = [0_u16; 2];
                    for unit in c.encode_utf16(&mut units) {
                        let _ = write!(out, "\\u{unit:04x}");
                    }
                }
                c if escapes == JsonEscapes::Controls && c.is_control() => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c if (c as u32) < 0x20 => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    const ALL: [JsonEscapes; 4] = [
        JsonEscapes::Minimal,
        JsonEscapes::ShortForms,
        JsonEscapes::Controls,
        JsonEscapes::Ascii,
    ];

    #[test]
    fn every_spelling_matches_its_reference_on_fixed_cases() {
        let cases = [
            "",
            "plain ascii text 0123456789 ~!@#$%^&*()_+-=[]{};':,./<>?",
            "\"",
            "\\",
            "\n\r\t",
            "\u{0}\u{1}\u{8}\u{c}\u{1f}",
            "\u{7f}",
            "\u{80}\u{85}\u{9f}\u{a0}\u{bf}\u{c0}",
            "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
            "mixed \"quoted\" \\ back\\slash\n\ttab \u{e9}\u{1}end",
            "\u{2028}\u{2029}\u{feff}\u{e9}\"",
            "a/b",
            "sixteen byte run\u{1}sixteen byte run\u{7f}sixteen byte run\u{85}tail",
        ];
        for escapes in ALL {
            for case in cases {
                let mut fast = String::from("prefix");
                push_body(&mut fast, case, escapes);
                assert_eq!(
                    fast,
                    format!("prefix{}", reference(case, escapes)),
                    "{escapes:?} {case:?}"
                );
            }
        }
    }

    #[test]
    fn every_spelling_matches_its_reference_on_generated_values() {
        // Fixed-seed SplitMix64, so every run draws the same inputs. The
        // alphabet weights the stop bytes and their near neighbours.
        let alphabet: Vec<char> = "ab \"\\/\n\r\t\u{0}\u{8}\u{c}\u{1f}\u{20}\u{7e}\u{7f}\u{80}\u{9f}\u{a0}\u{bf}\u{e9}\u{2028}\u{1f431}"
            .chars()
            .collect();
        let mut state: u64 = 0x5EED_1234_ABCD_0042;
        let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
        for _ in 0..2_000 {
            let len = usize::try_from(next() % 70).expect("below 70");
            let value: String = (0..len)
                .map(|_| alphabet[usize::try_from(next() % alphabet.len() as u64).expect("fits")])
                .collect();
            for escapes in ALL {
                let mut fast = String::new();
                push_body(&mut fast, &value, escapes);
                assert_eq!(fast, reference(&value, escapes), "{escapes:?} {value:?}");
            }
        }
    }

    #[test]
    fn fragment_escaping_equals_whole_escaping() {
        let value = "x\"\u{1}\u{85}caf\u{e9}\u{7f}\\y";
        for escapes in ALL {
            let mut whole = String::new();
            push_body(&mut whole, value, escapes);
            for (at, _) in value.char_indices() {
                let mut pieces = String::new();
                push_body(&mut pieces, &value[..at], escapes);
                push_body(&mut pieces, &value[at..], escapes);
                assert_eq!(pieces, whole, "{escapes:?} split at {at}");
            }
        }
    }

    /// The `Ascii` spelling against bytes Python's
    /// `json.dumps(s, ensure_ascii=True)` writes for the same strings.
    #[test]
    fn ascii_spelling_is_python_json_dumps() {
        let cases = [
            ("a\"b\\c/d", "a\\\"b\\\\c/d"),
            (
                "\u{0}\u{8}\u{9}\u{a}\u{c}\u{d}\u{1f}",
                "\\u0000\\b\\t\\n\\f\\r\\u001f",
            ),
            (
                "\u{7f}\u{85}\u{a0}caf\u{e9}",
                "\\u007f\\u0085\\u00a0caf\\u00e9",
            ),
            ("\u{2028}\u{feff}\u{ffff}", "\\u2028\\ufeff\\uffff"),
            (
                "\u{1f431}\u{10000}\u{10ffff}",
                "\\ud83d\\udc31\\ud800\\udc00\\udbff\\udfff",
            ),
        ];
        for (value, python) in cases {
            let mut out = String::new();
            push_body(&mut out, value, JsonEscapes::Ascii);
            assert_eq!(out, python, "{value:?}");
        }
    }

    #[test]
    fn ascii_stop_class_is_exactly_its_table() {
        for b in 0..=u8::MAX {
            let member = b < 0x20 || b == b'"' || b == b'\\' || b >= 0x7F;
            assert_eq!(
                find_first_json_ascii_stop(&[b]).is_some(),
                member,
                "{b:#04X}"
            );
            let mut chunk = [b'a'; 32];
            chunk[31] = b;
            assert_eq!(
                find_first_json_ascii_stop(&chunk),
                member.then_some(31),
                "{b:#04X} in a chunk"
            );
        }
    }

    #[test]
    fn controls_stop_class_is_exactly_its_table() {
        for b in 0..=u8::MAX {
            let member = b < 0x20 || b == b'"' || b == b'\\' || b == 0x7F || b == 0xC2;
            assert_eq!(
                find_first_json_controls_stop(&[b]).is_some(),
                member,
                "{b:#04X}"
            );
            // The chunked path, not only the tail: the byte after 31 clean ones.
            let mut chunk = [b'a'; 32];
            chunk[31] = b;
            assert_eq!(
                find_first_json_controls_stop(&chunk),
                member.then_some(31),
                "{b:#04X} in a chunk"
            );
        }
    }

    // ---- decoding -----------------------------------------------------------

    fn kind(text: &str) -> Result<String, JsonUnescapeErrorKind> {
        unescape(text)
            .map(Cow::into_owned)
            .map_err(|error| error.kind)
    }

    #[test]
    fn unescape_resolves_every_escape_and_borrows_when_there_is_none() {
        assert_eq!(kind("caf\\u00e9"), Ok("caf\u{e9}".to_owned()));
        assert_eq!(kind("caf\\u00E9"), Ok("caf\u{e9}".to_owned()));
        assert_eq!(kind("\\ud83d\\ude00"), Ok("\u{1f600}".to_owned()));
        assert_eq!(kind("\\uD83D\\uDE00"), Ok("\u{1f600}".to_owned()));
        assert_eq!(
            kind("\\\" \\\\ \\/ \\b \\f \\n \\r \\t"),
            Ok("\" \\ / \u{8} \u{c} \n \r \t".to_owned())
        );
        assert_eq!(kind("\\u0000"), Ok("\u{0}".to_owned()));
        // Raw non-ASCII, DEL and C1 are `unescaped`: they pass through, and a
        // body with no escape at all is a borrow.
        let clean = "caf\u{e9} \u{7f}\u{85} \u{1f600} a/b";
        assert!(matches!(unescape(clean), Ok(Cow::Borrowed(s)) if s == clean));
        assert!(matches!(unescape(""), Ok(Cow::Borrowed(""))));
        // An escape anywhere makes it owned, and the runs around it are whole.
        assert!(matches!(
            unescape("sixteen byte run \\n sixteen byte run"),
            Ok(Cow::Owned(s)) if s == "sixteen byte run \n sixteen byte run"
        ));
    }

    #[test]
    fn unescape_refuses_what_rfc_8259_refuses_and_admits_the_neighbour() {
        use JsonUnescapeErrorKind as K;
        // (refused, kind, the valid neighbour, its text)
        let cases: [(&str, K, &str, &str); 8] = [
            ("\\ud83d", K::LoneSurrogate, "\\ud83d\\ude00", "\u{1f600}"),
            (
                "\\ud83dx",
                K::LoneSurrogate,
                "\\ud83d\\ude00x",
                "\u{1f600}x",
            ),
            (
                "\\ud83d\\u0041",
                K::LoneSurrogate,
                "\\ud83d\\ude00",
                "\u{1f600}",
            ),
            ("\\ude00", K::LoneSurrogate, "\\u00e9", "\u{e9}"),
            ("\\u+041", K::MalformedUnicodeEscape, "\\u0041", "A"),
            ("\\u041", K::MalformedUnicodeEscape, "\\u0041", "A"),
            ("\\q", K::UnknownEscape, "\\n", "\n"),
            ("a\\", K::TruncatedEscape, "a\\\\", "a\\"),
        ];
        for (bad, expected, good, text) in cases {
            assert_eq!(kind(bad), Err(expected), "{bad:?}");
            assert_eq!(kind(good), Ok(text.to_owned()), "{good:?}");
        }
        // Raw bytes the grammar forbids, and the neighbours it admits.
        assert_eq!(kind("a\nb"), Err(K::RawControl));
        assert_eq!(kind("a\u{1f}b"), Err(K::RawControl));
        assert_eq!(kind("a\u{20}b"), Ok("a b".to_owned()));
        assert_eq!(kind("a\u{7f}b"), Ok("a\u{7f}b".to_owned()));
        assert_eq!(kind("a\"b"), Err(K::RawQuote));
        assert_eq!(kind("a\\\"b"), Ok("a\"b".to_owned()));
        // The offset names the byte refused, or the `\` that opens the escape.
        assert_eq!(
            unescape("ab\u{1}"),
            Err(JsonUnescapeError {
                at: 2,
                kind: K::RawControl
            })
        );
        assert_eq!(
            unescape("ab\\ud83d\\u12"),
            Err(JsonUnescapeError {
                at: 8,
                kind: K::MalformedUnicodeEscape
            })
        );
        assert_eq!(
            unescape("ab\\ud83d\\u0041").map_err(|e| e.at),
            Err(2),
            "a lone high surrogate is named at its own escape"
        );
        assert_eq!(
            unescape("\\ude00").unwrap_err().to_string(),
            "JSON string body at byte 0: a `\\u` escape naming a UTF-16 surrogate that is not \
             half of a complete pair"
        );
    }

    #[test]
    fn decode_u_escape_is_exactly_four_hex_digits() {
        for unit in [0_u16, 0x41, 0xE9, 0xD83D, 0xDE00, 0xFFFF] {
            let lower = format!("{unit:04x}");
            let upper = format!("{unit:04X}");
            assert_eq!(decode_u_escape(lower.as_bytes()), Some(unit));
            assert_eq!(decode_u_escape(upper.as_bytes()), Some(unit));
        }
        for bad in [
            &b""[..],
            b"0",
            b"041",
            b"00041",
            b"+041",
            b"-041",
            b"00g1",
            b"00 1",
            b"0x41",
        ] {
            assert_eq!(decode_u_escape(bad), None, "{bad:?}");
        }
    }

    /// Every spelling is undone by the one decoder, over generated values that
    /// weight the stop bytes and the scalars the spellings disagree on.
    #[test]
    fn unescape_inverts_every_spelling_on_generated_values() {
        let alphabet: Vec<char> = "ab \"\\/\n\r\t\u{0}\u{8}\u{c}\u{1f}\u{20}\u{7e}\u{7f}\u{80}\u{9f}\u{a0}\u{bf}\u{e9}\u{2028}\u{ffff}\u{10000}\u{1f431}\u{10ffff}"
            .chars()
            .collect();
        let mut state: u64 = 0x0DEC_0DE5_EED0_0001;
        let mut next = || purrdf_testkit::rng::splitmix64_next(&mut state);
        let mut owned = 0_usize;
        for _ in 0..2_000 {
            let len = usize::try_from(next() % 70).expect("below 70");
            let value: String = (0..len)
                .map(|_| alphabet[usize::try_from(next() % alphabet.len() as u64).expect("fits")])
                .collect();
            for escapes in ALL {
                let mut body = String::new();
                push_body(&mut body, &value, escapes);
                let back = unescape(&body).unwrap_or_else(|error| {
                    panic!("{escapes:?} {value:?} escaped to {body:?}, refused: {error}")
                });
                assert_eq!(back, value, "{escapes:?} {body:?}");
                owned += usize::from(matches!(back, Cow::Owned(_)));
            }
        }
        assert!(owned > 0, "non-vacuity: some value needed an escape");
    }
}
