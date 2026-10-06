// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The observable roles of hidden match witnesses share one typed admission rule.

#[path = "support/patterns.rs"]
mod patterns;

use purrdf_sparql_algebra::scope::{validate_pattern, validate_quad, validate_query};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, BlankNode, Child, Expression, GraphPattern,
    GraphUpdateOperation, GroundTerm, NamedNodePattern, ObserverRole, OrderExpression, ParseError,
    PropertyFunctionCall, QuadPattern, Query, QueryDataset, ScopeError, ScopeHazard, ScopeRegion,
    ScopeSite, SparqlParser, TermPattern, TriplePattern, Variable, try_pattern_to_select_query,
};

const WITNESS: &str = "joint";

fn selected(pattern: GraphPattern) -> Query {
    Query::Select {
        pattern,
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    }
}

fn count() -> AggregateExpression {
    AggregateExpression::new(AggregateFunction::Count, vec![], vec![], vec![], false)
        .expect("COUNT(*) has no arguments")
}

fn observer_patterns() -> [(ObserverRole, GraphPattern); 8] {
    let hidden = Variable::hidden(WITNESS);
    [
        (
            ObserverRole::Projection,
            GraphPattern::Project {
                inner: Child::new(GraphPattern::empty_bgp()),
                variables: vec![Variable::new("visible"), hidden.clone()],
            },
        ),
        (
            ObserverRole::GroupKey,
            GraphPattern::Group {
                inner: Child::new(GraphPattern::empty_bgp()),
                variables: vec![hidden.clone()],
                aggregates: vec![],
            },
        ),
        (
            ObserverRole::AggregateOutput,
            GraphPattern::Group {
                inner: Child::new(GraphPattern::empty_bgp()),
                variables: vec![],
                aggregates: vec![(hidden.clone(), count())],
            },
        ),
        (
            ObserverRole::BindTarget,
            GraphPattern::Extend {
                inner: Child::new(GraphPattern::empty_bgp()),
                variable: hidden.clone(),
                expression: Expression::Variable(Variable::new("visible")),
            },
        ),
        (
            ObserverRole::UnfoldElement,
            GraphPattern::Unfold {
                inner: Child::new(GraphPattern::empty_bgp()),
                expression: Expression::Variable(Variable::new("list")),
                element: hidden.clone(),
                companion: None,
            },
        ),
        (
            ObserverRole::UnfoldCompanion,
            GraphPattern::Unfold {
                inner: Child::new(GraphPattern::empty_bgp()),
                expression: Expression::Variable(Variable::new("list")),
                element: Variable::new("element"),
                companion: Some(hidden.clone()),
            },
        ),
        (
            ObserverRole::ExpressionRead,
            GraphPattern::Filter {
                expr: Expression::Variable(hidden.clone()),
                inner: Child::new(GraphPattern::empty_bgp()),
            },
        ),
        (
            ObserverRole::BoundTest,
            GraphPattern::Filter {
                expr: Expression::Bound(hidden),
                inner: Child::new(GraphPattern::empty_bgp()),
            },
        ),
    ]
}

#[test]
fn each_pattern_observer_has_a_typed_role_and_the_same_legacy_refusal() {
    for (role, pattern) in observer_patterns() {
        let error = validate_pattern(&pattern).expect_err("hidden observation");
        assert_eq!(error.hazard, ScopeHazard::HiddenObservation);
        assert_eq!(error.role, role);
        assert_eq!(error.identity, Variable::hidden(WITNESS));
        assert_eq!(error.site.region, ScopeRegion::Pattern);
        assert_eq!(error.site.item, 0);
        assert_eq!(
            error.site.slot,
            usize::from(role == ObserverRole::Projection)
        );
        assert_ne!(error.action(), "");
        assert!(!error.to_string().contains('\0'));
        let legacy = ParseError::from(error.clone());
        assert_eq!(pattern.validate_hidden_variables(), Err(legacy.clone()));
        assert_eq!(try_pattern_to_select_query(&pattern), Err(legacy.clone()));
        let query = selected(pattern);
        assert_eq!(validate_query(&query), Err(error));
        assert_eq!(query.validate_hidden_variables(), Err(legacy.clone()));
        assert_eq!(query.validate(), Err(legacy.clone()));
        assert_eq!(query.custom_function_calls(), Err(legacy));
    }
}

#[test]
fn nested_expression_observers_are_reached_through_every_owner() {
    let aggregate = AggregateExpression::new(
        AggregateFunction::Count,
        vec![Expression::Variable(Variable::hidden(WITNESS))],
        vec![],
        vec![],
        false,
    )
    .expect("COUNT has one expression argument");
    let read = || Expression::Variable(Variable::hidden(WITNESS));
    let nested = || GraphPattern::Filter {
        expr: read(),
        inner: Child::new(GraphPattern::empty_bgp()),
    };
    for pattern in [
        GraphPattern::LeftJoin {
            left: Child::new(GraphPattern::empty_bgp()),
            right: Child::new(GraphPattern::empty_bgp()),
            expression: Some(read()),
        },
        GraphPattern::Filter {
            expr: Expression::Exists(Child::new(nested())),
            inner: Child::new(GraphPattern::empty_bgp()),
        },
        GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(patterns::example("graph")),
            inner: Child::new(nested()),
        },
        GraphPattern::Project {
            inner: Child::new(nested()),
            variables: vec![Variable::new("visible")],
        },
        GraphPattern::OrderBy {
            inner: Child::new(GraphPattern::empty_bgp()),
            expression: vec![OrderExpression::Desc(read())],
        },
        GraphPattern::Group {
            inner: Child::new(GraphPattern::empty_bgp()),
            variables: vec![],
            aggregates: vec![(Variable::new("count"), aggregate)],
        },
        GraphPattern::Extend {
            inner: Child::new(GraphPattern::empty_bgp()),
            variable: Variable::new("visible"),
            expression: Expression::Not(Child::new(read())),
        },
    ] {
        let error = validate_pattern(&pattern).expect_err("nested expression input");
        assert_eq!(error.role, ObserverRole::ExpressionRead);
        assert!(error.site.node > 0);
        assert_eq!(selected(pattern).validate(), Err(ParseError::from(error)));
    }
}

#[test]
fn template_observers_include_nested_quoted_slots_and_graph_names() {
    for (role, slot) in [
        (ObserverRole::TemplateSubject, 0),
        (ObserverRole::TemplatePredicate, 1),
        (ObserverRole::TemplateObject, 2),
        (ObserverRole::TemplateGraph, 0),
    ] {
        for quoted in [false, true] {
            let mut triple = TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(patterns::example("p")),
                object: TermPattern::Variable(Variable::new("o")),
            };
            match role {
                ObserverRole::TemplateSubject => {
                    triple.subject = TermPattern::Variable(Variable::hidden(WITNESS));
                }
                ObserverRole::TemplatePredicate => {
                    triple.predicate = NamedNodePattern::Variable(Variable::hidden(WITNESS));
                }
                ObserverRole::TemplateObject => {
                    triple.object = TermPattern::Variable(Variable::hidden(WITNESS));
                }
                ObserverRole::TemplateGraph => {}
                _ => unreachable!("these are the four template positions"),
            }
            if quoted {
                triple = TriplePattern {
                    subject: TermPattern::NamedNode(patterns::example("statement")),
                    predicate: NamedNodePattern::NamedNode(patterns::example("quoted")),
                    object: TermPattern::Triple(Child::new(triple)),
                };
            }
            let quad = QuadPattern {
                triple,
                graph: (role == ObserverRole::TemplateGraph)
                    .then(|| NamedNodePattern::Variable(Variable::hidden(WITNESS))),
            };
            let error = validate_quad(&quad).expect_err("template observes hidden binding");
            assert_eq!(error.role, role);
            assert_eq!(error.site.region, ScopeRegion::Template);
            assert_eq!(error.site.slot, slot);
            assert!(
                !quoted || error.site.node > 0,
                "a nested slot has a node after the outer triple"
            );
            let operation = GraphUpdateOperation::InsertData {
                data: vec![quad.clone()],
            };
            assert_eq!(operation.try_to_sparql(), Err(ParseError::from(error)));
            let ordinary = QuadPattern {
                triple: TriplePattern {
                    subject: TermPattern::BlankNode(BlankNode::new("allocation")),
                    predicate: NamedNodePattern::NamedNode(patterns::example("p")),
                    object: TermPattern::Variable(Variable::new("visible")),
                },
                graph: None,
            };
            let query = Query::Construct {
                template: vec![ordinary, quad],
                pattern: GraphPattern::empty_bgp(),
                dataset: QueryDataset::default(),
                base_iri: None,
                version: None,
            };
            let error = validate_query(&query).expect_err("second template quad");
            assert_eq!(error.role, role);
            assert_eq!(error.site.item, 1);
            assert_eq!(query.validate(), Err(ParseError::from(error)));
        }
    }
}

#[test]
fn describe_heads_report_the_target_position() {
    let query = Query::Describe {
        targets: vec![
            NamedNodePattern::NamedNode(patterns::example("visible")),
            NamedNodePattern::Variable(Variable::hidden(WITNESS)),
        ],
        pattern: GraphPattern::empty_bgp(),
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let error = validate_query(&query).expect_err("explicit hidden description");
    assert_eq!(error.role, ObserverRole::DescribeTarget);
    assert_eq!(
        error.site,
        ScopeSite {
            region: ScopeRegion::Description,
            item: 1,
            node: 0,
            slot: 0,
        }
    );
    assert_eq!(
        query.validate_hidden_variables(),
        Err(ParseError::from(error.clone()))
    );
    assert_eq!(query.validate(), Err(ParseError::from(error)));
}

#[test]
fn match_values_quoted_terms_and_generated_unions_keep_hidden_connections() {
    let hidden = Variable::hidden(WITNESS);
    let triple = TriplePattern {
        subject: TermPattern::Variable(hidden.clone()),
        predicate: NamedNodePattern::Variable(hidden.clone()),
        object: TermPattern::Triple(Child::new(TriplePattern {
            subject: TermPattern::Variable(hidden.clone()),
            predicate: NamedNodePattern::Variable(hidden.clone()),
            object: TermPattern::Variable(hidden.clone()),
        })),
    };
    let bgp = GraphPattern::Bgp {
        patterns: vec![triple],
    };
    let pattern = GraphPattern::Join {
        left: Child::new(GraphPattern::union(bgp.clone(), bgp)),
        right: Child::new(GraphPattern::Values {
            variables: vec![hidden.clone()],
            bindings: vec![vec![Some(GroundTerm::BlankNode(BlankNode::new("dataset")))]],
        }),
    };
    validate_pattern(&pattern).expect("hidden variables connect match positions");
    selected(pattern)
        .validate()
        .expect("raw match VALUES is admitted");
    let call = GraphPattern::PropertyFunction(PropertyFunctionCall {
        iri: "http://example.org/relation".to_owned(),
        subject_args: vec![TermPattern::Variable(hidden.clone())],
        object_args: vec![TermPattern::Variable(hidden)],
    });
    selected(call)
        .validate()
        .expect("relation match positions remain legal");
    let query = SparqlParser::new()
        .parse_query("SELECT * WHERE { ?s (<http://example.org/p>|<http://example.org/q>)/<http://example.org/r> ?o }")
        .expect("the generated UNION preserves its shared intermediate identity");
    validate_query(&query).expect("generated shared witnesses are legal");
    query.validate().expect("ordinary full admission agrees");
}

#[test]
fn witness_names_do_not_capture_callers_or_other_internal_categories() {
    for name in [
        "__purrdf_hidden_0",
        "___purrdf_hidden_0",
        "bgpblank:joint",
        "\0bnode:joint",
        "\0prebound:joint",
    ] {
        let variable = Variable::new(name);
        assert!(!variable.is_hidden());
        let pattern = GraphPattern::Project {
            inner: Child::new(GraphPattern::Filter {
                expr: Expression::Variable(variable.clone()),
                inner: Child::new(GraphPattern::empty_bgp()),
            }),
            variables: vec![variable],
        };
        validate_pattern(&pattern).expect("only the hidden match category is checked");
        if !name.contains(['\0', ':']) {
            selected(pattern)
                .validate()
                .expect("a legal caller lookalike stays distinguished");
        } else {
            let error = selected(pattern)
                .validate()
                .expect_err("ordinary name admission is separate");
            assert!(!error.to_string().contains("non-distinguished"));
        }
    }
    let independent = || GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::BlankNode(BlankNode::new("local")),
            predicate: NamedNodePattern::NamedNode(patterns::example("p")),
            object: TermPattern::Variable(Variable::new("visible")),
        }],
    };
    selected(GraphPattern::union(independent(), independent()))
        .validate()
        .expect("separate compiler-built arm blanks remain local");
    assert!(
        SparqlParser::new()
            .parse_query("SELECT * WHERE { { _:local <http://example.org/p> ?x } UNION { _:local <http://example.org/q> ?x } }")
            .is_err(),
        "source syntax independently forbids the same label in separate BGPs"
    );
}

#[test]
fn diagnostics_use_source_order_and_stable_node_addresses() {
    let pattern = GraphPattern::Join {
        left: Child::new(GraphPattern::empty_bgp()),
        right: Child::new(GraphPattern::Filter {
            expr: Expression::Bound(Variable::hidden(WITNESS)),
            inner: Child::new(GraphPattern::empty_bgp()),
        }),
    };
    let expected = validate_pattern(&pattern).expect_err("the right filter observes a witness");
    assert_eq!(expected.role, ObserverRole::BoundTest);
    assert_eq!(expected.site.node, 3);
    for _ in 0..16 {
        assert_eq!(validate_pattern(&pattern.clone()), Err(expected.clone()));
        assert_eq!(
            selected(pattern.clone()).validate(),
            Err(ParseError::from(expected.clone()))
        );
    }
    let (_, left) = observer_patterns()
        .into_iter()
        .next()
        .expect("projection role");
    let either = GraphPattern::union(left, pattern);
    let first = validate_pattern(&either).expect_err("both arms have observers");
    assert_eq!(first.role, ObserverRole::Projection);
    assert_eq!(first.site.node, 1);

    let mut quad = QuadPattern {
        triple: TriplePattern {
            subject: TermPattern::Variable(Variable::hidden("subject")),
            predicate: NamedNodePattern::Variable(Variable::hidden("predicate")),
            object: TermPattern::Variable(Variable::hidden("object")),
        },
        graph: Some(NamedNodePattern::Variable(Variable::hidden("graph"))),
    };
    for (role, slot, identity) in [
        (ObserverRole::TemplateSubject, 0, "subject"),
        (ObserverRole::TemplatePredicate, 1, "predicate"),
        (ObserverRole::TemplateObject, 2, "object"),
        (ObserverRole::TemplateGraph, 0, "graph"),
    ] {
        let error = validate_quad(&quad).expect_err("the first invalid field is deterministic");
        assert_eq!(error.role, role);
        assert_eq!(error.site.slot, slot);
        assert_eq!(error.identity, Variable::hidden(identity));
        match role {
            ObserverRole::TemplateSubject => {
                quad.triple.subject = TermPattern::Variable(Variable::new("subject"));
            }
            ObserverRole::TemplatePredicate => {
                quad.triple.predicate = NamedNodePattern::NamedNode(patterns::example("predicate"));
            }
            ObserverRole::TemplateObject => {
                quad.triple.object = TermPattern::Variable(Variable::new("object"));
            }
            ObserverRole::TemplateGraph => quad.graph = None,
            _ => unreachable!("only template roles are checked here"),
        }
    }
    validate_quad(&quad).expect("all explicit outputs are distinguished");
}

#[test]
fn a_hundred_thousand_nested_scopes_are_checked_on_a_small_stack() {
    purrdf_stack::on_stack(128 * 1024, || {
        const DEPTH: usize = 100_000;
        let mut pattern = GraphPattern::Filter {
            expr: Expression::Bound(Variable::hidden(WITNESS)),
            inner: Child::new(GraphPattern::empty_bgp()),
        };
        let graph = NamedNodePattern::NamedNode(patterns::example("graph"));
        for _ in 0..DEPTH {
            pattern = GraphPattern::Graph {
                name: graph.clone(),
                inner: Child::new(pattern),
            };
        }
        let error: ScopeError = validate_pattern(&pattern).expect_err("deep hidden observer");
        assert_eq!(error.site.node, DEPTH + 1);
        assert_eq!(error.role, ObserverRole::BoundTest);
        assert_eq!(selected(pattern).validate(), Err(ParseError::from(error)));
    })
    .expect("the small-stack computation runs");
}
