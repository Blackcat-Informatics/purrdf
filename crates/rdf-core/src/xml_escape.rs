// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lossless XML 1.0 egress for character data and double-quoted attributes.
//!
//! XML 1.0 §2.11 normalizes raw carriage returns; §3.3.3 also normalizes
//! attribute tabs and line feeds. Character references preserve these scalars.
//! A scalar outside the `Char` production has no XML spelling and is refused.

use core::fmt;
use std::borrow::Cow;

use purrdf_iri::terminals::{find_first_xml_special, is_xml_char};

/// The XML context containing an escaped value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// Character data between tags.
    Text,
    /// An attribute delimited by double quotes.
    Attribute,
}

/// An input scalar that XML 1.0 cannot represent, even by reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidXmlChar {
    /// Byte offset of the scalar in the supplied value.
    pub byte_offset: usize,
    /// The forbidden Unicode scalar.
    pub character: char,
}

impl std::fmt::Display for InvalidXmlChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "U+{:04X} at byte {} is not permitted in XML 1.0",
            self.character as u32, self.byte_offset
        )
    }
}

impl std::error::Error for InvalidXmlChar {}

fn replacement(character: char, context: Context) -> Option<&'static str> {
    match character {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '\r' => Some("&#xD;"),
        '"' if context == Context::Attribute => Some("&quot;"),
        '\t' if context == Context::Attribute => Some("&#x9;"),
        '\n' if context == Context::Attribute => Some("&#xA;"),
        _ => None,
    }
}

/// The scalar that begins at `offset`, a char boundary of `value`.
fn scalar_at(value: &str, offset: usize) -> char {
    value[offset..]
        .chars()
        .next()
        .expect("the XML scan stops on char boundaries")
}

/// Escape a value, borrowing it when no scalar needs an XML reference.
///
/// All scalars are validated, including the non-ASCII clean path.
///
/// One pass of [`find_first_xml_special`], which stops only at the bytes that
/// begin a scalar needing a reference in either context or a scalar outside
/// `Char`: everything between two stops is lawful content, copied whole.
///
/// # Errors
/// Returns the first scalar excluded by XML 1.0 §2.2 production `[2]`.
pub fn escape(value: &str, context: Context) -> Result<Cow<'_, str>, InvalidXmlChar> {
    let bytes = value.as_bytes();
    let mut output: Option<String> = None;
    let mut start = 0;
    let mut at = 0;
    while let Some(offset) = find_first_xml_special(&bytes[at..]) {
        let hit = at + offset;
        let character = scalar_at(value, hit);
        if !is_xml_char(character) {
            return Err(InvalidXmlChar {
                byte_offset: hit,
                character,
            });
        }
        at = hit + character.len_utf8();
        if let Some(reference) = replacement(character, context) {
            let out = output.get_or_insert_with(|| String::with_capacity(value.len() + 8));
            out.push_str(&value[start..hit]);
            out.push_str(reference);
            start = at;
        }
    }
    Ok(match output {
        Some(mut out) => {
            out.push_str(&value[start..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(value),
    })
}

/// How many references [`push_into`]'s one pass remembers before it emits.
///
/// Emission waits for the whole value to validate, so the offsets of the
/// references found on the way are held in a fixed array on the stack rather
/// than rediscovered. A value with more references than this has its tail,
/// from the first reference not held, scanned a second time for emission only.
const HELD_REFERENCES: usize = 32;

/// Append an escaped value to any text sink.
///
/// Validity is decided before a single byte is emitted, so a rejected value
/// leaves `output` untouched **without a rewind**. That matters because the
/// sinks this feeds may already have drained earlier bytes downstream, where
/// `truncate` is not available and never will be.
///
/// Validation and the search for references are one pass: one
/// [`find_first_xml_special`] scan stops at every byte that begins a scalar
/// outside `Char` or a scalar needing a reference, the scalar there is checked
/// and classified at once, and the offsets of the first
/// [`HELD_REFERENCES`] references are held on the stack. Emission then copies
/// the runs between them whole, with no second scan; only a value holding more
/// references than that is scanned again, from the first one not held. Nothing
/// is allocated.
///
/// Emission itself is infallible here by contract: a sink that can fail records
/// its own sticky error and surfaces it at its own terminal, so this function
/// reports only the XML-validity failure and never a write failure.
///
/// # Errors
/// Returns the first scalar excluded by XML 1.0 §2.2 production `[2]` — the same
/// scalar, at the same byte offset, that the single-pass form reported.
pub fn push_into<W: fmt::Write + ?Sized>(
    value: &str,
    context: Context,
    output: &mut W,
) -> Result<(), InvalidXmlChar> {
    let bytes = value.as_bytes();
    let mut held = [(0_usize, ""); HELD_REFERENCES];
    let mut held_count = 0;
    let mut first_unheld = None;
    let mut at = 0;
    while let Some(offset) = find_first_xml_special(&bytes[at..]) {
        let hit = at + offset;
        let character = scalar_at(value, hit);
        if !is_xml_char(character) {
            return Err(InvalidXmlChar {
                byte_offset: hit,
                character,
            });
        }
        at = hit + character.len_utf8();
        if let Some(reference) = replacement(character, context) {
            if held_count < HELD_REFERENCES {
                held[held_count] = (hit, reference);
                held_count += 1;
            } else if first_unheld.is_none() {
                first_unheld = Some(hit);
            }
        }
    }
    // Every referenced scalar is ASCII, so it is one byte long. The sink owns
    // its own failure channel; see the contract above.
    let mut start = 0;
    for &(hit, reference) in &held[..held_count] {
        if start < hit {
            let _ = output.write_str(&value[start..hit]);
        }
        let _ = output.write_str(reference);
        start = hit + 1;
    }
    if let Some(from) = first_unheld {
        let mut at = from;
        while let Some(offset) = find_first_xml_special(&bytes[at..]) {
            let hit = at + offset;
            let character = scalar_at(value, hit);
            at = hit + character.len_utf8();
            if let Some(reference) = replacement(character, context) {
                if start < hit {
                    let _ = output.write_str(&value[start..hit]);
                }
                let _ = output.write_str(reference);
                start = at;
            }
        }
    }
    if start < value.len() {
        let _ = output.write_str(&value[start..]);
    }
    Ok(())
}

/// Append an escaped value to an existing `String`.
///
/// The `String` spelling of [`push_into`], which is the one implementation; this
/// is not a second escaper.
///
/// # Errors
/// Returns the first forbidden scalar. The output is unchanged on failure.
pub fn push(value: &str, context: Context, output: &mut String) -> Result<(), InvalidXmlChar> {
    push_into(value, context, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two-pass escaper `push_into` replaced, kept verbatim as the oracle:
    /// a whole validation pass, then a per-`char` replacement pass.
    fn reference_push_into<W: fmt::Write + ?Sized>(
        value: &str,
        context: Context,
        output: &mut W,
    ) -> Result<(), InvalidXmlChar> {
        for (offset, character) in value.char_indices() {
            if !is_xml_char(character) {
                return Err(InvalidXmlChar {
                    byte_offset: offset,
                    character,
                });
            }
        }
        let mut start = 0;
        for (offset, character) in value.char_indices() {
            if let Some(reference) = replacement(character, context) {
                let _ = output.write_str(&value[start..offset]);
                let _ = output.write_str(reference);
                start = offset + character.len_utf8();
            }
        }
        let _ = output.write_str(&value[start..]);
        Ok(())
    }

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

    /// Every special scalar of either context, the non-`Char`s a `str` can
    /// hold, their lawful neighbours in the `0xEF` block, and non-ASCII in
    /// every UTF-8 width.
    const SCALARS: &[char] = &[
        '&',
        '<',
        '>',
        '"',
        '\'',
        '\t',
        '\n',
        '\r',
        '\u{0}',
        '\u{7}',
        '\u{1F}',
        '\u{7F}',
        '\u{85}',
        '\u{E9}',
        '\u{D7FF}',
        '\u{E000}',
        '\u{EFFF}',
        '\u{F000}',
        '\u{FFFD}',
        '\u{FFFE}',
        '\u{FFFF}',
        '\u{10000}',
        '\u{1F408}',
        '\u{10FFFF}',
    ];

    #[test]
    fn one_pass_agrees_with_the_two_pass_escaper() {
        let mut rng = SplitMix(0x0E5C_A9E0_0000_0A11);
        let (mut ok, mut refused, mut overflowed) = (0_usize, 0_usize, 0_usize);
        for len in (0..=70).chain([127, 128, 129, 255, 1000, 4099]) {
            for round in 0..40 {
                // Every fourth value draws a special on every second scalar,
                // so a value holds far more references than one pass holds.
                let density = if round % 4 == 0 { 2 } else { 8 };
                // Half the values draw from the specials without the
                // non-`Char`s, so long valid values are common too.
                let pool = if round % 2 == 0 {
                    &SCALARS[..8]
                } else {
                    SCALARS
                };
                let value: String = (0..len)
                    .map(|_| {
                        if rng.below(density) == 0 {
                            pool[rng.below(pool.len())]
                        } else {
                            'x'
                        }
                    })
                    .collect();
                for (skip, _) in value.char_indices().take(4) {
                    let input = &value[skip..];
                    for context in [Context::Text, Context::Attribute] {
                        let mut got = String::from("prefix");
                        let mut expected = String::from("prefix");
                        let result = push_into(input, context, &mut got);
                        assert_eq!(
                            result,
                            reference_push_into(input, context, &mut expected),
                            "{context:?} {input:?}"
                        );
                        assert_eq!(got, expected, "{context:?} {input:?}");
                        match escape(input, context) {
                            Ok(escaped) => {
                                assert!(result.is_ok());
                                assert_eq!(&got["prefix".len()..], escaped.as_ref());
                                ok += 1;
                                let references = input
                                    .chars()
                                    .filter(|&c| replacement(c, context).is_some())
                                    .count();
                                overflowed += usize::from(references > HELD_REFERENCES);
                            }
                            Err(error) => {
                                assert_eq!(Err(error), result);
                                assert_eq!(got, "prefix", "a refusal emits nothing");
                                refused += 1;
                            }
                        }
                    }
                }
            }
        }
        // Non-vacuity: valid values, refused values, and values past the held
        // references all occurred.
        assert!(
            ok > 0 && refused > 0 && overflowed > 0,
            "{ok} {refused} {overflowed}"
        );
    }

    /// A value with more references than one pass holds, then a forbidden
    /// scalar: still refused before any byte is written.
    #[test]
    fn a_refusal_after_many_references_emits_nothing() {
        let value = format!("{}\u{FFFE}", "&".repeat(HELD_REFERENCES * 3));
        let mut out = String::new();
        let error = push_into(&value, Context::Text, &mut out).unwrap_err();
        assert_eq!(error.byte_offset, HELD_REFERENCES * 3);
        assert_eq!(out, "");
        // The neighbour without the forbidden scalar is written in full.
        let value = "&".repeat(HELD_REFERENCES * 3);
        push_into(&value, Context::Text, &mut out).expect("valid");
        assert_eq!(out, "&amp;".repeat(HELD_REFERENCES * 3));
    }

    #[test]
    fn unchanged_values_are_borrowed_and_invalid_values_are_atomic() {
        assert!(matches!(
            escape("plain 🐈", Context::Text).unwrap(),
            Cow::Borrowed(_)
        ));
        let mut out = String::from("prefix");
        let error = push("&🐈\u{FFFF}", Context::Text, &mut out).unwrap_err();
        assert_eq!(error.byte_offset, 5);
        assert_eq!(out, "prefix");
    }

    /// The atomicity the single-pass form bought with `truncate` now holds for a
    /// sink that CANNOT rewind: a rejected value emits nothing at all, rather than
    /// emitting a prefix and retracting it.
    #[test]
    fn a_rejected_value_emits_nothing_into_a_sink_that_cannot_rewind() {
        /// Counts writes as well as bytes, so "emitted a prefix then retracted it"
        /// is distinguishable from "never emitted".
        struct CountingWrites {
            text: String,
            writes: usize,
        }
        impl fmt::Write for CountingWrites {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.writes += 1;
                self.text.push_str(s);
                Ok(())
            }
        }

        let mut sink = CountingWrites {
            text: String::new(),
            writes: 0,
        };
        // `&` at offset 0 WOULD be replaced, and the single-pass form wrote that
        // replacement before discovering the forbidden scalar at offset 5.
        let error = push_into("&🐈\u{FFFF}", Context::Text, &mut sink).unwrap_err();
        assert_eq!(error.byte_offset, 5);
        assert_eq!(error.character, '\u{FFFF}');
        assert_eq!(sink.writes, 0, "no write was attempted");
        assert_eq!(sink.text, "");
    }

    /// `push_into` and `escape` are one escaper: same bytes, for every context and
    /// every shape that exercises a replacement, a clean run, or both.
    #[test]
    fn push_into_agrees_with_escape_for_every_valid_value() {
        let values = [
            "",
            "plain",
            "&<>\"'\t\n\r",
            "🐈 mixed \u{4e2d}\u{6587} &amp; tail",
            "trailing&",
            "&leading",
        ];
        for context in [Context::Text, Context::Attribute] {
            for value in values {
                let mut pushed = String::new();
                push_into(value, context, &mut pushed).expect("value is valid");
                assert_eq!(
                    pushed,
                    escape(value, context).expect("value is valid").as_ref(),
                    "context {context:?}, value {value:?}"
                );
            }
        }
    }

    #[test]
    fn normalization_sensitive_characters_are_references() {
        assert_eq!(
            escape("\t\n\r\r\n<&>\"'", Context::Text).unwrap(),
            "\t\n&#xD;&#xD;\n&lt;&amp;&gt;\"'"
        );
        assert_eq!(
            escape("\t\n\r<&>\"'", Context::Attribute).unwrap(),
            "&#x9;&#xA;&#xD;&lt;&amp;&gt;&quot;'"
        );
    }
}
