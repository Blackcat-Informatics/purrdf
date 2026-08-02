// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! # Two enforcement points, because there are two recursions
//!
//! * The first-party text parsers (N-Triples, N-Quads, Turtle, TriG) count their own
//!   descent. See `native_codecs::text_parse`.
//! * The XML front end cannot: `roxmltree`'s tokenizer recurses `parse_content` ⇄
//!   `parse_element`, one frame pair per element, and it overflows before any first-party
//!   code sees a tree. It exposes a node-COUNT ceiling and no depth one, and a node count is
//!   not a substitute (it would refuse a wide document to bound a deep one). The only place
//!   left to stand is in front of it, so [`guard_xml_nesting`] measures the element nesting
//!   of the source text and every `Document::parse` in this crate is preceded by it.
//!
//! [`guard_xml_nesting`] is what bounds the first-party XML walks too. `roxmltree` builds a
//! flat arena, so the only thing that can make an XML walk deep is element nesting, and the
//! guard sits immediately before the `Document::parse` whose tree that walk consumes. There
//! is deliberately no second, unreachable depth counter inside those walks: a guard that
//! cannot fire tells a reader the opposite of the truth about where the bound lives.
//!
//! JSON-LD needs neither: `serde_json` enforces its own 128-deep recursion limit and returns
//! it as an error. TriX, HexTuples and the OKF binary reader do not nest.

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

/// Refuse `text` if its XML element nesting is deeper than [`MAX_PARSE_NESTING_DEPTH`],
/// reporting the depth at which it first exceeded the limit.
///
/// The error type is the depth rather than a diagnostic because the four callers report in
/// three different error vocabularies (`RdfDiagnostic` for the RDF/XML and TriX codecs, a
/// `ProjectionError` for the GraphML and DataCite projections); each keeps its own.
///
/// # What is counted
///
/// One level per element that is not self-closing — exactly the nesting `roxmltree`'s
/// `parse_element` ⇄ `parse_content` pair recurses on, and exactly the nesting a first-party
/// walk over the resulting tree descends. Comments, CDATA sections, processing instructions
/// and `<!…>` declarations are skipped, and a start tag's attribute VALUES are skipped as
/// quoted runs so that a `>` or `/>` written inside one is not mistaken for markup.
///
/// A malformed document (an unterminated tag, more end tags than start tags) is not this
/// function's business: the count saturates rather than panicking and the XML parser behind
/// it reports the real syntax error.
pub(crate) fn guard_xml_nesting(text: &str) -> Result<(), usize> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let mut depth = 0usize;
    while at < bytes.len() {
        let Some(offset) = memchr::memchr(b'<', &bytes[at..]) else {
            break;
        };
        at += offset;
        match bytes.get(at + 1) {
            // A lone trailing `<`: malformed, and the XML parser will say so.
            None => break,
            Some(b'/') => {
                depth = depth.saturating_sub(1);
                at = end_of_markup(bytes, at + 2).0;
            }
            Some(b'?') => at = skip_past(bytes, at + 2, b"?>"),
            Some(b'!') => {
                if bytes[at..].starts_with(b"<!--") {
                    at = skip_past(bytes, at + 4, b"-->");
                } else if bytes[at..].starts_with(b"<![CDATA[") {
                    at = skip_past(bytes, at + 9, b"]]>");
                } else {
                    // A declaration (`<!DOCTYPE`, `<!ENTITY`, …). Scanned quote-aware so a
                    // `<a>` written inside a quoted entity value adds no depth — those
                    // documents are refused for their DTD anyway, and this keeps THIS guard
                    // from pre-empting that refusal with a wrong one.
                    at = end_of_markup(bytes, at + 2).0;
                }
            }
            Some(_) => {
                let (next, self_closing) = end_of_markup(bytes, at + 1);
                if !self_closing {
                    depth += 1;
                    if depth > MAX_PARSE_NESTING_DEPTH {
                        return Err(depth);
                    }
                }
                at = next;
            }
        }
    }
    Ok(())
}

/// Scan from `at` to the `>` that closes the markup it is inside, skipping quoted attribute
/// values whole. Returns the offset just past the `>` (or the end of input) and whether the
/// markup was self-closing (`… />`).
fn end_of_markup(bytes: &[u8], mut at: usize) -> (usize, bool) {
    // The byte before the `>`, which decides `/>` — never a quote, because a quoted run is
    // consumed as a unit and cannot end a tag.
    let mut previous = 0u8;
    while at < bytes.len() {
        let byte = bytes[at];
        match byte {
            b'"' | b'\'' => {
                at += 1;
                while at < bytes.len() && bytes[at] != byte {
                    at += 1;
                }
                at = at.saturating_add(1).min(bytes.len());
                previous = byte;
            }
            b'>' => return (at + 1, previous == b'/'),
            _ => {
                previous = byte;
                at += 1;
            }
        }
    }
    (bytes.len(), false)
}

/// The offset just past the first occurrence of `needle` at or after `at`, or the end of
/// input when there is none.
fn skip_past(bytes: &[u8], at: usize, needle: &[u8]) -> usize {
    if at >= bytes.len() {
        return bytes.len();
    }
    match bytes[at..]
        .windows(needle.len())
        .position(|window| window == needle)
    {
        Some(offset) => at + offset + needle.len(),
        None => bytes.len(),
    }
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
    use super::{MAX_PARSE_NESTING_DEPTH, guard_xml_nesting};

    /// `depth` nested `<a>` elements around a leaf.
    fn nested(depth: usize) -> String {
        format!(
            "<r>{}<leaf/>{}</r>",
            "<a>".repeat(depth),
            "</a>".repeat(depth)
        )
    }

    /// The limit is a limit ON the depth: one under passes, one over is refused. The `<r>`
    /// wrapper is itself a level, so `MAX - 1` inner elements is exactly at the limit.
    #[test]
    fn the_limit_is_exact() {
        assert!(guard_xml_nesting(&nested(MAX_PARSE_NESTING_DEPTH - 1)).is_ok());
        assert_eq!(
            guard_xml_nesting(&nested(MAX_PARSE_NESTING_DEPTH)),
            Err(MAX_PARSE_NESTING_DEPTH + 1)
        );
    }

    /// SIBLINGS ARE NOT NESTING. A count that added a level per start tag without
    /// subtracting one per end tag would refuse a perfectly flat document of a few hundred
    /// elements — the exact "bound that rejects what the stack round-trips" this guard must
    /// not be.
    #[test]
    fn a_wide_document_is_not_a_deep_one() {
        let wide = format!("<r>{}</r>", "<a>x</a>".repeat(10_000));
        assert!(guard_xml_nesting(&wide).is_ok());
    }

    /// A self-closing element opens no level, so a document of them nests one deep.
    #[test]
    fn a_self_closing_element_is_not_a_level() {
        let flat = format!("<r>{}</r>", "<a/>".repeat(10_000));
        assert!(guard_xml_nesting(&flat).is_ok());
    }

    /// `>` and `/>` inside an attribute VALUE are text, not markup. Reading them as markup
    /// would mis-close the tag and mis-count every level after it.
    #[test]
    fn markup_inside_an_attribute_value_is_not_markup() {
        let value = format!(
            "<r>{}<leaf/>{}</r>",
            "<a b=\"/&gt;&gt;\" c='&gt;'>".repeat(MAX_PARSE_NESTING_DEPTH - 1),
            "</a>".repeat(MAX_PARSE_NESTING_DEPTH - 1)
        );
        // The escaped forms above are what an XML author writes; the raw ones are legal in
        // an attribute value too, and are what actually exercises the scanner.
        let raw = format!(
            "<r>{}<leaf/>{}</r>",
            "<a b=\"/>>\" c='>'>".repeat(MAX_PARSE_NESTING_DEPTH - 1),
            "</a>".repeat(MAX_PARSE_NESTING_DEPTH - 1)
        );
        assert!(guard_xml_nesting(&value).is_ok());
        assert!(guard_xml_nesting(&raw).is_ok());
    }

    /// Comments, CDATA and processing instructions carry no depth, whatever they contain.
    #[test]
    fn comments_cdata_and_processing_instructions_carry_no_depth() {
        let noisy = format!(
            "<?xml version=\"1.0\"?><r><!-- {} --><![CDATA[{}]]><?pi {}?></r>",
            "<a>".repeat(10_000),
            "<a>".repeat(10_000),
            "<a>".repeat(10_000),
        );
        assert!(guard_xml_nesting(&noisy).is_ok());
    }

    /// A declaration's quoted value is skipped whole, so a DTD-bearing document is refused
    /// for its DTD by the XML parser rather than pre-empted by a bogus depth here.
    #[test]
    fn a_declaration_carries_no_depth() {
        let dtd = format!(
            "<!DOCTYPE r [<!ENTITY e \"{}\">]><r/>",
            "<a>".repeat(10_000)
        );
        assert!(guard_xml_nesting(&dtd).is_ok());
    }

    /// Unterminated markup and unbalanced end tags terminate the scan instead of panicking
    /// or underflowing; the XML parser behind the guard reports the syntax error.
    #[test]
    fn malformed_input_neither_panics_nor_underflows() {
        for malformed in [
            "<",
            "<a",
            "<a b=\"",
            "</a></a></a>",
            "<!--",
            "<![CDATA[",
            "<?",
        ] {
            assert!(guard_xml_nesting(malformed).is_ok(), "{malformed:?}");
        }
    }
}
