// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The simple (non-numeric, non-temporal) XSD value spaces: `boolean` and `string`.
//!
//! `xsd:string`'s value space is its lexical space, so it has no dedicated parser
//! (the [`crate::parse`] entry maps it straight to [`crate::XsdValue::String`]).

use crate::datatype::XsdDatatype;
use crate::value::XsdError;

/// `xsd:boolean`: lexical space is `true | false | 1 | 0`; canonical is `true|false`.
pub fn parse_boolean(s: &str) -> Result<bool, XsdError> {
    match s {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(XsdError::InvalidLexical {
            datatype: XsdDatatype::Boolean,
            lexical: s.to_string(),
            reason: "expected one of: true, false, 1, 0",
        }),
    }
}

/// Apply the XSD `whiteSpace = replace` facet (the value space of `xsd:normalizedString`).
///
/// Per XSD 1.1 Part 2 §4.3.6, `replace` maps every occurrence of `#x9` (tab), `#xA`
/// (line feed), and `#xD` (carriage return) to a single `#x20` (space). No other
/// character — including other Unicode whitespace such as `U+00A0` — is touched, and
/// no collapsing or trimming is performed (that is the `collapse` facet's job).
///
/// The result has the same number of characters as the input.
#[must_use]
pub fn normalize_whitespace_replace(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

/// Apply the XSD `whiteSpace = collapse` facet (the value space of `xsd:token`).
///
/// Per XSD 1.1 Part 2 §4.3.6, `collapse` first performs the `replace` facet (each
/// `#x9`/`#xA`/`#xD` becomes `#x20`), then collapses every run of contiguous `#x20`
/// spaces to a single space and strips all leading and trailing spaces. Only the
/// four XSD whitespace characters participate; other Unicode whitespace is preserved
/// verbatim (it is not part of the XSD `whiteSpace` facet).
#[must_use]
pub fn normalize_whitespace_collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for ch in s.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r') {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_lexicals() {
        assert_eq!(parse_boolean("true"), Ok(true));
        assert_eq!(parse_boolean("1"), Ok(true));
        assert_eq!(parse_boolean("false"), Ok(false));
        assert_eq!(parse_boolean("0"), Ok(false));
        assert!(parse_boolean("TRUE").is_err());
        assert!(parse_boolean("yes").is_err());
        assert!(parse_boolean("").is_err());
    }

    // ── whiteSpace = replace (xsd:normalizedString) ───────────────────────────────

    #[test]
    fn replace_maps_each_xsd_whitespace_to_space() {
        assert_eq!(normalize_whitespace_replace("\two\nw"), " wo w");
        assert_eq!(
            normalize_whitespace_replace("hey\nthere\ta tab\rcarriage return"),
            "hey there a tab carriage return"
        );
    }

    #[test]
    fn replace_preserves_length_and_does_not_collapse_or_trim() {
        // Two consecutive whitespace characters become two spaces (no collapse);
        // leading/trailing whitespace becomes leading/trailing spaces (no trim).
        assert_eq!(
            normalize_whitespace_replace("\tBeing a Doctor Is\n\ta Full-Time Job\r"),
            " Being a Doctor Is  a Full-Time Job "
        );
    }

    #[test]
    fn replace_leaves_non_xsd_whitespace_untouched() {
        // U+00A0 (no-break space) is NOT an XSD whitespace character.
        assert_eq!(normalize_whitespace_replace("a\u{00A0}b"), "a\u{00A0}b");
        assert_eq!(normalize_whitespace_replace(""), "");
    }

    // ── whiteSpace = collapse (xsd:token) ─────────────────────────────────────────

    #[test]
    fn collapse_collapses_runs_and_strips_ends() {
        assert_eq!(
            normalize_whitespace_collapse("       hey\nthere      "),
            "hey there"
        );
        assert_eq!(
            normalize_whitespace_collapse("\tBeing a Doctor    Is\n\ta Full-Time Job\r"),
            "Being a Doctor Is a Full-Time Job"
        );
    }

    #[test]
    fn collapse_leading_trailing_interior() {
        assert_eq!(
            normalize_whitespace_collapse(
                "\n  hey -  white  space is collapsed for xsd:token       and preceding and trailing whitespace is stripped     "
            ),
            "hey - white space is collapsed for xsd:token and preceding and trailing whitespace is stripped"
        );
    }

    #[test]
    fn collapse_edge_cases() {
        assert_eq!(normalize_whitespace_collapse(""), "");
        assert_eq!(normalize_whitespace_collapse("   "), "");
        assert_eq!(normalize_whitespace_collapse("\t\n\r"), "");
        assert_eq!(normalize_whitespace_collapse("word"), "word");
        // Non-XSD whitespace is preserved (not a separator).
        assert_eq!(
            normalize_whitespace_collapse("  a\u{00A0}b  "),
            "a\u{00A0}b"
        );
    }
}
