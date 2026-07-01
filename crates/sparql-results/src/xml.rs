// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL Results XML (SRX) serializer plus the additive, provenance-carrying
//! `purrdf` extension.
//!
//! Document shape follows <https://www.w3.org/TR/rdf-sparql-XMLres/>: a
//! `<sparql xmlns="http://www.w3.org/2005/sparql-results#">` root with a
//! `<head>` of `<variable>`s, then either `<results>` (SELECT) or `<boolean>`
//! (ASK). The CONSTRUCT (`Graph`) kind is undefined for SRX and hard-fails with
//! [`Error::Format`].
//!
//! Two additive, namespaced extensions are emitted only when present, so the
//! default output stays pure W3C: a `purrdf:dir` attribute on `<literal>` carries
//! an RDF-1.2 base direction, and a `<purrdf:provenance>` element (after
//! `</results>`/`<boolean>`) carries a non-empty [`ResultProvenance`]. Both
//! inline the `xmlns:purrdf="https://purrdf.dev/ns/results#"` declaration so the
//! document needs no fixed prologue namespace.

use crate::error::Error;
use crate::model::ResultProvenance;
use crate::SerializeOutcome;
use purrdf_core::{SparqlResult, TermValue};

/// The `xsd:string` IRI; a literal carrying it (with no language) serializes
/// bare (no `datatype` attribute), matching the JSON/Turtle abbreviation.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The `purrdf` results-extension namespace IRI.
const PURRDF_NS: &str = "https://purrdf.dev/ns/results#";

/// Serialize a [`SparqlResult`] to SPARQL Results XML, appending the additive
/// `purrdf` extensions when present.
///
/// XML carries everything that is requested, so the returned
/// [`SerializeOutcome::provenance_dropped`] is always `false`.
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
) -> Result<SerializeOutcome, Error> {
    let mut out = String::new();
    write_srx(result, provenance, &mut out)?;
    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: false,
    })
}

/// Write the full SRX document (root + head + body + optional provenance).
fn write_srx(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    out: &mut String,
) -> Result<(), Error> {
    out.push_str("<?xml version=\"1.0\"?>\n");
    out.push_str("<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\n");

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

    if !provenance.is_empty() {
        write_provenance(result, provenance, out)?;
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
        TermValue::Blank { label, .. } => {
            out.push_str("<bnode>");
            xml_escape_text(label, out)?;
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
                out.push_str(" purrdf:dir=\"");
                out.push_str(direction.as_str());
                out.push_str("\" xmlns:purrdf=\"");
                out.push_str(PURRDF_NS);
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

/// Write the additive `<purrdf:provenance>` element (only present fields).
fn write_provenance(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    out: &mut String,
) -> Result<(), Error> {
    out.push_str("  <purrdf:provenance xmlns:purrdf=\"");
    out.push_str(PURRDF_NS);
    out.push_str("\">\n");

    out.push_str("    <purrdf:queryForm>");
    out.push_str(query_form(result));
    out.push_str("</purrdf:queryForm>\n");

    if let Some(query_hash) = &provenance.query_hash {
        out.push_str("    <purrdf:queryHash>");
        xml_escape_text(query_hash, out)?;
        out.push_str("</purrdf:queryHash>\n");
    }
    if let Some(engine) = &provenance.engine {
        out.push_str("    <purrdf:engine>");
        xml_escape_text(engine, out)?;
        out.push_str("</purrdf:engine>\n");
    }
    for solution in &provenance.solutions {
        out.push_str("    <purrdf:solution>\n");
        for source in &solution.sources {
            out.push_str("      <purrdf:source>");
            xml_escape_text(source, out)?;
            out.push_str("</purrdf:source>\n");
        }
        out.push_str("    </purrdf:solution>\n");
    }

    out.push_str("  </purrdf:provenance>\n");
    Ok(())
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
    for ch in value.chars() {
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
    for ch in value.chars() {
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

    fn xml_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        let outcome = to_xml(result, prov).expect("serialization succeeds");
        assert!(!outcome.provenance_dropped, "xml never drops provenance");
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
        let err = to_xml(&result, &ResultProvenance::default())
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
        let err = to_xml(&result, &ResultProvenance::default())
            .expect_err("bnode predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm, got: {err:?}"
        );
    }

    #[test]
    fn directional_literal_carries_dir_and_ns() {
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
        assert!(text.contains("purrdf:dir=\"ltr\""), "missing dir: {text}");
        assert!(
            text.contains("xmlns:purrdf=\"https://purrdf.dev/ns/results#\""),
            "missing inline ns: {text}"
        );
        // xml:lang precedes the purrdf ns+dir on the literal element.
        assert!(
            text.contains(
                "<literal xml:lang=\"en\" purrdf:dir=\"ltr\" xmlns:purrdf=\"https://purrdf.dev/ns/results#\">hello</literal>"
            ),
            "unexpected directional literal: {text}"
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
        assert!(!text.contains("purrdf:dir"), "must stay clean: {text}");
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
    fn populated_provenance_present() {
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
        let text = xml_text(&result, &provenance);
        assert!(
            text.contains("<purrdf:provenance xmlns:purrdf=\"https://purrdf.dev/ns/results#\">"),
            "missing provenance: {text}"
        );
        assert!(
            text.contains("<purrdf:queryForm>select</purrdf:queryForm>"),
            "missing queryForm: {text}"
        );
        assert!(
            text.contains("<purrdf:queryHash>deadbeef</purrdf:queryHash>"),
            "missing queryHash: {text}"
        );
        assert!(
            text.contains("<purrdf:engine>purrdf-sparql-eval</purrdf:engine>"),
            "missing engine: {text}"
        );
        assert!(
            text.contains("<purrdf:source>http://example.org/g1</purrdf:source>"),
            "missing source: {text}"
        );
        // Provenance sits after </results>, before </sparql>.
        let after_results = text
            .split_once("</results>")
            .map(|(_, rest)| rest)
            .unwrap_or("");
        assert!(
            after_results.contains("<purrdf:provenance"),
            "provenance must follow </results>: {text}"
        );
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
        let err = to_xml(&result, &ResultProvenance::default()).expect_err("graph rejected");
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
        let err = to_xml(&result, &ResultProvenance::default())
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
