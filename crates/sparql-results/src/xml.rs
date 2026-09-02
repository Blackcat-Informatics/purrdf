// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL Results XML (SRX) serializer plus the additive, provenance-carrying
//! extension.
//!
//! Document shape follows the SPARQL 1.2 Query Results XML Format
//! specification (<https://www.w3.org/TR/sparql12-results-xml/>; the SPARQL 1.1
//! predecessor is <https://www.w3.org/TR/rdf-sparql-XMLres/>): a
//! `<sparql xmlns="http://www.w3.org/2005/sparql-results#">` root with a
//! `<head>` of `<variable>`s, then either `<results>` (SELECT) or `<boolean>`
//! (ASK). The CONSTRUCT (`Graph`) kind is undefined for SRX and hard-fails with
//! [`Error::Format`].
//!
//! # RDF 1.2 base direction — the `its:dir` spelling
//!
//! A directional literal's base direction is carried as `its:dir="…"` on
//! `<literal>`, using the ITS (Internationalization Tag Set) namespace
//! `http://www.w3.org/2005/11/its`. This is the spelling the SPARQL 1.2 Query
//! Results XML Format specification itself specifies
//! (<https://www.w3.org/TR/sparql12-results-xml/#example>, §2.3.1): its
//! "Variable Binding Results" encoding-template table gives, for "RDF Literal
//! *S* with language *L* with base direction *B*":
//! `<binding><literal xml:lang="L" its:dir="B">S</literal></binding>`.
//! No vendored `.srx` fixture under `crates/sparql-conformance/suite/` happens
//! to carry a directional literal (the base-direction W3C cases in
//! `w3c-sparql12/lang-basedir/` are `.srj`-only, per upstream
//! `w3c/rdf-tests`'s own manifest), so the spec quote above is the evidence
//! trail for the attribute name/namespace, recorded here per the "no
//! draft/provisional" contract: this is the SPARQL 1.2 Query Results
//! specification, not a working draft. `crates/sparql-conformance/suite/
//! purrdf-extend/basedir.{rq,srx}` closes the fixture gap: a
//! `STRLANGDIR(...)`-producing query, constructed directly from this same
//! spec section's own worked example content (the قطة/`ar`/`rtl` literal),
//! whose expected `.srx` pins this writer's exact spelling end to end.
//!
//! ## Where the `xmlns:its` declaration lives — the root, not the literal
//!
//! Every full-document example in §2.1 ("Document Element"), §2.2 ("Header"),
//! and the bulk of §2.3.1 ("Variable Binding Results") declares
//! `xmlns:its="http://www.w3.org/2005/11/its" its:version="2.0"` on the
//! **root** `<sparql>` element — repeated verbatim across seven separate
//! worked examples in that document. The spec's §2.1 also gives the inverse
//! rule explicitly: "If no literals with base direction appear in the
//! results, the `sparql` document element may be simplified" to drop the
//! `xmlns:its`/`its:version` attributes entirely. Only ONE example in §2.3.1 —
//! introduced by the spec's own words "As an alternative to including the
//! `xml:its` declaration in every result set, the namespace can be declared on
//! specific elements as needed" — shows the namespace declared inline on
//! individual `<literal>` elements instead; the spec frames this as a
//! secondary option, not the default.
//!
//! This writer follows the spec's DEFAULT (root-declared) style, which is also
//! the repo's only other vendored precedent for this attribute pair:
//! `suite/w3c-sparql12/eval-triple-terms/results-reifiedtriples-1.srx:2`
//! declares `xmlns:its="http://www.w3.org/2005/11/its" its:version="2.0"` on
//! its root `<sparql>` element (that particular fixture happens to carry no
//! directional literal at all, so the upstream W3C test generator clearly
//! treats the root declaration as the unconditional default style, not
//! something added only when needed). Concretely: `write_srx` scans the whole
//! result for any directional literal up front and, only when at least one is
//! present, emits `xmlns:its="…" its:version="2.0"` on `<sparql>` — the spec's
//! own "simplified" root form applies whenever none is present, so a document
//! with no directional literals never carries an unused namespace
//! declaration. `its:version="2.0"` is emitted alongside `xmlns:its` because
//! every one of the spec's worked examples pairs the two (ITS 2.0's own
//! `its:version` convention for host formats, like SRX, that are not
//! themselves ITS-aware).
//!
//! # The additive provenance extension
//!
//! A `<{prefix}:provenance>` element (after `</results>`/`<boolean>`) carries a
//! non-empty [`ResultProvenance`] when a caller supplies a
//! [`crate::model::ProvenanceNamespace`]. PurRDF mints no vocabulary IRIs of its
//! own, so without a caller-supplied namespace no such element is ever emitted,
//! however populated `provenance` is — see [`crate::model::ProvenanceNamespace`].

use crate::SerializeOutcome;
use crate::error::Error;
use crate::model::{ProvenanceNamespace, ResultProvenance};
use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
use purrdf_core::{SparqlResult, TermValue};

/// The `xsd:string` IRI; a literal carrying it (with no language) serializes
/// bare (no `datatype` attribute), matching the JSON/Turtle abbreviation.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The ITS (Internationalization Tag Set) namespace IRI the SPARQL 1.2 Query
/// Results specification uses for the `dir` attribute — see the module docs.
const ITS_NS: &str = "http://www.w3.org/2005/11/its";

/// Whether `result` carries at least one directional literal anywhere in its
/// bound terms (recursing into triple-term components). Determines whether
/// the SRX root `<sparql>` element needs the ITS namespace declaration — see
/// the module docs' "Where the `xmlns:its` declaration lives" section.
fn result_has_directional_literal(result: &SparqlResult) -> bool {
    match result {
        SparqlResult::Solutions { rows, .. } => rows
            .iter()
            .flatten()
            .flatten()
            .any(term_has_directional_literal),
        SparqlResult::Boolean(_) | SparqlResult::Graph(_) => false,
    }
}

/// Whether `value` is, or recursively contains (as a triple-term component),
/// a directional literal.
fn term_has_directional_literal(value: &TermValue) -> bool {
    match value {
        TermValue::Literal { direction, .. } => direction.is_some(),
        TermValue::Triple { s, p, o } => {
            term_has_directional_literal(s)
                || term_has_directional_literal(p)
                || term_has_directional_literal(o)
        }
        TermValue::Iri(_) | TermValue::Blank { .. } => false,
    }
}

/// Serialize a [`SparqlResult`] to SPARQL Results XML, appending the additive
/// provenance extension — under `namespace`'s prefix/IRI — when `provenance` is
/// non-empty and `namespace` is supplied.
///
/// XML carries everything that is requested PROVIDED a namespace is supplied to
/// anchor it under (PurRDF mints no vocabulary IRIs of its own — see
/// [`crate::model::ProvenanceNamespace`]). [`SerializeOutcome::provenance_dropped`]
/// is `true` only when `provenance` is non-empty but no `namespace` was given.
///
/// # Errors
///
/// Returns [`Error::Format`] for a `Graph` (CONSTRUCT) result, which has no
/// defined SRX representation.
///
/// Returns [`Error::Format`] if any dynamic string value contains an
/// XML-1.0-illegal C0 control character (U+0001–U+001F excluding U+0009,
/// U+000A, U+000D), which cannot be represented even as a numeric character
/// reference in XML 1.0.
pub fn to_xml(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
) -> Result<SerializeOutcome, Error> {
    // Cheap lower-bound pre-size (capacity is unobservable in the output):
    // the fixed skeleton plus a modest per-cell estimate saves the early
    // doubling reallocations on large result sets.
    let mut out = String::with_capacity(output_size_hint(result));
    write_srx(result, provenance, namespace, &mut out)?;
    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: !provenance.is_empty() && namespace.is_none(),
    })
}

/// Write the full SRX document (root + head + body + optional provenance).
fn write_srx(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
    out: &mut String,
) -> Result<(), Error> {
    out.push_str("<?xml version=\"1.0\"?>\n");
    if result_has_directional_literal(result) {
        // The spec's DEFAULT (root-declared) style — see the module docs.
        out.push_str("<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\" xmlns:its=\"");
        out.push_str(ITS_NS);
        out.push_str("\" its:version=\"2.0\">\n");
    } else {
        // The spec's own "simplified" root form when no directional literal
        // appears anywhere in the result.
        out.push_str("<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n");
    }

    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            write_head(variables, out)?;
            write_results(variables, rows, out)?;
        }
        SparqlResult::Boolean(value) => {
            // ASK has no variables → empty head.
            out.push_str("  <head></head>\n");
            out.push_str("  <boolean>");
            out.push_str(if *value { "true" } else { "false" });
            out.push_str("</boolean>\n");
        }
        SparqlResult::Graph(_) => {
            return Err(Error::Format(
                "SPARQL Results XML is undefined for CONSTRUCT graphs; serialize the graph as RDF"
                    .to_string(),
            ));
        }
    }

    if !provenance.is_empty()
        && let Some(namespace) = namespace
    {
        write_provenance(result, provenance, namespace, out)?;
    }

    out.push_str("</sparql>\n");
    Ok(())
}

/// Write the `<head>` of `<variable>` declarations.
fn write_head(variables: &[String], out: &mut String) -> Result<(), Error> {
    if variables.is_empty() {
        out.push_str("  <head></head>\n");
        return Ok(());
    }
    out.push_str("  <head>\n");
    for var in variables {
        out.push_str("    <variable name=\"");
        xml_escape_attr(var, out)?;
        out.push_str("\"/>\n");
    }
    out.push_str("  </head>\n");
    Ok(())
}

/// Write the `<results>` block (one `<result>` per row; unbound cells omitted).
fn write_results(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    out: &mut String,
) -> Result<(), Error> {
    out.push_str("  <results>\n");
    for row in rows {
        out.push_str("    <result>\n");
        for (column, cell) in row.iter().enumerate() {
            if let Some(value) = cell {
                let var = variables.get(column).ok_or_else(|| {
                    Error::MalformedTerm(format!(
                        "binding column {column} has no variable header (row has {} vars)",
                        variables.len()
                    ))
                })?;
                out.push_str("      <binding name=\"");
                xml_escape_attr(var, out)?;
                out.push_str("\">");
                write_term(value, out)?;
                out.push_str("</binding>\n");
            }
        }
        out.push_str("    </result>\n");
    }
    out.push_str("  </results>\n");
    Ok(())
}

/// Write a single bound term element (`<uri>`/`<bnode>`/`<literal>`/`<triple>`).
fn write_term(value: &TermValue, out: &mut String) -> Result<(), Error> {
    match value {
        TermValue::Iri(iri) => {
            out.push_str("<uri>");
            xml_escape_text(iri, out)?;
            out.push_str("</uri>");
        }
        TermValue::Blank { label, scope } => {
            // A `<bnode>` id is a blank-node LABEL, not free text, so the
            // `(label, scope)` pair is encoded into the W3C BLANK_NODE_LABEL
            // alphabet — matching the JSON/CSV/TSV writers, and incidentally
            // removing every character XML 1.0 cannot represent.
            out.push_str("<bnode>");
            xml_escape_text(
                &encode_blank_label(label, *scope, LabelAlphabet::BlankNodeLabel),
                out,
            )?;
            out.push_str("</bnode>");
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push_str("<literal");
            if let Some(language) = language {
                out.push_str(" xml:lang=\"");
                xml_escape_attr(language, out)?;
                out.push('"');
            } else if datatype != XSD_STRING {
                out.push_str(" datatype=\"");
                xml_escape_attr(datatype, out)?;
                out.push('"');
            }
            if let Some(direction) = direction {
                // No inline `xmlns:its` declaration here — it is declared
                // once on the document root when needed (see the module
                // docs' "Where the `xmlns:its` declaration lives" section).
                out.push_str(" its:dir=\"");
                out.push_str(direction.as_str());
                out.push('"');
            }
            out.push('>');
            xml_escape_text(lexical_form, out)?;
            out.push_str("</literal>");
        }
        TermValue::Triple { s, p, o } => {
            // RDF predicates must be IRIs; a non-IRI predicate has no valid SRX
            // <predicate> form → hard-fail per the serializer contract.
            if !matches!(p.as_ref(), TermValue::Iri(_)) {
                return Err(Error::MalformedTerm(
                    "triple-term predicate is not an IRI".to_string(),
                ));
            }
            out.push_str("<triple><subject>");
            write_term(s, out)?;
            out.push_str("</subject><predicate>");
            write_term(p, out)?;
            out.push_str("</predicate><object>");
            write_term(o, out)?;
            out.push_str("</object></triple>");
        }
    }
    Ok(())
}

/// Write the additive `<{prefix}:provenance>` element (only present fields),
/// under the caller-supplied [`ProvenanceNamespace`].
fn write_provenance(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: &ProvenanceNamespace,
    out: &mut String,
) -> Result<(), Error> {
    let prefix = namespace.prefix();
    out.push_str("  <");
    out.push_str(prefix);
    out.push_str(":provenance xmlns:");
    out.push_str(prefix);
    out.push_str("=\"");
    xml_escape_attr(namespace.iri(), out)?;
    out.push_str("\">\n");

    out.push_str("    <");
    out.push_str(prefix);
    out.push_str(":queryForm>");
    out.push_str(query_form(result));
    out.push_str("</");
    out.push_str(prefix);
    out.push_str(":queryForm>\n");

    if let Some(query_hash) = &provenance.query_hash {
        out.push_str("    <");
        out.push_str(prefix);
        out.push_str(":queryHash>");
        xml_escape_text(query_hash, out)?;
        out.push_str("</");
        out.push_str(prefix);
        out.push_str(":queryHash>\n");
    }
    if let Some(engine) = &provenance.engine {
        out.push_str("    <");
        out.push_str(prefix);
        out.push_str(":engine>");
        xml_escape_text(engine, out)?;
        out.push_str("</");
        out.push_str(prefix);
        out.push_str(":engine>\n");
    }
    for solution in &provenance.solutions {
        out.push_str("    <");
        out.push_str(prefix);
        out.push_str(":solution>\n");
        for source in &solution.sources {
            out.push_str("      <");
            out.push_str(prefix);
            out.push_str(":source>");
            xml_escape_text(source, out)?;
            out.push_str("</");
            out.push_str(prefix);
            out.push_str(":source>\n");
        }
        out.push_str("    </");
        out.push_str(prefix);
        out.push_str(":solution>\n");
    }

    out.push_str("  </");
    out.push_str(prefix);
    out.push_str(":provenance>\n");
    Ok(())
}

/// A cheap lower-bound estimate of the serialized size, used only to pre-size
/// the output buffer. The `Graph` arm never serializes (CONSTRUCT hard-fails)
/// but is named exhaustively.
fn output_size_hint(result: &SparqlResult) -> usize {
    const SKELETON: usize = 128;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => SKELETON.saturating_add(
            rows.len()
                .saturating_mul(variables.len().saturating_add(1))
                .saturating_mul(48),
        ),
        SparqlResult::Graph(_) | SparqlResult::Boolean(_) => SKELETON,
    }
}

/// The `queryForm` discriminator emitted in provenance. The `Graph` arm is
/// unreachable here (CONSTRUCT hard-fails earlier) but is named exhaustively.
fn query_form(result: &SparqlResult) -> &'static str {
    match result {
        SparqlResult::Solutions { .. } => "select",
        SparqlResult::Boolean(_) => "ask",
        SparqlResult::Graph(_) => "construct",
    }
}

/// Escape XML *text content*: `&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`.
/// Tab, newline, and CR are legal in XML 1.0 text content and are passed
/// through literally.
///
/// # Errors
///
/// Returns [`Error::Format`] if `value` contains any XML-1.0-illegal C0
/// control character (U+0001–U+001F, excluding U+0009, U+000A, U+000D and
/// U+0000). These characters cannot be represented in XML 1.0, not even as
/// numeric character references, so the only safe policy is to hard-fail.
fn xml_escape_text(value: &str, out: &mut String) -> Result<(), Error> {
    let mut rest = value;
    while !rest.is_empty() {
        // Bulk-copy the clean run in one `push_str`; every trigger (`&<>` and
        // the C0 controls) is an ASCII byte, so non-ASCII text never splits
        // and the first offending control character is still the one reported.
        let run = rest
            .bytes()
            .position(|b| b < 0x20 || matches!(b, b'&' | b'<' | b'>'))
            .unwrap_or(rest.len());
        out.push_str(&rest[..run]);
        rest = &rest[run..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // U+0009 (tab), U+000A (LF), U+000D (CR) are legal in XML 1.0
            // text content — pass them through literally.
            '\t' | '\n' | '\r' => out.push(ch),
            c if (c as u32) < 0x20 => {
                return Err(Error::Format(format!(
                    "XML 1.0 cannot represent control character U+{:04X}",
                    c as u32
                )));
            }
            c => out.push(c),
        }
        rest = &rest[ch.len_utf8()..];
    }
    Ok(())
}

/// The original per-`char` text escaper, kept as the oracle for
/// [`xml_escape_text`].
#[cfg(test)]
fn xml_escape_text_reference(value: &str, out: &mut String) -> Result<(), Error> {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' | '\n' | '\r' => out.push(ch),
            c if (c as u32) < 0x20 => {
                return Err(Error::Format(format!(
                    "XML 1.0 cannot represent control character U+{:04X}",
                    c as u32
                )));
            }
            c => out.push(c),
        }
    }
    Ok(())
}

/// Escape an XML *attribute value*: `&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`,
/// `"`→`&quot;`. Tab, newline, and CR are subject to XML attribute-value
/// normalization (collapsed to spaces on parse), so they are emitted as
/// numeric character references (`&#x9;`, `&#xA;`, `&#xD;`) to round-trip
/// faithfully.
///
/// # Errors
///
/// Returns [`Error::Format`] if `value` contains any XML-1.0-illegal C0
/// control character (U+0001–U+001F, excluding U+0009, U+000A, U+000D and
/// U+0000). These characters cannot be represented in XML 1.0, not even as
/// numeric character references, so the only safe policy is to hard-fail.
fn xml_escape_attr(value: &str, out: &mut String) -> Result<(), Error> {
    let mut rest = value;
    while !rest.is_empty() {
        // Bulk-copy the clean run in one `push_str`; every trigger (`&<>"` and
        // the C0 controls) is an ASCII byte, so non-ASCII text never splits
        // and the first offending control character is still the one reported.
        let run = rest
            .bytes()
            .position(|b| b < 0x20 || matches!(b, b'&' | b'<' | b'>' | b'"'))
            .unwrap_or(rest.len());
        out.push_str(&rest[..run]);
        rest = &rest[run..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // Tab/LF/CR are subject to attribute-value normalization in XML
            // 1.0 (§3.3.3), so emit as numeric character references to
            // preserve their identity across a parse round-trip.
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            c if (c as u32) < 0x20 => {
                return Err(Error::Format(format!(
                    "XML 1.0 cannot represent control character U+{:04X}",
                    c as u32
                )));
            }
            c => out.push(c),
        }
        rest = &rest[ch.len_utf8()..];
    }
    Ok(())
}

/// The original per-`char` attribute escaper, kept as the oracle for
/// [`xml_escape_attr`].
#[cfg(test)]
fn xml_escape_attr_reference(value: &str, out: &mut String) -> Result<(), Error> {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            c if (c as u32) < 0x20 => {
                return Err(Error::Format(format!(
                    "XML 1.0 cannot represent control character U+{:04X}",
                    c as u32
                )));
            }
            c => out.push(c),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SolutionProvenance;
    use pretty_assertions::assert_eq;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm, RdfTextDirection};

    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

    /// Every case an escaper must agree with its per-`char` oracle on: empty,
    /// clean ASCII, each trigger alone, control characters (including the
    /// error path and which character it names first), multibyte, and mixes.
    const ESCAPE_CASES: [&str; 18] = [
        "",
        "plain ascii text 0123456789 ~!@#$%^*()_+-=[]{};':,./?",
        "&",
        "<",
        ">",
        "\"",
        "\t",
        "\n",
        "\r",
        "\u{0}",
        "\u{1}",
        "\u{1f}",
        "\u{7f}\u{80}\u{85}\u{9f}",
        "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
        "a & b < c > d \"e\" \u{e9}\t\n\r end",
        "ok \u{e9} then \u{1} then \u{2} later",
        "<\u{1f431}&\"\u{2028}>",
        "&amp; already <escaped>",
    ];

    #[test]
    fn xml_escape_text_matches_reference() {
        for case in ESCAPE_CASES {
            let mut fast = String::from("prefix");
            let mut reference = String::from("prefix");
            let fast_result = xml_escape_text(case, &mut fast);
            let reference_result = xml_escape_text_reference(case, &mut reference);
            assert_eq!(fast_result, reference_result, "{case:?}");
            assert_eq!(fast, reference, "{case:?}");
        }
    }

    #[test]
    fn xml_escape_attr_matches_reference() {
        for case in ESCAPE_CASES {
            let mut fast = String::from("prefix");
            let mut reference = String::from("prefix");
            let fast_result = xml_escape_attr(case, &mut fast);
            let reference_result = xml_escape_attr_reference(case, &mut reference);
            assert_eq!(fast_result, reference_result, "{case:?}");
            assert_eq!(fast, reference, "{case:?}");
        }
    }

    /// A namespace used by the populated-provenance tests below — caller-chosen,
    /// `example.org`-scoped per repository convention (never a fabricated
    /// default; the crate itself mints nothing).
    fn test_namespace() -> ProvenanceNamespace {
        ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("test namespace is a valid NCName prefix + absolute IRI")
    }

    fn xml_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        let outcome = to_xml(result, prov, None).expect("serialization succeeds");
        assert!(
            !outcome.provenance_dropped,
            "empty provenance is never dropped"
        );
        String::from_utf8(outcome.bytes).expect("UTF-8 output")
    }

    fn xml_text_ns(
        result: &SparqlResult,
        prov: &ResultProvenance,
        namespace: &ProvenanceNamespace,
    ) -> String {
        let outcome = to_xml(result, prov, Some(namespace)).expect("serialization succeeds");
        assert!(!outcome.provenance_dropped, "namespace was supplied");
        String::from_utf8(outcome.bytes).expect("UTF-8 output")
    }

    fn lit(lex: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lex.to_string(),
            datatype: datatype.to_string(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn select_full_shape() {
        let result = SparqlResult::Solutions {
            variables: vec![
                "s".to_string(),
                "b".to_string(),
                "name".to_string(),
                "age".to_string(),
                "label".to_string(),
            ],
            rows: vec![
                vec![
                    Some(TermValue::Iri("http://example.org/s".to_string())),
                    Some(TermValue::Blank {
                        label: "b0".to_string(),
                        scope: BlankScope(0),
                    }),
                    Some(lit("Ada", XSD_STRING)),
                    Some(lit("42", XSD_INTEGER)),
                    Some(TermValue::Literal {
                        lexical_form: "bonjour".to_string(),
                        datatype: RDF_LANGSTRING.to_string(),
                        language: Some("fr".to_string()),
                        direction: None,
                    }),
                ],
                vec![
                    Some(TermValue::Iri("http://example.org/s2".to_string())),
                    None,
                    Some(lit("Bob", XSD_STRING)),
                    None,
                    Some(lit("Grace", XSD_STRING)),
                ],
            ],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let expected = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n",
            "  <head>\n",
            "    <variable name=\"s\"/>\n",
            "    <variable name=\"b\"/>\n",
            "    <variable name=\"name\"/>\n",
            "    <variable name=\"age\"/>\n",
            "    <variable name=\"label\"/>\n",
            "  </head>\n",
            "  <results>\n",
            "    <result>\n",
            "      <binding name=\"s\"><uri>http://example.org/s</uri></binding>\n",
            "      <binding name=\"b\"><bnode>b0</bnode></binding>\n",
            "      <binding name=\"name\"><literal>Ada</literal></binding>\n",
            "      <binding name=\"age\"><literal datatype=\"http://www.w3.org/2001/XMLSchema#integer\">42</literal></binding>\n",
            "      <binding name=\"label\"><literal xml:lang=\"fr\">bonjour</literal></binding>\n",
            "    </result>\n",
            "    <result>\n",
            "      <binding name=\"s\"><uri>http://example.org/s2</uri></binding>\n",
            "      <binding name=\"name\"><literal>Bob</literal></binding>\n",
            "      <binding name=\"label\"><literal>Grace</literal></binding>\n",
            "    </result>\n",
            "  </results>\n",
            "</sparql>\n",
        );
        assert_eq!(xml_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn ask_true_exact() {
        let result = SparqlResult::Boolean(true);
        let expected = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n",
            "  <head></head>\n",
            "  <boolean>true</boolean>\n",
            "</sparql>\n",
        );
        assert_eq!(xml_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn ask_false_exact() {
        let result = SparqlResult::Boolean(false);
        let expected = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n",
            "  <head></head>\n",
            "  <boolean>false</boolean>\n",
            "</sparql>\n",
        );
        assert_eq!(xml_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn triple_term_shape() {
        let triple = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(
            text.contains(concat!(
                "<binding name=\"t\"><triple>",
                "<subject><uri>http://example.org/s</uri></subject>",
                "<predicate><uri>http://example.org/p</uri></predicate>",
                "<object><uri>http://example.org/o</uri></object>",
                "</triple></binding>",
            )),
            "unexpected triple shape: {text}"
        );
    }

    /// BYTE PIN — the FULL document for a single-row, single-variable
    /// triple-term SELECT result, pinned exactly (not just a substring).
    #[test]
    fn triple_term_document_exact_bytes() {
        let triple = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        let expected = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n",
            "  <head>\n",
            "    <variable name=\"t\"/>\n",
            "  </head>\n",
            "  <results>\n",
            "    <result>\n",
            "      <binding name=\"t\"><triple>",
            "<subject><uri>http://example.org/s</uri></subject>",
            "<predicate><uri>http://example.org/p</uri></predicate>",
            "<object><uri>http://example.org/o</uri></object>",
            "</triple></binding>\n",
            "    </result>\n",
            "  </results>\n",
            "</sparql>\n",
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn non_iri_triple_predicate_is_malformed_error() {
        // A triple-term whose predicate is a plain literal (not an IRI) must
        // hard-fail with MalformedTerm rather than emitting structurally invalid
        // SRX output.
        let triple = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(lit("not-an-iri", XSD_STRING)),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_xml(&result, &ResultProvenance::default(), None)
            .expect_err("non-IRI predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm, got: {err:?}"
        );
    }

    #[test]
    fn non_iri_bnode_triple_predicate_is_malformed_error() {
        // A blank-node predicate is equally invalid.
        let triple = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_xml(&result, &ResultProvenance::default(), None)
            .expect_err("bnode predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm, got: {err:?}"
        );
    }

    #[test]
    fn directional_literal_carries_its_dir_and_root_ns() {
        let result = SparqlResult::Solutions {
            variables: vec!["d".to_string()],
            rows: vec![vec![Some(TermValue::Literal {
                lexical_form: "hello".to_string(),
                datatype: RDF_LANGSTRING.to_string(),
                language: Some("en".to_string()),
                direction: Some(RdfTextDirection::Ltr),
            })]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(text.contains("its:dir=\"ltr\""), "missing dir: {text}");
        // BYTE PIN — the ITS namespace + its:version are declared ONCE on the
        // document root (the spec's default style — see the module docs),
        // NOT inline on the literal.
        assert!(
            text.contains(
                "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\" xmlns:its=\"http://www.w3.org/2005/11/its\" its:version=\"2.0\">"
            ),
            "missing root ns decl: {text}"
        );
        assert!(
            text.contains("<literal xml:lang=\"en\" its:dir=\"ltr\">hello</literal>"),
            "unexpected directional literal: {text}"
        );
        assert!(
            !text.contains("<literal xml:lang=\"en\" xmlns:its"),
            "must not redeclare xmlns:its inline on the literal: {text}"
        );
    }

    #[test]
    fn non_directional_literal_is_clean() {
        let result = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(lit("x", XSD_STRING))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(!text.contains("its:dir"), "must stay clean: {text}");
        assert!(!text.contains("xmlns:its"), "must stay clean: {text}");
    }

    #[test]
    fn escaping_in_text_and_attr() {
        let result = SparqlResult::Solutions {
            variables: vec!["v<&>\"".to_string()],
            rows: vec![vec![Some(lit("a & b < c > d \"e\"", XSD_STRING))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(
            text.contains("<variable name=\"v&lt;&amp;&gt;&quot;\"/>"),
            "attr escaping: {text}"
        );
        assert!(
            text.contains("<literal>a &amp; b &lt; c &gt; d \"e\"</literal>"),
            "text escaping (no quot in text): {text}"
        );
    }

    #[test]
    fn populated_provenance_with_namespace_present() {
        let result = SparqlResult::Solutions {
            variables: vec!["s".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/s".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let provenance = ResultProvenance {
            query_hash: Some("deadbeef".to_string()),
            engine: Some("purrdf-sparql-eval".to_string()),
            solutions: vec![SolutionProvenance {
                sources: vec!["http://example.org/g1".to_string()],
            }],
        };
        let namespace = test_namespace();
        let text = xml_text_ns(&result, &provenance, &namespace);
        assert!(
            text.contains("<prov:provenance xmlns:prov=\"http://example.org/ns/prov#\">"),
            "missing provenance: {text}"
        );
        assert!(
            text.contains("<prov:queryForm>select</prov:queryForm>"),
            "missing queryForm: {text}"
        );
        assert!(
            text.contains("<prov:queryHash>deadbeef</prov:queryHash>"),
            "missing queryHash: {text}"
        );
        assert!(
            text.contains("<prov:engine>purrdf-sparql-eval</prov:engine>"),
            "missing engine: {text}"
        );
        assert!(
            text.contains("<prov:source>http://example.org/g1</prov:source>"),
            "missing source: {text}"
        );
        // Provenance sits after </results>, before </sparql>.
        let after_results = text.split_once("</results>").map_or("", |(_, rest)| rest);
        assert!(
            after_results.contains("<prov:provenance"),
            "provenance must follow </results>: {text}"
        );
    }

    // ---- Namespace-prefix injection: unconstructible, not merely escaped ----

    /// A prefix engineered to close the `xmlns:{prefix}="…"` attribute and the
    /// `<{prefix}:provenance …>` start tag, splicing an attacker-controlled
    /// element into the document, must never construct. Every character this
    /// string needs (`"`, `<`, `>`, space) individually violates the XML
    /// `NCName` grammar `ProvenanceNamespace::new` enforces, so there is no
    /// path from caller input to this writer emitting it.
    #[test]
    fn crafted_breakout_prefix_cannot_be_constructed() {
        let crafted = "prov\"><evil xmlns:x=\"http://evil.example/";
        let err = ProvenanceNamespace::new(crafted, "http://example.org/ns/prov#")
            .expect_err("a markup-breakout prefix must never construct");
        assert!(
            matches!(err, Error::InvalidNamespace(_)),
            "expected InvalidNamespace, got: {err:?}"
        );
    }

    /// Defence in depth: for every namespace `ProvenanceNamespace::new` DOES
    /// accept (plain ASCII, a non-leading digit/hyphen/dot, a leading
    /// underscore, and a non-ASCII `NCNameStartChar`), the XML this writer
    /// emits still parses cleanly through this crate's own SRX reader
    /// (`from_xml`) — the base SELECT document (variables + bindings) is
    /// unaffected by the appended provenance element, exactly as the "the
    /// writer must never emit a document it cannot itself read" contract
    /// requires.
    #[test]
    fn accepted_namespaces_round_trip_through_the_crate_reader() {
        let accepted = [
            ("prov", "http://example.org/ns/prov#"),
            ("_prov-1.thing_2", "http://example.org/ns/prov#"),
            ("_leading_underscore", "http://example.org/ns/prov#"),
            // A non-ASCII NCNameStartChar (Cyrillic Ze, U+0417) — the
            // NCName production admits far more than ASCII letters.
            ("\u{417}prov", "http://example.org/ns/prov#"),
        ];
        for (prefix, iri) in accepted {
            let namespace =
                ProvenanceNamespace::new(prefix, iri).expect("accepted shape must construct");
            let result = SparqlResult::Solutions {
                variables: vec!["s".to_string()],
                rows: vec![vec![Some(TermValue::Iri(
                    "http://example.org/s".to_string(),
                ))]],
                aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
            };
            let provenance = ResultProvenance {
                engine: Some("purrdf-sparql-eval".to_string()),
                ..Default::default()
            };
            let outcome = to_xml(&result, &provenance, Some(&namespace))
                .unwrap_or_else(|e| panic!("prefix {prefix:?} must serialize: {e:?}"));
            let parsed = crate::xml_read::from_xml(&outcome.bytes)
                .unwrap_or_else(|e| panic!("prefix {prefix:?} document must re-parse: {e:?}"));
            assert_eq!(parsed.variables, ["s"]);
            assert_eq!(
                parsed.rows,
                [vec![Some(TermValue::Iri(
                    "http://example.org/s".to_string()
                ))]]
            );
        }
    }

    /// The legitimate custom-prefix surface stays fully usable end to end: a
    /// caller-chosen prefix serializes, and the base document (bindings) reads
    /// back correctly via this crate's own reader — the fix must not break the
    /// happy path while closing the injection hole.
    #[test]
    fn valid_custom_prefix_write_then_read_back() {
        let namespace = ProvenanceNamespace::new("myProv", "https://example.org/ns/my-prov#")
            .expect("custom prefix is a valid NCName + absolute IRI");
        let result = SparqlResult::Solutions {
            variables: vec!["s".to_string(), "label".to_string()],
            rows: vec![vec![
                Some(TermValue::Iri("http://example.org/s".to_string())),
                Some(lit("hello", XSD_STRING)),
            ]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let provenance = ResultProvenance {
            query_hash: Some("deadbeef".to_string()),
            ..Default::default()
        };
        let outcome = to_xml(&result, &provenance, Some(&namespace)).expect("serializes");
        assert!(!outcome.provenance_dropped, "namespace was supplied");
        let text = String::from_utf8(outcome.bytes.clone()).expect("UTF-8");
        assert!(
            text.contains("<myProv:provenance xmlns:myProv=\"https://example.org/ns/my-prov#\">"),
            "missing provenance element: {text}"
        );

        let parsed = crate::xml_read::from_xml(&outcome.bytes).expect("document re-parses");
        assert_eq!(parsed.variables, ["s", "label"]);
        assert_eq!(
            parsed.rows,
            [vec![
                Some(TermValue::Iri("http://example.org/s".to_string())),
                Some(TermValue::Literal {
                    lexical_form: "hello".to_string(),
                    datatype: XSD_STRING.to_string(),
                    language: None,
                    direction: None,
                }),
            ]]
        );
    }

    /// DE-MINTING — populated provenance with NO namespace supplied emits no
    /// `<…:provenance>` element at all (PurRDF mints no vocabulary IRIs of its
    /// own) and the drop is signalled, exactly like the CSV/TSV exit gate.
    #[test]
    fn populated_provenance_without_namespace_is_dropped_and_signalled() {
        let result = SparqlResult::Boolean(true);
        let provenance = ResultProvenance {
            engine: Some("e".to_string()),
            ..Default::default()
        };
        let outcome = to_xml(&result, &provenance, None).expect("serializes");
        assert!(
            outcome.provenance_dropped,
            "non-empty provenance with no namespace must be signalled as dropped"
        );
        let text = String::from_utf8(outcome.bytes).expect("UTF-8");
        assert!(
            !text.contains("provenance"),
            "no fabricated provenance element without a namespace: {text}"
        );
        let expected = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n",
            "  <head></head>\n",
            "  <boolean>true</boolean>\n",
            "</sparql>\n",
        );
        assert_eq!(text, expected, "base document must stay pure W3C");
    }

    #[test]
    fn graph_is_format_error() {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_owned_quad(&RdfQuad {
            subject: RdfTerm::iri("http://example.org/s"),
            predicate: "http://example.org/p".to_string(),
            object: RdfTerm::iri("http://example.org/o"),
            graph_name: None,
            location: None,
        });
        let dataset = builder.freeze().expect("dataset freezes");
        let result = SparqlResult::Graph(dataset);
        let err = to_xml(&result, &ResultProvenance::default(), None).expect_err("graph rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
    }

    #[test]
    fn illegal_control_char_is_format_error() {
        // U+0001 in the lexical form of a literal → must hard-fail with Format error.
        let result = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(lit("bad\u{1}value", XSD_STRING))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_xml(&result, &ResultProvenance::default(), None)
            .expect_err("illegal C0 control char must be rejected");
        match &err {
            Error::Format(msg) => {
                assert!(
                    msg.contains("U+0001"),
                    "error message must mention U+0001, got: {msg}"
                );
            }
            other => panic!("expected Error::Format, got: {other:?}"),
        }
    }

    #[test]
    fn attribute_tab_newline_become_char_refs() {
        // A tab in the datatype IRI lands in a `datatype="..."` attribute.
        // It must be written as &#x9; so it round-trips past attribute-value normalization.
        let result = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(TermValue::Literal {
                lexical_form: "value".to_string(),
                datatype: "http://example.org/d\tt".to_string(),
                language: None,
                direction: None,
            })]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(
            text.contains("&#x9;"),
            "tab in datatype attr must become &#x9;, got: {text}"
        );
        // Also verify newline and CR in an attribute value become char refs.
        let result2 = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(TermValue::Literal {
                lexical_form: "value".to_string(),
                datatype: "http://example.org/d\nt".to_string(),
                language: None,
                direction: None,
            })]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text2 = xml_text(&result2, &ResultProvenance::default());
        assert!(
            text2.contains("&#xA;"),
            "newline in datatype attr must become &#xA;, got: {text2}"
        );
        let result3 = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(TermValue::Literal {
                lexical_form: "value".to_string(),
                datatype: "http://example.org/d\rt".to_string(),
                language: None,
                direction: None,
            })]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text3 = xml_text(&result3, &ResultProvenance::default());
        assert!(
            text3.contains("&#xD;"),
            "CR in datatype attr must become &#xD;, got: {text3}"
        );
    }

    #[test]
    fn text_content_keeps_legal_whitespace() {
        // A literal lexical form with \n must appear as a literal newline in
        // the <literal> text content — NOT as &#xA;.
        let result = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(lit("a\nb", XSD_STRING))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = xml_text(&result, &ResultProvenance::default());
        assert!(
            text.contains("<literal>a\nb</literal>"),
            "literal newline in text content must be passed through literally, got: {text}"
        );
        assert!(
            !text.contains("&#xA;"),
            "text content must NOT use &#xA; for newline, got: {text}"
        );
    }
}
