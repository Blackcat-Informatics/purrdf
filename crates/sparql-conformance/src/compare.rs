// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Comparing a run outcome against the manifest's expected result.
//!
//! * `SELECT` → the solution sequence as a **multiset** of bound (var, term)
//!   mappings (W3C solution-set equality).
//! * `ASK` → boolean equality.
//! * `CONSTRUCT`/`DESCRIBE` → canonical (RDFC-1.0) N-Quads equality.
//! * syntax tests → parse success/failure matches the kind.
//!
//! Blank-node isomorphism is **not** performed: results containing blank nodes
//! are compared by label and any genuine bnode-labelling difference must be
//! recorded in [`crate::xfail`] rather than masked.

use std::collections::BTreeMap;
use std::path::Path;

use purrdf_core::{SparqlResult, TermValue};
use purrdf_sparql_results::ParsedSolutions;

use crate::manifest::{ExpectedResult, SparqlTestCase, TestKind};
use crate::run::RunOutcome;

/// Compare `outcome` against `case`'s expected result.
///
/// # Errors
///
/// Returns a human-readable mismatch description; `Ok(())` means the case passed.
pub fn compare(case: &SparqlTestCase, outcome: &RunOutcome) -> Result<(), String> {
    match outcome {
        RunOutcome::Syntax { parsed_ok } => match case.kind {
            TestKind::PositiveSyntax if *parsed_ok => Ok(()),
            TestKind::PositiveSyntax => Err("positive-syntax test failed to parse".to_owned()),
            TestKind::NegativeSyntax if !*parsed_ok => Ok(()),
            TestKind::NegativeSyntax => {
                Err("negative-syntax test parsed but should have failed".to_owned())
            }
            other => Err(format!("syntax outcome for non-syntax kind {other:?}")),
        },
        RunOutcome::Eval(result) => compare_eval(case, result),
    }
}

/// Compare an evaluation result against the expected result file.
fn compare_eval(case: &SparqlTestCase, result: &SparqlResult) -> Result<(), String> {
    match (&case.expected, result) {
        (ExpectedResult::Srx(path) | ExpectedResult::Srj(path), SparqlResult::Boolean(actual)) => {
            let expected = read_boolean(path, matches!(case.expected, ExpectedResult::Srj(_)))?;
            if expected == *actual {
                Ok(())
            } else {
                Err(format!("ASK mismatch: expected {expected}, got {actual}"))
            }
        }
        (
            ExpectedResult::Srx(path) | ExpectedResult::Srj(path),
            SparqlResult::Solutions {
                variables, rows, ..
            },
        ) => {
            let expected = read_solutions(path, matches!(case.expected, ExpectedResult::Srj(_)))?;
            compare_solutions(variables, rows, &expected)
        }
        (ExpectedResult::Graph(path), SparqlResult::Graph(actual)) => {
            let expected_bytes =
                std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let media = media_type_of(path);
            let expected = purrdf::parse_dataset(&expected_bytes, media, None)
                .map_err(|e| format!("parse expected graph {}: {e}", path.display()))?;
            let actual_canon = purrdf_core::canonicalize(actual).nquads;
            let expected_canon = purrdf_core::canonicalize(&expected).nquads;
            if actual_canon == expected_canon {
                Ok(())
            } else {
                Err("CONSTRUCT graph differs from expected (canonical N-Quads mismatch)".to_owned())
            }
        }
        (ExpectedResult::Unsupported(path), _) => Err(format!(
            "unsupported expected-result format: {}",
            path.display()
        )),
        (ExpectedResult::None, _) => Err("evaluation case has no expected result".to_owned()),
        (expected, actual) => Err(format!(
            "result-kind mismatch: expected {expected:?}, got a {}",
            result_kind(actual)
        )),
    }
}

/// Compare a native solution sequence against the expected one as a multiset of
/// bound (variable, term) mappings.
fn compare_solutions(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    expected: &ParsedSolutions,
) -> Result<(), String> {
    let actual_keys = solution_multiset(variables, rows);
    let expected_keys = solution_multiset(&expected.variables, &expected.rows);
    if actual_keys == expected_keys {
        Ok(())
    } else {
        Err(format!(
            "solution multiset mismatch: {} expected rows vs {} actual rows",
            expected.rows.len(),
            rows.len()
        ))
    }
}

/// A canonical, sorted multiset of solution rows. Each row is the sorted list of
/// its bound `var=term` pairs (unbound cells omitted), so variable *order* and
/// unbound columns do not affect equality (W3C solution-set semantics).
fn solution_multiset(variables: &[String], rows: &[Vec<Option<TermValue>>]) -> Vec<String> {
    let mut out: Vec<String> = rows
        .iter()
        .map(|row| {
            let mut pairs: BTreeMap<&str, String> = BTreeMap::new();
            for (var, cell) in variables.iter().zip(row) {
                if let Some(term) = cell {
                    pairs.insert(var, term_key(term));
                }
            }
            pairs
                .into_iter()
                .map(|(v, t)| format!("{v}={t}"))
                .collect::<Vec<_>>()
                .join("\u{1f}")
        })
        .collect();
    out.sort();
    out
}

/// Field separator used inside a Literal key.  US (U+001F) cannot appear in
/// any RDF lexical form, datatype IRI, or language tag, so it is collision-free
/// as an internal delimiter while the row-level separator at the call site also
/// uses U+001F.  The type-prefix character (`L:`, `I:`, etc.) keeps variants
/// mutually unambiguous even without an additional guard byte.
const FIELD_SEP: char = '\u{1f}';

/// A canonical string key for a term value.
fn term_key(term: &TermValue) -> String {
    match term {
        TermValue::Iri(i) => format!("I:{i}"),
        TermValue::Blank { label, .. } => format!("B:{label}"),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => format!(
            "L:{lexical_form}{FIELD_SEP}{datatype}{FIELD_SEP}{}{FIELD_SEP}{}",
            language.as_deref().unwrap_or(""),
            direction.map(|d| d.as_str()).unwrap_or("")
        ),
        TermValue::Triple { s, p, o } => {
            format!("T:({} {} {})", term_key(s), term_key(p), term_key(o))
        }
    }
}

/// Read an expected SELECT result file (SRX or SRJ).
fn read_solutions(path: &Path, json: bool) -> Result<ParsedSolutions, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if json {
        purrdf_sparql_results::from_json(&bytes)
    } else {
        purrdf_sparql_results::from_xml(&bytes)
    }
    .map_err(|e| format!("parse expected results {}: {e}", path.display()))
}

/// Read an expected ASK boolean file (SRX or SRJ).
fn read_boolean(path: &Path, json: bool) -> Result<bool, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if json {
        purrdf_sparql_results::from_json_boolean(&bytes)
    } else {
        purrdf_sparql_results::from_xml_boolean(&bytes)
    }
    .map_err(|e| format!("parse expected boolean {}: {e}", path.display()))
}

/// Map a result file's extension to a native RDF media type.
fn media_type_of(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("nt") => "application/n-triples",
        Some("nq") => "application/n-quads",
        Some("rdf") => "application/rdf+xml",
        _ => "text/turtle",
    }
}

/// A short label for a result kind, for diagnostics.
fn result_kind(result: &SparqlResult) -> &'static str {
    match result {
        SparqlResult::Solutions { .. } => "SELECT solutions",
        SparqlResult::Boolean(_) => "ASK boolean",
        SparqlResult::Graph(_) => "graph",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(lexical_form: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical_form.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn term_key_pipe_in_lexical_form_does_not_collide_with_pipe_in_datatype() {
        // Prior to the fix, `L:a|b|dt||` and `L:a|b|dt||` were the same key
        // for two structurally distinct literals.  With FIELD_SEP = U+001F the
        // keys are distinct because the separator byte cannot appear in either
        // field.
        let key1 = term_key(&lit("a|b", "dt"));
        let key2 = term_key(&lit("a", "b|dt"));
        assert_ne!(
            key1, key2,
            "literals with | in different fields must produce different keys"
        );
    }

    #[test]
    fn term_key_iri_cannot_collide_with_literal() {
        // An IRI key starts with `I:` and a Literal key starts with `L:`, so
        // they can never be equal regardless of content.
        let iri_key = term_key(&TermValue::Iri("L:something".to_owned()));
        let lit_key = term_key(&lit("something", "dt"));
        assert_ne!(iri_key, lit_key, "IRI and Literal keys must stay distinct");
    }
}
