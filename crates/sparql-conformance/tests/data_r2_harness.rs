// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native failing-first coverage of the older corpus's actual result carriers.

use purrdf_sparql_conformance::{compare, manifest, run};
use std::path::Path;

use purrdf_core::{SparqlResult, TermValue, datatype};

fn case(group: &str, local: &str) -> manifest::SparqlTestCase {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("suite/w3c-sparql10")
        .join(group)
        .join("manifest.ttl");
    manifest::load(&path)
        .expect("frozen manifest")
        .into_iter()
        .find(|case| case.iri.ends_with(&format!("#{local}")))
        .expect("exact frozen case")
}

fn passes(group: &str, local: &str) {
    let case = case(group, local);
    let outcome = run::run(&case, None).expect("native evaluation");
    compare::compare(&case, &outcome).unwrap_or_else(|error| panic!("{group}/{local}: {error}"));
}

#[test]
fn turtle_boolean_results_grade_true_and_false_ask_answers() {
    passes("type-promotion", "type-promotion-01");
    passes("type-promotion", "type-promotion-23");
}

#[test]
fn rdf_xml_solutions_and_explicit_indices_grade_the_frozen_order() {
    passes("sort", "dawg-sort-1");
    passes("sort", "dawg-sort-builtin");
    passes("sort", "dawg-sort-function");
    passes("solution-seq", "offset-1");
}

#[test]
fn the_manifest_lax_cardinality_grades_both_reduced_cases() {
    passes("reduced", "reduced-1");
    passes("reduced", "reduced-2");
}

#[test]
fn query_dataset_sources_are_loaded_under_their_declared_iris() {
    for local in ["01", "03", "05", "06", "07", "08", "11", "12b"] {
        passes("dataset", &format!("dawg-dataset-{local}"));
    }
}

#[test]
fn named_graph_blank_scopes_survive_the_global_result_comparison() {
    passes("graph", "dawg-graph-11");
}

/// These are literal mathematical answers for the pinned four-type value 3,
/// with XPath numeric promotion. No production arithmetic or renderer computes
/// the expectations. Echoed source bindings remain exactly "3" in every type.
#[test]
fn arithmetic_values_types_and_source_bindings_have_independent_literal_expectations() {
    const TYPES: [&str; 4] = [
        datatype::XSD_INTEGER,
        datatype::XSD_DECIMAL,
        datatype::XSD_FLOAT,
        datatype::XSD_DOUBLE,
    ];
    for (local, scalar, exponential, division) in [
        ("add-numbers-cast", "6", "6.0E0", false),
        ("subtract-numbers-cast", "0", "0.0E0", false),
        ("multiply-numbers-cast", "9", "9.0E0", false),
        ("divide-numbers-cast", "1", "1.0E0", true),
    ] {
        let case = case("expr-ops", local);
        let run::RunOutcome::Eval {
            result: SparqlResult::Solutions {
                variables, rows, ..
            },
            ..
        } = run::run(&case, None).expect("native arithmetic")
        else {
            panic!("SELECT result")
        };
        let left = variables
            .iter()
            .position(|var| var == "left")
            .expect("left");
        let right = variables
            .iter()
            .position(|var| var == "right")
            .expect("right");
        let result = variables
            .iter()
            .position(|var| var == "result")
            .expect("result");
        assert_eq!(rows.len(), 16);
        let mut pairs = std::collections::BTreeSet::new();
        for row in rows {
            let ranks = [left, right].map(|at| {
                let Some(TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                }) = &row[at]
                else {
                    panic!("numeric source literal")
                };
                assert_eq!(lexical_form, "3", "an echoed source is not canonicalized");
                assert!(language.is_none() && direction.is_none());
                TYPES
                    .iter()
                    .position(|kind| *kind == datatype)
                    .expect("source type")
            });
            assert!(pairs.insert(ranks), "each source-type pair occurs once");
            let promoted = ranks[0].max(ranks[1]).max(usize::from(division));
            let lexical = if promoted < 2 { scalar } else { exponential };
            assert_eq!(
                row[result],
                Some(TermValue::typed_literal(lexical, TYPES[promoted])),
                "{local} {ranks:?}"
            );
        }
    }
    for (local, scalar, exponential) in [("unplus-2", "3", "3.0E0"), ("unminus-2", "-3", "-3.0E0")]
    {
        let case = case("expr-ops", local);
        let run::RunOutcome::Eval {
            result: SparqlResult::Solutions {
                variables, rows, ..
            },
            ..
        } = run::run(&case, None).expect("native unary arithmetic")
        else {
            panic!("SELECT result")
        };
        let input = variables.iter().position(|var| var == "v").expect("source");
        let result = variables
            .iter()
            .position(|var| var == "result")
            .expect("result");
        assert_eq!(rows.len(), 4);
        let mut types = std::collections::BTreeSet::new();
        for row in rows {
            let Some(TermValue::Literal {
                lexical_form,
                datatype,
                ..
            }) = &row[input]
            else {
                panic!("numeric source literal")
            };
            assert_eq!(lexical_form, "3");
            let rank = TYPES
                .iter()
                .position(|kind| *kind == datatype)
                .expect("source type");
            assert!(types.insert(rank));
            assert_eq!(
                row[result],
                Some(TermValue::typed_literal(
                    if rank < 2 { scalar } else { exponential },
                    TYPES[rank]
                ))
            );
        }
    }
}

#[test]
fn both_outer_and_inner_optional_filters_constrain_the_joined_solution() {
    let case = case("algebra", "opt-filter-2");
    let run::RunOutcome::Eval {
        result: SparqlResult::Solutions {
            variables, rows, ..
        },
        ..
    } = run::run(&case, None).expect("native OPTIONAL")
    else {
        panic!("SELECT result")
    };
    let position = |name| {
        variables
            .iter()
            .position(|var| var == name)
            .expect("projected variable")
    };
    assert_eq!(rows.len(), 2);
    for (subject, value, joined) in [("x1", "1", false), ("x2", "2", true)] {
        let row = rows
            .iter()
            .find(|row| {
                row[position("x")] == Some(TermValue::iri(format!("http://example/{subject}")))
            })
            .expect("each outer solution is retained");
        assert_eq!(
            row[position("v")],
            Some(TermValue::typed_literal(value, datatype::XSD_INTEGER))
        );
        assert_eq!(
            row[position("y")],
            joined.then(|| TermValue::iri("http://example/x3")),
            "{subject}: {row:?}"
        );
        assert_eq!(
            row[position("w")],
            joined.then(|| TermValue::typed_literal("3", datatype::XSD_INTEGER)),
            "{subject}: {row:?}"
        );
    }
}

/// SPARQL 1.2 §18.6.2 evaluates both LeftJoin inputs under the same context.
/// Only EXISTS/NOT EXISTS installs an outer solution as that context, so a
/// nested OPTIONAL group cannot see the outer title binding in its filter.
#[test]
fn the_nested_optional_group_does_not_receive_the_outer_solution_as_context() {
    let case = case("optional-filter", "dawg-optional-filter-005-not-simplified");
    let run::RunOutcome::Eval {
        result: SparqlResult::Solutions {
            variables, rows, ..
        },
        ..
    } = run::run(&case, None).expect("native nested OPTIONAL")
    else {
        panic!("SELECT result")
    };
    let title = variables
        .iter()
        .position(|var| var == "title")
        .expect("title");
    let price = variables
        .iter()
        .position(|var| var == "price")
        .expect("price");
    assert_eq!(rows.len(), 3);
    for name in ["TITLE 1", "TITLE 2", "TITLE 3"] {
        let row = rows
            .iter()
            .find(|row| row[title] == Some(TermValue::simple_literal(name)))
            .expect("retained title");
        assert_eq!(row[price], None, "{name}: {row:?}");
    }
}

/// RDF 1.2 §3.4.1 makes both surface forms the same xsd:string datatype.
/// ORDER BY therefore sorts all eight values together, preserving both rows
/// for each name rather than placing a separate plain-literal class first.
#[test]
fn simple_and_typed_string_spellings_share_one_ordered_value_space() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql10/sort/extended-manifest.ttl");
    let case = manifest::load(&path)
        .expect("frozen extended manifest")
        .remove(0);
    let run::RunOutcome::Eval {
        result: SparqlResult::Solutions {
            variables, rows, ..
        },
        ..
    } = run::run(&case, None).expect("native ordering")
    else {
        panic!("SELECT result")
    };
    assert_eq!(variables, ["name"]);
    let expected: Vec<_> = ["Alice", "Alice", "Bob", "Bob", "Eve", "Eve", "Fred", "Fred"]
        .map(|name| vec![Some(TermValue::simple_literal(name))])
        .into();
    assert_eq!(rows, expected);
}

/// The literal truth table is independent of the evaluator: known strings and
/// language strings have distinct values; ill-typed and unknown literal pairs
/// error unless they are the same term; a blank/IRI operand is known unequal.
/// SPARQL 1.2 §17.4.2.2 orders those rules. The four cases declare
/// `mf:KnownTypesDefault2Neq`, so they run with the language-string extension,
/// under which a language-tagged string is also unequal to the ill-typed and the
/// unknown-datatype literal; the same cases with the declaration removed grade
/// the core table, so each half of the requirement is executed.
#[test]
fn open_world_comparisons_preserve_known_inequality_and_literal_type_errors() {
    for extended in [false, true] {
        open_world_truth_tables(extended);
    }
}

fn open_world_truth_tables(extended: bool) {
    const SAME: [[u8; 8]; 8] = [
        [1, 0, 0, 1, 2, 2, 0, 0],
        [0, 1, 1, 0, 2, 2, 0, 0],
        [0, 1, 1, 0, 2, 2, 0, 0],
        [1, 0, 0, 1, 2, 2, 0, 0],
        [2, 2, 2, 2, 1, 2, 0, 0],
        [2, 2, 2, 2, 2, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 1, 0],
        [0, 0, 0, 0, 0, 0, 0, 1],
    ];
    const DIFFERENT: [[u8; 8]; 8] = [
        [0, 0, 0, 0, 2, 2, 0, 0],
        [0, 0, 0, 0, 2, 2, 0, 0],
        [0, 0, 0, 0, 2, 2, 0, 0],
        [0, 0, 0, 0, 2, 2, 0, 0],
        [2, 2, 2, 2, 2, 2, 0, 0],
        [2, 2, 2, 2, 2, 2, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];
    // The extension decides only a language string (rows/columns 2 and 3)
    // against the ill-typed and unknown-datatype literals (5 and 6): unequal.
    let extend = |mut truth: [[u8; 8]; 8]| {
        if extended {
            for (a, b) in [(1, 4), (1, 5), (2, 4), (2, 5)] {
                truth[a][b] = 0;
                truth[b][a] = 0;
            }
        }
        truth
    };
    let counts: [usize; 4] = if extended {
        [42, 52, 52, 10]
    } else {
        [34, 44, 44, 18]
    };
    let mut mismatches = Vec::new();
    for ((local, names, prefix, truth, keep), count) in [
        ("open-eq-08", ["x1", "x2"], ["x", "x"], extend(SAME), 0),
        ("open-eq-10", ["x", "y"], ["x", "y"], extend(DIFFERENT), 0),
        ("open-eq-11", ["x", "y"], ["x", "y"], extend(DIFFERENT), 0),
        ("open-eq-12", ["x", "y"], ["x", "x"], extend(SAME), 2),
    ]
    .into_iter()
    .zip(counts)
    {
        let mut case = case("open-world", local);
        assert!(case.requires(manifest::RequiredFeature::KnownTypesDefault2Neq));
        if !extended {
            case.requires.clear();
        }
        let run::RunOutcome::Eval {
            result: SparqlResult::Solutions {
                variables, rows, ..
            },
            ..
        } = run::run(&case, None).expect("native comparison")
        else {
            panic!("SELECT result")
        };
        let columns = names.map(|name| {
            variables
                .iter()
                .position(|var| var == name)
                .expect("subject")
        });
        let mut actual = std::collections::BTreeSet::new();
        for row in &rows {
            let subjects = columns.map(|at| row[at].clone().expect("bound subject"));
            assert!(actual.insert(subjects), "each source pair occurs once");
        }
        let mut expected = std::collections::BTreeSet::new();
        for (left, row) in truth.into_iter().enumerate() {
            for (right, answer) in row.into_iter().enumerate() {
                if answer == keep {
                    expected.insert([
                        TermValue::iri(format!(
                            "http://example/{}{number}",
                            prefix[0],
                            number = left + 1
                        )),
                        TermValue::iri(format!(
                            "http://example/{}{number}",
                            prefix[1],
                            number = right + 1
                        )),
                    ]);
                }
            }
        }
        assert_eq!(expected.len(), count, "independent truth table: {local}");
        if actual != expected {
            mismatches.push(format!(
                "{local} (extended: {extended}): {} actual, {count} expected; missing {:?}; extra {:?}",
                actual.len(),
                expected.difference(&actual).collect::<Vec<_>>(),
                actual.difference(&expected).collect::<Vec<_>>()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// XSD 1.1 §2.2.2 distinguishes primitive date and dateTime value spaces.
/// Under SPARQL 1.2 sameValue §17.4.2.2, their known inequality is false
/// rather than an error, independently of ordering's partial relation.
#[test]
fn a_known_datetime_value_is_unequal_to_a_date_value() {
    let case = case("open-world", "date-2");
    let run::RunOutcome::Eval {
        result: SparqlResult::Solutions {
            variables, rows, ..
        },
        ..
    } = run::run(&case, None).expect("native temporal comparison")
    else {
        panic!("SELECT result")
    };
    let subject = variables
        .iter()
        .position(|var| var == "x")
        .expect("subject");
    let value = variables.iter().position(|var| var == "v").expect("value");
    assert_eq!(rows.len(), 3);
    for (name, lexical, kind) in [
        ("dt1", "2006-08-23T09:00:00+01:00", datatype::XSD_DATE_TIME),
        ("d4", "2001-01-01", datatype::XSD_DATE),
        ("d5", "2001-01-01Z", datatype::XSD_DATE),
    ] {
        let row = rows
            .iter()
            .find(|row| row[subject] == Some(TermValue::iri(format!("http://example/{name}"))))
            .expect("each known unequal value remains selected");
        assert_eq!(row[value], Some(TermValue::typed_literal(lexical, kind)));
    }
}
