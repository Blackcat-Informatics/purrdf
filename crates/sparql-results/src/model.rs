// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The result-provenance carrier — the "maximal information flow" extension that
//! travels alongside a SPARQL result.
//!
//! When [`ResultProvenance`] is empty (the common case today) the serializers
//! emit pure-W3C output. When populated, the JSON/XML writers append an
//! additive `purrdf` extension block; the CSV/TSV writers cannot carry it and
//! flag the drop via `SerializeOutcome::provenance_dropped`.
//!
//! Honesty note: population of this structure is **incremental**. The fields are
//! typed and threaded through the serializer surface now, but the evaluator and
//! the S11 derivation graph fill them in progressively — most results
//! today carry an empty value.

/// Result-level provenance carried alongside a SPARQL result. Default is empty →
/// pure-W3C serialization. Populated → additive `purrdf` extension in JSON/XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultProvenance {
    /// Optional opaque query identity (e.g. a content hash of the source query).
    pub query_hash: Option<String>,
    /// Optional engine/producer label.
    pub engine: Option<String>,
    /// Per-solution provenance, index-aligned with `SparqlResult::Solutions.rows`
    /// when present. Empty = none carried (the common case today).
    pub solutions: Vec<SolutionProvenance>,
}

impl ResultProvenance {
    /// True when no provenance is carried: no query hash, no engine label, and no
    /// per-solution entries. Serializers use this to decide whether to append the
    /// additive `purrdf` extension at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.query_hash.is_none() && self.engine.is_none() && self.solutions.is_empty()
    }
}

/// Per-solution provenance hook. Typed-but-mostly-empty today; populated as the
/// evaluator / S11 derivation graph begins producing it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolutionProvenance {
    /// Source references (e.g. named-graph / quad IRIs) that produced this solution.
    pub sources: Vec<String>,
}

/// Caller-supplied namespace configuration for the additive provenance
/// extension tree (`queryForm`/`queryHash`/`engine`/`solution`/`source`).
///
/// PurRDF mints no vocabulary IRIs of its own (see AGENTS.md's "PurRDF is NOT
/// an ontology" contract), so the provenance tree has no built-in namespace:
/// the JSON/XML writers emit it only when a caller supplies one, and under
/// exactly the identifiers the caller supplies — nothing is fabricated on
/// their behalf. When `namespace` is `None`, [`crate::to_json`]/[`crate::to_xml`]
/// emit no provenance element/member at all, however populated a
/// [`ResultProvenance`] is (and set [`crate::SerializeOutcome::provenance_dropped`]
/// to signal that the data was present but had nowhere to go, exactly as
/// CSV/TSV already do for any non-empty provenance).
///
/// # Why this is validated at construction, not at write time
///
/// `prefix` is spliced VERBATIM into XML element and attribute names by this
/// crate's `xml` writer (`<{prefix}:provenance>`, `xmlns:{prefix}="…"`,
/// `<{prefix}:queryForm>`, …) — it is markup syntax, not text content, so it
/// cannot be entity-escaped the way `iri` and every other dynamic string in
/// this crate are. The only way to keep the XML writer from ever being asked
/// to emit a document it cannot itself read back is to make an
/// [`Error::InvalidNamespace`](crate::error::Error::InvalidNamespace) `prefix`
/// unconstructible — fields are therefore private and [`ProvenanceNamespace::new`]
/// is the only way to build one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceNamespace {
    /// The identifier the provenance tree is anchored under: the bare
    /// top-level JSON member key, and the XML namespace prefix (bound via
    /// `xmlns:{prefix}="{iri}"` on the `<{prefix}:provenance>` element).
    prefix: String,
    /// The XML namespace IRI bound to `prefix`. The JSON writer has no
    /// namespace mechanism to bind it into and uses only `prefix`.
    iri: String,
}

/// `true` for a Unicode scalar value in the XML 1.0 (Fifth Edition)
/// `NameStartChar` production, EXCLUDING `:` — i.e. the XML Namespaces
/// `NCNameStartChar` character class
/// (<https://www.w3.org/TR/xml-names/#NT-NCName> defines `NCName` as the XML
/// `Name` production with every `:` removed;
/// <https://www.w3.org/TR/xml/#NT-NameStartChar> is the source `NameStartChar`
/// production this restricts).
const fn is_ncname_start_char(c: char) -> bool {
    matches!(c,
        'A'..='Z'
        | '_'
        | 'a'..='z'
        | '\u{C0}'..='\u{D6}'
        | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}'
        | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}'
    )
}

/// `true` for a Unicode scalar value in the XML 1.0 `NameChar` production,
/// EXCLUDING `:` — the XML Namespaces `NCNameChar` character class (every
/// `NCNameStartChar` plus the additional non-leading `NameChar` extras:
/// `-`, `.`, digits, the middle dot, and two combining-mark ranges).
const fn is_ncname_char(c: char) -> bool {
    is_ncname_start_char(c)
        || matches!(c,
            '-'
            | '.'
            | '0'..='9'
            | '\u{B7}'
            | '\u{0300}'..='\u{036F}'
            | '\u{203F}'..='\u{2040}'
        )
}

/// `true` iff `s` is a valid XML Namespaces `NCName`
/// (<https://www.w3.org/TR/xml-names/#NT-NCName>): non-empty, its first
/// character is an `NCNameStartChar`, and every subsequent character is an
/// `NCNameChar`. Notably this rejects `:` anywhere in `s` (the entire point of
/// the "NC" — "no colon" — restriction), which is also what keeps a prefix
/// from being confused with a full `QName`.
fn is_valid_ncname(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if is_ncname_start_char(c) => {}
        _ => return false,
    }
    chars.all(is_ncname_char)
}

/// `true` iff `prefix` is one of the two prefixes the XML Namespaces
/// specification reserves outright: `xml` (permanently bound to
/// `http://www.w3.org/XML/1998/namespace`, and which "MUST NOT be … bound to
/// any other namespace name") and `xmlns` (permanently bound to
/// `http://www.w3.org/2000/xmlns/`, and which "MUST NOT be declared") — see
/// <https://www.w3.org/TR/xml-names/#ns-decl>.
///
/// The comparison is ASCII case-insensitive rather than an exact-lowercase
/// match: XML 1.0 §2.3 separately reserves EVERY case combination of names
/// beginning with the letters `xml` "for standardization in this or future
/// versions of this specification" (<https://www.w3.org/TR/xml/#NT-Name>:
/// "Names beginning with the string 'xml', or with any string which would
/// match `(('X'|'x')('M'|'m')('L'|'l'))`, are reserved"), so accepting
/// `XML`/`Xmlns`/`XMLNS`/etc. as a caller prefix would defeat the same
/// reservation the lowercase spellings exist to protect.
fn is_reserved_prefix(prefix: &str) -> bool {
    prefix.eq_ignore_ascii_case("xml") || prefix.eq_ignore_ascii_case("xmlns")
}

impl ProvenanceNamespace {
    /// Validate and construct a caller-supplied provenance namespace.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::InvalidNamespace`] when:
    ///
    /// - `prefix` is not a valid XML Namespaces `NCName` (see
    ///   `is_valid_ncname`) — this is the exact grammar the XML writer's
    ///   `xmlns:{prefix}="…"` declaration and `<{prefix}:…>` element/attribute
    ///   names require in order to be well-formed XML. This is also what makes
    ///   markup injection via `prefix` (e.g. a value containing `"`, `<`, `>`,
    ///   or whitespace, which would otherwise splice out of the element/
    ///   attribute it is written into) unconstructible rather than merely
    ///   escaped — `prefix` is markup syntax, not text content, so there is no
    ///   escaping transform that would make an arbitrary string safe there;
    /// - `prefix` case-insensitively equals `xml` or `xmlns` (see
    ///   `is_reserved_prefix`) — names the XML Namespaces specification
    ///   forbids a document from (re)declaring;
    /// - `iri` does not parse as a syntactically valid, ABSOLUTE RFC-3987 IRI
    ///   (validated via [`purrdf_iri::parse`], the repo's one IRI-syntax
    ///   authority — see `crates/iri`). Absolute is required — not merely "a
    ///   valid IRI reference" — for the same reason every other IRI that
    ///   anchors a vocabulary/namespace in this workspace requires it (e.g.
    ///   `purrdf_sparql_algebra::ast::NamedNode::new`, which rejects a
    ///   relative reference in term position): a namespace name with no
    ///   scheme has no fixed meaning once a document is combined with others,
    ///   and `xmlns:{prefix}="{iri}"` has no defined behavior for a relative
    ///   `iri` in the XML Namespaces spec.
    pub fn new(
        prefix: impl Into<String>,
        iri: impl Into<String>,
    ) -> Result<Self, crate::error::Error> {
        let prefix = prefix.into();
        let iri = iri.into();
        if !is_valid_ncname(&prefix) {
            return Err(crate::error::Error::InvalidNamespace(format!(
                "namespace prefix {prefix:?} is not a valid XML NCName"
            )));
        }
        if is_reserved_prefix(&prefix) {
            return Err(crate::error::Error::InvalidNamespace(format!(
                "namespace prefix {prefix:?} is reserved (xml/xmlns, case-insensitive)"
            )));
        }
        let parsed = purrdf_iri::parse(&iri).map_err(|e| {
            crate::error::Error::InvalidNamespace(format!(
                "namespace IRI {iri:?} is not a valid IRI: {e}"
            ))
        })?;
        if !parsed.has_scheme() {
            return Err(crate::error::Error::InvalidNamespace(format!(
                "namespace IRI {iri:?} is a relative IRI reference (no scheme); \
                 a namespace IRI must be absolute"
            )));
        }
        Ok(Self { prefix, iri })
    }

    /// The validated namespace prefix — a well-formed XML NCName that is
    /// neither `xml` nor `xmlns` (checked case-insensitively).
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The validated, absolute namespace IRI.
    #[must_use]
    pub fn iri(&self) -> &str {
        &self.iri
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn default_provenance_is_empty() {
        assert!(ResultProvenance::default().is_empty());
    }

    #[test]
    fn query_hash_makes_it_non_empty() {
        let prov = ResultProvenance {
            query_hash: Some("abc".to_string()),
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }

    #[test]
    fn engine_makes_it_non_empty() {
        let prov = ResultProvenance {
            engine: Some("purrdf-sparql-eval".to_string()),
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }

    #[test]
    fn solutions_make_it_non_empty() {
        let prov = ResultProvenance {
            solutions: vec![SolutionProvenance::default()],
            ..Default::default()
        };
        assert!(!prov.is_empty());
    }

    // ---- ProvenanceNamespace::new — accepted shapes -----------------------

    #[test]
    fn valid_simple_prefix_constructs() {
        let ns = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("simple ASCII prefix + absolute IRI must construct");
        assert_eq!(ns.prefix(), "prov");
        assert_eq!(ns.iri(), "http://example.org/ns/prov#");
    }

    #[test]
    fn valid_prefix_with_non_leading_digit_hyphen_dot_underscore() {
        // Digits, `-`, `.` are legal NCNameChars everywhere but the first
        // position; `_` is legal even as the first character.
        let ns = ProvenanceNamespace::new("_prov-1.thing_2", "http://example.org/ns/prov#")
            .expect("digits/hyphen/dot/underscore after the first char must construct");
        assert_eq!(ns.prefix(), "_prov-1.thing_2");
    }

    #[test]
    fn valid_prefix_starting_with_underscore() {
        ProvenanceNamespace::new("_prov", "http://example.org/ns/prov#")
            .expect("a leading underscore is a valid NCNameStartChar");
    }

    // ---- ProvenanceNamespace::new — rejected prefix shapes -----------------

    #[test]
    fn empty_prefix_is_rejected() {
        let err = ProvenanceNamespace::new("", "http://example.org/ns/prov#")
            .expect_err("empty prefix must be rejected");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_starting_with_digit_is_rejected() {
        let err = ProvenanceNamespace::new("1prov", "http://example.org/ns/prov#")
            .expect_err("a leading digit is not a valid NCNameStartChar");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_containing_colon_is_rejected() {
        let err = ProvenanceNamespace::new("prov:x", "http://example.org/ns/prov#")
            .expect_err("a colon makes it a QName, not an NCName");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_containing_whitespace_is_rejected() {
        let err = ProvenanceNamespace::new("pro v", "http://example.org/ns/prov#")
            .expect_err("whitespace is not an NCNameChar");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_containing_double_quote_is_rejected() {
        let err = ProvenanceNamespace::new("prov\"x", "http://example.org/ns/prov#")
            .expect_err("a double quote is not an NCNameChar");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_containing_greater_than_is_rejected() {
        let err = ProvenanceNamespace::new("prov>x", "http://example.org/ns/prov#")
            .expect_err("`>` is not an NCNameChar");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_containing_less_than_is_rejected() {
        let err = ProvenanceNamespace::new("prov<x", "http://example.org/ns/prov#")
            .expect_err("`<` is not an NCNameChar");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_xml_lowercase_is_rejected() {
        let err = ProvenanceNamespace::new("xml", "http://example.org/ns/prov#")
            .expect_err("`xml` is reserved");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_xmlns_lowercase_is_rejected() {
        let err = ProvenanceNamespace::new("xmlns", "http://example.org/ns/prov#")
            .expect_err("`xmlns` is reserved");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn prefix_xml_any_case_combination_is_rejected() {
        for candidate in ["XML", "Xml", "xML", "XmL", "xmL"] {
            let result = ProvenanceNamespace::new(candidate, "http://example.org/ns/prov#");
            assert!(
                matches!(result, Err(Error::InvalidNamespace(_))),
                "expected rejection for {candidate:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn prefix_xmlns_any_case_combination_is_rejected() {
        for candidate in ["XMLNS", "Xmlns", "xMLns", "XmlNS"] {
            let result = ProvenanceNamespace::new(candidate, "http://example.org/ns/prov#");
            assert!(
                matches!(result, Err(Error::InvalidNamespace(_))),
                "expected rejection for {candidate:?}, got {result:?}"
            );
        }
    }

    // ---- ProvenanceNamespace::new — rejected IRI shapes ---------------------

    #[test]
    fn relative_iri_is_rejected() {
        let err = ProvenanceNamespace::new("prov", "/ns/prov#")
            .expect_err("a relative IRI reference must be rejected");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn syntactically_malformed_iri_is_rejected() {
        let err = ProvenanceNamespace::new("prov", "http://example.org/<bad>")
            .expect_err("a syntactically invalid IRI must be rejected");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn empty_iri_is_rejected() {
        let err = ProvenanceNamespace::new("prov", "").expect_err("an empty IRI must be rejected");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    // ---- Injection regression -----------------------------------------------

    /// The exact class of attack the constructor exists to make
    /// unconstructible: a prefix engineered to close the `xmlns:{prefix}="…"`
    /// attribute and the `<{prefix}:provenance …>` start tag early, splicing
    /// an attacker-controlled attribute/element into the document. Every
    /// character this string needs (`"`, `<`, `>`, space) is individually
    /// disallowed in an NCName, so the whole crafted prefix is rejected as one
    /// `NCName` violation — there is no escaping path that would make this
    /// string safe to splice into markup, so construction must fail outright.
    #[test]
    fn xml_breakout_crafted_prefix_is_rejected() {
        let crafted = "prov\"><evil xmlns:x=\"http://evil.example/";
        let err = ProvenanceNamespace::new(crafted, "http://example.org/ns/prov#")
            .expect_err("a markup-breakout prefix must never construct");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }

    #[test]
    fn xml_breakout_crafted_prefix_closing_element_is_rejected() {
        let crafted = "prov></prov:provenance><script>alert(1)</script";
        let err = ProvenanceNamespace::new(crafted, "http://example.org/ns/prov#")
            .expect_err("a markup-breakout prefix must never construct");
        assert!(matches!(err, Error::InvalidNamespace(_)), "{err:?}");
    }
}
