// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Write as _;
use std::io;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{ProjectionError, ProjectionLimits};

struct LimitedJsonBytes {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl LimitedJsonBytes {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }
}

impl io::Write for LimitedJsonBytes {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(buffer.len())
            .is_none_or(|length| length > self.limit)
        {
            self.exceeded = true;
            return Err(io::Error::other("projection JSON byte limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn canonical_json_bounded<T: Serialize>(
    value: &T,
    limits: ProjectionLimits,
    description: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let mut output = LimitedJsonBytes::new(limits.max_artifact_bytes());
    if let Err(error) = serde_json::to_writer(&mut output, value) {
        if output.exceeded {
            return Err(ProjectionError::limit(format!(
                "{description} exceeds the {}-byte artifact limit",
                limits.max_artifact_bytes()
            )));
        }
        return Err(ProjectionError::syntax(format!(
            "serialize {description}: {error}"
        )));
    }
    Ok(output.bytes)
}

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
/// This is the single gate every IRI-valued projection configuration field passes through —
/// dataset identities, generated-resource bases, scheme IRIs, entity bases, document bases,
/// graph names, vocabulary role tables, predicates and datatypes alike — so what "absolute
/// IRI" means in a projection configuration is decided once rather than per profile.
///
/// It is [`purrdf_iri::BaseIri::parse`], the workspace's shared "a valid RFC-3987 IRI that
/// has a scheme" primitive, and the failure carries
/// [`purrdf_iri::IriError::diagnostic_code`] — the workspace's single owner of those
/// spellings. It used to be `purrdf_sparql_algebra::NamedNode::new`, which reaches the same
/// grammar but wraps it in a private reason of its own ("relative IRI reference in term
/// position"), so a configuration field failed with a SPARQL term-position sentence and no
/// shared code for a consumer to switch on.
///
/// The relative case gets a remedy written for the surface the value came from. The shared
/// `iri-non-absolute-base` remedy names a BASE ("supply a base IRI that has a scheme"), and
/// most fields checked here are not bases; and a configuration document is not an RDF
/// document, so the `@base`/`xml:base` remedy could not be applied to it either. Naming a fix
/// the caller cannot apply is worse than naming none.
///
/// # Errors
///
/// Returns a configuration error naming `field` when `value` is not an absolute IRI.
pub fn validate_absolute_iri(value: &str, field: &str) -> Result<(), ProjectionError> {
    let Err(error) = purrdf_iri::BaseIri::parse(value) else {
        return Ok(());
    };
    let code = error.diagnostic_code();
    if code == "iri-non-absolute-base" {
        return Err(ProjectionError::configuration(format!(
            "{field} must be an absolute IRI: {code}: `{value}` is a relative IRI reference. A \
             projection configuration is not an RDF document and has no base of its own, so \
             there is nothing to resolve it against: write the value in absolute form, with a \
             scheme"
        )));
    }
    Err(ProjectionError::configuration(format!(
        "{field} must be an absolute IRI: {code}: {error}"
    )))
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

    /// Every failure names the FIELD and carries the shared `purrdf_iri` diagnostic code, so
    /// a projection configuration failure groups with every other IRI failure in the
    /// workspace instead of spelling a private reason of its own.
    #[test]
    fn absolute_iri_validation_fails_closed() {
        // A fragment is part of an absolute IRI and must survive: nearly every vocabulary
        // role in these profiles is a `…#term`.
        assert!(validate_absolute_iri("http://example.org/p", "predicate").is_ok());
        assert!(
            validate_absolute_iri(
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                "predicate"
            )
            .is_ok()
        );

        // RELATIVE: the shared code, the field, the value, and a remedy that fits a
        // configuration document rather than an RDF one.
        let relative = validate_absolute_iri("relative", "predicate").expect_err("relative");
        let text = relative.to_string();
        assert!(text.contains("iri-non-absolute-base"), "{text}");
        assert!(
            text.contains("predicate") && text.contains("`relative`"),
            "{text}"
        );
        assert!(text.contains("write the value in absolute form"), "{text}");
        assert!(
            !text.contains("@base") && !text.contains("term position"),
            "no remedy the caller cannot apply, and no SPARQL term-position sentence: {text}"
        );

        // MALFORMED: the specific shared code for what is wrong with it.
        let malformed = validate_absolute_iri("ht tp://example.org/p", "predicate")
            .expect_err("malformed scheme");
        assert!(
            malformed.to_string().contains("iri-bad-scheme"),
            "{malformed}"
        );

        // The EMPTY string gets its own shared code rather than being folded into the
        // relative case: `iri-empty` says which of the two a caller actually wrote.
        let empty = validate_absolute_iri("", "predicate").expect_err("empty");
        assert!(empty.to_string().contains("iri-empty"), "{empty}");
    }
}
