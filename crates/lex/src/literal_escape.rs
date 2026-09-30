// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one escaper for the body of an RDF string literal — the text between
//! the quotes of an N-Triples, N-Quads, Turtle, TriG or SPARQL literal.
//!
//! # Why one escaper serves three carriers
//!
//! Every quoted-literal production in the RDF 1.2 text grammars shares its
//! escape vocabulary:
//!
//! ```text
//! ECHAR ::= '\' [tbnrf"'\]
//! UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX
//! ```
//!
//! (N-Triples 1.2 §7 `[153s]`/`[10]`, Turtle 1.2 §6.5 `[159s]`/`[26]`.) A reader
//! decodes any escape back to its scalar, so the byte stream a writer emits is
//! a choice, and two writers that choose differently disagree about bytes
//! while agreeing about data — which is exactly what a canonical form, a
//! digest or a golden file notices. The choice is therefore made once, here,
//! and the only thing that may vary is the *carrier* the literal travels in,
//! because a carrier adds constraints the grammar does not:
//!
//! | [`Carrier`] | `"` `\` | LF | CR TAB BS FF | other C0, DEL | C1 (U+0080-U+009F) | U+FFFE, U+FFFF |
//! |---|---|---|---|---|---|---|
//! | [`Canonical`](Carrier::Canonical) | ECHAR | `\n` | ECHAR | `\u00XX` | raw | raw |
//! | [`Xml`](Carrier::Xml) | ECHAR | `\n` | ECHAR | `\u00XX` | `\u00XX` | `\uFFFE` `\uFFFF` |
//! | [`TurtleLong`](Carrier::TurtleLong) | see below | raw | ECHAR | `\u00XX` | `\u00XX` | `\uFFFE` `\uFFFF` |
//!
//! * **`Canonical`** is the RDF 1.2 N-Triples canonical form (N-Triples 1.2
//!   §4 "Canonical N-Triples"): `"`, `\`, LF, CR, TAB, BACKSPACE and FORM FEED
//!   as their `ECHAR`, every other scalar below U+0020 and U+007F as `UCHAR`
//!   with upper-case hex, and every other scalar raw. That last clause keeps
//!   the C1 block raw, as the W3C RDFC-1.0 suite pins (test060).
//! * **`Xml`** is the canonical form for a literal whose serialized text is
//!   embedded in an XML text node or attribute (RDF/XML and TriX text, the
//!   CL-dialect payload). XML 1.0 §2.2 `Char` excludes U+FFFE and U+FFFF and
//!   discourages the C1 controls, and XML 1.1 §2.11 rewrites NEL (U+0085) on
//!   read, so those scalars ride as `UCHAR` and the literal survives the round
//!   trip through the carrier. Everything else is `Canonical`.
//! * **`TurtleLong`** is the body of a Turtle long string, `"""…"""`
//!   (Turtle 1.2 `STRING_LITERAL_LONG_QUOTE` `[25]`), whose grammar admits a
//!   raw LINE FEED, so a multi-line value stays readable. Every other scalar
//!   follows `Xml`. A `"` rides raw except where it would begin the closing
//!   delimiter: a `"` followed by another `"`, or the last scalar of the
//!   value, is written `\"`, so no run of three raw quotes and no quote next
//!   to the closing `"""` is ever produced.
//!
//! # The scan
//!
//! Each carrier has one chunked [`ByteClass`] kernel over the bytes that can
//! begin an escaped scalar; the runs between two stops are copied whole. The
//! [`Canonical`](Carrier::Canonical) kernel, [`find_first_literal_escape`], is
//! all-ASCII, so every stop is an escape. The other two also stop at the lead
//! bytes `0xC2` (the C1 block) and `0xEF` (U+FFFE, U+FFFF) and at `"`, which are
//! candidates: the scalar there is decided before anything is emitted.

use std::borrow::Cow;

use crate::terminals::{ByteClass, byte_run_count};
use crate::text_out::TextOut;

/// The carrier a literal body is escaped for. Each is a specification's
/// constraint, not a style; see the [module docs](self) for the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Carrier {
    /// The RDF 1.2 N-Triples canonical form: C1 raw.
    Canonical,
    /// The canonical form for text embedded in an XML 1.0 document: C1, U+FFFE
    /// and U+FFFF as `UCHAR`.
    Xml,
    /// The body of a Turtle `"""…"""` long string: the `Xml` rules with LINE
    /// FEED raw and `"` escaped only where it would close the string.
    TurtleLong,
}

/// The UTF-8 lead byte of U+0080-U+00BF, the block holding the C1 controls.
const C1_LEAD: u8 = 0xC2;
/// The UTF-8 lead byte of U+F000-U+FFFF, the block holding U+FFFE and U+FFFF.
const NONCHAR_LEAD: u8 = 0xEF;

/// The bytes the canonical form escapes: `"`, `\`, the C0 controls and DEL.
/// Every member is ASCII and every member is escaped.
const CANONICAL_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'"' as usize] = 1;
    table[b'\\' as usize] = 1;
    table[0x7F] = 1;
    table
};

/// The canonical bytes plus the two lead bytes whose blocks hold the scalars
/// an XML carrier escapes.
const XML_TABLE: [u8; 256] = {
    let mut table = CANONICAL_TABLE;
    table[C1_LEAD as usize] = 1;
    table[NONCHAR_LEAD as usize] = 1;
    table
};

/// The XML carrier's bytes with LINE FEED removed: a long string carries it
/// raw.
const TURTLE_LONG_TABLE: [u8; 256] = {
    let mut table = XML_TABLE;
    table[b'\n' as usize] = 0;
    table
};

const CANONICAL: ByteClass<{ byte_run_count(&CANONICAL_TABLE) }> =
    ByteClass::from_table(CANONICAL_TABLE);
const XML: ByteClass<{ byte_run_count(&XML_TABLE) }> = ByteClass::from_table(XML_TABLE);
const TURTLE_LONG: ByteClass<{ byte_run_count(&TURTLE_LONG_TABLE) }> =
    ByteClass::from_table(TURTLE_LONG_TABLE);

/// The offset of the first byte of `bytes` the [`Canonical`](Carrier::Canonical)
/// form escapes (`"`, `\`, a C0 control or DEL), or `None` when the whole value
/// rides verbatim.
///
/// Every stop is an escape and every stop is ASCII, so the offset is a char
/// boundary of a `str`'s bytes. The chunked kernel of [`ByteClass`]: sixteen-byte
/// packed compares on every target with a vector unit.
///
/// ```
/// use purrdf_lex::literal_escape::find_first_literal_escape;
///
/// assert_eq!(find_first_literal_escape(b"plain text"), None);
/// assert_eq!(find_first_literal_escape(b"say \"hi\""), Some(4));
/// // The C1 block rides raw in the canonical form.
/// assert_eq!(find_first_literal_escape("a\u{85}".as_bytes()), None);
/// ```
#[inline(never)]
#[must_use]
pub fn find_first_literal_escape(bytes: &[u8]) -> Option<usize> {
    CANONICAL.find_first(bytes)
}

/// The offset of the first byte of `bytes` that can begin a scalar the
/// [`Xml`](Carrier::Xml) carrier escapes, or `None` when none can.
///
/// A candidate, not a verdict: at `0xC2` only a C1 control is escaped and at
/// `0xEF` only U+FFFE and U+FFFF; every ASCII stop is an escape.
///
/// ```
/// use purrdf_lex::literal_escape::find_first_xml_carrier_escape;
///
/// assert_eq!(find_first_xml_carrier_escape("a\u{85}".as_bytes()), Some(1));
/// assert_eq!(find_first_xml_carrier_escape("caf\u{e9}".as_bytes()), None);
/// ```
#[inline(never)]
#[must_use]
pub fn find_first_xml_carrier_escape(bytes: &[u8]) -> Option<usize> {
    XML.find_first(bytes)
}

/// The offset of the first byte of `bytes` that can begin a scalar the
/// [`TurtleLong`](Carrier::TurtleLong) carrier escapes, or `None` when none can.
///
/// The [`find_first_xml_carrier_escape`] class without LINE FEED. `"` is a
/// candidate: it is escaped only where it would close the long string.
///
/// ```
/// use purrdf_lex::literal_escape::find_first_turtle_long_escape;
///
/// assert_eq!(find_first_turtle_long_escape(b"line one\nline two"), None);
/// assert_eq!(find_first_turtle_long_escape(b"tab\there"), Some(3));
/// ```
#[inline(never)]
#[must_use]
pub fn find_first_turtle_long_escape(bytes: &[u8]) -> Option<usize> {
    TURTLE_LONG.find_first(bytes)
}

/// What a candidate byte turns into.
#[derive(Clone, Copy)]
enum Escape {
    /// A fixed `ECHAR` spelling.
    Echar(&'static str),
    /// `\u00XX`, for a scalar at most U+00FF.
    Low(u8),
    /// `\uFFXX`, for U+FFFE or U+FFFF.
    High(u8),
}

/// The escape for the scalar that begins at `bytes[at]` (a stop of `carrier`'s
/// kernel) and the byte length it replaces, or `None` when the scalar rides
/// raw and the scan goes on past the one byte.
#[inline]
fn escape_at(bytes: &[u8], at: usize, carrier: Carrier) -> Option<(Escape, usize)> {
    let b = bytes[at];
    Some(match b {
        b'\\' => (Escape::Echar("\\\\"), 1),
        b'"' => {
            if carrier == Carrier::TurtleLong && bytes.get(at + 1).is_some_and(|&n| n != b'"') {
                return None;
            }
            (Escape::Echar("\\\""), 1)
        }
        b'\n' => (Escape::Echar("\\n"), 1),
        b'\r' => (Escape::Echar("\\r"), 1),
        b'\t' => (Escape::Echar("\\t"), 1),
        0x08 => (Escape::Echar("\\b"), 1),
        0x0C => (Escape::Echar("\\f"), 1),
        0x00..=0x1F | 0x7F => (Escape::Low(b), 1),
        C1_LEAD => match bytes.get(at + 1) {
            // U+0080-U+009F is `C2 80`-`C2 9F`.
            Some(&second @ 0x80..=0x9F) => (Escape::Low(second), 2),
            _ => return None,
        },
        NONCHAR_LEAD => match (bytes.get(at + 1), bytes.get(at + 2)) {
            // U+FFFE is `EF BF BE`, U+FFFF is `EF BF BF`.
            (Some(0xBF), Some(&third @ (0xBE | 0xBF))) => (Escape::High(third + 0x40), 3),
            _ => return None,
        },
        _ => return None,
    })
}

/// Append one escape.
#[inline]
fn push_escape<W: TextOut + ?Sized>(escape: Escape, out: &mut W) {
    let (prefix, byte) = match escape {
        Escape::Echar(text) => {
            out.push_str(text);
            return;
        }
        Escape::Low(byte) => ("\\u00", byte),
        Escape::High(byte) => ("\\uFF", byte),
    };
    let mut digits = [0_u8; 2];
    out.push_str(prefix);
    out.push_str(
        purrdf_hash::hex::encode_upper_to_slice(&[byte], &mut digits)
            .expect("one byte renders in two digits"),
    );
}

/// The kernel for `carrier`.
#[inline]
fn finder(carrier: Carrier) -> fn(&[u8]) -> Option<usize> {
    match carrier {
        Carrier::Canonical => find_first_literal_escape,
        Carrier::Xml => find_first_xml_carrier_escape,
        Carrier::TurtleLong => find_first_turtle_long_escape,
    }
}

/// Append `value[at..]` escaped for `carrier`; `value[run..at]` is a clean run
/// not yet copied. `at` and `run` are char boundaries.
fn write_from<W: TextOut + ?Sized>(
    value: &str,
    mut run: usize,
    mut at: usize,
    carrier: Carrier,
    out: &mut W,
) {
    let bytes = value.as_bytes();
    let find = finder(carrier);
    while let Some(offset) = find(&bytes[at..]) {
        let hit = at + offset;
        match escape_at(bytes, hit, carrier) {
            Some((escape, len)) => {
                out.push_str(&value[run..hit]);
                push_escape(escape, out);
                at = hit + len;
                run = at;
            }
            // A candidate that rides raw: step past its first byte. The next
            // bytes are continuation bytes or a `"`, never a class member that
            // starts mid-scalar, so the run stays whole.
            None => at = hit + 1,
        }
    }
    out.push_str(&value[run..]);
}

/// Append `value`, escaped for `carrier`, to `out` — the body only, without
/// the surrounding quotes.
///
/// ```
/// use purrdf_lex::literal_escape::{Carrier, write};
///
/// let mut out = String::new();
/// write("a\"b\\c\n\u{8}\u{1}\u{7f}\u{85}é", Carrier::Canonical, &mut out);
/// assert_eq!(out, "a\\\"b\\\\c\\n\\b\\u0001\\u007F\u{85}é");
///
/// out.clear();
/// write("\u{85}\u{ffff}", Carrier::Xml, &mut out);
/// assert_eq!(out, "\\u0085\\uFFFF");
///
/// out.clear();
/// write("one\ntwo \"\"\" end\"", Carrier::TurtleLong, &mut out);
/// assert_eq!(out, "one\ntwo \\\"\\\"\" end\\\"");
/// ```
pub fn write<W: TextOut + ?Sized>(value: &str, carrier: Carrier, out: &mut W) {
    write_from(value, 0, 0, carrier, out);
}

/// `value` escaped for `carrier`, borrowed when no scalar needs an escape.
///
/// The same bytes as [`write()`]. The scan that finds the first escape is the
/// scan that emits, so a value that needs escaping is still read once.
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_lex::literal_escape::{Carrier, escape};
///
/// assert!(matches!(escape("caf\u{e9}", Carrier::Xml), Cow::Borrowed(_)));
/// assert_eq!(escape("a\tb", Carrier::Canonical), "a\\tb");
/// ```
#[must_use]
pub fn escape(value: &str, carrier: Carrier) -> Cow<'_, str> {
    let bytes = value.as_bytes();
    let find = finder(carrier);
    let mut at = 0;
    while let Some(offset) = find(&bytes[at..]) {
        let hit = at + offset;
        if escape_at(bytes, hit, carrier).is_some() {
            let mut out = String::with_capacity(value.len() + 8);
            write_from(value, 0, hit, carrier, &mut out);
            return Cow::Owned(out);
        }
        at = hit + 1;
    }
    Cow::Borrowed(value)
}

#[cfg(test)]
mod tests {
    use super::{
        CANONICAL_TABLE, Carrier, TURTLE_LONG_TABLE, XML_TABLE, escape, find_first_literal_escape,
        write,
    };
    use std::borrow::Cow;
    use std::fmt::Write as _;

    const CARRIERS: [Carrier; 3] = [Carrier::Canonical, Carrier::Xml, Carrier::TurtleLong];

    /// Whether `carrier` writes the scalar `c` as a `UCHAR` (never an ECHAR).
    fn is_uchar_scalar(c: char, carrier: Carrier) -> bool {
        let v = u32::from(c);
        let control = v < 0x20 || v == 0x7F;
        match carrier {
            Carrier::Canonical => control,
            Carrier::Xml | Carrier::TurtleLong => {
                control || (0x80..=0x9F).contains(&v) || v == 0xFFFE || v == 0xFFFF
            }
        }
    }

    /// The per-scalar writer, kept as the oracle for the clean-run emitter.
    fn reference(value: &str, carrier: Carrier) -> String {
        let mut out = String::new();
        let chars: Vec<char> = value.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' if carrier == Carrier::TurtleLong => {
                    if chars.get(i + 1).is_none_or(|&n| n == '"') {
                        out.push_str("\\\"");
                    } else {
                        out.push('"');
                    }
                }
                '"' => out.push_str("\\\""),
                '\n' if carrier == Carrier::TurtleLong => out.push('\n'),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{08}' => out.push_str("\\b"),
                '\u{0C}' => out.push_str("\\f"),
                c if is_uchar_scalar(c, carrier) => {
                    let _ = write!(out, "\\u{:04X}", u32::from(c));
                }
                c => out.push(c),
            }
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

    /// Every ASCII scalar, the whole `0xC2` block, the `0xEF` block's edges and
    /// non-ASCII neighbours in every UTF-8 width.
    fn alphabet() -> Vec<char> {
        let mut out: Vec<char> = (0_u8..0x80).map(char::from).collect();
        out.extend(('\u{80}'..='\u{BF}').chain([
            '\u{C0}',
            '\u{E9}',
            '\u{FF}',
            '\u{2028}',
            '\u{F000}',
            '\u{FEFF}',
            '\u{FFFD}',
            '\u{FFFE}',
            '\u{FFFF}',
            '\u{1F408}',
            '\u{10FFFF}',
        ]));
        out
    }

    #[test]
    fn every_carrier_agrees_with_the_per_scalar_writer() {
        let alphabet = alphabet();
        let mut rng = SplitMix(0x00CA_0010_E5CA_9E00);
        let mut escaped_past_first_chunk = 0_usize;
        let (mut borrowed, mut owned) = (0_usize, 0_usize);
        for len in (0..=70).chain([127, 128, 129, 255, 1000, 4099]) {
            for _ in 0..30 {
                let value: String = (0..len)
                    .map(|_| {
                        if rng.below(5) == 0 {
                            alphabet[rng.below(alphabet.len())]
                        } else {
                            'q'
                        }
                    })
                    .collect();
                for (skip, _) in value.char_indices().take(3) {
                    let input = &value[skip..];
                    for carrier in CARRIERS {
                        let expected = reference(input, carrier);
                        let mut got = String::from("\"");
                        write(input, carrier, &mut got);
                        assert_eq!(&got[1..], expected, "{carrier:?} {input:?}");
                        let cow = escape(input, carrier);
                        assert_eq!(cow, expected, "{carrier:?} {input:?}");
                        match cow {
                            Cow::Borrowed(_) => {
                                borrowed += 1;
                                assert_eq!(expected, input);
                            }
                            Cow::Owned(_) => {
                                owned += 1;
                                assert_ne!(expected, input);
                            }
                        }
                    }
                    escaped_past_first_chunk += usize::from(
                        find_first_literal_escape(input.as_bytes()).is_some_and(|i| i >= 16),
                    );
                }
            }
        }
        assert!(
            escaped_past_first_chunk > 0,
            "the chunked path found escapes"
        );
        assert!(borrowed > 0 && owned > 0, "{borrowed} {owned}");
    }

    #[test]
    fn canonical_form_uses_every_echar_and_keeps_c1_raw() {
        let mut out = String::new();
        write(
            "\"\\\n\r\t\u{8}\u{c}\u{0}\u{1f}\u{7f}",
            Carrier::Canonical,
            &mut out,
        );
        assert_eq!(out, "\\\"\\\\\\n\\r\\t\\b\\f\\u0000\\u001F\\u007F");
        // C1 raw, per RDFC-1.0 test060; U+FFFE and U+FFFF raw too.
        assert_eq!(
            escape("\u{80}\u{85}\u{9f}\u{fffe}\u{ffff}", Carrier::Canonical),
            "\u{80}\u{85}\u{9f}\u{fffe}\u{ffff}"
        );
    }

    #[test]
    fn a_plain_tab_is_still_the_tab_echar_in_every_carrier() {
        for carrier in CARRIERS {
            assert_eq!(escape("a\tb", carrier), "a\\tb", "{carrier:?}");
        }
    }

    #[test]
    fn the_xml_carrier_escapes_c1_and_the_two_noncharacters_only() {
        assert_eq!(
            escape("\u{80}\u{9f}\u{fffe}\u{ffff}", Carrier::Xml),
            "\\u0080\\u009F\\uFFFE\\uFFFF"
        );
        // The neighbours on the far side of each boundary ride raw: U+00A0
        // (led by 0xC2), U+FFFD and U+F000 (led by 0xEF), U+FEFF.
        for raw in [
            "\u{a0}", "\u{bf}", "\u{fffd}", "\u{f000}", "\u{feff}", "\u{ffef}",
        ] {
            assert!(
                matches!(escape(raw, Carrier::Xml), Cow::Borrowed(_)),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn a_long_string_keeps_line_feeds_and_escapes_only_closing_quotes() {
        let mut out = String::new();
        write("a\nb", Carrier::TurtleLong, &mut out);
        assert_eq!(out, "a\nb");
        // One or two raw quotes followed by other text are lawful.
        assert_eq!(
            escape("say \"hi\" now", Carrier::TurtleLong),
            "say \"hi\" now"
        );
        // A quote before another quote, or at the very end, is escaped.
        assert_eq!(escape("\"\"\"", Carrier::TurtleLong), "\\\"\\\"\\\"");
        assert_eq!(escape("end\"", Carrier::TurtleLong), "end\\\"");
        assert_eq!(escape("\"\"x", Carrier::TurtleLong), "\\\"\"x");
        // CR is not raw: only LINE FEED is.
        assert_eq!(escape("a\r\nb", Carrier::TurtleLong), "a\\r\nb");
    }

    #[test]
    fn a_long_string_body_never_contains_three_raw_quotes_or_ends_in_one() {
        let alphabet = ['"', 'a', '\\', '\n'];
        let mut rng = SplitMix(0x7700_1234_0000_0001);
        for len in 0..40 {
            for _ in 0..50 {
                let value: String = (0..len).map(|_| alphabet[rng.below(4)]).collect();
                let body = escape(&value, Carrier::TurtleLong);
                // Read the body as a lexer would: an escape is two bytes, and
                // a raw quote extends the current run of raw quotes.
                let bytes = body.as_bytes();
                let (mut at, mut run, mut last_raw_quote) = (0, 0, false);
                while at < bytes.len() {
                    if bytes[at] == b'\\' {
                        at += 2;
                        run = 0;
                        last_raw_quote = false;
                        continue;
                    }
                    last_raw_quote = bytes[at] == b'"';
                    run = if last_raw_quote { run + 1 } else { 0 };
                    assert!(run < 3, "{value:?} -> {body:?}");
                    at += 1;
                }
                assert!(!last_raw_quote, "{value:?} -> {body:?}");
            }
        }
    }

    #[test]
    fn the_tables_nest_as_the_carriers_do() {
        for b in 0..=u8::MAX {
            let i = usize::from(b);
            let canonical = CANONICAL_TABLE[i] != 0;
            assert_eq!(canonical, b < 0x20 || b == b'"' || b == b'\\' || b == 0x7F);
            assert_eq!(
                XML_TABLE[i] != 0,
                canonical || b == 0xC2 || b == 0xEF,
                "{b:#04X}"
            );
            assert_eq!(
                TURTLE_LONG_TABLE[i] != 0,
                XML_TABLE[i] != 0 && b != b'\n',
                "{b:#04X}"
            );
        }
    }
}
