// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resource bounds for recursively processed document structures.
//!
//! Quoted and reifying RDF terms use admitted heap continuations in the native
//! text parsers. Their structural depth does not consume this document guard.
//! Turtle/TriG property lists, collections and annotation blocks still recurse
//! while lowering to flat statements, so their independent syntactic descent is
//! refused before the next recursive frame exceeds the document policy.
//!
//! XML readers keep open elements on a heap stack and receive the same explicit
//! document-depth ceiling for the later format-specific tree processors. JSON-LD
//! separately supplies this document envelope to its JSON/context reader. These
//! are document resource policies, independent of native RDF term admissibility.

use purrdf_lex::xml::{Document, Dtd, Options, XmlError, XmlErrorKind};

use crate::RdfDiagnostic;

/// Maximum syntactic descent for remaining recursive document productions.
/// Quoted/reifying term continuations do not consume these levels.
pub(crate) const MAX_PARSE_NESTING_DEPTH: usize = 128;

/// Why [`parse_xml`] refused a document.
#[derive(Debug)]
pub(crate) enum XmlReadError {
    /// An element nests past [`MAX_PARSE_NESTING_DEPTH`]; the payload is the depth of the
    /// first element too deep (the limit plus one).
    TooDeep(usize),
    /// The document type declaration names an external subset or references an external
    /// parameter entity. Neither is fetched, so entity declarations and default attributes
    /// may be missing from what was read; a document is refused rather than read with
    /// declarations silently unapplied.
    DeclarationsUnread,
    /// Any other refusal of the XML reader.
    Malformed(XmlError),
}

impl XmlReadError {
    /// The message for [`XmlReadError::DeclarationsUnread`], one spelling for every reader.
    pub(crate) const UNREAD_MESSAGE: &'static str = "the document type declaration names an external DTD subset or references an external parameter entity, which is never fetched, so default attributes and entity declarations may be missing";
}

/// Read `text` as an XML document the way every XML codec in this crate does: element
/// nesting past [`MAX_PARSE_NESTING_DEPTH`] refused as [`XmlReadError::TooDeep`], and a
/// document type declaration read as XML 1.0 §5.1 requires of a non-validating processor
/// ([`Dtd::internal_subset`]). RDF/XML documents routinely declare internal entities such
/// as `&xsd;` for datatype IRIs, so those are declared and expanded under the reader's
/// expansion budget; internal parameter entities are read too. Nothing is ever fetched: a
/// reference to an external entity is refused, and so is a document whose external subset or
/// external parameter entity would have to be read for its default attributes and entity
/// declarations ([`XmlReadError::DeclarationsUnread`], from
/// [`Document::declarations_unread`]), so no document is read with declarations silently
/// unapplied. The GraphML and DataCite projections
/// refuse any DTD before calling this.
///
/// The error is not a diagnostic because the four callers report in three different error
/// vocabularies (`RdfDiagnostic` for the RDF/XML and TriX codecs, a `ProjectionError` for
/// the GraphML and DataCite projections); each keeps its own.
pub(crate) fn parse_xml(text: &str) -> Result<Document<'_>, XmlReadError> {
    let document = Document::parse_with_options(
        text,
        Options {
            max_depth: MAX_PARSE_NESTING_DEPTH,
            dtd: Dtd::internal_subset(),
        },
    )
    .map_err(|error| match error.kind() {
        XmlErrorKind::DepthLimit { limit } => XmlReadError::TooDeep(limit + 1),
        _ => XmlReadError::Malformed(error),
    })?;
    if document.declarations_unread() {
        return Err(XmlReadError::DeclarationsUnread);
    }
    Ok(document)
}

/// Report the located refusal of the next recursive document production through
/// its original caller storage/progress authority.
pub(crate) fn nesting_too_deep_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    line: u64,
    column: u32,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<RdfDiagnostic, purrdf_lex::allocation::StorageError> {
    let mut diagnostic = RdfDiagnostic::try_error_with_memory(
        "native-codec-parse",
        &format_args!("term nesting exceeds the parser limit of {MAX_PARSE_NESTING_DEPTH} levels"),
        memory,
    )?;
    diagnostic.location = Some(memory.boxed(crate::RdfLocation {
        line: Some(line),
        column: Some(column),
        ..crate::RdfLocation::default()
    })?);
    Ok(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::{MAX_PARSE_NESTING_DEPTH, XmlReadError, parse_xml};

    /// `depth` nested `<a>` elements around a leaf.
    fn nested(depth: usize) -> String {
        format!(
            "<r>{}<leaf/>{}</r>",
            "<a>".repeat(depth),
            "</a>".repeat(depth)
        )
    }

    /// The limit is a limit ON the depth: at the limit reads, one over is refused. The `<r>`
    /// wrapper is itself a level and so is the self-closing `<leaf/>`, so `MAX - 2` inner
    /// elements put the leaf exactly at the limit.
    #[test]
    fn the_limit_is_exact() {
        assert!(parse_xml(&nested(MAX_PARSE_NESTING_DEPTH - 2)).is_ok());
        assert!(matches!(
            parse_xml(&nested(MAX_PARSE_NESTING_DEPTH - 1)),
            Err(XmlReadError::TooDeep(depth)) if depth == MAX_PARSE_NESTING_DEPTH + 1
        ));
    }

    /// SIBLINGS ARE NOT NESTING: a flat document of ten thousand elements reads.
    #[test]
    fn a_wide_document_is_not_a_deep_one() {
        let wide = format!("<r>{}</r>", "<a>x</a>".repeat(10_000));
        assert!(parse_xml(&wide).is_ok());
    }

    /// Any other refusal is the reader's own error, not a depth.
    #[test]
    fn a_malformed_document_is_not_a_depth_refusal() {
        assert!(matches!(parse_xml("<a>"), Err(XmlReadError::Malformed(_))));
    }

    /// An internal entity is declared and expanded; a reference to an external one is
    /// refused, so no document can make the reader fetch.
    #[test]
    fn an_internal_entity_expands_and_an_external_one_is_refused() {
        let internal =
            parse_xml(r#"<!DOCTYPE r [<!ENTITY e "http://example.org/">]><r a="&e;x"/>"#)
                .expect("an internal entity is read");
        assert_eq!(
            internal.root_element().attribute("a"),
            Some("http://example.org/x")
        );
        assert!(matches!(
            parse_xml(r#"<!DOCTYPE r [<!ENTITY e SYSTEM "http://example.org/e">]><r>&e;</r>"#),
            Err(XmlReadError::Malformed(_))
        ));
        // An external subset is named, never fetched: the document is refused rather than
        // read with its declarations unapplied. The internal-only neighbour is read.
        assert!(matches!(
            parse_xml(r#"<!DOCTYPE r SYSTEM "http://example.org/r.dtd"><r/>"#),
            Err(XmlReadError::DeclarationsUnread)
        ));
        assert!(matches!(
            parse_xml(r#"<!DOCTYPE r [<!ENTITY % x SYSTEM "x.ent">%x;]><r/>"#),
            Err(XmlReadError::DeclarationsUnread)
        ));
        assert!(
            parse_xml(r#"<!DOCTYPE r [<!ENTITY % p "<!ENTITY e 'v'>">%p;]><r>&e;</r>"#).is_ok()
        );
        assert!(matches!(
            parse_xml(r#"<!DOCTYPE r SYSTEM "http://example.org/r.dtd"><r>&e;</r>"#),
            Err(XmlReadError::Malformed(_))
        ));
    }
}
