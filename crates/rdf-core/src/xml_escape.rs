// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lossless XML 1.0 egress for character data and double-quoted attributes.
//!
//! XML 1.0 §2.11 normalizes raw carriage returns; §3.3.3 also normalizes
//! attribute tabs and line feeds. Character references preserve these scalars.
//! A scalar outside the `Char` production has no XML spelling and is refused.

use core::fmt;
use std::borrow::Cow;

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

/// Escape a value, borrowing it when no scalar needs an XML reference.
///
/// All scalars are validated, including the non-ASCII clean path.
///
/// # Errors
/// Returns the first scalar excluded by XML 1.0 §2.2 production `[2]`.
pub fn escape(value: &str, context: Context) -> Result<Cow<'_, str>, InvalidXmlChar> {
    let mut output: Option<String> = None;
    let mut start = 0;
    for (offset, character) in value.char_indices() {
        if !purrdf_iri::terminals::is_xml_char(character) {
            return Err(InvalidXmlChar {
                byte_offset: offset,
                character,
            });
        }
        if let Some(reference) = replacement(character, context) {
            let out = output.get_or_insert_with(|| String::with_capacity(value.len()));
            out.push_str(&value[start..offset]);
            out.push_str(reference);
            start = offset + character.len_utf8();
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

/// Append an escaped value to any text sink.
///
/// Validity is decided in a FIRST pass, before a single byte is emitted, so a
/// rejected value leaves `output` untouched **without the rewind** the `String`
/// form used to perform. That matters because the sinks this feeds may already
/// have drained earlier bytes downstream, where `truncate` is not available and
/// never will be.
///
/// The cost is that a valid value is scanned twice. It is paid back at the call
/// sites that previously reached for [`escape`], which allocates a whole `String`
/// whenever any scalar needs a reference; this allocates nothing.
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
    for (offset, character) in value.char_indices() {
        if !purrdf_iri::terminals::is_xml_char(character) {
            return Err(InvalidXmlChar {
                byte_offset: offset,
                character,
            });
        }
    }
    let mut start = 0;
    for (offset, character) in value.char_indices() {
        if let Some(reference) = replacement(character, context) {
            // The sink owns its own failure channel; see the contract above.
            let _ = output.write_str(&value[start..offset]);
            let _ = output.write_str(reference);
            start = offset + character.len_utf8();
        }
    }
    let _ = output.write_str(&value[start..]);
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
