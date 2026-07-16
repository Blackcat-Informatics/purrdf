// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use super::ProjectionError;

/// Build a stable collision-resistant identifier from a caller-owned ASCII prefix
/// and arbitrary key bytes.
///
/// The full SHA-256 digest is retained, so the helper never depends on iteration
/// order, random seeds, process identity, time, or a truncation collision policy.
///
/// # Errors
///
/// Returns a configuration error unless `prefix` starts with an ASCII letter and
/// otherwise contains only ASCII alphanumerics or `_`.
pub fn stable_identifier(prefix: &str, key: &[u8]) -> Result<String, ProjectionError> {
    let mut chars = prefix.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(ProjectionError::configuration(
            "identifier prefix must start with an ASCII letter and contain only ASCII alphanumerics or `_`",
        ));
    }
    let digest = Sha256::digest(key);
    let mut output = String::with_capacity(prefix.len() + 1 + digest.len() * 2);
    output.push_str(prefix);
    output.push('_');
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    Ok(output)
}

/// Validate a mandatory absolute IRI configuration field.
///
/// # Errors
///
/// Returns a configuration error naming `field` when `value` is not an absolute IRI.
pub fn validate_absolute_iri(value: &str, field: &str) -> Result<(), ProjectionError> {
    purrdf_sparql_algebra::NamedNode::new(value.to_owned()).map_err(|error| {
        ProjectionError::configuration(format!("{field} must be an absolute IRI: {error}"))
    })?;
    Ok(())
}

/// Escape an openCypher backtick-delimited identifier body.
pub fn escape_cypher_identifier(value: &str) -> String {
    value.replace('`', "``")
}

/// Escape an openCypher single-quoted string body.
pub fn escape_cypher_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            control if control.is_control() => {
                let _ = write!(output, "\\u{:04x}", control as u32);
            }
            other => output.push(other),
        }
    }
    output
}

/// Escape XML 1.0 character-data text.
///
/// # Errors
///
/// Returns a term error when `value` contains a character forbidden by XML 1.0.
pub fn escape_xml_text(value: &str) -> Result<String, ProjectionError> {
    escape_xml(value, false)
}

/// Escape a double-quoted XML 1.0 attribute value.
///
/// # Errors
///
/// Returns a term error when `value` contains a character forbidden by XML 1.0.
pub fn escape_xml_attribute(value: &str) -> Result<String, ProjectionError> {
    escape_xml(value, true)
}

fn escape_xml(value: &str, attribute: bool) -> Result<String, ProjectionError> {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        let code = ch as u32;
        let valid = matches!(code, 0x9 | 0xa | 0xd)
            || (0x20..=0xd7ff).contains(&code)
            || (0xe000..=0xfffd).contains(&code)
            || (0x1_0000..=0x10_ffff).contains(&code);
        if !valid {
            return Err(ProjectionError::term(format!(
                "U+{code:04X} is not permitted in XML 1.0"
            )));
        }
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' if attribute => output.push_str("&quot;"),
            '\'' if attribute => output.push_str("&apos;"),
            other => output.push(other),
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_is_full_digest_and_repeatable() {
        let first = stable_identifier("node", b"http://example.org/a").expect("identifier");
        let second = stable_identifier("node", b"http://example.org/a").expect("identifier");
        assert_eq!(first, second);
        assert_eq!(first.len(), "node_".len() + 64);
        assert!(stable_identifier("bad-prefix", b"x").is_err());
    }

    #[test]
    fn cypher_escaping_is_injection_safe() {
        assert_eq!(escape_cypher_identifier("a`b"), "a``b");
        assert_eq!(escape_cypher_string("a'\\\nb"), "a\\'\\\\\\nb");
    }

    #[test]
    fn xml_text_and_attribute_escaping_are_distinct() {
        assert_eq!(escape_xml_text("<&>\"'").expect("text"), "&lt;&amp;&gt;\"'");
        assert_eq!(
            escape_xml_attribute("<&>\"'").expect("attribute"),
            "&lt;&amp;&gt;&quot;&apos;"
        );
        assert!(escape_xml_text("bad\0value").is_err());
    }

    #[test]
    fn absolute_iri_validation_fails_closed() {
        assert!(validate_absolute_iri("http://example.org/p", "predicate").is_ok());
        assert!(validate_absolute_iri("relative", "predicate").is_err());
    }
}
