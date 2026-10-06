// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Comparing a run outcome against the manifest's expected result.
//!
//! * `SELECT` → the solution sequence compared with a **single global
//!   blank-node bijection** over the whole result set (W3C solution-set
//!   equality), as a multiset when there is no top-level `ORDER BY` and as an
//!   ordered sequence when there is (see the private `compare_solutions`).
//! * `ASK` → boolean equality.
//! * `CONSTRUCT`/`DESCRIBE` → canonical (RDFC-1.0) N-Quads equality.
//! * syntax tests → parse success/failure matches the kind.
//!
//! Blank-node equality reuses the same RDFC-1.0 canonicalizer the
//! CONSTRUCT/UPDATE dataset comparison is built on ([`purrdf_core::canonicalize`]):
//! the **entire** result set is encoded as ONE synthetic dataset (a distinct
//! solution blank node per row, one quad `(solution, var-predicate, value)` per
//! bound cell) and canonicalized once, so a single bijection must map every
//! blank node across every row at once (never a looser per-row bijection) while
//! non-blank terms — IRIs, literals including datatype/language/base-direction,
//! and their variable positions — still compare exactly. See
//! the private `encode_solution_set`.

use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, Nested, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId, TermValue,
    try_fold_nested,
};
use purrdf_sparql_results::ParsedSolutions;

use crate::manifest::{ExpectedResult, SparqlTestCase, TestKind};
use crate::run::RunOutcome;

/// Compare `outcome` against `case`'s expected result.
///
/// # Errors
///
/// Returns a human-readable mismatch description; `Ok(())` means the case passed.
pub fn compare(case: &SparqlTestCase, outcome: &RunOutcome) -> Result<(), String> {
    // A case that expects a hard failure produced an outcome instead. Checked here,
    // before any kind-specific comparison, so the verdict is the same sentence
    // whether the case was a query, an update, or a syntax test — and so the
    // expectation can never be satisfied by a run that quietly succeeded.
    if let ExpectedResult::EvalError(path) = &case.expected {
        return Err(format!(
            "expected a hard failure matching {} — the case ran to completion instead",
            path.display()
        ));
    }
    match outcome {
        RunOutcome::Syntax { parsed_ok } => match case.kind {
            TestKind::PositiveSyntax | TestKind::PositiveUpdateSyntax if *parsed_ok => Ok(()),
            TestKind::PositiveSyntax | TestKind::PositiveUpdateSyntax => {
                Err("positive-syntax test failed to parse".to_owned())
            }
            TestKind::NegativeSyntax | TestKind::NegativeUpdateSyntax if !*parsed_ok => Ok(()),
            TestKind::NegativeSyntax | TestKind::NegativeUpdateSyntax => {
                Err("negative-syntax test parsed but should have failed".to_owned())
            }
            other => Err(format!("syntax outcome for non-syntax kind {other:?}")),
        },
        RunOutcome::Eval { result, ordered } => compare_eval(case, result, *ordered),
        RunOutcome::Update(actual) => compare_update(case, actual),
    }
}

/// Grade the diagnostic a case FAILED with against its expectation.
///
/// A case whose `mf:result` is a `.err` file
/// ([`ExpectedResult::EvalError`]) expects the run to refuse: every non-empty,
/// non-comment line of that file is a substring the diagnostic must contain, and all
/// of them must appear. Substrings rather than a whole-message match, so the
/// expectation pins the *reason* (which relation, which mode, which position) without
/// freezing incidental framing like the case IRI a caller prefixes; several of them,
/// so a one-word expectation cannot pass by accident.
///
/// A case that expected an ordinary result gets its diagnostic back unchanged, which
/// is exactly the failure it always was — so this is the single seam through which a
/// run failure becomes a verdict.
///
/// # Errors
///
/// The message to report as the case's failure: the original diagnostic when no
/// failure was expected, or a description of how the diagnostic missed the
/// expectation.
pub fn compare_failure(case: &SparqlTestCase, message: &str) -> Result<(), String> {
    let ExpectedResult::EvalError(path) = &case.expected else {
        return Err(message.to_owned());
    };
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("read expected-error file {}: {e}", path.display()))?;
    let expectations: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if expectations.is_empty() {
        return Err(format!(
            "expected-error file {} names no expectation, so it would accept ANY failure",
            path.display()
        ));
    }
    for expectation in expectations {
        if !message.contains(expectation) {
            return Err(format!(
                "the failure diagnostic does not contain the expected text\n    \
                 expected: {expectation}\n    diagnostic: {message}"
            ));
        }
    }
    Ok(())
}

/// Compare two SPARQL results for equality.
///
/// `SELECT` solutions are compared as a multiset when `ordered` is `false` and as an
/// ordered sequence when `ordered` is `true`. `ASK` booleans are compared directly.
/// `CONSTRUCT`/`DESCRIBE` graphs are compared by RDFC-1.0 canonical N-Quads.
///
/// # Errors
///
/// Returns a human-readable mismatch description; `Ok(())` means the results are
/// equal under the chosen comparison.
pub fn compare_results(
    left: &SparqlResult,
    right: &SparqlResult,
    ordered: bool,
) -> Result<(), String> {
    match (left, right) {
        (SparqlResult::Boolean(l), SparqlResult::Boolean(r)) if l == r => Ok(()),
        (SparqlResult::Boolean(l), SparqlResult::Boolean(r)) => {
            Err(format!("ASK mismatch: {l} vs {r}"))
        }
        (
            SparqlResult::Solutions {
                variables: l_vars,
                rows: l_rows,
                ..
            },
            SparqlResult::Solutions {
                variables: r_vars,
                rows: r_rows,
                ..
            },
        ) => {
            if l_vars != r_vars {
                return Err(format!("variable lists differ: {l_vars:?} vs {r_vars:?}"));
            }
            compare_solutions(
                l_vars,
                l_rows,
                &ParsedSolutions {
                    variables: r_vars.clone(),
                    rows: r_rows.clone(),
                },
                ordered,
            )
        }
        (SparqlResult::Graph(l), SparqlResult::Graph(r)) => {
            let l_canon = purrdf_core::canonicalize(l).nquads;
            let r_canon = purrdf_core::canonicalize(r).nquads;
            if l_canon == r_canon {
                Ok(())
            } else {
                Err("graph results differ (canonical N-Quads mismatch)".to_owned())
            }
        }
        (l, r) => Err(format!(
            "result kind mismatch: {} vs {}",
            l.query_form(),
            r.query_form()
        )),
    }
}

/// Compare an UPDATE post-state dataset against the expected [`ExpectedResult::DatasetState`].
///
/// The mutated dataset and the expected dataset are compared graph-preservingly
/// by RDFC-1.0 canonical N-Quads — the same equality CONSTRUCT graphs use — so a
/// differing default graph OR any named graph surfaces as a mismatch.
fn compare_update(case: &SparqlTestCase, actual: &Arc<RdfDataset>) -> Result<(), String> {
    let ExpectedResult::DatasetState { data, graph_data } = &case.expected else {
        return Err(format!(
            "update case {} has no DatasetState expected result",
            case.iri
        ));
    };
    let expected = crate::run::build_dataset(&case.base, data, graph_data)?;
    let actual_canon = purrdf_core::canonicalize(actual).nquads;
    let expected_canon = purrdf_core::canonicalize(&expected).nquads;
    if actual_canon == expected_canon {
        Ok(())
    } else {
        Err("UPDATE post-state differs from expected (canonical N-Quads mismatch)".to_owned())
    }
}

/// Compare an evaluation result against the expected result file.
///
/// `ordered` is `true` when the query has a top-level `ORDER BY` (§18.5): a
/// `SELECT`'s solution rows are then compared as an ordered *sequence* rather
/// than a multiset (see [`compare_solutions`]).
fn compare_eval(case: &SparqlTestCase, result: &SparqlResult, ordered: bool) -> Result<(), String> {
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
            compare_case_solutions(case, variables, rows, &expected, ordered)
        }
        (ExpectedResult::ResultSetRdf(path), result) => {
            let expected = crate::rs_resultset::parse_result(
                &case.base,
                crate::run::data_media_type(path),
                &std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?,
            )
            .map_err(|e| format!("parse expected rs:ResultSet {}: {e}", path.display()))?;
            match (expected, result) {
                (
                    crate::rs_resultset::RdfResult::Boolean(expected),
                    SparqlResult::Boolean(actual),
                ) => {
                    if expected == *actual {
                        Ok(())
                    } else {
                        Err(format!("ASK mismatch: expected {expected}, got {actual}"))
                    }
                }
                (
                    crate::rs_resultset::RdfResult::Solutions {
                        solutions,
                        ordered: indexed,
                    },
                    SparqlResult::Solutions {
                        variables, rows, ..
                    },
                ) => compare_case_solutions(case, variables, rows, &solutions, indexed),
                _ => Err("DAWG RDF result kind differs from the query result".to_owned()),
            }
        }
        (ExpectedResult::Graph(path), SparqlResult::Graph(actual)) => {
            let expected_bytes =
                std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let media = crate::run::data_media_type(path);
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
            "result-kind mismatch: expected {expected:?}, got a {} result",
            actual.query_form()
        )),
    }
}

/// Apply the manifest's explicit REDUCED cardinality rule, otherwise exact equality.
fn compare_case_solutions(
    case: &SparqlTestCase,
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    expected: &ParsedSolutions,
    ordered: bool,
) -> Result<(), String> {
    if !case.lax_cardinality {
        return compare_solutions(variables, rows, expected, ordered);
    }
    // The W3C test-case rules require each solution at least once and no more
    // often than its expected multiplicity. Match distinct rows with ONE global
    // blank-node bijection: ordered prefix comparison pins each chosen row pair,
    // so per-row aliases cannot conceal inconsistent cross-row coreference.
    // <https://www.w3.org/2009/sparql/docs/tests/README.html>
    let actual = row_multiplicities(rows);
    let wanted = row_multiplicities(&expected.rows);
    if actual.len() != wanted.len() {
        return Err("REDUCED result has missing or extra distinct solutions".to_owned());
    }
    let actual_rows: Vec<_> = actual.iter().map(|(row, _)| (*row).clone()).collect();
    let mut pending = vec![Vec::<usize>::new()];
    while let Some(selected) = pending.pop() {
        let at = selected.len();
        if at == actual.len() {
            return Ok(());
        }
        for (candidate, &(row, maximum)) in wanted.iter().enumerate() {
            if selected.contains(&candidate) || actual[at].1 > maximum {
                continue;
            }
            let mut paired: Vec<_> = selected
                .iter()
                .map(|&index| wanted[index].0.clone())
                .collect();
            paired.push(row.clone());
            let paired = ParsedSolutions {
                variables: expected.variables.clone(),
                rows: paired,
            };
            if compare_solutions(variables, &actual_rows[..=at], &paired, true).is_ok() {
                let mut next = selected.clone();
                next.push(candidate);
                pending.push(next);
            }
        }
    }
    Err("REDUCED multiplicities or global blank-node correspondence differ".to_owned())
}

/// Group equal rows without changing the identity of their value blank nodes.
fn row_multiplicities(rows: &[Vec<Option<TermValue>>]) -> Vec<(&Vec<Option<TermValue>>, usize)> {
    let mut groups: Vec<(&Vec<Option<TermValue>>, usize)> = Vec::new();
    for row in rows {
        if let Some((_, count)) = groups.iter_mut().find(|(value, _)| *value == row) {
            *count += 1;
        } else {
            groups.push((row, 1));
        }
    }
    groups
}

/// Compare a native solution sequence against the expected one under W3C
/// solution-set equality with a **single global blank-node bijection**.
///
/// Both sides are encoded as one synthetic dataset each (see
/// [`encode_solution_set`]) and canonicalized once; equality is byte-equality of
/// the two canonical N-Quads strings. When `ordered` is `true` (a top-level
/// `ORDER BY`) each row also carries an ordinal literal so position is pinned
/// into the canonical output — the comparison becomes an ordered-sequence match
/// while blank-node identity stays global-bijection-normalized.
fn compare_solutions(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    expected: &ParsedSolutions,
    ordered: bool,
) -> Result<(), String> {
    let actual_canon = encode_solution_set(variables, rows, ordered, Literals::ByValue)?;
    let expected_canon = encode_solution_set(
        &expected.variables,
        &expected.rows,
        ordered,
        Literals::ByValue,
    )?;
    if actual_canon == expected_canon {
        Ok(())
    } else {
        let mode = if ordered {
            "ordered sequence"
        } else {
            "multiset"
        };
        Err(format!(
            "solution {mode} mismatch: {} expected rows vs {} actual rows; first canonical difference: {}",
            expected.rows.len(),
            rows.len(),
            actual_canon
                .lines()
                .zip(expected_canon.lines())
                .find(|(actual, expected)| actual != expected)
                .map_or_else(
                    || "one result's canonical graph ends earlier".to_owned(),
                    |(actual, expected)| format!("expected {expected} / actual {actual}")
                )
        ))
    }
}

/// The reserved namespace for the synthetic terms [`encode_solution_set`]
/// mints. No real query result term can occupy `urn:purrdf:conformance:` and
/// `write_iri_escaped` escapes exotic variable names injectively, so the
/// per-variable predicate IRIs and the ordinal predicate cannot collide with,
/// or be forged from, result data.
const CONFORMANCE_NS: &str = "urn:purrdf:conformance:";

/// The scope every value blank node is interned under, so a blank node shared
/// across rows keeps ONE [`TermId`] and its global coreference survives into
/// the canonical form. Distinct from [`SOLUTION_SCOPE`] so a value blank can
/// never accidentally alias a synthetic solution blank.
const VALUE_SCOPE: BlankScope = BlankScope(1);

/// The scope the per-row synthetic *solution* blank nodes are interned under.
const SOLUTION_SCOPE: BlankScope = BlankScope(2);

/// `xsd:integer` — the datatype of the ordinal literal pinning row position in
/// the ordered-comparison encoding.
use purrdf_core::datatype::XSD_INTEGER;

/// Encode a whole solution set as canonical RDFC-1.0 N-Quads.
///
/// Every row becomes a distinct **solution blank node** (in [`SOLUTION_SCOPE`]);
/// each bound cell `(var, value)` becomes one quad
/// `(solution, urn:purrdf:conformance:var:<var>, value)`. Because the entire
/// set is one dataset canonicalized once, RDFC-1.0 must find a SINGLE bijection
/// mapping every blank node across every row simultaneously — so a result whose
/// blanks only line up row-by-row (but not globally) is correctly UNEQUAL.
/// Value blank nodes are interned in [`VALUE_SCOPE`] under an injective encoding
/// of their original `(scope, label)`,
/// so a blank shared across rows keeps one [`TermId`] and its coreference is
/// preserved; two structurally-identical rows produce two automorphic solution
/// blanks that RDFC-1.0 still emits as two lines, preserving multiplicity.
///
/// When `ordered`, each solution blank additionally gets
/// `(solution, urn:purrdf:conformance:index, "<i>"^^xsd:integer)`: the ordinal
/// literal is a ground term, so it distinguishes row `i` from row `j` in the
/// canonical output and the comparison becomes position-sensitive.
///
/// # Errors
///
/// Returns the freeze diagnostic if the interned terms do not form a
/// structurally valid dataset (e.g. an ill-formed triple term with a literal in
/// subject/predicate position from a crafted fixture) — propagated rather than
/// panicked, honoring the harness's never-panic intent.
fn encode_solution_set(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    ordered: bool,
    literals: Literals,
) -> Result<String, String> {
    let mut builder = RdfDatasetBuilder::new();
    let index_predicate = builder.intern_iri(&format!("{CONFORMANCE_NS}index"));
    for (i, row) in rows.iter().enumerate() {
        // A distinct solution blank per row (its label is the row ordinal, but
        // its identity is bijection-normalized away by RDFC-1.0 — only its
        // structure, i.e. the cells hanging off it, is observable).
        let solution = builder.intern_blank(&format!("row{i}"), SOLUTION_SCOPE);
        for (var, cell) in variables.iter().zip(row) {
            if let Some(term) = cell {
                let predicate = builder.intern_iri(&format!("{CONFORMANCE_NS}var:{var}"));
                let object = intern_term_value(&mut builder, term, literals);
                builder.push_quad(solution, predicate, object, None);
            }
        }
        if !ordered && row.iter().all(Option::is_none) {
            // A wholly unbound solution still has a multiplicity. Without a
            // statement the synthetic solution node would vanish from RDF.
            let empty = builder.intern_iri(&format!("{CONFORMANCE_NS}empty"));
            builder.push_quad(solution, empty, empty, None);
        }
        if ordered {
            let ordinal = builder.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XSD_INTEGER.to_owned()),
                language: None,
                direction: None,
            });
            builder.push_quad(solution, index_predicate, ordinal, None);
        }
    }
    let dataset = builder
        .freeze()
        .map_err(|e| format!("encode solution set for comparison: {e}"))?;
    Ok(purrdf_core::canonicalize(&dataset).nquads)
}

/// The canonical form a solution sequence is compared in: the RDFC-1.0 canonical
/// N-Quads of its `encode_solution_set` encoding: one fresh blank node per row, one
/// quad per bound cell, plus a row-ordinal quad when `ordered`.
///
/// Two sequences are equal under the comparer exactly when these strings are equal,
/// so a snapshot that records this string pins a result up to the same blank-node
/// relabelling (and, unless `ordered`, the same row reordering) the comparer allows.
///
/// # Errors
///
/// Returns the freeze diagnostic if the cells do not form a structurally valid
/// dataset.
pub fn canonical_solutions(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    ordered: bool,
) -> Result<String, String> {
    // Exact: a trace that records this string must see the lexical forms the
    // evaluator produced, so a change in how it spells a number still moves it.
    encode_solution_set(variables, rows, ordered, Literals::Exact)
}

/// How [`encode_solution_set`] spells a literal.
#[derive(Clone, Copy)]
enum Literals {
    /// As written.
    Exact,
    /// A same-datatype numeric literal by its value ([`comparison_lexical`]): the
    /// spelling result comparison uses.
    ByValue,
}

/// Intern one [`TermValue`] into `builder`, recursively for triple terms.
///
/// Every value blank node — top-level or nested in a triple term — is interned
/// under the single shared [`VALUE_SCOPE`], keyed by both its original scope
/// and label. A scoped blank shared across rows keeps ONE [`TermId`], while
/// equal labels from different source documents remain distinct. Canonicalization
/// then normalizes those opaque identities with one global bijection.
///
/// A triple term is interned over [`try_fold_nested`]'s work list: its subject,
/// predicate and object, each fully before the next, then the triple itself.
fn intern_term_value(
    builder: &mut RdfDatasetBuilder,
    term: &TermValue,
    literals: Literals,
) -> TermId {
    let interned = try_fold_nested(
        term,
        builder,
        |builder, term| {
            Ok::<_, Infallible>(Nested::Leaf(match term {
                TermValue::Iri(iri) => builder.intern_iri(iri),
                TermValue::Blank { label, scope } => {
                    builder.intern_blank(&format!("{}:{label}", scope.0), VALUE_SCOPE)
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => builder.intern_literal(RdfLiteral {
                    lexical_form: match literals {
                        Literals::Exact => lexical_form.clone(),
                        Literals::ByValue => comparison_lexical(lexical_form, datatype),
                    },
                    datatype: Some(datatype.clone()),
                    language: language.clone(),
                    direction: *direction,
                }),
                TermValue::Triple { s, p, o } => return Ok(Nested::Triple(&**s, &**p, &**o)),
            }))
        },
        |builder, _, s, p, o| Ok(builder.intern_triple(s, p, o)),
    );
    match interned {
        Ok(id) => id,
    }
}

/// The lexical form a literal is compared under: for a well-typed literal of an XSD
/// NUMERIC datatype, the canonical spelling of its value under that same datatype;
/// for every other literal, its own lexical form.
///
/// Two numeric literals of the same datatype therefore compare by value (the
/// approach Jena's result-set comparison takes): `"2"^^xsd:decimal` matches
/// `"2.0"^^xsd:decimal`, and `"1050"^^xsd:double` matches `"1.05E3"^^xsd:double`.
/// The W3C expected results spell computed numbers inconsistently within one
/// group (`"1050"` and `"2.5E0"` for `xsd:double`; `"2.0"` and `"2"` for an
/// integer-valued `xsd:decimal`), so no single serializer could match them lexically.
/// The datatype is never rewritten, so `"2"^^xsd:integer` still differs from
/// `"2.0"^^xsd:decimal`. An ill-typed numeric literal keeps its spelling, and so
/// does every non-numeric literal: those still compare as exact terms.
///
/// NaN: a result file holds RDF terms, and result-set equivalence is term identity,
/// not `op:numeric-equal`, so a NaN cell matches a NaN cell of the same datatype
/// (its canonical spelling is `NaN`); comparing by `=` would make a row holding one
/// match nothing, itself included. The infinities match the same way.
fn comparison_lexical(lexical: &str, datatype: &str) -> String {
    use purrdf::xsd::XsdDatatype;
    match XsdDatatype::from_iri(datatype) {
        Some(dt) if dt.is_numeric() => purrdf::xsd::parse(lexical, dt)
            .map_or_else(|_| lexical.to_owned(), |value| value.canonical_lexical()),
        _ => lexical.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::TestKind;

    /// A minimal case whose `mf:result` is the given expectation.
    fn case_with(expected: ExpectedResult) -> SparqlTestCase {
        SparqlTestCase {
            iri: "http://purrdf.test/property-functions#probe".to_owned(),
            base: crate::manifest::BASE_ROOT.to_owned(),
            name: "probe".to_owned(),
            kind: TestKind::QueryEval,
            query: std::path::PathBuf::new(),
            data: Vec::new(),
            graph_data: Vec::new(),
            service_data: Vec::new(),
            construct_data: None,
            regime: None,
            aggregate_namespace: None,
            expected,
            lax_cardinality: false,
        }
    }

    /// The suite's own expected-error fixture, so the test grades the file the
    /// harness actually reads rather than a copy of it.
    fn suite_error_file(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("suite/purrdf-property-functions")
            .join(name)
    }

    #[test]
    fn an_expected_error_file_grades_the_diagnostic_it_names() {
        let case = case_with(ExpectedResult::EvalError(suite_error_file(
            "unregistered-relation.err",
        )));
        let real = "evaluate http://purrdf.test/property-functions#unregisteredRelation: error \
                    native-sparql-property-function: host function error: no property function \
                    is registered for <https://example.org/rel/notRegistered>";
        assert!(compare_failure(&case, real).is_ok());

        // A refusal of some OTHER call is not this case's refusal.
        let wrong_iri = real.replace("notRegistered", "somethingElse");
        let error = compare_failure(&case, &wrong_iri).expect_err("the IRI is named in full");
        assert!(
            error.contains("does not contain the expected text"),
            "{error}"
        );

        // Nor is an unrelated failure that happens to mention a property function.
        let unrelated = "evaluate x: error native-sparql-parse: syntax error";
        assert!(compare_failure(&case, unrelated).is_err());
    }

    #[test]
    fn a_case_with_no_error_expectation_gets_its_diagnostic_back_unchanged() {
        let case = case_with(ExpectedResult::None);
        assert_eq!(
            compare_failure(&case, "boom").expect_err("a failure is still a failure"),
            "boom"
        );
    }

    #[test]
    fn an_expected_failure_is_not_satisfied_by_a_run_that_succeeded() {
        let case = case_with(ExpectedResult::EvalError(suite_error_file(
            "infeasible-chain.err",
        )));
        let error = compare(
            &case,
            &RunOutcome::Eval {
                result: SparqlResult::Boolean(true),
                ordered: false,
            },
        )
        .expect_err("an expected refusal cannot be met by a completed run");
        assert!(error.contains("ran to completion instead"), "{error}");
    }

    fn lit(lexical_form: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical_form.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    fn blank(label: &str) -> TermValue {
        TermValue::Blank {
            label: label.to_owned(),
            scope: BlankScope::DEFAULT,
        }
    }

    /// Encode a one-variable, one-row set (unordered) to its canonical form.
    fn one(term: TermValue) -> String {
        encode_solution_set(
            &["x".to_owned()],
            &[vec![Some(term)]],
            false,
            Literals::Exact,
        )
        .expect("encode")
    }

    /// `xsd:string`-shaped stand-in datatypes for the boundary fixtures below. They
    /// are ABSOLUTE because the IR admits no other kind of IRI term (see
    /// `purrdf_core::ir::absolute`), datatypes included.
    const DT_B: &str = "http://example.org/b";
    const DT_X: &str = "http://example.org/x";

    /// Two structurally distinct literals must never fold onto one canonical form.
    ///
    /// The *datatype* leg of this hazard is now closed by construction rather than by
    /// assertion: a datatype is an IRI term, the IR admits only absolute IRIs, and
    /// every byte an encoder might reach for as a field delimiter (`|`, `"`, `^`,
    /// `<`, `>`) is outside the RFC-3987 grammar — so no datatype can carry one. What
    /// stays reachable is the LEXICAL form, which may legally contain the entire
    /// `"lex"^^<iri>` framing verbatim, and that is what this pins.
    #[test]
    fn literal_field_boundaries_do_not_collide() {
        assert_ne!(
            one(lit("a", DT_B)),
            one(lit(&format!("a\"^^<{DT_B}>"), DT_X)),
            "a lexical form that spells the datatype framing must not encode as that \
             framing"
        );
    }

    #[test]
    fn iri_cannot_collide_with_literal() {
        assert_ne!(
            one(TermValue::Iri("L:something".to_owned())),
            one(lit("something", DT_X)),
            "IRI and Literal encodings must stay distinct"
        );
    }

    #[test]
    fn blank_node_relabelling_is_isomorphic() {
        // A single-column row bound to a blank node must encode identically
        // regardless of the blank's original label — the core isomorphism fix.
        assert_eq!(
            one(blank("b0")),
            one(blank("totally-different")),
            "blank-node label must not affect equality"
        );
    }

    #[test]
    fn equal_labels_from_different_source_scopes_remain_distinct() {
        let variables = vec!["x".to_owned(), "y".to_owned()];
        let scoped = |scope| TermValue::Blank {
            label: "same".to_owned(),
            scope: BlankScope(scope),
        };
        let actual = vec![vec![Some(scoped(3)), Some(scoped(4))]];
        let distinct = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![vec![Some(blank("a")), Some(blank("b"))]],
        };
        assert!(compare_solutions(&variables, &actual, &distinct, false).is_ok());
        let aliased = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![vec![Some(blank("a")), Some(blank("a"))]],
        };
        assert!(compare_solutions(&variables, &actual, &aliased, false).is_err());
    }

    #[test]
    fn unbound_solution_multiplicity_is_observable() {
        let variables = vec!["x".to_owned()];
        let expected = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![vec![None]],
        };
        assert!(compare_solutions(&variables, &[], &expected, false).is_err());
        assert!(compare_solutions(&variables, &[vec![None]], &expected, false).is_ok());
        assert!(
            compare_solutions(&variables, &[vec![None], vec![None]], &expected, false).is_err()
        );
    }

    #[test]
    fn shared_blank_across_columns_is_distinguished_from_two_distinct_blanks() {
        // ?x and ?y bound to the SAME blank (coreference) must differ from ?x
        // and ?y bound to two independent blanks, even though both relabel away.
        let vars = ["x".to_owned(), "y".to_owned()];
        let shared = encode_solution_set(
            &vars,
            &[vec![Some(blank("b0")), Some(blank("b0"))]],
            false,
            Literals::Exact,
        )
        .expect("encode");
        let distinct = encode_solution_set(
            &vars,
            &[vec![Some(blank("b0")), Some(blank("b1"))]],
            false,
            Literals::Exact,
        )
        .expect("encode");
        assert_ne!(
            shared, distinct,
            "coreference between two variables is observable, not just blank count"
        );
        // ...but relabelling the SAME coreference must still collapse.
        let shared_relabelled = encode_solution_set(
            &vars,
            &[vec![Some(blank("zzz")), Some(blank("zzz"))]],
            false,
            Literals::Exact,
        )
        .expect("encode");
        assert_eq!(shared, shared_relabelled);
    }

    #[test]
    fn term_position_is_not_normalized_away() {
        // The same blank bound to ?x vs to ?y is a different row (position
        // matters); only blank *identity* is bijection-normalized.
        let vars = ["x".to_owned(), "y".to_owned()];
        let a = encode_solution_set(
            &vars,
            &[vec![Some(blank("b0")), None]],
            false,
            Literals::Exact,
        )
        .expect("encode");
        let b = encode_solution_set(
            &vars,
            &[vec![None, Some(blank("b0"))]],
            false,
            Literals::Exact,
        )
        .expect("encode");
        assert_ne!(a, b, "variable position must still compare exactly");
    }

    /// DEFECT 1 gate: the WHOLE-SET bijection must reject a result whose blanks
    /// only line up per row. Expected `[{?x=_:a},{?x=_:b}]` (two distinct
    /// blanks) vs actual `[{?x=_:z},{?x=_:z}]` (one blank in both rows): no
    /// global bijection exists, so they MUST be unequal.
    #[test]
    fn cross_row_bijection_rejects_row_local_relabelling() {
        let vars = ["x".to_owned()];
        let two_distinct = &[vec![Some(blank("a"))], vec![Some(blank("b"))]];
        let one_shared = &[vec![Some(blank("z"))], vec![Some(blank("z"))]];
        assert_ne!(
            encode_solution_set(&vars, two_distinct, false, Literals::Exact).expect("encode"),
            encode_solution_set(&vars, one_shared, false, Literals::Exact).expect("encode"),
            "two distinct blanks must NOT equal the same blank repeated (no global bijection)"
        );
    }

    /// DEFECT 1 gate: a genuine GLOBAL relabelling must still be equal.
    /// `[{?x=_:a},{?x=_:b}]` vs `[{?x=_:p},{?x=_:q}]` — one bijection a↦p, b↦q.
    #[test]
    fn cross_row_global_relabelling_is_equal() {
        let vars = ["x".to_owned()];
        let ab = &[vec![Some(blank("a"))], vec![Some(blank("b"))]];
        let pq = &[vec![Some(blank("p"))], vec![Some(blank("q"))]];
        assert_eq!(
            encode_solution_set(&vars, ab, false, Literals::Exact).expect("encode"),
            encode_solution_set(&vars, pq, false, Literals::Exact).expect("encode"),
            "a global bijection over all rows must compare equal"
        );
    }

    /// DEFECT 2 gate: with `ordered = true`, row order is observable — the same
    /// rows in a different order must NOT be equal; the identical order must be.
    #[test]
    fn ordered_comparison_is_position_sensitive() {
        let vars = ["x".to_owned()];
        let forward = &[
            vec![Some(lit("1", XSD_INTEGER))],
            vec![Some(lit("2", XSD_INTEGER))],
        ];
        let reverse = &[
            vec![Some(lit("2", XSD_INTEGER))],
            vec![Some(lit("1", XSD_INTEGER))],
        ];
        assert_ne!(
            encode_solution_set(&vars, forward, true, Literals::Exact).expect("encode"),
            encode_solution_set(&vars, reverse, true, Literals::Exact).expect("encode"),
            "ordered: a different row order must compare unequal"
        );
        assert_eq!(
            encode_solution_set(&vars, forward, true, Literals::Exact).expect("encode"),
            encode_solution_set(&vars, forward, true, Literals::Exact).expect("encode"),
            "ordered: identical order must compare equal"
        );
        // The SAME two orders compare EQUAL when unordered (multiset).
        assert_eq!(
            encode_solution_set(&vars, forward, false, Literals::Exact).expect("encode"),
            encode_solution_set(&vars, reverse, false, Literals::Exact).expect("encode"),
            "unordered: row order must not matter"
        );
    }

    /// The `compare_solutions` seam reports ordered vs multiset in its error and
    /// passes an equal set either way.
    #[test]
    fn compare_solutions_ordered_flag_threads_through() {
        let vars = vec!["x".to_owned()];
        let rows = vec![vec![Some(lit("1", XSD_INTEGER))]];
        let expected = ParsedSolutions {
            variables: vars.clone(),
            rows: rows.clone(),
        };
        assert!(compare_solutions(&vars, &rows, &expected, true).is_ok());
        assert!(compare_solutions(&vars, &rows, &expected, false).is_ok());

        let reversed_expected = ParsedSolutions {
            variables: vars.clone(),
            rows: vec![
                vec![Some(lit("2", XSD_INTEGER))],
                vec![Some(lit("1", XSD_INTEGER))],
            ],
        };
        let two_rows = vec![
            vec![Some(lit("1", XSD_INTEGER))],
            vec![Some(lit("2", XSD_INTEGER))],
        ];
        let err = compare_solutions(&vars, &two_rows, &reversed_expected, true).unwrap_err();
        assert!(err.contains("ordered sequence"), "message: {err}");
        // Same rows, opposite order, compare EQUAL when unordered.
        assert!(compare_solutions(&vars, &two_rows, &reversed_expected, false).is_ok());
    }

    #[test]
    fn lax_cardinality_accepts_only_the_declared_multiplicity_range() {
        let mut case = case_with(ExpectedResult::None);
        case.lax_cardinality = true;
        let variables = vec!["x".to_owned()];
        let a = vec![Some(TermValue::simple_literal("a"))];
        let b = vec![Some(TermValue::simple_literal("b"))];
        let expected = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![a.clone(), a.clone(), a.clone(), b.clone()],
        };
        for count in 1..=3 {
            let mut rows = vec![a.clone(); count];
            rows.push(b.clone());
            assert!(compare_case_solutions(&case, &variables, &rows, &expected, false).is_ok());
        }
        for rows in [
            vec![a.clone()],
            vec![a.clone(), a.clone(), a.clone(), a.clone(), b.clone()],
            vec![a.clone(), b.clone(), b.clone()],
            vec![a, b, vec![None]],
        ] {
            assert!(compare_case_solutions(&case, &variables, &rows, &expected, false).is_err());
        }
    }

    #[test]
    fn lax_cardinality_preserves_global_blank_coreference_and_count_correspondence() {
        let mut case = case_with(ExpectedResult::None);
        case.lax_cardinality = true;
        let variables = vec!["x".to_owned(), "y".to_owned()];
        let expected_first = vec![Some(blank("a")), Some(blank("b"))];
        let expected_second = vec![Some(blank("a")), Some(blank("c"))];
        let expected = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![expected_first.clone(), expected_first, expected_second],
        };
        let first = vec![Some(blank("renamed")), Some(blank("one"))];
        let second = vec![Some(blank("renamed")), Some(blank("two"))];
        assert!(
            compare_case_solutions(
                &case,
                &variables,
                &[first.clone(), first.clone(), second],
                &expected,
                false
            )
            .is_ok()
        );
        let inconsistent = vec![Some(blank("other")), Some(blank("two"))];
        assert!(
            compare_case_solutions(&case, &variables, &[first, inconsistent], &expected, false)
                .is_err()
        );
        // Pin row identities with a ground cell so a valid bijection cannot swap
        // the expected multiplicities between otherwise symmetric blank rows.
        let actual_rows = vec![
            vec![Some(blank("one")), Some(TermValue::simple_literal("a"))],
            vec![Some(blank("two")), Some(TermValue::simple_literal("b"))],
        ];
        let expected = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![
                actual_rows[0].clone(),
                actual_rows[1].clone(),
                actual_rows[1].clone(),
            ],
        };
        let swapped_counts = vec![
            actual_rows[0].clone(),
            actual_rows[0].clone(),
            actual_rows[1].clone(),
        ];
        assert!(
            compare_case_solutions(&case, &variables, &swapped_counts, &expected, false).is_err()
        );
        assert!(compare_case_solutions(&case, &variables, &actual_rows, &expected, false).is_ok());
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! Result-value interning against its recursive reference, and at a hundred thousand
    //! levels on a 128 KiB thread.

    use purrdf_core::term_fixture::TermShape;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermId, TermValue};

    use super::intern_term_value;

    fn reference(builder: &mut RdfDatasetBuilder, term: &TermValue) -> TermId {
        match term {
            TermValue::Triple { s, p, o } => {
                let s = reference(builder, s);
                let p = reference(builder, p);
                let o = reference(builder, o);
                builder.intern_triple(s, p, o)
            }
            // Keep source identity directly in this independent reference. The
            // production encoder reserves its own scope with opaque labels.
            TermValue::Blank { label, scope } => builder.intern_blank(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => builder.intern_literal(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
            TermValue::Iri(iri) => builder.intern_iri(iri),
        }
    }

    /// Every generated value interns into a fresh builder as the recursive reference
    /// interns it: the same id, and the same next id after it.
    #[test]
    fn interning_agrees_with_its_recursive_reference_on_generated_values() {
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                TermShape::Any,
            );
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                intern_term_value(&mut found, &value, super::Literals::Exact),
                reference(&mut expected, &value),
                "seed {seed}"
            );
            let sentinel = "http://example.org/sentinel";
            assert_eq!(
                found.intern_iri(sentinel),
                expected.intern_iri(sentinel),
                "seed {seed}"
            );
        }
    }

    /// A value a hundred thousand triple terms deep interns on a thread whose whole
    /// stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_value_interns_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut builder = RdfDatasetBuilder::new();
            assert_eq!(
                intern_term_value(&mut builder, &value, super::Literals::Exact).index(),
                LEVELS + 2
            );
        })
        .expect("the thread starts");
    }
}

#[cfg(test)]
mod numeric_value_tests {
    //! Same-datatype numeric literals compare by value; everything else by term.

    use purrdf_core::TermValue;
    use purrdf_sparql_results::ParsedSolutions;

    use super::compare_solutions;

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn literal(lexical: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    fn xsd(lexical: &str, local: &str) -> TermValue {
        literal(lexical, &format!("{XSD}{local}"))
    }

    /// Whether the actual cell `a` matches the expected cell `e` in a one-row result.
    fn matches(a: TermValue, e: TermValue) -> bool {
        let variables = vec!["x".to_owned()];
        let expected = ParsedSolutions {
            variables: variables.clone(),
            rows: vec![vec![Some(e)]],
        };
        compare_solutions(&variables, &[vec![Some(a)]], &expected, false).is_ok()
    }

    #[test]
    fn same_datatype_value_equal_numerics_match() {
        for (a, e, local) in [
            ("2", "2.0", "decimal"),
            ("1.05E3", "1050", "double"),
            ("2.1E3", "2100", "double"),
            ("2E-1", "2.0E-1", "double"),
            ("1.5E0", "1.5", "float"),
            ("7", "+007", "integer"),
            ("3", "03", "int"),
            ("NaN", "NaN", "double"),
            ("INF", "INF", "float"),
        ] {
            assert!(
                matches(xsd(a, local), xsd(e, local)),
                "{a} vs {e} ^^{local}"
            );
        }
    }

    #[test]
    fn a_different_datatype_still_mismatches() {
        assert!(!matches(xsd("2", "integer"), xsd("2.0", "decimal")));
        assert!(!matches(xsd("2", "decimal"), xsd("2", "integer")));
        assert!(!matches(xsd("1.0E0", "double"), xsd("1.0E0", "float")));
        assert!(!matches(xsd("1", "int"), xsd("1", "integer")));
    }

    #[test]
    fn a_different_value_still_mismatches() {
        assert!(!matches(xsd("2", "decimal"), xsd("2.5", "decimal")));
        assert!(!matches(xsd("1.05E3", "double"), xsd("1051", "double")));
        assert!(!matches(xsd("NaN", "double"), xsd("INF", "double")));
        assert!(!matches(xsd("0", "double"), xsd("NaN", "double")));
    }

    #[test]
    fn a_non_numeric_lexical_difference_still_mismatches() {
        assert!(!matches(xsd("true", "boolean"), xsd("1", "boolean")));
        assert!(!matches(xsd("2.0", "string"), xsd("2", "string")));
        assert!(!matches(
            xsd("2002-10-10T17:00:00Z", "dateTime"),
            xsd("2002-10-10T17:00:00+00:00", "dateTime")
        ));
        assert!(!matches(
            literal("2.0", "http://example.org/num"),
            literal("2", "http://example.org/num")
        ));
        // An ill-typed numeric literal keeps its spelling.
        assert!(!matches(xsd("two", "decimal"), xsd("2", "decimal")));
        // And exact neighbours still match.
        assert!(matches(xsd("true", "boolean"), xsd("true", "boolean")));
        assert!(matches(xsd("two", "decimal"), xsd("two", "decimal")));
    }
}
