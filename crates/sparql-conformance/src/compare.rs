// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Comparing a run outcome against the manifest's expected result.
//!
//! * `SELECT` → the solution sequence compared with a **single global
//!   blank-node bijection** over the whole result set (W3C solution-set
//!   equality), as a multiset when there is no top-level `ORDER BY` and as an
//!   ordered sequence when there is (see [`compare_solutions`]).
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
//! [`encode_solution_set`].

use std::path::Path;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId, TermValue,
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
            result_kind(l),
            result_kind(r)
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
            compare_solutions(variables, rows, &expected, ordered)
        }
        (
            ExpectedResult::ResultSetTurtle(path),
            SparqlResult::Solutions {
                variables, rows, ..
            },
        ) => {
            let expected = crate::rs_resultset::parse(
                &case.base,
                &std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?,
            )
            .map_err(|e| format!("parse expected rs:ResultSet {}: {e}", path.display()))?;
            compare_solutions(variables, rows, &expected, ordered)
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
    let actual_canon = encode_solution_set(variables, rows, ordered)?;
    let expected_canon = encode_solution_set(&expected.variables, &expected.rows, ordered)?;
    if actual_canon == expected_canon {
        Ok(())
    } else {
        let mode = if ordered {
            "ordered sequence"
        } else {
            "multiset"
        };
        Err(format!(
            "solution {mode} mismatch: {} expected rows vs {} actual rows",
            expected.rows.len(),
            rows.len()
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
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Encode a whole solution set as canonical RDFC-1.0 N-Quads.
///
/// Every row becomes a distinct **solution blank node** (in [`SOLUTION_SCOPE`]);
/// each bound cell `(var, value)` becomes one quad
/// `(solution, urn:purrdf:conformance:var:<var>, value)`. Because the entire
/// set is one dataset canonicalized once, RDFC-1.0 must find a SINGLE bijection
/// mapping every blank node across every row simultaneously — so a result whose
/// blanks only line up row-by-row (but not globally) is correctly UNEQUAL.
/// Value blank nodes are interned in [`VALUE_SCOPE`] keyed by `(label, scope)`,
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
                let object = intern_term_value(&mut builder, term);
                builder.push_quad(solution, predicate, object, None);
            }
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

/// Intern one [`TermValue`] into `builder`, recursively for triple terms.
///
/// Every value blank node — top-level or nested in a triple term — is interned
/// under the single shared [`VALUE_SCOPE`], keyed by its label, so a blank with
/// the same label in two different rows resolves to ONE [`TermId`] and its
/// coreference across the whole result set is preserved through
/// canonicalization. (Result blank nodes are single-scope in practice: both the
/// engine and the SRX/SRJ/`rs:ResultSet` readers mint them in the default
/// scope, so forcing one scope here cannot merge two originally-distinct
/// blanks.)
fn intern_term_value(builder: &mut RdfDatasetBuilder, term: &TermValue) -> TermId {
    match term {
        TermValue::Iri(iri) => builder.intern_iri(iri),
        TermValue::Blank { label, .. } => builder.intern_blank(label, VALUE_SCOPE),
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
        TermValue::Triple { s, p, o } => {
            let s = intern_term_value(builder, s);
            let p = intern_term_value(builder, p);
            let o = intern_term_value(builder, o);
            builder.intern_triple(s, p, o)
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
        encode_solution_set(&["x".to_owned()], &[vec![Some(term)]], false).expect("encode")
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
    fn shared_blank_across_columns_is_distinguished_from_two_distinct_blanks() {
        // ?x and ?y bound to the SAME blank (coreference) must differ from ?x
        // and ?y bound to two independent blanks, even though both relabel away.
        let vars = ["x".to_owned(), "y".to_owned()];
        let shared =
            encode_solution_set(&vars, &[vec![Some(blank("b0")), Some(blank("b0"))]], false)
                .expect("encode");
        let distinct =
            encode_solution_set(&vars, &[vec![Some(blank("b0")), Some(blank("b1"))]], false)
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
        )
        .expect("encode");
        assert_eq!(shared, shared_relabelled);
    }

    #[test]
    fn term_position_is_not_normalized_away() {
        // The same blank bound to ?x vs to ?y is a different row (position
        // matters); only blank *identity* is bijection-normalized.
        let vars = ["x".to_owned(), "y".to_owned()];
        let a =
            encode_solution_set(&vars, &[vec![Some(blank("b0")), None]], false).expect("encode");
        let b =
            encode_solution_set(&vars, &[vec![None, Some(blank("b0"))]], false).expect("encode");
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
            encode_solution_set(&vars, two_distinct, false).expect("encode"),
            encode_solution_set(&vars, one_shared, false).expect("encode"),
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
            encode_solution_set(&vars, ab, false).expect("encode"),
            encode_solution_set(&vars, pq, false).expect("encode"),
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
            encode_solution_set(&vars, forward, true).expect("encode"),
            encode_solution_set(&vars, reverse, true).expect("encode"),
            "ordered: a different row order must compare unequal"
        );
        assert_eq!(
            encode_solution_set(&vars, forward, true).expect("encode"),
            encode_solution_set(&vars, forward, true).expect("encode"),
            "ordered: identical order must compare equal"
        );
        // The SAME two orders compare EQUAL when unordered (multiset).
        assert_eq!(
            encode_solution_set(&vars, forward, false).expect("encode"),
            encode_solution_set(&vars, reverse, false).expect("encode"),
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
}
