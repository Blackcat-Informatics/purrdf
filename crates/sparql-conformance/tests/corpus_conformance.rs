// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PurRDF-owned native SPARQL regression corpus.
//!
//! This test intentionally does **not** load `generated/dist/purrdf.gts` and does not
//! replay the historical ontology query corpus. The manifest-driven harness in
//! `tests/sparql_conformance.rs` covers vendored W3C tests and purrdf extension
//! fixtures. This file covers small, purrdf-owned regression datasets whose scope is
//! the library itself: native RDF loading, graph/solution comparison, property paths,
//! EXISTS evaluation, CONSTRUCT canonicalization, and `$this` substitution.

use std::path::{Path, PathBuf};

use purrdf::{NativeRdfFormat, canonicalize, parse_dataset};
use purrdf_core::{BlankScope, SparqlEngine, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::NativeSparqlEngine;

fn goldens_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

/// The stable, order-insensitive key for a solution row.
fn row_key(row: &[Option<TermValue>]) -> String {
    format!("{row:?}")
}

/// Render a SELECT result as a deterministic golden: projection variables on line
/// one, then sorted row Debug keys.
fn solutions_golden(variables: &[String], rows: &[Vec<Option<TermValue>>]) -> String {
    let mut out = String::new();
    out.push_str(&variables.join("\t"));
    out.push('\n');
    let mut keys: Vec<String> = rows.iter().map(|r| row_key(r)).collect();
    keys.sort();
    for k in keys {
        out.push_str(&k);
        out.push('\n');
    }
    out
}

const CORE_DATA_TTL: &str = r#"
@prefix ex: <http://purrdf.test/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:alice ex:knows ex:bob ;
    ex:age 30 ;
    ex:name "Alice" .

ex:bob ex:knows ex:carol ;
    ex:age 17 ;
    ex:name "Bob" .

ex:carol ex:member ex:club ;
    ex:name "Carol" .

ex:club ex:name "Club" .
"#;

enum Expected {
    Rows(&'static str),
    Ask(bool),
    GraphNTriples(&'static str),
}

struct RegressionCase {
    name: &'static str,
    data: &'static str,
    format: NativeRdfFormat,
    query: &'static str,
    expected: Expected,
}

fn regression_cases() -> Vec<RegressionCase> {
    vec![
        RegressionCase {
            name: "property_path_transitive_closure",
            data: CORE_DATA_TTL,
            format: NativeRdfFormat::Turtle,
            query: r"
                SELECT ?friend WHERE {
                    <http://purrdf.test/alice> <http://purrdf.test/knows>+ ?friend
                }
            ",
            expected: Expected::Rows(concat!(
                "friend\n",
                "[Some(Iri(\"http://purrdf.test/bob\"))]\n",
                "[Some(Iri(\"http://purrdf.test/carol\"))]\n",
            )),
        },
        RegressionCase {
            name: "exists_filter_reuses_outer_bindings",
            data: CORE_DATA_TTL,
            format: NativeRdfFormat::Turtle,
            query: r"
                SELECT ?person WHERE {
                    ?person <http://purrdf.test/knows> ?friend .
                    FILTER EXISTS {
                        ?person <http://purrdf.test/age> ?age
                        FILTER(?age > 18)
                    }
                }
            ",
            expected: Expected::Rows(concat!(
                "person\n",
                "[Some(Iri(\"http://purrdf.test/alice\"))]\n",
            )),
        },
        RegressionCase {
            name: "ask_path_sequence",
            data: CORE_DATA_TTL,
            format: NativeRdfFormat::Turtle,
            query: r"
                ASK {
                    <http://purrdf.test/alice>
                        <http://purrdf.test/knows>/<http://purrdf.test/knows>
                        <http://purrdf.test/carol>
                }
            ",
            expected: Expected::Ask(true),
        },
        RegressionCase {
            name: "construct_canonical_graph",
            data: CORE_DATA_TTL,
            format: NativeRdfFormat::Turtle,
            query: r"
                CONSTRUCT {
                    ?person <http://purrdf.test/summary> ?name
                }
                WHERE {
                    ?person <http://purrdf.test/name> ?name
                    FILTER(?person = <http://purrdf.test/alice>)
                }
            ",
            expected: Expected::GraphNTriples(
                r#"<http://purrdf.test/alice> <http://purrdf.test/summary> "Alice" .
"#,
            ),
        },
    ]
}

fn canonical_ntriples(text: &str) -> String {
    let dataset = parse_dataset(
        text.as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("parse expected graph");
    canonicalize(&dataset).nquads
}

fn compare_result(case: &RegressionCase, result: SparqlResult) -> Result<(), String> {
    match (&case.expected, result) {
        (
            Expected::Rows(expected),
            SparqlResult::Solutions {
                variables, rows, ..
            },
        ) => {
            let native = solutions_golden(&variables, &rows);
            if &native == expected {
                Ok(())
            } else {
                Err(format!(
                    "SELECT rows differ:\n--- expected ---\n{expected}--- native ---\n{native}"
                ))
            }
        }
        (Expected::Ask(expected), SparqlResult::Boolean(native)) if expected == &native => Ok(()),
        (Expected::Ask(expected), SparqlResult::Boolean(native)) => Err(format!(
            "ASK boolean differs: expected {expected}, native {native}"
        )),
        (Expected::GraphNTriples(expected), SparqlResult::Graph(native_graph)) => {
            let expected = canonical_ntriples(expected);
            let native = canonicalize(&native_graph).nquads;
            if native == expected {
                Ok(())
            } else {
                Err(format!(
                    "CONSTRUCT canonical N-Quads differ:\n--- expected ---\n{expected}\
                     --- native ---\n{native}"
                ))
            }
        }
        (expected, other) => Err(format!(
            "result kind mismatch for {}: expected {}, native returned {other:?}",
            case.name,
            expected_kind(expected)
        )),
    }
}

fn expected_kind(expected: &Expected) -> &'static str {
    match expected {
        Expected::Rows(_) => "SELECT solutions",
        Expected::Ask(_) => "ASK boolean",
        Expected::GraphNTriples(_) => "CONSTRUCT graph",
    }
}

#[test]
fn purrdf_regression_corpus() {
    let engine = NativeSparqlEngine::new();
    let mut matched = 0usize;
    let mut mismatches = Vec::new();

    for case in regression_cases() {
        let dataset = parse_dataset(case.data.as_bytes(), case.format.media_type(), None)
            .unwrap_or_else(|e| panic!("parse regression data {}: {e}", case.name));
        let request = SparqlRequest {
            query: case.query,
            base_iri: None,
            substitutions: &[],
        };
        match engine.query(&dataset, request) {
            Ok(result) => match compare_result(&case, result) {
                Ok(()) => matched += 1,
                Err(msg) => mismatches.push((case.name, msg)),
            },
            Err(e) => mismatches.push((case.name, format!("native errored: {e}"))),
        }
    }

    eprintln!(
        "purrdf SPARQL regression corpus: {matched} matched, {} mismatches",
        mismatches.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} purrdf regression case(s) mismatched native engine:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .map(|(name, err)| format!("  {name}\n    -> {err}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        matched >= 4,
        "purrdf regression corpus shrank: {matched} matched < 4"
    );
}

// ---------------------------------------------------------------------------
// GAP-A substitution sub-gate (goldens/substitution/).
// ---------------------------------------------------------------------------

/// Parse one `<name>.subst` line of the form `var={TermValue:?}`. The capture only
/// ever emits `Iri("...")` and `Blank { label: "...", scope: BlankScope(N) }`;
/// reject any other Debug form rather than silently misparse.
fn parse_subst(text: &str) -> Vec<(String, TermValue)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (var, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("malformed .subst line (no '='): {line:?}"));
        let value = value.trim();
        let term = if let Some(rest) = value.strip_prefix("Iri(\"") {
            let iri = rest
                .strip_suffix("\")")
                .unwrap_or_else(|| panic!("malformed Iri(..) in .subst: {value:?}"));
            TermValue::Iri(iri.to_owned())
        } else if let Some(rest) = value.strip_prefix("Blank { label: \"") {
            let (label, tail) = rest
                .split_once("\", scope: BlankScope(")
                .unwrap_or_else(|| panic!("malformed Blank{{..}} in .subst: {value:?}"));
            let scope_str = tail
                .strip_suffix(") }")
                .unwrap_or_else(|| panic!("malformed Blank scope in .subst: {value:?}"));
            let scope: u32 = scope_str
                .parse()
                .unwrap_or_else(|e| panic!("bad BlankScope number in .subst {value:?}: {e}"));
            assert_eq!(
                BlankScope(scope),
                BlankScope::DEFAULT,
                "substitution blank scope must be DEFAULT for the single-load dataset"
            );
            TermValue::Blank {
                label: label.to_owned(),
                scope: BlankScope(scope),
            }
        } else {
            panic!(
                "unsupported TermValue Debug form in .subst (only Iri/Blank captured): {value:?}"
            )
        };
        out.push((var.trim().to_owned(), term));
    }
    out
}

#[test]
fn purrdf_substitution_goldens() {
    let subst_dir = goldens_root().join("substitution");
    let dataset_nt =
        std::fs::read(subst_dir.join("dataset.nt")).expect("read substitution dataset.nt golden");
    let dataset = parse_dataset(&dataset_nt, NativeRdfFormat::NTriples.media_type(), None)
        .expect("parse substitution dataset.nt");
    let engine = NativeSparqlEngine::new();

    let mut shapes: Vec<String> = std::fs::read_dir(&subst_dir)
        .expect("read goldens/substitution")
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension().and_then(|x| x.to_str()) == Some("query"))
                .then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    shapes.sort();
    assert!(
        shapes.len() >= 6,
        "substitution shape inventory shrank: {} < 6",
        shapes.len()
    );

    let mut matched = 0usize;
    let mut mismatches: Vec<(String, String)> = Vec::new();
    for name in &shapes {
        let query = std::fs::read_to_string(subst_dir.join(format!("{name}.query")))
            .unwrap_or_else(|e| panic!("read {name}.query: {e}"));
        let subst_text = std::fs::read_to_string(subst_dir.join(format!("{name}.subst")))
            .unwrap_or_else(|e| panic!("read {name}.subst: {e}"));
        let golden_rows = std::fs::read_to_string(subst_dir.join(format!("{name}.rows")))
            .unwrap_or_else(|e| panic!("read {name}.rows: {e}"));
        let subst = parse_subst(&subst_text);
        let request = SparqlRequest {
            query: query.trim(),
            base_iri: None,
            substitutions: &subst,
        };
        match engine.query(&dataset, request) {
            Ok(SparqlResult::Solutions {
                variables, rows, ..
            }) => {
                let native = solutions_golden(&variables, &rows);
                if native == golden_rows {
                    matched += 1;
                } else {
                    mismatches.push((
                        name.clone(),
                        format!(
                            "substitution rows differ:\n--- golden ---\n{golden_rows}\
                             --- native ---\n{native}"
                        ),
                    ));
                }
            }
            Ok(other) => mismatches.push((
                name.clone(),
                format!("expected SELECT solutions, native returned {other:?}"),
            )),
            Err(e) => mismatches.push((name.clone(), format!("native errored: {e}"))),
        }
    }

    eprintln!(
        "substitution conformance: {} matched / {} shapes, {} mismatches",
        matched,
        shapes.len(),
        mismatches.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} substitution shape(s) mismatched native engine:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .map(|(f, e)| format!("  {f}\n    -> {e}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        matched,
        shapes.len(),
        "every substitution shape must match its frozen golden"
    );
}
