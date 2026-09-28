// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE parameterised escaper for the body of a `"…"` N-Triples/Turtle
//! string literal.
//!
//! The production every writer here answers to is the short string of
//! N-Triples 1.2 / Turtle 1.2 `[22t]`:
//!
//! ```text
//! STRING_LITERAL_QUOTE ::= '"' ( [^#x22#x5C#xA#xD] | ECHAR | UCHAR )* '"'
//! ECHAR                ::= '\' [tbnrf"'\]
//! UCHAR                ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX
//! ```
//!
//! It forbids exactly four scalars raw — `"`, `\`, LINE FEED and CARRIAGE
//! RETURN — and admits every other scalar either raw or escaped, because
//! `ECHAR` and `UCHAR` are alternatives at every position. Over-escaping is
//! therefore lossless (any conforming parser decodes `\u0008` back to
//! U+0008), which is how this workspace came to carry SEVEN spellings of the
//! same law, differing only in which of the *permitted* scalars they escape
//! anyway and how. Each of those spellings is pinned by goldens, frozen
//! vectors or a conformance suite, so none can change bytes; what can change
//! is the number of implementations, and this module makes it one.
//! [`LiteralEscapes`] names the five profiles, and every function here takes
//! one.
//!
//! Every profile writes `\` as `\\`, `"` as `\"`, LINE FEED as `\n`, CARRIAGE
//! RETURN as `\r` and TAB as `\t`; every `UCHAR` a profile writes is `\u00XX`
//! in upper-case hex, because no profile escapes a scalar above U+009F. The
//! profiles differ only in the three questions the table below answers, and
//! every scalar not escaped by a profile — DEL, the C1 block and everything
//! non-ASCII where the profile leaves them, and every scalar at or above
//! U+00A0 always — rides verbatim as UTF-8.
//!
//! | Profile | `\b` `\f` short | other C0 | DEL | C1 (U+0080–U+009F) |
//! |---|---|---|---|---|
//! | [`Canonical`](LiteralEscapes::Canonical) | yes | `\u00XX` | `\u007F` | raw |
//! | [`XmlCarrier`](LiteralEscapes::XmlCarrier) | no (`\u0008` `\u000C`) | `\u00XX` | `\u007F` | `\u00XX` |
//! | [`C0Only`](LiteralEscapes::C0Only) | no (`\u0008` `\u000C`) | `\u00XX` | raw | raw |
//! | [`ShortOnly`](LiteralEscapes::ShortOnly) | yes | raw | raw | raw |
//! | [`CanonicalPlusC1`](LiteralEscapes::CanonicalPlusC1) | yes | `\u00XX` | `\u007F` | `\u00XX` |
//!
//! The body is written by one pass of a chunked byte-class scan
//! ([`find_first_escape`]) whose class is derived from the profile's law at
//! compile time, so the scan and the law cannot drift. Every run between two
//! escapes is copied whole with one `write_str`, and a value with nothing to
//! escape is one `write_str` — or, through [`escaped`], a borrow.

use crate::scan::{ByteClass, byte_run_count};
use core::fmt;
use std::borrow::Cow;

/// Which spelling of the literal body a writer emits. All five are valid
/// `STRING_LITERAL_QUOTE` bodies that decode to the same text; the choice is
/// fixed by the bytes a writer has always produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralEscapes {
    /// The RDFC-1.0 canonical N-Quads spelling: the canonical N-Triples
    /// `ECHAR` set (`\\`, `\"`, `\n`, `\r`, `\t`, `\b`, `\f`), every other C0
    /// control and DEL as `\u00XX`, and the C1 block raw. The W3C RDFC-1.0
    /// suite pins the last point (its test060 carries C1 raw in a literal).
    Canonical,
    /// The spelling of the native N-Triples/Turtle/TriG serializer, whose
    /// output is embedded verbatim inside an XML text node by the CL-dialect
    /// carrier: an XML processor normalizes or replaces raw control code
    /// points on read, so EVERY control rides as `\u00XX` — C0 (BACKSPACE and
    /// FORM FEED included: no `\b`, no `\f`), DEL and the C1 block.
    XmlCarrier,
    /// The spelling of the SHACL report and GTS fold-view writers: `\\`,
    /// `\"`, `\n`, `\r`, `\t`, every other C0 control as `\u00XX` (BACKSPACE
    /// and FORM FEED included), and DEL and the C1 block raw.
    C0Only,
    /// The spelling of the SPARQL algebra and ShEx shape-map renderers: the
    /// seven short `ECHAR`s and nothing else. Every other control rides raw,
    /// because the text is re-read by a scanner that admits them raw and never
    /// crosses a carrier that would not.
    ShortOnly,
    /// The spelling of the CDT (composite datatype) renderer:
    /// [`Canonical`](Self::Canonical) with the C1 block escaped as `\u00XX`
    /// too, so no control scalar at all rides raw.
    CanonicalPlusC1,
}

impl LiteralEscapes {
    /// Whether BACKSPACE and FORM FEED take their short escapes `\b` and `\f`
    /// rather than `\u0008` and `\u000C`.
    const fn short_forms(self) -> bool {
        matches!(
            self,
            Self::Canonical | Self::ShortOnly | Self::CanonicalPlusC1
        )
    }

    /// Whether the C0 controls with no short escape ride as `\u00XX`.
    const fn escapes_other_c0(self) -> bool {
        !matches!(self, Self::ShortOnly)
    }

    /// Whether DEL (U+007F) rides as `\u007F`.
    const fn escapes_del(self) -> bool {
        matches!(
            self,
            Self::Canonical | Self::XmlCarrier | Self::CanonicalPlusC1
        )
    }

    /// Whether the C1 block (U+0080–U+009F) rides as `\u00XX`.
    const fn escapes_c1(self) -> bool {
        matches!(self, Self::XmlCarrier | Self::CanonicalPlusC1)
    }
}

/// How one scalar rides in the body under a profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spelling {
    /// As itself.
    Raw,
    /// As this `ECHAR`.
    Short(&'static str),
    /// As `\u00XX`.
    Uchar,
}

/// The law: how scalar `cp` rides under `escapes`. Total over every scalar,
/// and the one place the table in the module docs is spelled.
const fn spelling(cp: u32, escapes: LiteralEscapes) -> Spelling {
    match cp {
        0x5C => Spelling::Short("\\\\"),
        0x22 => Spelling::Short("\\\""),
        0x0A => Spelling::Short("\\n"),
        0x0D => Spelling::Short("\\r"),
        0x09 => Spelling::Short("\\t"),
        0x08 if escapes.short_forms() => Spelling::Short("\\b"),
        0x0C if escapes.short_forms() => Spelling::Short("\\f"),
        0x00..=0x1F => {
            if escapes.escapes_other_c0() {
                Spelling::Uchar
            } else {
                Spelling::Raw
            }
        }
        0x7F => {
            if escapes.escapes_del() {
                Spelling::Uchar
            } else {
                Spelling::Raw
            }
        }
        0x80..=0x9F => {
            if escapes.escapes_c1() {
                Spelling::Uchar
            } else {
                Spelling::Raw
            }
        }
        _ => Spelling::Raw,
    }
}

/// The UTF-8 lead byte of every scalar in U+0080-U+00BF, the block that holds
/// the C1 controls.
const C1_LEAD: u8 = 0xC2;

/// The stop class of `escapes`: every ASCII byte whose scalar the profile
/// escapes, plus — for a profile that escapes the C1 block — the lead byte
/// `0xC2`. Derived from [`spelling`], so the scan cannot drift from the law.
const fn stop_table(escapes: LiteralEscapes) -> [u8; 256] {
    let mut table = [0_u8; 256];
    let mut b = 0_u32;
    while b < 0x80 {
        if !matches!(spelling(b, escapes), Spelling::Raw) {
            table[b as usize] = 1;
        }
        b += 1;
    }
    if escapes.escapes_c1() {
        table[C1_LEAD as usize] = 1;
    }
    table
}

/// One profile's stop class and its scanner: an `#[inline(never)]` monomorphic
/// function per profile, so each compiles to one kernel with its runs folded
/// in as constants and the assembly audit can find it by name.
macro_rules! stop_class {
    ($(#[$doc:meta])* $table:ident, $class:ident, $find:ident, $profile:expr) => {
        const $table: [u8; 256] = stop_table($profile);
        const $class: ByteClass<{ byte_run_count(&$table) }> = ByteClass::from_table($table);

        $(#[$doc])*
        #[inline(never)]
        fn $find(bytes: &[u8]) -> Option<usize> {
            $class.find_first(bytes)
        }
    };
}

stop_class! {
    /// The offset of the first [`LiteralEscapes::Canonical`] stop byte.
    CANONICAL_TABLE, CANONICAL_STOPS, find_first_canonical_escape, LiteralEscapes::Canonical
}
stop_class! {
    /// The offset of the first [`LiteralEscapes::XmlCarrier`] stop byte.
    XML_CARRIER_TABLE, XML_CARRIER_STOPS, find_first_xml_carrier_escape, LiteralEscapes::XmlCarrier
}
stop_class! {
    /// The offset of the first [`LiteralEscapes::C0Only`] stop byte.
    C0_ONLY_TABLE, C0_ONLY_STOPS, find_first_c0_only_escape, LiteralEscapes::C0Only
}
stop_class! {
    /// The offset of the first [`LiteralEscapes::ShortOnly`] stop byte.
    SHORT_ONLY_TABLE, SHORT_ONLY_STOPS, find_first_short_only_escape, LiteralEscapes::ShortOnly
}
stop_class! {
    /// The offset of the first [`LiteralEscapes::CanonicalPlusC1`] stop byte.
    CANONICAL_PLUS_C1_TABLE, CANONICAL_PLUS_C1_STOPS, find_first_canonical_plus_c1_escape,
    LiteralEscapes::CanonicalPlusC1
}

// Every stop byte is ASCII or the one lead byte, never a continuation byte, so
// a scanner's offset is always a char boundary of a `str`'s bytes.
const _: () = {
    let tables = [
        &CANONICAL_TABLE,
        &XML_CARRIER_TABLE,
        &C0_ONLY_TABLE,
        &SHORT_ONLY_TABLE,
        &CANONICAL_PLUS_C1_TABLE,
    ];
    let mut t = 0;
    while t < tables.len() {
        let mut b = 0x80;
        while b < 0x100 {
            assert!(
                tables[t][b] == 0 || b == C1_LEAD as usize,
                "a literal stop class holds ASCII bytes and the C1 lead byte only"
            );
            b += 1;
        }
        t += 1;
    }
};

/// The offset of the first byte of `bytes` that can begin a scalar `escapes`
/// writes as an escape, or `None` when every scalar of `bytes` rides verbatim.
///
/// A byte *candidate*, not a verdict, for the two profiles that escape the C1
/// block ([`XmlCarrier`](LiteralEscapes::XmlCarrier) and
/// [`CanonicalPlusC1`](LiteralEscapes::CanonicalPlusC1)): an ASCII stop is
/// always escaped, but at the lead byte `0xC2` the scalar it begins is escaped
/// only when it is a C1 control (U+0080-U+009F) and rides verbatim when it is
/// U+00A0-U+00BF, so a caller decodes the scalar there and asks the law. For
/// the other three profiles every stop is escaped. Every byte before the
/// offset belongs to a scalar that rides verbatim, and the offset is always a
/// char boundary of a `str`'s bytes (each class holds ASCII bytes and at most
/// one lead byte).
///
/// Each profile is its own monomorphic chunked kernel, the
/// [`terminals::ByteClass`](crate::terminals::ByteClass) scan over the class
/// derived from that profile's law.
///
/// ```
/// use purrdf_iri::literal_escape::{LiteralEscapes, find_first_escape};
///
/// assert_eq!(find_first_escape(b"plain \"quoted\"", LiteralEscapes::Canonical), Some(6));
/// assert_eq!(find_first_escape(b"plain", LiteralEscapes::Canonical), None);
/// // DEL is escaped by `Canonical` and rides raw under `C0Only`.
/// assert_eq!(find_first_escape(b"a\x7fb", LiteralEscapes::Canonical), Some(1));
/// assert_eq!(find_first_escape(b"a\x7fb", LiteralEscapes::C0Only), None);
/// // A C1 control's lead byte is a candidate only where C1 is escaped.
/// assert_eq!(find_first_escape("a\u{85}".as_bytes(), LiteralEscapes::XmlCarrier), Some(1));
/// assert_eq!(find_first_escape("a\u{85}".as_bytes(), LiteralEscapes::Canonical), None);
/// ```
#[must_use]
pub fn find_first_escape(bytes: &[u8], escapes: LiteralEscapes) -> Option<usize> {
    match escapes {
        LiteralEscapes::Canonical => find_first_canonical_escape(bytes),
        LiteralEscapes::XmlCarrier => find_first_xml_carrier_escape(bytes),
        LiteralEscapes::C0Only => find_first_c0_only_escape(bytes),
        LiteralEscapes::ShortOnly => find_first_short_only_escape(bytes),
        LiteralEscapes::CanonicalPlusC1 => find_first_canonical_plus_c1_escape(bytes),
    }
}

/// Upper-case hex digits, for the `\u00XX` spelling.
const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Write the `UCHAR` of `ch`, a scalar at most U+009F: `\u00XX` in upper-case
/// hex, the bytes `write!(out, "\\u{:04X}", ch as u32)` produces.
fn write_uchar<W: fmt::Write + ?Sized>(out: &mut W, ch: char) -> fmt::Result {
    let v = u32::from(ch);
    debug_assert!(v <= 0x9F, "only scalars up to U+009F are escaped");
    out.write_str("\\u00")?;
    out.write_char(char::from(HEX_UPPER[((v >> 4) & 0xF) as usize]))?;
    out.write_char(char::from(HEX_UPPER[(v & 0xF) as usize]))
}

/// Write `run` unless it is empty, so a clean run costs one `write_str` and an
/// empty one costs nothing.
fn write_run<W: fmt::Write + ?Sized>(out: &mut W, run: &str) -> fmt::Result {
    if run.is_empty() {
        Ok(())
    } else {
        out.write_str(run)
    }
}

/// Write `value[at..]` escaped per `escapes`, copying each run between two
/// escapes whole.
///
/// `at` is a char boundary. A candidate that needs no escape (a non-C1 scalar
/// led by `0xC2`, such as U+00A0) stays inside the run it interrupts.
fn escape_body_from<W: fmt::Write + ?Sized>(
    out: &mut W,
    value: &str,
    mut at: usize,
    escapes: LiteralEscapes,
) -> fmt::Result {
    let bytes = value.as_bytes();
    let mut run_start = at;
    while let Some(offset) = find_first_escape(&bytes[at..], escapes) {
        let hit = at + offset;
        let ch = value[hit..]
            .chars()
            .next()
            .expect("an escape offset is a char boundary");
        at = hit + ch.len_utf8();
        match spelling(u32::from(ch), escapes) {
            Spelling::Raw => continue,
            Spelling::Short(short) => {
                write_run(out, &value[run_start..hit])?;
                out.write_str(short)?;
            }
            Spelling::Uchar => {
                write_run(out, &value[run_start..hit])?;
                write_uchar(out, ch)?;
            }
        }
        run_start = at;
    }
    write_run(out, &value[run_start..])
}

/// Write the body of `value` — everything between the quotes — to `out`,
/// spelled per `escapes`.
///
/// One scan finds each escape and every run between two of them is copied
/// whole with one [`fmt::Write::write_str`]; a value with nothing to escape is
/// one `write_str`. The `Err` is the sink's own — a `String` never fails — and
/// the function stops at the first fragment the sink refuses.
///
/// ```
/// use purrdf_iri::literal_escape::{LiteralEscapes, escape_body};
///
/// let mut out = String::new();
/// escape_body(&mut out, "a\"b\\c\n\u{8}\u{1}\u{7f}\u{85}\u{e9}", LiteralEscapes::Canonical)?;
/// assert_eq!(out, "a\\\"b\\\\c\\n\\b\\u0001\\u007F\u{85}\u{e9}");
/// # Ok::<(), core::fmt::Error>(())
/// ```
pub fn escape_body<W: fmt::Write + ?Sized>(
    out: &mut W,
    value: &str,
    escapes: LiteralEscapes,
) -> fmt::Result {
    escape_body_from(out, value, 0, escapes)
}

/// Append the body of `value` to `out`, spelled per `escapes`.
///
/// ```
/// use purrdf_iri::literal_escape::{LiteralEscapes, push_body};
///
/// let mut out = String::from("\"");
/// push_body(&mut out, "tab\there\u{85}", LiteralEscapes::XmlCarrier);
/// assert_eq!(out, "\"tab\\there\\u0085");
/// ```
pub fn push_body(out: &mut String, value: &str, escapes: LiteralEscapes) {
    escape_body(out, value, escapes).expect("a String sink never fails");
}

/// The body of `value` spelled per `escapes`, borrowed when no scalar needs an
/// escape.
///
/// The same bytes as [`escape_body`]. The scan that finds the first escaped
/// scalar is the scan that emits, so a value that needs escaping is still read
/// once: its clean prefix is copied whole and the emission continues from the
/// first escape.
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_iri::literal_escape::{LiteralEscapes, escaped};
///
/// assert!(matches!(escaped("caf\u{e9}", LiteralEscapes::Canonical), Cow::Borrowed(_)));
/// assert_eq!(escaped("say \"hi\"", LiteralEscapes::Canonical), "say \\\"hi\\\"");
/// // What one profile escapes another leaves raw, and borrows.
/// assert_eq!(escaped("\u{85}", LiteralEscapes::CanonicalPlusC1), "\\u0085");
/// assert!(matches!(escaped("\u{85}", LiteralEscapes::Canonical), Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn escaped(value: &str, escapes: LiteralEscapes) -> Cow<'_, str> {
    let bytes = value.as_bytes();
    let mut at = 0;
    while let Some(offset) = find_first_escape(&bytes[at..], escapes) {
        let hit = at + offset;
        let ch = value[hit..]
            .chars()
            .next()
            .expect("an escape offset is a char boundary");
        if spelling(u32::from(ch), escapes) != Spelling::Raw {
            let mut out = String::with_capacity(value.len() + 8);
            out.push_str(&value[..hit]);
            escape_body_from(&mut out, value, hit, escapes).expect("a String sink never fails");
            return Cow::Owned(out);
        }
        at = hit + ch.len_utf8();
    }
    Cow::Borrowed(value)
}

#[cfg(test)]
mod tests {
    use super::{
        C0_ONLY_TABLE, CANONICAL_PLUS_C1_TABLE, CANONICAL_TABLE, LiteralEscapes, SHORT_ONLY_TABLE,
        XML_CARRIER_TABLE, escape_body, escaped, find_first_escape, push_body,
    };
    use core::fmt;
    use std::borrow::Cow;
    use std::fmt::Write as _;

    const ALL: [LiteralEscapes; 5] = [
        LiteralEscapes::Canonical,
        LiteralEscapes::XmlCarrier,
        LiteralEscapes::C0Only,
        LiteralEscapes::ShortOnly,
        LiteralEscapes::CanonicalPlusC1,
    ];

    /// The `\u00XX` every source spells with `write!(out, "\\u{:04X}", c as u32)`.
    fn uchar(c: char, out: &mut String) {
        let _ = write!(out, "\\u{:04X}", c as u32);
    }

    /// The per-`char` writers each profile replaced, transcribed verbatim from
    /// their match arms and kept as the oracle: the goldens and frozen vectors
    /// were produced by these, so a disagreement at one scalar changes bytes.
    ///
    /// The canonical N-Quads writer (`purrdf_core::ir::canon`): the `ECHAR`
    /// set with `\b` and `\f`, every other C0 control and DEL as `\u00XX`.
    fn reference_canonical(c: char, out: &mut String) {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => uchar(c, out),
            c => out.push(c),
        }
    }

    /// The native serializer (`purrdf_rdf::native_codecs::ser_model`): five
    /// short escapes, then `char::is_control` as `\u00XX`.
    fn reference_xml_carrier(c: char, out: &mut String) {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => uchar(c, out),
            c => out.push(c),
        }
    }

    /// The SHACL report and GTS fold-view writers (`purrdf_shapes::term`,
    /// `purrdf_rdf::gts_view`): five short escapes, then `< 0x20` as `\u00XX`.
    fn reference_c0_only(c: char, out: &mut String) {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => uchar(c, out),
            c => out.push(c),
        }
    }

    /// The SPARQL algebra and ShEx renderers (`purrdf_sparql_algebra::serialize`,
    /// `purrdf_shex::validate`): the seven short escapes and nothing else.
    fn reference_short_only(c: char, out: &mut String) {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c => out.push(c),
        }
    }

    /// The CDT renderer (`purrdf_cdt::render`): the seven short escapes, then
    /// `char::is_control` as `\u` plus four upper-case hex digits.
    fn reference_canonical_plus_c1(c: char, out: &mut String) {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c.is_control() => uchar(c, out),
            c => out.push(c),
        }
    }

    fn reference(value: &str, escapes: LiteralEscapes) -> String {
        let per_char: fn(char, &mut String) = match escapes {
            LiteralEscapes::Canonical => reference_canonical,
            LiteralEscapes::XmlCarrier => reference_xml_carrier,
            LiteralEscapes::C0Only => reference_c0_only,
            LiteralEscapes::ShortOnly => reference_short_only,
            LiteralEscapes::CanonicalPlusC1 => reference_canonical_plus_c1,
        };
        let mut out = String::new();
        for c in value.chars() {
            per_char(c, &mut out);
        }
        out
    }

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            purrdf_testkit::rng::splitmix64_next(&mut self.0)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

    /// The three surfaces, checked against the oracle on one value: the
    /// streaming writer (after a prefix, so a stale-offset bug shows), the
    /// `String` appender, and the borrowing form with its arm asserted.
    fn check(value: &str, escapes: LiteralEscapes) -> bool {
        let expected = reference(value, escapes);
        let mut streamed = String::from("prefix:");
        escape_body(&mut streamed, value, escapes).expect("a String sink never fails");
        assert_eq!(
            &streamed["prefix:".len()..],
            expected,
            "{escapes:?} {value:?}"
        );
        let mut pushed = String::from("prefix:");
        push_body(&mut pushed, value, escapes);
        assert_eq!(
            &pushed["prefix:".len()..],
            expected,
            "{escapes:?} {value:?}"
        );
        let cow = escaped(value, escapes);
        assert_eq!(cow, expected, "{escapes:?} {value:?}");
        let borrowed = matches!(cow, Cow::Borrowed(_));
        assert_eq!(
            borrowed,
            expected == value,
            "{escapes:?} {value:?}: borrowed only when unchanged"
        );
        if let Some(i) = find_first_escape(value.as_bytes(), escapes) {
            assert!(value.is_char_boundary(i), "{escapes:?} {value:?} at {i}");
        }
        borrowed
    }

    #[test]
    fn every_profile_matches_its_oracle_on_every_scalar_through_u_00ff() {
        for escapes in ALL {
            let mut escaped_count = 0_usize;
            for cp in 0..=0xFF_u32 {
                let c = char::from_u32(cp).expect("below the surrogates");
                let alone = c.to_string();
                escaped_count += usize::from(!check(&alone, escapes));
                // In context: a clean run on either side, so the run copies
                // and the offsets are exercised, not just the lone scalar.
                check(&format!("ab{c}cd"), escapes);
                check(&format!("{c}{c}"), escapes);
            }
            // Non-vacuity: every profile escapes something and leaves
            // something raw.
            assert!(escaped_count > 0 && escaped_count < 256, "{escapes:?}");
        }
    }

    /// Every scalar with a `0xC2` lead, the boundary neighbours in every UTF-8
    /// width, and the ASCII stops.
    fn alphabet() -> Vec<char> {
        let mut out: Vec<char> = (0_u8..0x80).map(char::from).collect();
        out.extend(('\u{80}'..='\u{BF}').chain([
            '\u{C0}',
            '\u{E9}',
            '\u{FF}',
            '\u{100}',
            '\u{2028}',
            '\u{3000}',
            '\u{FFFD}',
            '\u{1F600}',
            '\u{10FFFF}',
        ]));
        out
    }

    #[test]
    fn every_profile_matches_its_oracle_on_generated_values() {
        let alphabet = alphabet();
        let mut rng = SplitMix(0x0011_7E12_A1E5_CA9E);
        let mut borrowed = 0_usize;
        let mut owned = 0_usize;
        for len in (0..=70).chain([127, 128, 129, 255, 1000]) {
            for _ in 0..20 {
                let value: String = (0..len)
                    .map(|_| {
                        if rng.below(6) == 0 {
                            alphabet[rng.below(alphabet.len())]
                        } else {
                            'a'
                        }
                    })
                    .collect();
                for escapes in ALL {
                    if check(&value, escapes) {
                        borrowed += 1;
                    } else {
                        owned += 1;
                    }
                }
            }
        }
        // Non-vacuity: both arms were taken.
        assert!(borrowed > 0 && owned > 0, "{borrowed} {owned}");
    }

    #[test]
    fn the_exact_bytes_of_the_scalars_the_profiles_disagree_on() {
        // (input, Canonical, XmlCarrier, C0Only, ShortOnly, CanonicalPlusC1)
        let cases: [(&str, [&str; 5]); 7] = [
            ("\u{8}", ["\\b", "\\u0008", "\\u0008", "\\b", "\\b"]),
            ("\u{c}", ["\\f", "\\u000C", "\\u000C", "\\f", "\\f"]),
            (
                "\u{7f}",
                ["\\u007F", "\\u007F", "\u{7f}", "\u{7f}", "\\u007F"],
            ),
            (
                "\u{85}",
                ["\u{85}", "\\u0085", "\u{85}", "\u{85}", "\\u0085"],
            ),
            (
                "\u{1}",
                ["\\u0001", "\\u0001", "\\u0001", "\u{1}", "\\u0001"],
            ),
            ("\u{e9}", ["\u{e9}"; 5]),
            ("\u{1f600}", ["\u{1f600}"; 5]),
        ];
        for (input, expected) in cases {
            for (escapes, want) in ALL.into_iter().zip(expected) {
                assert_eq!(escaped(input, escapes), want, "{escapes:?} {input:?}");
                let mut out = String::new();
                push_body(&mut out, input, escapes);
                assert_eq!(out, want, "{escapes:?} {input:?}");
            }
        }
        // And the five every profile agrees on.
        for escapes in ALL {
            assert_eq!(
                escaped("\\\"\n\r\t", escapes),
                "\\\\\\\"\\n\\r\\t",
                "{escapes:?}"
            );
        }
    }

    #[test]
    fn each_stop_table_is_exactly_its_documented_class() {
        let c0: Vec<u8> = (0x00..0x20).collect();
        let short_only: Vec<u8> = vec![0x08, 0x09, 0x0A, 0x0C, 0x0D, b'"', b'\\'];
        let mut canonical = c0.clone();
        canonical.extend([b'"', b'\\', 0x7F]);
        let mut c0_only = c0;
        c0_only.extend_from_slice(b"\"\\");
        let mut with_c1 = canonical.clone();
        with_c1.push(0xC2);
        for (name, table, expected) in [
            ("canonical", &CANONICAL_TABLE, &canonical),
            ("xml carrier", &XML_CARRIER_TABLE, &with_c1),
            ("c0 only", &C0_ONLY_TABLE, &c0_only),
            ("short only", &SHORT_ONLY_TABLE, &short_only),
            ("canonical plus c1", &CANONICAL_PLUS_C1_TABLE, &with_c1),
        ] {
            let members: Vec<u8> = (0..=u8::MAX)
                .filter(|&b| table[usize::from(b)] != 0)
                .collect();
            assert_eq!(&members, expected, "{name}");
        }
    }

    #[test]
    fn every_escaped_scalar_begins_with_a_stop_byte_and_no_raw_ascii_does() {
        for escapes in ALL {
            for cp in 0..=0x0010_FFFF_u32 {
                let Some(c) = char::from_u32(cp) else {
                    continue;
                };
                let mut buf = [0_u8; 4];
                let lead = c.encode_utf8(&mut buf).as_bytes()[0];
                let stops = find_first_escape(&[lead], escapes).is_some();
                let alone = c.to_string();
                let expected = reference(&alone, escapes);
                if expected != alone {
                    assert!(
                        stops,
                        "{escapes:?} {c:?} is escaped but its lead is not a stop"
                    );
                } else if c.is_ascii() {
                    assert!(!stops, "{escapes:?} {c:?} rides raw but is a stop");
                }
            }
        }
    }

    #[test]
    fn a_candidate_that_needs_no_escape_does_not_split_the_run() {
        for escapes in [LiteralEscapes::XmlCarrier, LiteralEscapes::CanonicalPlusC1] {
            // U+00A0 is led by 0xC2 and rides verbatim, so the value is borrowed.
            assert!(
                matches!(escaped("a\u{a0}b", escapes), Cow::Borrowed(_)),
                "{escapes:?}"
            );
            // Its neighbour U+009F is escaped.
            assert_eq!(escaped("a\u{9f}b", escapes), "a\\u009Fb", "{escapes:?}");
        }
    }

    #[test]
    fn fragment_escaping_equals_whole_escaping() {
        let value = "x\"\u{1}\u{85}caf\u{e9}\u{7f}\\y\u{8}\u{c}\u{a0}";
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

    /// The sink is any `fmt::Write`, including a `Formatter` behind a `Display`
    /// impl, and its error is propagated rather than swallowed.
    #[test]
    fn escape_body_writes_through_any_fmt_write_and_propagates_its_error() {
        struct Body<'a>(&'a str, LiteralEscapes);
        impl fmt::Display for Body<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                escape_body(f, self.0, self.1)
            }
        }
        assert_eq!(
            format!("\"{}\"", Body("a\"b\u{8}", LiteralEscapes::Canonical)),
            "\"a\\\"b\\b\""
        );

        /// A sink that refuses everything.
        struct Refusing;
        impl fmt::Write for Refusing {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        for escapes in ALL {
            assert_eq!(escape_body(&mut Refusing, "a", escapes), Err(fmt::Error));
            assert_eq!(escape_body(&mut Refusing, "\"", escapes), Err(fmt::Error));
            // The neighbour: nothing to write, nothing to refuse.
            assert_eq!(escape_body(&mut Refusing, "", escapes), Ok(()));
        }
    }

    #[test]
    fn empty_and_clean_inputs_borrow_and_find_nothing() {
        for escapes in ALL {
            assert!(matches!(escaped("", escapes), Cow::Borrowed(_)));
            assert_eq!(find_first_escape(b"", escapes), None);
            let clean = [b'a'; 100];
            assert_eq!(find_first_escape(&clean, escapes), None);
            // And the neighbour: the same input with one member at the far end.
            let mut dirty = clean;
            dirty[99] = b'"';
            assert_eq!(find_first_escape(&dirty, escapes), Some(99));
        }
    }
}
