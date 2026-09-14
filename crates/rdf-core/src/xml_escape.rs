// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lossless XML 1.0 egress for character data and double-quoted attributes.
//!
//! XML 1.0 §2.11 normalizes raw carriage returns; §3.3.3 also normalizes
//! attribute tabs and line feeds. Character references preserve these scalars.
//! A scalar outside the `Char` production has no XML spelling and is refused.

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

/// Append an escaped value to an existing output buffer.
///
/// # Errors
/// Returns the first forbidden scalar. The output is unchanged on failure.
pub fn push(value: &str, context: Context, output: &mut String) -> Result<(), InvalidXmlChar> {
    let original_length = output.len();
    let mut start = 0;
    for (offset, character) in value.char_indices() {
        if !purrdf_iri::terminals::is_xml_char(character) {
            output.truncate(original_length);
            return Err(InvalidXmlChar {
                byte_offset: offset,
                character,
            });
        }
        if let Some(reference) = replacement(character, context) {
            output.push_str(&value[start..offset]);
            output.push_str(reference);
            start = offset + character.len_utf8();
        }
    }
    output.push_str(&value[start..]);
    Ok(())
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
