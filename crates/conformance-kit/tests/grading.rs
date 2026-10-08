// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable adversarial controls for parsed metadata and complete result grading.

use purrdf_conformance_kit::{bag, graph, manifest, outcome, report, result_set};
use purrdf_core::{BlankScope, GraphMatch, RdfDatasetBuilder, TermValue};
use purrdf_iri::vocab::{mf, rdf, rs, sh};

fn empty_and_unbound_rows_are_observable() {
    assert!(bag::compare_solutions(&[], &[vec![]], &[], &[], false).is_err());
    assert!(bag::compare_solutions(&[], &[vec![], vec![]], &[], &[vec![]], false).is_err());
    let vars = vec!["x".to_owned()];
    assert!(bag::compare_solutions(&vars, &[vec![None]], &vars, &[], false).is_err());
    assert!(bag::compare_solutions(&vars, &[vec![]], &vars, &[vec![None]], false).is_err());
    assert!(bag::canonical_solutions(&["x".into(), "x".into()], &[], false).is_err());
}

fn scope_partition_and_global_row_links_are_observable() {
    let vars = vec!["x".to_owned()];
    let expected = vec![
        vec![Some(TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(11),
        })],
        vec![Some(TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(12),
        })],
    ];
    let renamed = vec![
        vec![Some(TermValue::Blank {
            label: "a".into(),
            scope: BlankScope::DEFAULT,
        })],
        vec![Some(TermValue::Blank {
            label: "b".into(),
            scope: BlankScope::DEFAULT,
        })],
    ];
    bag::compare_solutions(&vars, &expected, &vars, &renamed, false).unwrap();
    let collapsed = vec![renamed[0].clone(), renamed[0].clone()];
    assert!(bag::compare_solutions(&vars, &expected, &vars, &collapsed, false).is_err());
    let vars = vec!["left".into(), "right".into()];
    let a = renamed[0][0].clone();
    let b = renamed[1][0].clone();
    let cycle = vec![vec![a.clone(), b.clone()], vec![b.clone(), a.clone()]];
    let broken = vec![vec![a.clone(), b.clone()], vec![a, b]];
    assert!(bag::compare_solutions(&vars, &cycle, &vars, &broken, false).is_err());
}

fn nested_triples_share_the_same_global_bijection() {
    let blank = TermValue::Blank {
        label: "b".into(),
        scope: BlankScope(3),
    };
    let triple = TermValue::Triple {
        s: Box::new(blank.clone()).into(),
        p: Box::new(TermValue::Iri("http://example.org/p".into())).into(),
        o: Box::new(TermValue::Iri("http://example.org/o".into())).into(),
    };
    let vars = vec!["direct".into(), "quoted".into()];
    let good = vec![vec![Some(blank), Some(triple.clone())]];
    let bad = vec![vec![
        Some(TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(4),
        }),
        Some(triple),
    ]];
    assert!(bag::compare_solutions(&vars, &good, &vars, &bad, false).is_err());
}

fn header_order_is_metadata_and_row_order_is_explicit() {
    let a = TermValue::Iri("http://example.org/a".into());
    let b = TermValue::Iri("http://example.org/b".into());
    let left = vec![vec![Some(a.clone()), Some(b.clone())]];
    let right = vec![vec![Some(b.clone()), Some(a.clone())]];
    bag::compare_solutions(
        &["a".into(), "b".into()],
        &left,
        &["b".into(), "a".into()],
        &right,
        false,
    )
    .unwrap();
    let sequence = vec![vec![Some(a)], vec![Some(b)]];
    let reversed = vec![sequence[1].clone(), sequence[0].clone()];
    bag::compare_solutions(&["x".into()], &sequence, &["x".into()], &reversed, false).unwrap();
    assert!(
        bag::compare_solutions(&["x".into()], &sequence, &["x".into()], &reversed, true).is_err()
    );
}

fn structural_result_reader_keeps_all_empty_solutions() {
    let mut b = RdfDatasetBuilder::new();
    let root = b.intern_blank("root", BlankScope::DEFAULT);
    let class = b.intern_iri(rs::RESULT_SET);
    let ty = b.intern_iri(rdf::TYPE);
    let solution = b.intern_iri(rs::SOLUTION);
    b.push_quad(root, ty, class, None);
    for label in ["first", "second"] {
        let row = b.intern_blank(label, BlankScope::DEFAULT);
        b.push_quad(root, solution, row, None);
    }
    let ds = b.freeze().unwrap();
    let result = result_set::decode(graph::Reader {
        dataset: &ds,
        graph: GraphMatch::Default,
    })
    .unwrap();
    assert_eq!(result.variables, [] as [String; 0]);
    assert_eq!(result.rows, vec![vec![], vec![]]);
}

fn report_only_relabelling_cannot_excuse_a_wrong_source_focus() {
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_blank("a", BlankScope(11));
    let c = b.intern_blank("a", BlankScope(12));
    let p = b.intern_iri("http://example.org/anchor");
    let first = b.intern_iri("http://example.org/first");
    let second = b.intern_iri("http://example.org/second");
    b.push_quad(first, p, a, None);
    b.push_quad(second, p, c, None);
    let source = b.freeze().unwrap();
    let mut reports = Vec::new();
    for label in ["expected", "actual"] {
        let mut b = RdfDatasetBuilder::new();
        let result = b.intern_blank("result", BlankScope::DEFAULT);
        let focus = b.intern_blank(label, BlankScope::DEFAULT);
        let p = b.intern_iri(sh::FOCUS_NODE);
        b.push_quad(result, p, focus, None);
        reports.push(b.freeze().unwrap());
    }
    report::compare_graphs(&reports[0], &reports[1]).unwrap();
    let sources = [report::SourceContext {
        dataset: &source,
        domain: "data",
        role: report::SourceRole::DataGraph,
    }];
    let expected = [report::BlankCorrespondence {
        report: report::BlankIdentity {
            label: "expected".into(),
            scope: BlankScope::DEFAULT,
        },
        source_role: report::SourceRole::DataGraph,
        source: report::BlankIdentity {
            label: "a".into(),
            scope: BlankScope(11),
        },
    }];
    let wrong = [report::BlankCorrespondence {
        report: report::BlankIdentity {
            label: "actual".into(),
            scope: BlankScope::DEFAULT,
        },
        source_role: report::SourceRole::DataGraph,
        source: report::BlankIdentity {
            label: "a".into(),
            scope: BlankScope(12),
        },
    }];
    let good = [report::BlankCorrespondence {
        source: expected[0].source.clone(),
        ..wrong[0].clone()
    }];
    let ctx = |index: usize, links| report::ReportContext {
        report: reports[index].as_ref(),
        root: None,
        sources: &sources,
        correspondence: links,
    };
    report::compare_contextual(ctx(0, &expected), ctx(1, &good)).unwrap();
    assert!(report::compare_contextual(ctx(0, &expected), ctx(1, &wrong)).is_err());
}

fn expected_rejection_does_not_accept_environmental_failures() {
    let expected = outcome::Observation {
        kind: outcome::OutcomeKind::IntendedRejection,
        reason: Some("profile-restriction".into()),
        message: String::new(),
    };
    outcome::grade(&expected, &expected).unwrap();
    for kind in [
        outcome::OutcomeKind::Crash,
        outcome::OutcomeKind::Malformed,
        outcome::OutcomeKind::ResourceExhausted,
        outcome::OutcomeKind::Unsupported,
        outcome::OutcomeKind::Unexecuted,
        outcome::OutcomeKind::Inapplicable,
    ] {
        assert!(
            outcome::grade(
                &expected,
                &outcome::Observation {
                    kind,
                    ..expected.clone()
                }
            )
            .is_err()
        );
    }
    let wrong = outcome::Observation {
        reason: Some("wrong-restriction".into()),
        ..expected.clone()
    };
    assert!(outcome::grade(&expected, &wrong).is_err());
}

fn manifest_lists_use_the_same_selected_graph() {
    let mut b = RdfDatasetBuilder::new();
    let root = b.intern_iri("http://example.org/manifest");
    let head = b.intern_blank("list", BlankScope::DEFAULT);
    let entry = b.intern_iri("http://example.org/case");
    let entries = b.intern_iri(mf::ENTRIES);
    let first = b.intern_iri(rdf::FIRST);
    let rest = b.intern_iri(rdf::REST);
    let nil = b.intern_iri(rdf::NIL);
    let graph = b.intern_iri("http://example.org/other");
    b.push_quad(root, entries, head, None);
    b.push_quad(head, first, entry, None);
    b.push_quad(head, rest, nil, Some(graph));
    let ds = b.freeze().unwrap();
    assert!(
        manifest::decode(graph::Reader {
            dataset: &ds,
            graph: GraphMatch::Default
        })
        .is_err()
    );
    let reader = graph::Reader {
        dataset: &ds,
        graph: GraphMatch::Any,
    };
    assert_eq!(manifest::decode(reader).unwrap().entries.len(), 1);
}

fn all_catalogued_inventory_schemas_preserve_every_case() {
    use purrdf_conformance_kit::inventory;
    let catalog =
        inventory::decode_catalog(include_bytes!("../../../corpora/community/catalog.json"))
            .unwrap();
    let inventories = [inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap()];
    assert_eq!(
        inventories
            .iter()
            .map(|inventory| inventory.cases.len())
            .sum::<usize>(),
        catalog.case_count
    );
    assert_eq!(catalog.suites.len(), inventories.len());
    let mut executions = std::collections::BTreeMap::<&str, usize>::new();
    for (suite, inventory) in catalog.suites.iter().zip(&inventories) {
        assert_eq!(suite.case_count, inventory.cases.len());
        for case in &inventory.cases {
            let profiles = case.selected_profiles().unwrap();
            assert_ne!(profiles, [] as [&str; 0]);
            for profile in profiles {
                *executions.entry(profile).or_default() += 1;
            }
            if let inventory::Expected::Outcome(spec) = &case.expected
                && matches!(
                    spec.kind.as_str(),
                    "admission-rejection" | "semantic-failure"
                )
            {
                assert!(
                    spec.reason
                        .as_deref()
                        .is_some_and(|reason| !reason.is_empty())
                );
            }
        }
    }
    assert_eq!(
        executions,
        [("shacl-20170720", 57), ("shacl12-20260918", 58)].into()
    );
}

fn unlinked_context(report: &purrdf_core::RdfDataset) -> report::ReportContext<'_> {
    report::ReportContext {
        report,
        root: None,
        sources: &[],
        correspondence: &[],
    }
}

fn parsed_report(body: &str) -> std::sync::Arc<purrdf_core::RdfDataset> {
    purrdf_rdf::parse_dataset(
        format!(
            "@prefix sh: <{}> . @prefix ex: <http://example.org/> . {body}",
            sh::NS
        )
        .as_bytes(),
        "text/turtle",
        None,
    )
    .unwrap()
}

fn report_policy_preserves_obligations_and_path_topology() {
    let expected = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:r . _:r a sh:ValidationResult; sh:focusNode ex:a; sh:sourceShape ex:Shape; sh:sourceConstraint ex:Constraint; sh:sourceConstraintComponent sh:SPARQLConstraintComponent; sh:resultSeverity sh:Violation; sh:resultPath (ex:p ex:q) .",
    );
    let good = parsed_report(
        "ex:report a sh:ValidationReport; sh:conforms false; sh:shapesGraphWellFormed true; sh:result ex:r . ex:r a sh:ValidationResult; sh:focusNode ex:a; sh:sourceShape ex:Shape; sh:sourceConstraint ex:Constraint; sh:sourceConstraintComponent sh:SPARQLConstraintComponent; sh:resultSeverity sh:Violation; sh:resultMessage \"optional\"; sh:resultPath (ex:p ex:q) .",
    );
    let policy = report::ReportPolicy {
        format: report::ReportFormat::Recommendation2017,
        allow_unstated_source_constraint: false,
    };
    report::compare_with_policy(unlinked_context(&expected), unlinked_context(&good), policy)
        .unwrap();
    for replacement in ["sh:resultPath (ex:q ex:p)", "sh:resultPath (ex:p ex:p)"] {
        let bad = parsed_report(&format!(
            "_:report a sh:ValidationReport; sh:conforms false; sh:result _:r . _:r a sh:ValidationResult; sh:focusNode ex:a; sh:sourceShape ex:Shape; sh:sourceConstraint ex:Constraint; sh:sourceConstraintComponent sh:SPARQLConstraintComponent; sh:resultSeverity sh:Violation; {replacement} ."
        ));
        assert!(
            report::compare_with_policy(
                unlinked_context(&expected),
                unlinked_context(&bad),
                policy
            )
            .is_err()
        );
    }
    let missing = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:r . _:r a sh:ValidationResult; sh:focusNode ex:a; sh:sourceShape ex:Shape; sh:sourceConstraintComponent sh:SPARQLConstraintComponent; sh:resultSeverity sh:Violation; sh:resultPath (ex:p ex:q) .",
    );
    assert!(
        report::compare_with_policy(
            unlinked_context(&expected),
            unlinked_context(&missing),
            report::ReportPolicy {
                allow_unstated_source_constraint: true,
                ..policy
            }
        )
        .is_err()
    );
    let duplicate = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:r, _:extra . _:r a sh:ValidationResult; sh:focusNode ex:a . _:extra a sh:ValidationResult; sh:focusNode ex:a .",
    );
    assert!(
        report::compare_with_policy(
            unlinked_context(&expected),
            unlinked_context(&duplicate),
            policy
        )
        .is_err()
    );
}

fn optional_details_are_selected_per_corresponding_result() {
    let expected = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:a, _:b . _:a a sh:ValidationResult; sh:focusNode ex:a; sh:detail _:detail . _:detail a sh:ValidationResult; sh:focusNode ex:c . _:b a sh:ValidationResult; sh:focusNode ex:b .",
    );
    let good = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:a, _:b . _:a a sh:ValidationResult; sh:focusNode ex:a; sh:detail _:detail . _:detail a sh:ValidationResult; sh:focusNode ex:c . _:b a sh:ValidationResult; sh:focusNode ex:b; sh:detail _:optional . _:optional a sh:ValidationResult; sh:focusNode ex:d .",
    );
    let policy = report::ReportPolicy {
        format: report::ReportFormat::Draft20260918,
        allow_unstated_source_constraint: false,
    };
    report::compare_with_policy(unlinked_context(&expected), unlinked_context(&good), policy)
        .unwrap();
    let missing = parsed_report(
        "_:report a sh:ValidationReport; sh:conforms false; sh:result _:a, _:b . _:a a sh:ValidationResult; sh:focusNode ex:a . _:b a sh:ValidationResult; sh:focusNode ex:b; sh:detail _:detail . _:detail a sh:ValidationResult; sh:focusNode ex:c .",
    );
    assert!(
        report::compare_with_policy(
            unlinked_context(&expected),
            unlinked_context(&missing),
            policy
        )
        .is_err()
    );
}

fn deliberate_shared_domain_allows_only_cross_role_aliases() {
    let mut b = RdfDatasetBuilder::new();
    let blank = b.intern_blank("shared", BlankScope(7));
    let anchor = b.intern_iri("http://example.org/anchor");
    let p = b.intern_iri("http://example.org/p");
    b.push_quad(anchor, p, blank, None);
    let source = b.freeze().unwrap();
    let expected = parsed_report("_:r sh:focusNode _:shared; sh:sourceShape _:shared .");
    let actual = parsed_report("_:r sh:focusNode _:dg; sh:sourceShape _:sg .");
    let sources = [
        report::SourceContext {
            dataset: &source,
            domain: "deliberately-shared",
            role: report::SourceRole::DataGraph,
        },
        report::SourceContext {
            dataset: &source,
            domain: "deliberately-shared",
            role: report::SourceRole::ShapesGraph,
        },
    ];
    let source_identity = report::BlankIdentity {
        label: "shared".into(),
        scope: BlankScope(7),
    };
    let links = |labels: &[(&str, report::SourceRole)]| {
        labels
            .iter()
            .map(|&(label, role)| report::BlankCorrespondence {
                report: report::BlankIdentity {
                    label: label.into(),
                    scope: BlankScope::DEFAULT,
                },
                source_role: role,
                source: source_identity.clone(),
            })
            .collect::<Vec<_>>()
    };
    let expected_links = links(&[("shared", report::SourceRole::DataGraph)]);
    let actual_links = links(&[
        ("dg", report::SourceRole::DataGraph),
        ("sg", report::SourceRole::ShapesGraph),
    ]);
    let context = |report, correspondence| report::ReportContext {
        report,
        root: None,
        sources: &sources,
        correspondence,
    };
    report::compare_contextual(
        context(&expected, &expected_links),
        context(&actual, &actual_links),
    )
    .unwrap();
    let mut wrong = actual_links.clone();
    wrong[1].source_role = report::SourceRole::DataGraph;
    assert!(
        report::compare_contextual(
            context(&expected, &expected_links),
            context(&actual, &wrong)
        )
        .is_err()
    );
    let separate = [
        sources[0],
        report::SourceContext {
            domain: "independent-shapes",
            ..sources[1]
        },
    ];
    assert!(
        report::compare_contextual(
            context(&expected, &expected_links),
            report::ReportContext {
                sources: &separate,
                ..context(&actual, &actual_links)
            }
        )
        .is_err()
    );
    let duplicate = [actual_links[0].clone(), actual_links[0].clone()];
    assert!(
        report::compare_contextual(
            context(&expected, &expected_links),
            context(&actual, &duplicate)
        )
        .is_err()
    );
}

fn composite_literals_share_direct_and_nested_blank_identity() {
    use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
    let direct = TermValue::Blank {
        label: "same".into(),
        scope: BlankScope(17),
    };
    let spelling = encode_blank_label("same", BlankScope(17), LabelAlphabet::BlankNodeLabel);
    let literal = TermValue::Literal {
        lexical_form: format!(
            "[_:{spelling}, <<( _:{spelling} <http://example.org/p> <http://example.org/o> )>>]"
        ),
        datatype: purrdf_cdt::CDT_LIST.into(),
        language: None,
        direction: None,
    };
    let header = ["direct".into(), "list".into()];
    let expected = vec![vec![Some(direct), Some(literal.clone())]];
    let different = vec![vec![
        Some(TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(18),
        }),
        Some(literal),
    ]];
    assert!(bag::compare_solutions(&header, &expected, &header, &different, false).is_err());
}

fn composite_blank_renaming_is_global_across_permuted_rows() {
    use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
    let row = |label: &str, scope| {
        let spelling = encode_blank_label(label, scope, LabelAlphabet::BlankNodeLabel);
        vec![
            Some(TermValue::Blank {
                label: label.into(),
                scope,
            }),
            Some(TermValue::Literal {
                lexical_form: format!(
                    "[_:{spelling}, <<( _:{spelling} <http://example.org/p> <http://example.org/o> )>>]"
                ),
                datatype: purrdf_cdt::CDT_LIST.into(),
                language: None,
                direction: None,
            }),
        ]
    };
    let expected = [row("same", BlankScope(11)), row("same", BlankScope(12))];
    let renamed = [row("second", BlankScope(2)), row("first", BlankScope(2))];
    bag::compare_solutions(
        &["direct".into(), "list".into()],
        &expected,
        &["direct".into(), "list".into()],
        &renamed,
        false,
    )
    .unwrap();
}

fn source_metadata_and_empty_graph_declarations_are_observable() {
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let mut b = RdfDatasetBuilder::new();
    let graph = b.intern_blank("empty", BlankScope(4));
    b.declare_named_graph(graph);
    let declared = b.freeze().unwrap();
    assert!(report::compare_graphs(&empty, &declared).is_err());
    let mut b = RdfDatasetBuilder::new();
    let graph = b.intern_blank("renamed", BlankScope(9));
    b.declare_named_graph(graph);
    report::compare_graphs(&declared, &b.freeze().unwrap()).unwrap();
    let source = report::SourceContext {
        dataset: &empty,
        domain: "source",
        role: report::SourceRole::DataGraph,
    };
    assert!(report::expected_source_correspondence(&empty, &[source, source]).is_err());
    assert!(
        report::expected_source_correspondence(
            &empty,
            &[report::SourceContext {
                domain: "",
                ..source
            }]
        )
        .is_err()
    );
}

fn earl_verdicts_require_valid_expectations_and_actual_observations() {
    use purrdf_lex::json::record::{FromJson as _, ToJson as _};
    let expected = outcome::Observation {
        kind: outcome::OutcomeKind::IntendedRejection,
        reason: Some("prebound-values".into()),
        message: String::new(),
    };
    let good = outcome::Record {
        case: "http://example.org/case".into(),
        profile: "shacl-20170720".into(),
        surface: "native".into(),
        expected: expected.clone(),
        observed: expected,
        passed: true,
        artifacts: vec![],
    };
    let json = good.to_json();
    assert_eq!(outcome::Record::from_json(&json).unwrap(), good);
    let mut records = vec![good];
    let earl = outcome::to_earl(
        &records,
        "http://example.org/assertor",
        "http://example.org/engine",
    )
    .unwrap();
    assert_eq!(earl.quad_count(), 8);
    records[0].observed.kind = outcome::OutcomeKind::Crash;
    assert!(
        outcome::to_earl(
            &records,
            "http://example.org/assertor",
            "http://example.org/engine"
        )
        .is_err()
    );
    records[0].passed = false;
    outcome::to_earl(
        &records,
        "http://example.org/assertor",
        "http://example.org/engine",
    )
    .unwrap();
    records[0].expected.kind = outcome::OutcomeKind::Crash;
    assert!(
        outcome::to_earl(
            &records,
            "http://example.org/assertor",
            "http://example.org/engine"
        )
        .is_err()
    );
}

fn deep_terms_respect_kernel_admission_without_stack_recursion() {
    let nested = |label: &str, scope, depth| {
        let blank = TermValue::Blank {
            label: label.into(),
            scope,
        };
        let mut quoted = blank.clone();
        for _ in 0..depth {
            quoted = TermValue::Triple {
                s: Box::new(TermValue::Iri("http://example.org/s".into())).into(),
                p: Box::new(TermValue::Iri("http://example.org/p".into())).into(),
                o: Box::new(quoted).into(),
            };
        }
        vec![Some(blank), Some(quoted)]
    };
    let variables = ["direct".into(), "quoted".into()];
    let deep = [nested("source", BlankScope(11), 100_000)];
    assert!(matches!(
        bag::canonical_solutions(&variables, &deep, false),
        Err(purrdf_conformance_kit::GradeError::Malformed(_))
    ));
    let expected = [nested("source", BlankScope(11), 16)];
    let mut actual = [nested("renamed", BlankScope(2), 16)];
    bag::compare_solutions(&variables, &expected, &variables, &actual, false).unwrap();
    actual[0][0] = Some(TermValue::Blank {
        label: "renamed".into(),
        scope: BlankScope(3),
    });
    assert!(bag::compare_solutions(&variables, &expected, &variables, &actual, false).is_err());
}

fn malformed_result_set_bindings_and_partial_indices_are_refused() {
    let parse = |body: &str| {
        purrdf_rdf::parse_dataset(
            format!("@prefix rs: <{}> . {body}", rs::NS).as_bytes(),
            "text/turtle",
            None,
        )
        .unwrap()
    };
    for body in [
        "_:root a rs:ResultSet; rs:resultVariable \"x\"; rs:solution [ rs:binding [ rs:variable \"undeclared\"; rs:value <http://example.org/value> ] ] .",
        "_:root a rs:ResultSet; rs:resultVariable \"x\"; rs:solution [ rs:binding [ rs:variable \"x\"; rs:value <http://example.org/a> ], [ rs:variable \"x\"; rs:value <http://example.org/b> ] ] .",
        "_:root a rs:ResultSet; rs:solution [ rs:index 1 ], [] .",
        "_:root a rs:ResultSet; rs:solution [ rs:index 1 ], [ rs:index 1 ] .",
        "_:root a rs:ResultSet; rs:solution \"not a solution resource\" .",
        "_:root a rs:ResultSet; rs:solution [ rs:binding \"not a binding resource\" ] .",
    ] {
        let dataset = parse(body);
        assert!(
            result_set::decode(graph::Reader {
                dataset: &dataset,
                graph: GraphMatch::Default
            })
            .is_err()
        );
    }
}

fn inference_metadata_requires_three_term_rows_and_keeps_source_scope() {
    let parse =
        |body: &str| purrdf_rdf::parse_dataset(body.as_bytes(), "text/turtle", None).unwrap();
    let valid = parse(
        "<http://example.org/root> <http://example.org/expected> ((_:source <http://example.org/p> true)) .",
    );
    let root = valid
        .quads()
        .find(|quad| matches!(valid.term_value(quad.s), TermValue::Iri(_)))
        .unwrap()
        .o;
    let rows = manifest::inference_triples(
        graph::Reader {
            dataset: &valid,
            graph: GraphMatch::Default,
        },
        root,
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0][0],
        TermValue::Blank {
            label: "source".into(),
            scope: BlankScope::DEFAULT
        }
    );
    let invalid = parse(
        "<http://example.org/root> <http://example.org/expected> ((_:source <http://example.org/p>)) .",
    );
    let root = invalid
        .quads()
        .find(|quad| matches!(invalid.term_value(quad.s), TermValue::Iri(_)))
        .unwrap()
        .o;
    assert!(
        manifest::inference_triples(
            graph::Reader {
                dataset: &invalid,
                graph: GraphMatch::Default
            },
            root
        )
        .is_err()
    );
}

fn inventory_operation_and_carrier_cannot_be_changed_independently() {
    use purrdf_lex::json::record::ToJson as _;
    let mut inventory = purrdf_conformance_kit::inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap();
    let json = purrdf_lex::json::write_compact(&inventory.to_json());
    assert_eq!(
        purrdf_conformance_kit::inventory::decode_inventory(json.as_bytes()).unwrap(),
        inventory
    );
    "query-evaluation".clone_into(&mut inventory.cases[0].kind);
    let json = purrdf_lex::json::write_compact(&inventory.to_json());
    assert!(purrdf_conformance_kit::inventory::decode_inventory(json.as_bytes()).is_err());
}

fn accepted_oracles_require_exact_input_and_derivation_bytes() {
    use purrdf_conformance_kit::{inventory, reviews};
    use purrdf_lex::json::record::ToJson as _;
    let mut catalog =
        inventory::decode_catalog(include_bytes!("../../../corpora/community/catalog.json"))
            .unwrap();
    catalog.suites.truncate(1);
    catalog.suites[0].case_count = 1;
    catalog.case_count = 1;
    let mut inventory = inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap();
    inventory.cases.truncate(1);
    let case = format!("shacl/{}", inventory.cases[0].id);
    let manifest = format!("{case}/manifest.ttl");
    let shapes = format!("{case}/shapes.ttl");
    let payloads = [
        reviews::Payload {
            path: "shacl/manifest.ttl",
            bytes: b"independent suite manifest",
        },
        reviews::Payload {
            path: &manifest,
            bytes: b"independent manifest, data and expected report",
        },
        reviews::Payload {
            path: &shapes,
            bytes: b"independent shapes",
        },
        reviews::Payload {
            path: "reviews/shacl.md",
            bytes: b"independent derivation and approval",
        },
    ];
    let index = reviews::Index {
        contract_version: 1,
        copyright: "2026 fixture author".into(),
        license: "CC0-1.0".into(),
        cases: vec![reviews::Case {
            id: inventory.cases[0].id.clone(),
            status: "accepted".into(),
            reviewer: "independent fixture reviewer".into(),
            derivation: "reviews/shacl.md".into(),
            payloads: payloads
                .iter()
                .map(|payload| reviews::Artifact {
                    path: payload.path.to_owned(),
                    blake3: purrdf_hash::hex::Digest32::new(
                        *purrdf_hash::blake3::hash(payload.bytes).as_bytes(),
                    )
                    .to_hex(),
                })
                .collect(),
        }],
    };
    let json = purrdf_lex::json::write_compact(&index.to_json());
    assert_eq!(reviews::decode(json.as_bytes()).unwrap(), index);
    let inventories = [inventory];
    reviews::admit(&catalog, &inventories, &index, &payloads).unwrap();
    for position in [1, 2, 3] {
        let mut changed = payloads;
        changed[position].bytes = b"changed after approval";
        assert!(reviews::admit(&catalog, &inventories, &index, &changed).is_err());
    }
    assert!(reviews::admit(&catalog, &inventories, &index, &payloads[..3]).is_err());
    let mut wrong = index.clone();
    "unreviewed".clone_into(&mut wrong.cases[0].status);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases[0].payloads.pop();
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "catalog.json".clone_into(&mut wrong.cases[0].derivation);
    "catalog.json".clone_into(&mut wrong.cases[0].payloads[3].path);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "../outside.md".clone_into(&mut wrong.cases[0].derivation);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases.push(wrong.cases[0].clone());
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "other-case".clone_into(&mut wrong.cases[0].id);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases[0].payloads[0].blake3.make_ascii_uppercase();
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index;
    "shacl/another-case/shapes.ttl".clone_into(&mut wrong.cases[0].payloads[2].path);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
}

fn service_inventory_preserves_independent_declared_sources() {
    use purrdf_conformance_kit::inventory::{self, Input, ServiceInput};
    use purrdf_lex::json::record::ToJson as _;
    let mut inventory = inventory::decode_inventory(
        br#"{"contractVersion": 1, "copyright": "2026 fixture author", "license": "CC0-1.0",
            "cases": [{"id": "service-case", "kind": "query-evaluation",
                "query": "service.rq", "data": [{"path": "local.ttl"}],
                "expected": "service.srj", "profile": "fixture-profile",
                "categories": ["SERVICE"],
                "specifications": ["https://www.w3.org/TR/sparql11-federated-query/"],
                "rationale": "independent fixture rationale"}]}"#,
    )
    .unwrap();
    inventory.cases[0].services = ["a", "b"]
        .into_iter()
        .map(|endpoint| ServiceInput {
            endpoint: format!("http://example.org/service/{endpoint}"),
            data: Input {
                path: format!("endpoint-{endpoint}.ttl"),
                media_type: Some("text/turtle".into()),
                graph: None,
            },
        })
        .collect();
    let encode =
        |inventory: &inventory::Inventory| purrdf_lex::json::write_compact(&inventory.to_json());
    assert_eq!(
        inventory::decode_inventory(encode(&inventory).as_bytes()).unwrap(),
        inventory
    );
    let mut wrong = inventory.clone();
    let (first, second) = wrong.cases[0].services.split_at_mut(1);
    second[0].endpoint.clone_from(&first[0].endpoint);
    assert!(inventory::decode_inventory(encode(&wrong).as_bytes()).is_err());
    let mut wrong = inventory.clone();
    "../relative-endpoint".clone_into(&mut wrong.cases[0].services[0].endpoint);
    assert!(inventory::decode_inventory(encode(&wrong).as_bytes()).is_err());
    let mut wrong = inventory.clone();
    wrong.cases[0].services[0].data.graph = Some("http://example.org/wrong-graph".into());
    assert!(inventory::decode_inventory(encode(&wrong).as_bytes()).is_err());
    let mut wrong = inventory;
    "negative-syntax".clone_into(&mut wrong.cases[0].kind);
    wrong.cases[0].expected = inventory::Expected::None;
    assert!(inventory::decode_inventory(encode(&wrong).as_bytes()).is_err());
}

purrdf_testkit::prop_test! {
    #![prop_config(purrdf_testkit::prop::Config::with_cases(128))]
    fn seeded_global_bags_preserve_bijection_and_duplicate_counts(
        width in 1_usize..5, count in 2_usize..9, offset in 1_u32..128,
        include_empty in purrdf_testkit::prop::any::<bool>()
    ) {
        let variables: Vec<_> = (0..width).map(|column| format!("v{column}")).collect();
        let rows: Vec<_> = (0..count).map(|row| (0..width).map(|column| {
            if (row + column) % 3 == 0 { None } else { Some(TermValue::Blank { label: "same".into(), scope: BlankScope(u32::try_from((row + column) % count + 1).unwrap()) }) }
        }).collect::<Vec<_>>()).collect();
        let mut rows = rows;
        if include_empty { rows.push(vec![None; width]); }
        let renamed: Vec<_> = rows.iter().rev().map(|row| row.iter().map(|value| value.as_ref().map(|value| match value {
            TermValue::Blank { scope, .. } => TermValue::Blank { label: format!("renamed{}", scope.ordinal()), scope: BlankScope(offset) },
            value => value.clone(),
        })).collect::<Vec<_>>()).collect();
        purrdf_testkit::prop_assert!(bag::compare_solutions(&variables, &rows, &variables, &renamed, false).is_ok());
        let mut lost = renamed;
        lost.pop();
        purrdf_testkit::prop_assert!(bag::compare_solutions(&variables, &rows, &variables, &lost, false).is_err());
    }
}

purrdf_testkit::harness_main!(
    empty_and_unbound_rows_are_observable,
    scope_partition_and_global_row_links_are_observable,
    nested_triples_share_the_same_global_bijection,
    header_order_is_metadata_and_row_order_is_explicit,
    structural_result_reader_keeps_all_empty_solutions,
    report_only_relabelling_cannot_excuse_a_wrong_source_focus,
    expected_rejection_does_not_accept_environmental_failures,
    manifest_lists_use_the_same_selected_graph,
    all_catalogued_inventory_schemas_preserve_every_case,
    report_policy_preserves_obligations_and_path_topology,
    optional_details_are_selected_per_corresponding_result,
    deliberate_shared_domain_allows_only_cross_role_aliases,
    composite_literals_share_direct_and_nested_blank_identity,
    composite_blank_renaming_is_global_across_permuted_rows,
    source_metadata_and_empty_graph_declarations_are_observable,
    earl_verdicts_require_valid_expectations_and_actual_observations,
    deep_terms_respect_kernel_admission_without_stack_recursion,
    malformed_result_set_bindings_and_partial_indices_are_refused,
    inference_metadata_requires_three_term_rows_and_keeps_source_scope,
    inventory_operation_and_carrier_cannot_be_changed_independently,
    accepted_oracles_require_exact_input_and_derivation_bytes,
    service_inventory_preserves_independent_declared_sources,
    seeded_global_bags_preserve_bijection_and_duplicate_counts,
);
