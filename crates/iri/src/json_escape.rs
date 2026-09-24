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

use crate::scan::{ByteClass, byte_run_count, find_first_json_string_special};

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

#[cfg(test)]
mod tests {
    use super::{
        JsonEscapes, find_first_json_ascii_stop, find_first_json_controls_stop, push_body,
    };
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
        let mut next = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
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
}
