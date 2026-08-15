// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C SPARQL Results TSV serializer
//! (<https://www.w3.org/TR/sparql11-results-csv-tsv/>).
//!
//! TSV is defined **only** for SELECT variable bindings; ASK (`Boolean`) and
//! CONSTRUCT (`Graph`) have no TSV representation and hard-fail with
//! [`Error::Format`]. The header is the `?`-prefixed variable names, rows are
//! separated by LF, and each bound cell uses SPARQL/Turtle term syntax via the
//! kernel-backed [`crate::term::ntriples_token`] (`<iri>`, `"lex"` /
//! `"lex"@lang` / `"lex"^^<dt>`, `_:label`, or the non-asserting triple term
//! `<<( … )>>`). Unbound cells are the
//! empty string. Tab/newline characters inside literals are escaped by the
//! kernel's literal emitter, so no field-level quoting is needed for bound
//! values.
//!
//! TSV has NO quoting mechanism at all, unlike CSV's RFC-4180 quoting — so a
//! variable NAME containing a raw tab/CR/LF (which would silently shift a
//! later column, or start a phantom record) is a hard [`Error::Format`] at
//! the header rather than emitted raw; see [`to_tsv`].
//!
//! TSV is a flat exit gate with no extension point: a populated
//! [`ResultProvenance`] is trimmed and signalled via
//! [`SerializeOutcome::provenance_dropped`].

use crate::SerializeOutcome;
use crate::error::Error;
use crate::model::ResultProvenance;
use crate::term::ntriples_token;
use purrdf_core::SparqlResult;

/// Serialize a [`SparqlResult`] to W3C SPARQL Results TSV.
///
/// # Errors
///
/// Returns [`Error::Format`] for `Boolean` (ASK) and `Graph` (CONSTRUCT)
/// results, which W3C TSV does not define, and for a variable name that
/// contains a literal tab, CR, or LF — TSV has no quoting mechanism to escape
/// one inside the header, so emitting it raw would silently corrupt the
/// column/record structure a reader relies on.  Returns
/// [`Error::MalformedTerm`] if any solution row contains more bindings than
/// there are projected variables (over-wide rows are an invariant violation;
/// short rows are intentional and padded with empty fields for unbound
/// variables), or if a triple-term cell (at any nesting depth) carries a
/// predicate that is not an IRI — RDF 1.2 requires predicates to be IRIs, so a
/// malformed predicate is rejected rather than laundered into a fabricated
/// IRI cell.
pub fn to_tsv(
    result: &SparqlResult,
    provenance: &ResultProvenance,
) -> Result<SerializeOutcome, Error> {
    let (variables, rows) = match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        SparqlResult::Boolean(_) => {
            return Err(Error::Format(
                "SPARQL Results TSV is defined only for SELECT variable bindings, not ASK"
                    .to_string(),
            ));
        }
        SparqlResult::Graph(_) => {
            return Err(Error::Format(
                "SPARQL Results TSV is defined only for SELECT variable bindings, not CONSTRUCT graphs"
                    .to_string(),
            ));
        }
    };

    let mut out = String::new();

    // Header: `?`-prefixed variable names, tab-separated, LF-terminated.
    for (i, var) in variables.iter().enumerate() {
        if i > 0 {
            out.push('\t');
        }
        push_var_header(var, &mut out)?;
    }
    out.push('\n');

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
                out.push('\t');
            }
            if let Some(Some(value)) = row.get(column) {
                out.push_str(&ntriples_token(value)?);
            }
            // None or missing column → empty field.
        }
        out.push('\n');
    }

    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: !provenance.is_empty(),
    })
}

/// Append one `?`-prefixed TSV header field.
///
/// # Errors
///
/// Returns [`Error::Format`] if `var` contains a literal tab, CR, or LF. TSV
/// (unlike CSV) has no quoting mechanism, so those bytes cannot be escaped —
/// emitting one raw would either shift every later header field (a tab) or
/// start a phantom header record (a CR/LF), which no conformant reader could
/// distinguish from the caller's intended column/row boundary. This mirrors
/// the XML/JSON writers, which reject/escape the same class of caller-
/// controlled structural character rather than splicing it in unescaped.
fn push_var_header(var: &str, out: &mut String) -> Result<(), Error> {
    if var.contains(['\t', '\n', '\r']) {
        return Err(Error::Format(format!(
            "variable name {var:?} contains a tab/CR/LF, which SPARQL Results TSV \
             has no way to escape in the header"
        )));
    }
    out.push('?');
    out.push_str(var);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SolutionProvenance;
    use pretty_assertions::assert_eq;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm, TermValue};

    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

    fn tsv_outcome(result: &SparqlResult, prov: &ResultProvenance) -> SerializeOutcome {
        to_tsv(result, prov).expect("serialization succeeds")
    }

    fn tsv_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        String::from_utf8(tsv_outcome(result, prov).bytes).expect("UTF-8 output")
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
    fn select_full_shape_lf_and_question_header() {
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
            "?s\t?b\t?name\t?age\t?label\n",
            "<http://example.org/s>\t_:b0\t\"Ada\"\t\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>\t\"bonjour\"@fr\n",
            "<http://example.org/s2>\t\t\"Bob\"\t\t\"Grace\"\n",
        );
        assert_eq!(tsv_text(&result, &ResultProvenance::default()), expected);
    }

    #[test]
    fn typed_and_lang_tokens() {
        let result = SparqlResult::Solutions {
            variables: vec!["n".to_string(), "g".to_string()],
            rows: vec![vec![
                Some(lit("5", XSD_INTEGER)),
                Some(TermValue::Literal {
                    lexical_form: "x".to_string(),
                    datatype: RDF_LANGSTRING.to_string(),
                    language: Some("en".to_string()),
                    direction: None,
                }),
            ]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let text = tsv_text(&result, &ResultProvenance::default());
        assert!(
            text.contains("\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
            "typed token: {text}"
        );
        assert!(text.contains("\"x\"@en"), "lang token: {text}");
    }

    #[test]
    fn triple_term_literal_predicate_is_malformed_term_error() {
        let triple = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(lit("not-a-predicate", XSD_STRING)),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("literal predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn triple_term_blank_predicate_is_malformed_term_error() {
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
        let err = to_tsv(&result, &ResultProvenance::default())
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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let outer = TermValue::Triple {
            s: Box::new(inner),
            p: Box::new(TermValue::Iri("http://example.org/concludes".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o2".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(outer)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("nested malformed predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn boolean_is_format_error() {
        let err = to_tsv(&SparqlResult::Boolean(false), &ResultProvenance::default())
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
        let err = to_tsv(&SparqlResult::Graph(dataset), &ResultProvenance::default())
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
        let expected = concat!("?a\t?b\t?c\n", "<http://example.org/x>\t\t\n",);
        assert_eq!(tsv_text(&result, &ResultProvenance::default()), expected);
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
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("over-wide row must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn variable_name_with_tab_is_format_error() {
        // TSV has no quoting mechanism: a raw tab in a variable name would
        // silently shift every later header field, so it must hard-fail
        // rather than be emitted verbatim.
        let result = SparqlResult::Solutions {
            variables: vec!["a\tb".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/x".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("tab in variable name must be rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
    }

    #[test]
    fn variable_name_with_newline_is_format_error() {
        let result = SparqlResult::Solutions {
            variables: vec!["a\nb".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/x".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("newline in variable name must be rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
    }

    #[test]
    fn variable_name_with_cr_is_format_error() {
        let result = SparqlResult::Solutions {
            variables: vec!["a\rb".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/x".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
        };
        let err = to_tsv(&result, &ResultProvenance::default())
            .expect_err("CR in variable name must be rejected");
        assert!(matches!(err, Error::Format(_)), "expected Format: {err:?}");
    }

    #[test]
    fn populated_provenance_drops() {
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
        let outcome = tsv_outcome(&result, &provenance);
        assert!(outcome.provenance_dropped, "expected provenance_dropped");
        let text = String::from_utf8(outcome.bytes).expect("UTF-8");
        assert!(!text.contains("purrdf"), "TSV must stay pure W3C: {text}");
        assert_eq!(text, "?s\n<http://example.org/s>\n");
    }
}
