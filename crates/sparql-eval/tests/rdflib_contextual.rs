// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Native compiled-plan execution against frozen RDFLib 7.6 contextual answers.
//! Fixtures preserve columns, bags, ordered sequences, RDF terms and graph triples.
use purrdf_core::{RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_lex::json::{self, Value};
use purrdf_sparql_algebra::{GroundTerm, NamedNode, Variable};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
fn cell(cell: Option<TermValue>) -> Value {
    match cell {
        None => Value::Null,
        Some(TermValue::Iri(iri)) => {
            Value::Array(vec![Value::String("iri".into()), Value::String(iri)])
        }
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => Value::Array(vec![
            Value::String("literal".into()),
            Value::String(lexical_form),
            if datatype == purrdf_xsd::datatype::XSD_STRING {
                Value::Null
            } else {
                Value::String(datatype)
            },
            language.map_or(Value::Null, Value::String),
        ]),
        Some(term) => panic!("unexpected term {term:?}"),
    }
}
#[test]
fn contextual_matrix() {
    let fixtures = json::read(include_str!("fixtures/contextual-mappings.json")).unwrap();
    let mut failures = Vec::new();
    let mut passed = 0;
    for fixture in fixtures.as_array().unwrap() {
        let mut builder = RdfDatasetBuilder::new();
        for triple in fixture["triples"].as_array().unwrap() {
            let triple = triple.as_array().unwrap();
            let s = builder.intern_iri(&format!(
                "http://example.org/{}",
                triple[0].as_str().unwrap()
            ));
            let p = builder.intern_iri(&format!(
                "http://example.org/{}",
                triple[1].as_str().unwrap()
            ));
            let o = builder.intern_iri(&format!(
                "http://example.org/{}",
                triple[2].as_str().unwrap()
            ));
            builder.push_quad(s, p, o, None);
        }
        if let Some(quads) = fixture["quads"].as_array() {
            for quad in quads {
                let quad = quad.as_array().unwrap();
                let terms: Vec<_> = quad
                    .iter()
                    .map(|term| {
                        builder
                            .intern_iri(&format!("http://example.org/{}", term.as_str().unwrap()))
                    })
                    .collect();
                builder.push_quad(terms[0], terms[1], terms[2], Some(terms[3]));
            }
        }
        let data = builder.freeze().unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let text = case["query"].as_str().unwrap();
            let bindings: Vec<_> = case["initial"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, value)| {
                    (
                        Variable::new(name),
                        GroundTerm::NamedNode(NamedNode::new_unchecked(value.as_str().unwrap())),
                    )
                })
                .collect();
            let engine = NativeSparqlEngine::new();
            use purrdf_sparql_eval::{Arity, ExtensionEnv, UserFunctionRegistry, Volatility};
            use std::sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            };
            let calls = Arc::new(AtomicUsize::new(0));
            let counted = Arc::clone(&calls);
            let mut functions = UserFunctionRegistry::default();
            if case["count_callback"] == Value::Bool(true) {
                functions.register_native(
                    "http://example.org/counter",
                    Arity::Exact(1),
                    Volatility::Volatile,
                    Arc::new(move |_| {
                        let ordinal = counted.fetch_add(1, Ordering::Relaxed) + 1;
                        Ok(Some(TermValue::typed_literal(
                            ordinal.to_string(),
                            purrdf_xsd::datatype::XSD_INTEGER,
                        )))
                    }),
                );
            }
            let functions = engine
                .bind_functions(functions, ExtensionEnv::empty())
                .unwrap();
            let options = QueryOptions::EMPTY.with_functions(&functions);
            let actual = engine
                .prepare_rdflib_query(text, None, &bindings, options)
                .and_then(|plan| engine.query_rdflib_prepared_view(&*data, &plan, options));
            let oracle = &case["oracle"];
            let actual = match actual {
                Err(e) => {
                    failures.push(format!("{name}: {e:?}"));
                    continue;
                }
                Ok(SparqlResult::Solutions {
                    variables, rows, ..
                }) => {
                    let columns = oracle["columns"].as_array().unwrap();
                    let actual_columns: Vec<_> = variables
                        .iter()
                        .map(|name| Value::String(name.clone()))
                        .collect();
                    let mut actual_columns_sorted = actual_columns;
                    actual_columns_sorted.sort_by_key(ToString::to_string);
                    if Value::Array(actual_columns_sorted) != oracle["columns"] {
                        failures.push(format!(
                            "{name}: columns {variables:?}, expected {columns:?}"
                        ));
                        continue;
                    }
                    let mut used = vec![false; variables.len()];
                    let indices: Vec<_> = columns
                        .iter()
                        .map(|v| {
                            let index = variables
                                .iter()
                                .enumerate()
                                .position(|(index, name)| {
                                    !used[index] && Some(name.as_str()) == v.as_str()
                                })
                                .unwrap();
                            used[index] = true;
                            index
                        })
                        .collect();
                    let mut rows: Vec<_> = rows
                        .into_iter()
                        .map(|row| {
                            Value::Array(indices.iter().map(|i| cell(row[*i].clone())).collect())
                        })
                        .collect();
                    if case["ordered"] != Value::Bool(true) {
                        rows.sort_by_key(ToString::to_string);
                    }
                    Value::Array(rows)
                }
                Ok(SparqlResult::Boolean(value)) => Value::Bool(value),
                Ok(SparqlResult::Graph(graph)) => {
                    let mut triples: Vec<_> = graph
                        .quads()
                        .map(|q| {
                            Value::Array(vec![
                                cell(Some(graph.term_value(q.s))),
                                cell(Some(graph.term_value(q.p))),
                                cell(Some(graph.term_value(q.o))),
                            ])
                        })
                        .collect();
                    triples.sort_by_key(ToString::to_string);
                    Value::Array(triples)
                }
            };
            let mut expected = match oracle["type"].as_str().unwrap() {
                "SELECT" => oracle["rows"].clone(),
                "ASK" => match (oracle["answer"].as_bool(), oracle["boolean"].as_bool()) {
                    (Some(answer), None) | (None, Some(answer)) => Value::Bool(answer),
                    _ => panic!("ASK requires one explicit boolean answer"),
                },
                _ => oracle["triples"].clone(),
            };
            if case["ordered"] != Value::Bool(true)
                && let Value::Array(values) = &mut expected
            {
                values.sort_by_key(ToString::to_string);
            }
            if actual == expected {
                if case["count_callback"] == Value::Bool(true) {
                    assert_eq!(
                        calls.load(Ordering::Relaxed).to_string(),
                        oracle["callback_invocations"].to_string(),
                        "{name}: every successful RHS runs once"
                    );
                }
                passed += 1;
            } else {
                failures.push(format!("{name}: got {actual}, expected {expected}"));
            }
        }
    }
    eprintln!("{passed} oracle cases passed; {} failures", failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(passed, 241, "the entire frozen oracle matrix executed");
}

#[test]
fn contextual_application_keeps_one_optional_condition_and_rhs() {
    use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
    use purrdf_sparql_algebra::{Expression, GraphPattern};
    let mut previous = None;
    for levels in [16usize, 32, 64, 128] {
        let mut body = "BIND(<http://example.org/a> AS ?this)".to_owned();
        for _ in 0..levels {
            body = format!(
                "BIND(<http://example.org/a> AS ?this) OPTIONAL {{ {body} FILTER EXISTS {{ BIND(1 AS ?ok) }} }}"
            );
        }
        let query = format!("SELECT ?this WHERE {{ {body} }}");
        let plan = NativeSparqlEngine::new()
            .prepare_rdflib_query(&query, None, &[], QueryOptions::EMPTY)
            .unwrap();
        let mut optional = 0;
        let mut exists = 0;
        let mut nodes = 0;
        walk_pre_post(NodeRef::Pattern(plan.query().pattern()), |visit, node| {
            if visit == Visit::Enter {
                nodes += 1;
                if matches!(node,NodeRef::Pattern(GraphPattern::Apply {policy,..}) if policy.optional.is_some())
                {
                    optional += 1;
                }
                if matches!(node, NodeRef::Expr(Expression::Exists(_))) {
                    exists += 1;
                }
            }
            Flow::Descend
        });
        assert_eq!(optional, levels);
        assert_eq!(
            exists, levels,
            "each condition owns its executable body once"
        );
        let bytes = plan.retained_size_bytes();
        eprintln!("optional levels={levels} nodes={nodes} bytes={bytes}");
        if let Some((old_nodes, old_bytes)) = previous {
            assert!(nodes <= 2 * old_nodes, "node growth is proportional");
            assert!(
                bytes <= 2 * old_bytes,
                "fixed-width aliases keep byte growth proportional"
            );
        }
        previous = Some((nodes, bytes));
    }
}

#[test]
fn successful_optional_rhs_is_invoked_once_per_duplicate_driver() {
    use purrdf_sparql_eval::{Arity, ExtensionEnv, UserFunctionRegistry, Volatility};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let mut functions = UserFunctionRegistry::default();
    functions.register_native(
        "http://example.org/tick",
        Arity::Exact(0),
        Volatility::Volatile,
        Arc::new(move |_| {
            counted.fetch_add(1, Ordering::Relaxed);
            Ok(Some(TermValue::typed_literal(
                "1",
                purrdf_xsd::datatype::XSD_INTEGER,
            )))
        }),
    );
    let engine = NativeSparqlEngine::new();
    let functions = engine
        .bind_functions(functions, ExtensionEnv::empty())
        .unwrap();
    let options = QueryOptions::EMPTY.with_functions(&functions);
    let text = "SELECT ?driver ?value WHERE { VALUES ?driver { 1 1 } OPTIONAL { BIND(<http://example.org/tick>() AS ?value) } }";
    let plan = engine
        .prepare_rdflib_query(text, None, &[], options)
        .unwrap();
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let result = engine
        .query_rdflib_prepared_view(&*dataset, &plan, options)
        .unwrap();
    assert_eq!(
        calls.load(Ordering::Relaxed),
        2,
        "successful RHS blocks are never retried"
    );
    let (columns, rows) = result.solutions().unwrap();
    assert_eq!(columns, ["driver", "value"]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], rows[1], "bag duplicates remain distinct drivers");
}

#[test]
fn contextual_service_uses_serializable_protocol_algebra_and_shared_source() {
    use purrdf_sparql_eval::InProcessServiceResolver;
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri("http://example.org/a");
    let b = builder.intern_iri("http://example.org/b");
    let p = builder.intern_iri("http://example.org/p");
    let one = builder.intern_iri("http://example.org/one");
    let two = builder.intern_iri("http://example.org/two");
    builder.push_quad(a, p, one, None);
    builder.push_quad(b, p, two, None);
    let remote = builder.freeze().unwrap();
    let resolver =
        InProcessServiceResolver::new().with_endpoint("http://example.org/service", remote);
    let mut options = QueryOptions::EMPTY;
    options.remote = Some(&resolver);
    let engine = NativeSparqlEngine::new();
    let bindings = [(
        Variable::new("this"),
        GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/a")),
    )];
    let query = "SELECT ?this ?value WHERE { BIND(<http://example.org/b> AS ?this) SERVICE <http://example.org/service> { ?this <http://example.org/p> ?value } }";
    let plan = engine
        .prepare_rdflib_query(query, None, &bindings, options)
        .unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let result = engine
        .query_rdflib_prepared_view(&*data, &plan, options)
        .unwrap();
    let (variables, rows) = result.solutions().unwrap();
    assert_eq!(variables, ["this", "value"]);
    assert_eq!(
        rows,
        [vec![
            Some(TermValue::iri("http://example.org/b")),
            Some(TermValue::iri("http://example.org/one"))
        ]]
    );
}

#[test]
fn configured_relations_receive_the_context_in_the_existing_call_home() {
    use purrdf_sparql_eval::{ExtensionEnv, MemoryRelation, PropertyFunctionRegistry};
    use std::sync::Arc;
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(
        "http://example.org/relation",
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![
                    vec![
                        TermValue::iri("http://example.org/a"),
                        TermValue::iri("http://example.org/one"),
                    ],
                    vec![
                        TermValue::iri("http://example.org/b"),
                        TermValue::iri("http://example.org/two"),
                    ],
                ],
            )
            .unwrap(),
        ),
    );
    let env = ExtensionEnv::over_relations(relations).unwrap();
    let options = QueryOptions::EMPTY.with_env(&env);
    let engine = NativeSparqlEngine::new();
    let bindings = [(
        Variable::new("this"),
        GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/a")),
    )];
    let text = "SELECT ?this ?value WHERE { BIND(<http://example.org/b> AS ?this) ?this <http://example.org/relation> ?value }";
    let plan = engine
        .prepare_rdflib_query(text, None, &bindings, options)
        .unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let result = engine
        .query_rdflib_prepared_view(&*data, &plan, options)
        .unwrap();
    assert_eq!(
        result.solutions().unwrap().1,
        [vec![
            Some(TermValue::iri("http://example.org/b")),
            Some(TermValue::iri("http://example.org/one"))
        ]]
    );
}

#[test]
fn initial_terms_keep_rdf12_identity_and_hygienic_aliases() {
    use purrdf_sparql_algebra::{BaseDirection, BlankNode, Child, GroundTriple, Literal};
    let terms = [
        GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/focus")),
        GroundTerm::Literal(Literal::new_simple("plain")),
        GroundTerm::Literal(Literal::new_typed(
            "01",
            NamedNode::new_unchecked(purrdf_xsd::datatype::XSD_INTEGER),
        )),
        GroundTerm::Literal(Literal::new_lang("texte", "fr", None)),
        GroundTerm::Literal(Literal::new_lang("نص", "ar", Some(BaseDirection::Rtl))),
        GroundTerm::BlankNode(BlankNode::new("purrdfesc7_focus")),
        GroundTerm::Triple(Child::new(GroundTriple {
            subject: GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/s")),
            predicate: NamedNode::new_unchecked("http://example.org/p"),
            object: GroundTerm::Literal(Literal::new_lang("نص", "ar", Some(BaseDirection::Rtl))),
        })),
    ];
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    for term in terms {
        let expected = purrdf_sparql_eval::convert::ground_term_to_value(&term);
        let text = "SELECT ?this ?value ?__purrdf_mapping_0000000000000000 WHERE { BIND(?this AS ?value) BIND(1/0 AS ?this) BIND(?value AS ?__purrdf_mapping_0000000000000000) }";
        let plan = engine
            .prepare_rdflib_query(
                text,
                None,
                &[(Variable::new("this"), term)],
                QueryOptions::EMPTY,
            )
            .unwrap();
        let result = engine
            .query_rdflib_prepared_view(&*data, &plan, QueryOptions::EMPTY)
            .unwrap();
        let (columns, rows) = result.solutions().unwrap();
        assert_eq!(
            columns,
            ["this", "value", "__purrdf_mapping_0000000000000000"]
        );
        assert_eq!(
            rows,
            &[vec![
                Some(expected.clone()),
                Some(expected.clone()),
                Some(expected)
            ]]
        );
    }
}

#[test]
fn production_entry_executes_nested_exists_on_an_admitted_stack() {
    purrdf_stack::on_stack(32 * 1024 * 1024, || {
        let mut body = "BIND(1 AS ?x)".to_owned();
        for _ in 0..32 {
            body = format!("FILTER EXISTS {{ {body} }}");
        }
        let text = format!("ASK {{ {body} }}");
        let engine = NativeSparqlEngine::new();
        let data = RdfDatasetBuilder::new().freeze().unwrap();
        let plan = engine
            .prepare_rdflib_query(&text, None, &[], QueryOptions::EMPTY)
            .unwrap();
        assert!(matches!(
            engine
                .query_rdflib_prepared_view(&*data, &plan, QueryOptions::EMPTY)
                .unwrap(),
            SparqlResult::Boolean(true)
        ));
    })
    .unwrap();
}

#[test]
fn contextual_representation_scales_with_nodes_and_context_width() {
    use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
    for width in [1usize, 4, 16] {
        let mut previous = None;
        for levels in [8usize, 16, 32] {
            use std::fmt::Write;
            let mut seed = String::new();
            for v in 0..width {
                write!(&mut seed, "BIND({v} AS ?v{v}) ").unwrap();
            }
            let mut body = seed.clone();
            for _ in 0..levels {
                body = format!("{seed} OPTIONAL {{ {body} }}");
            }
            let text = format!("SELECT * WHERE {{ {body} }}");
            let plan = NativeSparqlEngine::new()
                .prepare_rdflib_query(&text, None, &[], QueryOptions::EMPTY)
                .unwrap();
            let mut nodes = 0;
            walk_pre_post(NodeRef::Pattern(plan.query().pattern()), |visit, _| {
                if visit == Visit::Enter {
                    nodes += 1;
                }
                Flow::Descend
            });
            let bytes = plan.retained_size_bytes();
            if let Some((old_nodes, old_bytes)) = previous {
                assert!(nodes <= 2 * old_nodes);
                assert!(bytes <= 2 * old_bytes);
            }
            eprintln!(
                "context width={width} levels={levels} source_bytes={} nodes={nodes} retained={bytes}",
                text.len()
            );
            previous = Some((nodes, bytes));
        }
    }
}

#[test]
fn governed_publication_preserves_duplicate_columns_and_certified_rows() {
    use purrdf_sparql_eval::{GovernedOutcome, PartialAnswers, QueryGovernors};
    let engine = NativeSparqlEngine::new();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let text = "SELECT (?x AS ?x) (?x AS ?x) WHERE { VALUES ?x { 1 2 } OPTIONAL { BIND(1/?x AS ?right) FILTER EXISTS { BIND(1 AS ?probe) } } }";
    let plan = engine
        .prepare_rdflib_query(text, None, &[], QueryOptions::EMPTY)
        .unwrap();
    let full = engine
        .query_rdflib_prepared_governed_view(
            &*data,
            &plan,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let GovernedOutcome::Complete { result, .. } = full else {
        panic!("an unbounded metered query completes")
    };
    let (columns, rows) = result.solutions().unwrap();
    assert_eq!(columns, ["x", "x"]);
    assert_eq!(rows.len(), 2);
    let expected = rows.to_vec();
    let mut trips = 0;
    for fuel in [0u64, 1, 2, 8, 16, 32, 64, 128, 256, 512] {
        let outcome = engine
            .query_rdflib_prepared_governed_view(
                &*data,
                &plan,
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_fuel(fuel),
            )
            .unwrap();
        match outcome {
            GovernedOutcome::Complete { result, .. } => {
                assert_eq!(result.solutions().unwrap().1, expected);
            }
            GovernedOutcome::BudgetExhausted(exhausted) => {
                trips += 1;
                if let PartialAnswers::Certain(partial) = exhausted.partial {
                    let (columns, rows) = partial.result().solutions().unwrap();
                    assert_eq!(columns, ["x", "x"]);
                    for row in rows {
                        assert!(
                            expected.contains(row),
                            "only certified completed driver rows cross"
                        );
                    }
                }
            }
        }
    }
    assert!(trips > 0, "the fuel exhaustion path executed");
}

#[test]
fn ordinary_transformation_admission_rejects_contextual_applications() {
    use purrdf_sparql_algebra::algebra::ApplicationPolicy;
    use purrdf_sparql_algebra::tree::Child;
    use purrdf_sparql_algebra::{Expression, GraphPattern, Query, QueryDataset};
    use purrdf_sparql_eval::PreparedQuery;
    let ask = |pattern| Query::Ask {
        pattern,
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let application = GraphPattern::Apply {
        left: Child::new(GraphPattern::empty_bgp()),
        right: Child::new(GraphPattern::empty_bgp()),
        policy: Box::new(ApplicationPolicy {
            row_pipeline: true,
            reduced_adjacent: false,
            group_domain: None,
            inputs: Vec::new(),
            optional: None,
        }),
    };
    let engine = NativeSparqlEngine::new();
    for pattern in [
        application.clone(),
        GraphPattern::Filter {
            inner: Child::new(GraphPattern::empty_bgp()),
            expr: Expression::Exists(Child::new(application.clone())),
        },
    ] {
        let query = ask(pattern);
        query.validate().unwrap();
        for error in [
            engine
                .prepare_algebra(query.clone(), QueryOptions::EMPTY)
                .unwrap_err(),
            PreparedQuery::rewritten(query, QueryOptions::EMPTY).unwrap_err(),
        ] {
            assert!(error.to_string().contains("typed contextual preparation"));
        }
    }
    engine
        .prepare_algebra(ask(GraphPattern::empty_bgp()), QueryOptions::EMPTY)
        .unwrap();
    PreparedQuery::rewritten(ask(GraphPattern::empty_bgp()), QueryOptions::EMPTY).unwrap();
    let GraphPattern::Apply { policy, .. } = &application else {
        unreachable!()
    };
    for change in [0, 1, 2, 3] {
        let mut invalid = application.clone();
        let GraphPattern::Apply {
            policy: invalid_policy,
            ..
        } = &mut invalid
        else {
            unreachable!()
        };
        invalid_policy.clone_from(policy);
        match change {
            0 => invalid_policy.reduced_adjacent = true,
            1 => invalid_policy.group_domain = Some(Vec::new().into_boxed_slice()),
            2 => invalid_policy.inputs = vec![(Variable::new("input"), Variable::new("driver")); 2],
            _ => {
                invalid_policy.optional =
                    Some(purrdf_sparql_algebra::algebra::OptionalApplication {
                        retry_inputs: Vec::new(),
                        forget_marker: Variable::new("marker"),
                    });
            }
        }
        assert!(
            ask(invalid.clone()).validate().is_err(),
            "invalid policy mode {change}"
        );
    }
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let typed = engine
        .prepare_rdflib_query(
            "SELECT ?x WHERE { VALUES ?x { 1 2 } BIND(0 AS ?x) }",
            None,
            &[],
            QueryOptions::EMPTY,
        )
        .unwrap();
    assert_eq!(
        engine
            .query_rdflib_prepared_view(&*data, &typed, QueryOptions::EMPTY)
            .unwrap()
            .solutions()
            .unwrap()
            .1
            .len(),
        2
    );
}

#[test]
fn grouped_continuation_withholds_incomplete_accumulators_at_every_fuel_cutoff() {
    use purrdf_core::ResourceDimension;
    use purrdf_sparql_eval::{
        Arity, ExtensionEnv, GovernedOutcome, PartialAnswers, QueryGovernors, UserFunctionRegistry,
        Volatility,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    for text in [
        "SELECT (SUM(?tick) AS ?input) (SUM(<http://example.org/counter>(1)) AS ?this) WHERE { VALUES ?driver { 1 2 3 } BIND(<http://example.org/counter>(1) AS ?tick) }",
        "SELECT ?group (SUM(?tick) AS ?input) (SUM(<http://example.org/counter>(1)) AS ?this) WHERE { VALUES ?group { 0 1 0 } BIND(<http://example.org/counter>(1) AS ?tick) } GROUP BY ?group",
        "SELECT (COUNT(DISTINCT *) AS ?n) WHERE { VALUES ?x { 1 2 } BIND(0 AS ?x) }",
        // Operational native-VM control: the genuine language does not lower this
        // aggregate EXISTS syntax, so this is a certificate law, not oracle parity.
        "SELECT (SUM(IF(EXISTS { VALUES ?probe { 1 2 } FILTER(?probe = ?x) }, 1, 0)) AS ?n) WHERE { VALUES ?x { 1 2 3 } }",
    ] {
        let engine = NativeSparqlEngine::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let mut registry = UserFunctionRegistry::default();
        registry.register_native(
            "http://example.org/counter",
            Arity::Exact(1),
            Volatility::Volatile,
            Arc::new(move |_| {
                Ok(Some(TermValue::integer(
                    i64::try_from(counted.fetch_add(1, Ordering::Relaxed) + 1).unwrap(),
                )))
            }),
        );
        let functions = engine
            .bind_functions(registry, ExtensionEnv::empty())
            .unwrap();
        let options = QueryOptions::EMPTY.with_functions(&functions);
        let plan = engine
            .prepare_rdflib_query(text, None, &[], options)
            .unwrap();
        let full = engine
            .query_rdflib_prepared_governed_view(&*data, &plan, options, &QueryGovernors::METERED)
            .unwrap();
        let cost = full.evidence().consumed.get(ResourceDimension::Fuel);
        let GovernedOutcome::Complete { result, .. } = full else {
            panic!("full group completes")
        };
        let (columns, expected) = result.solutions().unwrap();
        let expected_calls = calls.load(Ordering::Relaxed);
        let mut computing_trips = 0;
        let mut trips = 0;
        for fuel in 0..=cost {
            calls.store(0, Ordering::Relaxed);
            let outcome = engine
                .query_rdflib_prepared_governed_view(
                    &*data,
                    &plan,
                    options,
                    &QueryGovernors::METERED.with_fuel(fuel),
                )
                .unwrap();
            match outcome {
                GovernedOutcome::Complete { result, .. } => {
                    assert_eq!(result.solutions().unwrap(), (columns, expected));
                    assert_eq!(calls.load(Ordering::Relaxed), expected_calls);
                }
                GovernedOutcome::BudgetExhausted(exhausted) => {
                    trips += 1;
                    let invoked = calls.load(Ordering::Relaxed);
                    computing_trips += usize::from(invoked > 0 && invoked < expected_calls);
                    assert!(
                        invoked <= expected_calls,
                        "no callbacks after a latched trip"
                    );
                    if let PartialAnswers::Certain(partial) = exhausted.partial {
                        let (actual_columns, rows) = partial.result().solutions().unwrap();
                        assert_eq!(actual_columns, columns);
                        for row in rows {
                            assert!(
                                expected.contains(row),
                                "an incomplete input cannot certify a fabricated aggregate: {text}, fuel={fuel}, row={row:?}"
                            );
                        }
                    }
                }
            }
        }
        assert!(trips > 0);
        if expected_calls > 0 {
            assert!(
                computing_trips > 0,
                "cutoffs reach input and aggregate computation"
            );
        }
        eprintln!(
            "group certificate cutoffs={} computing_trips={computing_trips} query={text}",
            cost + 1
        );
    }
}

#[test]
fn contextual_blank_identity_is_shared_by_label_and_fresh_without_a_label() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    for initial in [
        Vec::new(),
        vec![(
            Variable::new("this"),
            GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/a")),
        )],
    ] {
        for (argument, identity_count) in [("\"label\"", 1), ("", 6)] {
            let text = format!(
                "SELECT ?x ?a ?b WHERE {{ VALUES ?x {{1 2 3}} BIND(BNODE({argument}) AS ?a) BIND(BNODE({argument}) AS ?b) }}"
            );
            let plan = engine
                .prepare_rdflib_query(&text, None, &initial, QueryOptions::EMPTY)
                .unwrap();
            let result = engine
                .query_rdflib_prepared_view(&*data, &plan, QueryOptions::EMPTY)
                .unwrap();
            let (columns, rows) = result.solutions().unwrap();
            assert_eq!(columns, &["x", "a", "b"]);
            assert_eq!(rows.len(), 3);
            let mut identities = std::collections::BTreeSet::new();
            for (index, row) in rows.iter().enumerate() {
                assert_eq!(
                    row[0],
                    Some(TermValue::integer(i64::try_from(index + 1).unwrap()))
                );
                for term in &row[1..] {
                    let Some(TermValue::Blank { .. }) = term else {
                        panic!("minted blank")
                    };
                    identities.insert(term.clone());
                }
            }
            assert_eq!(identities.len(), identity_count);
        }
        let plan = engine.prepare_rdflib_query("SELECT (COUNT(DISTINCT ?a) AS ?n) WHERE { VALUES ?x {1 2 3} BIND(BNODE(\"label\") AS ?a) }", None, &initial, QueryOptions::EMPTY).unwrap();
        let result = engine
            .query_rdflib_prepared_view(&*data, &plan, QueryOptions::EMPTY)
            .unwrap();
        assert_eq!(
            result.solutions().unwrap().1,
            &[vec![Some(TermValue::integer(1))]]
        );
    }
    // The ordinary language preserves its separate per-solution identity law.
    let plan = engine
        .prepare_query(
            "SELECT ?a WHERE { VALUES ?x {1 2 3} BIND(BNODE(\"label\") AS ?a) }",
            None,
        )
        .unwrap();
    let result = engine
        .query_prepared_view(&*data, &plan, &[], QueryOptions::EMPTY)
        .unwrap();
    let (_, rows) = result.solutions().unwrap();
    let identities: std::collections::BTreeSet<_> = rows.iter().map(|row| row[0].clone()).collect();
    assert_eq!(identities.len(), 3);
}

#[test]
fn governed_ask_settles_only_a_witness_and_stops_after_accepted_filters() {
    use purrdf_core::ResourceDimension;
    use purrdf_sparql_eval::{
        Arity, ExtensionEnv, GovernedOutcome, PartialAnswers, QueryGovernors, UserFunctionRegistry,
        Volatility,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let fixtures = json::read(include_str!("fixtures/contextual-mappings.json")).unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let mut executed = 0;
    for fixture in fixtures.as_array().unwrap() {
        for case in fixture["cases"].as_array().unwrap() {
            if case["oracle"]["type"].as_str() != Some("ASK")
                || case["count_callback"] != Value::Bool(true)
            {
                continue;
            }
            assert_eq!(fixture["triples"].as_array().unwrap().len(), 0);
            assert!(fixture["quads"].as_array().is_none());
            let engine = NativeSparqlEngine::new();
            let calls = Arc::new(AtomicUsize::new(0));
            let counted = Arc::clone(&calls);
            let mut registry = UserFunctionRegistry::default();
            registry.register_native(
                "http://example.org/counter",
                Arity::Exact(1),
                Volatility::Volatile,
                Arc::new(move |_| {
                    Ok(Some(TermValue::integer(
                        i64::try_from(counted.fetch_add(1, Ordering::Relaxed) + 1).unwrap(),
                    )))
                }),
            );
            let functions = engine
                .bind_functions(registry, ExtensionEnv::empty())
                .unwrap();
            let options = QueryOptions::EMPTY.with_functions(&functions);
            let initial: Vec<_> = case["initial"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, term)| {
                    (
                        Variable::new(name),
                        GroundTerm::NamedNode(NamedNode::new_unchecked(term.as_str().unwrap())),
                    )
                })
                .collect();
            let plan = engine
                .prepare_rdflib_query(case["query"].as_str().unwrap(), None, &initial, options)
                .unwrap();
            let ((Some(expected), None) | (None, Some(expected))) = (
                case["oracle"]["answer"].as_bool(),
                case["oracle"]["boolean"].as_bool(),
            ) else {
                panic!("one explicit ASK answer");
            };
            let expected_calls = case["oracle"]["callback_invocations"]
                .to_string()
                .parse::<usize>()
                .unwrap();
            let full = engine
                .query_rdflib_prepared_governed_view(
                    &*data,
                    &plan,
                    options,
                    &QueryGovernors::METERED,
                )
                .unwrap();
            let cost = full.evidence().consumed.get(ResourceDimension::Fuel);
            let GovernedOutcome::Complete { result, .. } = full else {
                panic!("complete ASK")
            };
            assert!(matches!(result, SparqlResult::Boolean(value) if value == expected));
            assert_eq!(calls.load(Ordering::Relaxed), expected_calls);
            let mut trips = 0;
            for fuel in 0..=cost {
                calls.store(0, Ordering::Relaxed);
                let outcome = engine
                    .query_rdflib_prepared_governed_view(
                        &*data,
                        &plan,
                        options,
                        &QueryGovernors::METERED.with_fuel(fuel),
                    )
                    .unwrap();
                match outcome {
                    GovernedOutcome::Complete { result, .. } => {
                        assert!(
                            matches!(result, SparqlResult::Boolean(value) if value == expected)
                        );
                        assert_eq!(calls.load(Ordering::Relaxed), expected_calls);
                    }
                    GovernedOutcome::BudgetExhausted(exhausted) => {
                        trips += 1;
                        assert!(calls.load(Ordering::Relaxed) <= expected_calls);
                        if let PartialAnswers::Certain(partial) = exhausted.partial {
                            assert!(
                                expected && matches!(partial.result(), SparqlResult::Boolean(true)),
                                "exhaustion can settle only an actual true witness"
                            );
                            assert_eq!(
                                calls.load(Ordering::Relaxed),
                                expected_calls,
                                "the accepted witness, including its filter, actually ran"
                            );
                        }
                    }
                }
            }
            assert!(trips > 0);
            executed += 1;
            eprintln!("ASK cutoffs={} case={}", cost + 1, case["name"]);
        }
    }
    assert_eq!(
        executed, 14,
        "all accepted, rejected and empty ASK witnesses ran"
    );
}
