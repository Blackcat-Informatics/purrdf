// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C SPARQL Results CSV serializer
//! (<https://www.w3.org/TR/sparql11-results-csv-tsv/>).
//!
//! CSV is defined **only** for SELECT variable bindings; ASK (`Boolean`) and
//! CONSTRUCT (`Graph`) have no CSV representation and hard-fail with
//! [`Error::Format`]. The header is the bare variable names (no `?`), records are
//! separated by CRLF per the RFC 4180 reference, and each cell is the "value":
//! the IRI string, the literal lexical form, `_:label` for a blank node, or the
//! N-Triples `<< … >>` token for a triple term (CSV predates RDF-1.2).
//!
//! CSV is a flat exit gate: it has no extension point, so a populated
//! [`ResultProvenance`] is trimmed and the drop is signalled via
//! [`SerializeOutcome::provenance_dropped`] (the no-silent-cap contract).

use crate::error::Error;
use crate::model::ResultProvenance;
use crate::term::ntriples_token;
use crate::SerializeOutcome;
use purrdf_core::{SparqlResult, TermValue};

/// Serialize a [`SparqlResult`] to W3C SPARQL Results CSV.
///
/// # Errors
///
/// Returns [`Error::Format`] for `Boolean` (ASK) and `Graph` (CONSTRUCT)
/// results, which W3C CSV does not define.  Returns [`Error::MalformedTerm`]
/// if any solution row contains more bindings than there are projected
/// variables (over-wide rows are an invariant violation; short rows are
/// intentional and padded with empty fields for unbound variables).
pub fn to_csv(
    result: &SparqlResult,
    provenance: &ResultProvenance,
) -> Result<SerializeOutcome, Error> {
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

    let mut out = String::new();

    // Header: bare variable names, comma-separated, CRLF-terminated.
    for (i, var) in variables.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_field(var, &mut out);
    }
    out.push_str("\r\n");

    for row in rows {
        if row.len() > variables.len() {
            return Err(Error::MalformedTerm(format!(
                "solution row has {} bindings but only {} variables are projected",
                row.len(),
                variables.len()
            )));
        }
        for column in 0..variables.len() {
            if column > 0 {
                out.push(',');
            }
            if let Some(Some(value)) = row.get(column) {
                push_field(cell_value(value).as_ref(), &mut out);
            }
            // None or missing column → empty field (nothing emitted between separators).
        }
        out.push_str("\r\n");
    }

    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: !provenance.is_empty(),
    })
}

/// The bare CSV "value" for a bound term.
///
/// Returns a [`std::borrow::Cow`] to avoid cloning the lexical string for the
/// two common cases (IRI and Literal); only blank-node labels and triple terms
/// require an owned allocation.
fn cell_value(value: &TermValue) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    match value {
        TermValue::Iri(iri) => Cow::Borrowed(iri),
        TermValue::Literal { lexical_form, .. } => Cow::Borrowed(lexical_form),
        TermValue::Blank { label, .. } => Cow::Owned(format!("_:{label}")),
        // CSV predates RDF-1.2; the N-Triples token is a reasonable rendering.
        TermValue::Triple { .. } => Cow::Owned(ntriples_token(value)),
    }
}

/// Append a single CSV field, applying RFC-4180 quoting only when required:
/// a value containing `"`, `,`, `\n`, or `\r` is wrapped in double quotes with
/// internal `"` doubled; otherwise it is emitted raw.
fn push_field(value: &str, out: &mut String) {
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
    use pretty_assertions::assert_eq;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm};

    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

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

    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        // The token contains spaces but no quoting trigger, so it is emitted raw.
        let expected = concat!(
            "t\r\n",
            "<< <http://example.org/s> <http://example.org/p> <http://example.org/o> >>\r\n",
        );
        assert_eq!(csv_text(&result, &ResultProvenance::default()), expected);
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
