// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How deeply an input document may nest, and the two ways that is enforced.
//!
//! # A parser without a nesting bound is not a parser, it is a crash
//!
//! Every document grammar this crate reads nests without limit on paper — a quoted triple
//! term inside a quoted triple term, a blank-node property list inside a collection inside
//! an annotation block, an element inside an element — and every parser reads that nesting
//! with recursion. An input is therefore an instruction about how much stack to consume, and
//! a stack overflow is not an error: nothing unwinds, no [`RdfDiagnostic`] reaches the
//! caller, `catch_unwind` does not see it, and a host process that embedded this library
//! dies with it. Twenty thousand levels of `<<( … )>>` in one N-Triples line, or twenty
//! thousand nested `rdf:Description` elements, aborted the `purrdf` binary with `SIGABRT`.
//!
//! So the depth is REFUSED, with an ordinary located diagnostic, like any other malformed
//! input — and the refusal happens where the recursion starts rather than after it.
//!
//! # Two enforcement points, because there are two readers
//!
//! * The first-party text parsers (N-Triples, N-Quads, Turtle, TriG) count their own
//!   descent. See `native_codecs::text_parse`.
//! * Every XML document (RDF/XML, TriX, GraphML, DataCite) is read by [`parse_xml`], which
//!   hands [`MAX_PARSE_NESTING_DEPTH`] to the workspace's one XML reader as its
//!   [`Options::max_depth`]. That reader keeps its open elements on a heap stack, so no
//!   input can exhaust the machine stack while it reads; the cap exists for the walks over
//!   the tree it returns, which recurse once per element. Those walks therefore need no
//!   depth counter of their own: the tree they descend is bounded by the call that built
//!   it.
//!
//! JSON-LD needs neither: `serde_json` enforces its own 128-deep recursion limit and returns
//! it as an error. TriX, HexTuples and the OKF binary reader do not nest.

use purrdf_lex::xml::{Document, Dtd, Options, XmlError, XmlErrorKind};

use crate::RdfDiagnostic;

/// The deepest nesting any document parser in this crate descends into.
///
/// # Why 128, and deliberately not the IR's 16
///
/// [`RdfDatasetBuilder::freeze`](purrdf_core::RdfDatasetBuilder::freeze) already refuses a
/// TRIPLE-TERM nesting deeper than 16, and the GTS transport agrees on the same cliff — so
/// for triple terms the stack's real ceiling is far below anything a parse-time bound needs
/// to police, and 128 can never reject a triple term the IR would have held.
///
/// 16 would nonetheless be a bound on the wrong thing, because most nesting a parser
/// descends never reaches the IR AS nesting. Turtle's `[ … ]`, `( … )` and `{| … |}` and
/// RDF/XML's element striping all lower to FLAT statements, so nothing downstream bounds
/// them and a 16 there would refuse documents the rest of the stack round-trips happily.
///
/// What does bound them is the thread's stack, and the smallest one PurRDF supports is
/// wasm32's 1 MiB. Measured there (`ulimit -s 1024`, one construct per document, release and
/// debug builds of the shipped CLI), the abort thresholds were: nested `rdf:Description`
/// 438, Turtle `[ … ]` 475, Turtle `<< … >>` 610, Turtle `<<( … )>>` 642, Turtle `{| … |}`
/// 778, N-Triples `<<( … )>>` 951, Turtle `( … )` 1049. 128 sits at least 3.4× below the
/// worst of those, and the most expensive construct per level (`[ … ]`, which descends
/// through both `DocParser::term` and `DocParser::predicate_object_list`) spends two of
/// these levels per syntactic one, so it stops at 64 real levels — 7.4× below its own cliff.
///
/// 128 is also the envelope this crate's structured lanes already publish: a JSON-LD
/// document is refused past `serde_json`'s 128-deep recursion limit and a JSON-LD context
/// document past `MAX_JSON_LD_DOCUMENT_DEPTH`, both 128. One number for the whole surface.
pub(crate) const MAX_PARSE_NESTING_DEPTH: usize = 128;

/// Why [`parse_xml`] refused a document.
#[derive(Debug)]
pub(crate) enum XmlReadError {
    /// An element nests past [`MAX_PARSE_NESTING_DEPTH`]; the payload is the depth of the
    /// first element too deep (the limit plus one).
    TooDeep(usize),
    /// Any other refusal of the XML reader.
    Malformed(XmlError),
}

/// Read `text` as an XML document the way every XML codec in this crate does: element
/// nesting past [`MAX_PARSE_NESTING_DEPTH`] refused as [`XmlReadError::TooDeep`], and a
/// document type declaration refused outright (no reader here expands an entity).
///
/// The error is not a diagnostic because the four callers report in three different error
/// vocabularies (`RdfDiagnostic` for the RDF/XML and TriX codecs, a `ProjectionError` for
/// the GraphML and DataCite projections); each keeps its own.
pub(crate) fn parse_xml(text: &str) -> Result<Document<'_>, XmlReadError> {
    Document::parse_with_options(
        text,
        Options {
            max_depth: MAX_PARSE_NESTING_DEPTH,
            dtd: Dtd::Refuse,
        },
    )
    .map_err(|error| match error.kind() {
        XmlErrorKind::DepthLimit { limit } => XmlReadError::TooDeep(limit + 1),
        _ => XmlReadError::Malformed(error),
    })
}

/// The diagnostic every first-party text codec returns for an input nested past
/// [`MAX_PARSE_NESTING_DEPTH`], located at the token that would have opened the level too
/// many. One spelling, so a caller matches one message whichever grammar produced it.
pub(crate) fn nesting_too_deep(line: u32, column: u32) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "native-codec-parse",
        format!("term nesting exceeds the parser limit of {MAX_PARSE_NESTING_DEPTH} levels"),
    )
    .with_location(crate::RdfLocation {
        line: Some(line),
        column: Some(column),
        ..crate::RdfLocation::default()
    })
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
        assert!(matches!(
            parse_xml("<!DOCTYPE r><r/>"),
            Err(XmlReadError::Malformed(_))
        ));
    }
}
