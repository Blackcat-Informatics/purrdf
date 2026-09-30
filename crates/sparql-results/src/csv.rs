// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! W3C SPARQL Results CSV serializer
//! (<https://www.w3.org/TR/sparql11-results-csv-tsv/>).
//!
//! CSV is defined **only** for SELECT variable bindings; ASK (`Boolean`) and
//! CONSTRUCT (`Graph`) have no CSV representation and hard-fail with
//! [`Error::Format`]. The header is the bare variable names (no `?`), records are
//! separated by CRLF per the RFC 4180 reference, and each cell is the "value":
//! the IRI string, the literal lexical form, `_:label` for a blank node, or the
//! N-Triples non-asserting triple-term token `<<( … )>>` (CSV predates RDF-1.2).
//!
//! CSV is a flat exit gate: it has no extension point, so a populated
//! [`ResultProvenance`] is trimmed and the drop is signalled via
//! [`SerializeOutcome::provenance_dropped`] (the no-silent-cap contract).

use crate::SerializeOutcome;
use crate::error::Error;
use crate::model::ResultProvenance;
use crate::term::ntriples_token;
use purrdf_core::csv::{Dialect, FieldWriter};
use purrdf_core::sink::TextOut;
use purrdf_core::{SparqlResult, TermValue};

/// The one RFC 4180 writer, under the SPARQL Results preset: `"`, `,`, LINE
/// FEED and CARRIAGE RETURN quote a field, a `"` inside one is doubled, and
/// records end in CRLF.
const FIELDS: FieldWriter = FieldWriter::new(Dialect::SPARQL_RESULTS);

/// Serialize a [`SparqlResult`] to W3C SPARQL Results CSV.
///
/// # Examples
///
/// The header is the bare variable names and records are CRLF-separated:
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_sparql_results::{ResultProvenance, SparqlResult, to_csv};
///
/// let result = SparqlResult::Solutions {
///     variables: vec!["s".to_string()],
///     rows: vec![vec![Some(TermValue::Iri("http://example.org/s".to_string()))]],
///     aux: RdfDatasetBuilder::new().freeze().expect("empty aux dataset"),
/// };
///
/// let csv = to_csv(&result, &ResultProvenance::default()).expect("SELECT serializes to CSV");
/// assert_eq!(csv.bytes, b"s\r\nhttp://example.org/s\r\n");
/// ```
///
/// # Errors
///
/// Returns [`Error::Format`] for `Boolean` (ASK) and `Graph` (CONSTRUCT)
/// results, which W3C CSV does not define.  Returns [`Error::MalformedTerm`]
/// if any solution row contains more bindings than there are projected
/// variables (over-wide rows are an invariant violation; short rows are
/// intentional and padded with empty fields for unbound variables), or if a
/// triple-term cell (at any nesting depth) carries a predicate that is not an
/// IRI — RDF 1.2 requires predicates to be IRIs, so a malformed predicate is
/// rejected rather than laundered into a fabricated IRI cell.
pub fn to_csv(
    result: &SparqlResult,
    provenance: &ResultProvenance,
) -> Result<SerializeOutcome, Error> {
    let mut out = String::new();
    write_csv(result, provenance, &mut out)?;
    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: !provenance.is_empty(),
    })
}

/// The ONE TO_CSV body. The whole-`String` spelling above is this function
/// over a `String`; the streaming entry point is the same function over a bounded
/// sink, which is what makes their bytes equal by construction.
pub(crate) fn write_csv<W: TextOut + ?Sized>(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    out: &mut W,
) -> Result<(), Error> {
    let _ = provenance;

    let (variables, rows) = match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        SparqlResult::Boolean(_) => {
            return Err(Error::Format(
                "SPARQL Results CSV is defined only for SELECT variable bindings, not ASK"
                    .to_string(),
            ));
        }
        SparqlResult::Graph(_) => {
            return Err(Error::Format(
                "SPARQL Results CSV is defined only for SELECT variable bindings, not CONSTRUCT graphs"
                    .to_string(),
            ));
        }
    };

    // The over-wide-row refusal is decided BEFORE the first byte. Eagerly the partial
    // document was discarded; an incremental sink has already sent it. The scan picks
    // the same first offending row the interleaved check did — iteration order is
    // unchanged — so the reported error is identical.
    for row in rows {
        if row.len() > variables.len() {
            return Err(Error::MalformedTerm(format!(
                "solution row has {} bindings but only {} variables are projected",
                row.len(),
                variables.len()
            )));
        }
    }

    // Header: bare variable names, comma-separated, CRLF-terminated.
    FIELDS.write_record(variables, out);

    for row in rows {
        for column in 0..variables.len() {
            if column > 0 {
                FIELDS.write_delimiter(out);
            }
            if let Some(Some(value)) = row.get(column) {
                FIELDS.write_field(cell_value(value)?.as_ref(), out);
            }
            // None or missing column → empty field (nothing emitted between separators).
        }
        FIELDS.end_record(out);
    }

    Ok(())
}

/// The bare CSV "value" for a bound term.
///
/// Returns a [`std::borrow::Cow`] to avoid cloning the lexical string for the
/// two common cases (IRI and Literal); only blank-node labels and triple terms
/// require an owned allocation.
///
/// # Errors
///
/// Returns [`Error::MalformedTerm`] when `value` is a triple term (at any
/// nesting depth) whose predicate is not an IRI — RDF 1.2 requires predicates
/// to be IRIs, so this is rejected rather than laundered into a fabricated
/// IRI cell.
fn cell_value(value: &TermValue) -> Result<std::borrow::Cow<'_, str>, Error> {
    use std::borrow::Cow;
    Ok(match value {
        TermValue::Iri(iri) => Cow::Borrowed(iri),
        TermValue::Literal { lexical_form, .. } => Cow::Borrowed(lexical_form),
        // The kernel token is `_:{qualified-label}`, escaped into the Turtle
        // BLANK_NODE_LABEL alphabet so the cell re-lexes.
        TermValue::Blank { .. } => Cow::Owned(ntriples_token(value)?),
        // CSV predates RDF-1.2; the N-Triples token is a reasonable rendering.
        TermValue::Triple { .. } => Cow::Owned(ntriples_token(value)?),
    })
}

/// The two-pass field writer the shared one-scan writer replaced, kept as the
/// oracle.
#[cfg(test)]
fn push_field_reference<W: TextOut + ?Sized>(value: &str, out: &mut W) {
    let needs_quoting = value
        .chars()
        .any(|c| c == '"' || c == ',' || c == '\n' || c == '\r');
    if !needs_quoting {
        out.push_str(value);
        return;
    }
    out.push('"');
    for ch in value.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SolutionProvenance;
    use purrdf_core::TermBox;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm};
    use purrdf_testkit::rng::SplitMix64;

    use purrdf_core::datatype::XSD_INTEGER;
    use purrdf_core::vocab::rdf::LANG_STRING as RDF_LANGSTRING;

    fn csv_outcome(result: &SparqlResult, prov: &ResultProvenance) -> SerializeOutcome {
        to_csv(result, prov).expect("serialization succeeds")
    }

    fn csv_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        String::from_utf8(csv_outcome(result, prov).bytes).expect("UTF-8 output")
    }

    fn lit(lex: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lex.to_string(),
            datatype: datatype.to_string(),
            language: None,
            direction: None,
        }
    }

    /// The one-scan field writer agrees with the two-pass one on every quoting
    /// byte, runs of adjacent quotes, the other controls, and non-ASCII in every
    /// UTF-8 width, at lengths 0-70 and past several chunks.
    #[test]
    fn the_shared_field_writer_agrees_with_the_two_pass_writer() {
        const SCALARS: &[char] = &[
            '"',
            ',',
            '\n',
            '\r',
            '\t',
            ' ',
            ';',
            '\'',
            '\\',
            '\u{0}',
            '\u{7F}',
            '\u{85}',
            '\u{E9}',
            '\u{2028}',
            '\u{FFFD}',
            '\u{1F408}',
        ];
        let mut rng = SplitMix64::new(0x0C5F_1E1D_0000_0001);
        let (mut quoted, mut raw) = (0_usize, 0_usize);
        for len in (0..=70).chain([127, 128, 129, 1000, 4099]) {
            for round in 0..40 {
                let density = if round % 2 == 0 { 3 } else { 60 };
                let value: String = (0..len)
                    .map(|_| {
                        if rng.below_usize(density) == 0 {
                            SCALARS[rng.below_usize(SCALARS.len())]
                        } else {
                            'v'
                        }
                    })
                    .collect();
                let (mut got, mut expected) = (String::new(), String::new());
                FIELDS.write_field(&value, &mut got);
                push_field_reference(&value, &mut expected);
                assert_eq!(got, expected, "{value:?}");
                if got == value {
                    raw += 1;
                } else {
                    quoted += 1;
                }
            }
        }
        assert!(quoted > 0 && raw > 0, "{quoted} {raw}");
        // Fixed edges: a lone quote, quotes at both ends, and the neighbour
        // with no quoting byte, which is written raw.
        for (value, expected) in [
            ("\"", "\"\"\"\""),
            ("\"a\"", "\"\"\"a\"\"\""),
            ("a,b", "\"a,b\""),
            ("a\r\nb", "\"a\r\nb\""),
            ("plain value", "plain value"),
        ] {
            let mut got = String::new();
            FIELDS.write_field(value, &mut got);
            assert_eq!(got, expected, "{value:?}");
        }
    }

    #[test]
    fn select_full_shape_crlf() {
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
            "s,b,name,age,label\r\n",
            "http://example.org/s,_:b0,Ada,42,bonjour\r\n",
            "http://example.org/s2,,Bob,,Grace\r\n",
        );
        assert_eq!(csv_text(&result, &ResultProvenance::default()), expected);
    }

    use purrdf_core::datatype::XSD_STRING;

    #[test]
    fn rfc4180_quoting() {
        let result = SparqlResult::Solutions {
            variables: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            rows: vec![vec![
                Some(lit("has, comma", XSD_STRING)),
                Some(lit("has \"quote\"", XSD_STRING)),
                Some(lit("line\nbreak", XSD_STRING)),
            ]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let expected = concat!(
            "a,b,c\r\n",
            "\"has, comma\",\"has \"\"quote\"\"\",\"line\nbreak\"\r\n",
        );
        assert_eq!(csv_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn triple_term_uses_ntriples_token() {
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        // The token contains spaces but no quoting trigger, so it is emitted raw.
        let expected = concat!(
            "t\r\n",
            "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>\r\n",
        );
        assert_eq!(csv_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn triple_term_literal_predicate_is_malformed_term_error() {
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(lit("not-a-predicate", XSD_STRING)),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_csv(&result, &ResultProvenance::default())
            .expect_err("literal predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn triple_term_blank_predicate_is_malformed_term_error() {
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_csv(&result, &ResultProvenance::default())
            .expect_err("blank-node predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn nested_triple_term_non_iri_predicate_is_malformed_term_error() {
        // The inner triple term (used as the outer subject) carries a
        // non-IRI predicate; the outer triple term's own predicate is fine.
        let inner = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let outer = TermValue::Triple {
            s: TermBox::new(inner),
            p: TermBox::new(TermValue::Iri("http://example.org/concludes".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o2".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(outer)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_csv(&result, &ResultProvenance::default())
            .expect_err("nested malformed predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn boolean_is_format_error() {
        let err = to_csv(&SparqlResult::Boolean(true), &ResultProvenance::default())
            .expect_err("ask rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
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
        let err = to_csv(&SparqlResult::Graph(dataset), &ResultProvenance::default())
            .expect_err("graph rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
    }

    #[test]
    fn short_row_pads_trailing_unbound() {
        // Row has only 1 bound cell for 3 variables; trailing 2 fields must be empty.
        let result = SparqlResult::Solutions {
            variables: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/x".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let expected = concat!("a,b,c\r\n", "http://example.org/x,,\r\n",);
        assert_eq!(csv_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn over_wide_row_is_malformed_error() {
        // 1 variable but row supplies 2 bound cells — over-wide rows must hard-fail.
        let iri = TermValue::Iri("http://example.org/x".to_string());
        let result = SparqlResult::Solutions {
            variables: vec!["a".to_string()],
            rows: vec![vec![Some(iri.clone()), Some(iri)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_csv(&result, &ResultProvenance::default())
            .expect_err("over-wide row must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn populated_provenance_drops_and_stays_pure() {
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
        let outcome = csv_outcome(&result, &provenance);
        assert!(outcome.provenance_dropped, "expected provenance_dropped");
        let text = String::from_utf8(outcome.bytes).expect("UTF-8");
        assert!(!text.contains("purrdf"), "CSV must stay pure W3C: {text}");
        assert!(!text.contains("deadbeef"), "no provenance leak: {text}");
        assert_eq!(text, "s\r\nhttp://example.org/s\r\n");
    }
}
