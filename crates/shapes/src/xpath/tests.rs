// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native ownership, ambient restoration and actual focus-worker failure order.

use purrdf_core::xsd_regex::xpath::Resource;

use crate::expression::{CustomFnKind, CustomFunction, NodeExpr};
use crate::shapes::Constraint;
use crate::term::{Literal, NamedNode};

use super::*;

const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape; sh:targetNode ex:a, ex:b; sh:pattern "a" .
"#;

#[test]
fn ordinary_query_failures_do_not_erase_an_execution_cause() {
    let prepared =
        PreparedShapes::new(Arc::new(crate::engine::parse_shapes(SHAPES, None).unwrap()));
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let _scope = enter(prepared.xpath_configuration(profile, Limits::new()));
        let ordinary =
            || RdfDiagnostic::error("native-sparql-custom-function", "missing authored function");
        assert!(query_error(ordinary()).contains("missing authored function"));
        assert!(root_result(Ok::<_, String>(())).is_ok());

        let fatal = RdfDiagnostic::error(
            purrdf_sparql_eval::EvalError::FUNCTION_OPERATIONAL_CODE,
            "opaque host execution failure",
        );
        let fatal_message = query_error(fatal);
        let _ = query_error(ordinary());
        let error = root_result(Ok::<_, String>(())).unwrap_err();
        assert!(error.message.contains("opaque host execution failure"));
        assert!(fatal_message.contains("opaque host execution failure"));
        let Some(cause) = error.cause else {
            panic!("the actual first execution cause must remain attached");
        };
        assert!(matches!(*cause, Cause::Query(ref diagnostic)
                if diagnostic.code == purrdf_sparql_eval::EvalError::FUNCTION_OPERATIONAL_CODE
                    && diagnostic.message == "opaque host execution failure"));
        assert!(take_cause().is_none());
    }
}

#[test]
fn query_refusal_retains_the_callers_execution_capsule_until_resolution() {
    let data = crate::text_ingest::parse_turtle_to_dataset(
        "<http://example.org/a> <http://example.org/p> \"a\" .",
        None,
    )
    .unwrap();
    let preparation = PreparedShapes::new(Arc::new(
        crate::engine::parse_shapes(
            r#"
                @prefix sh: <http://www.w3.org/ns/shacl#> .
                @prefix ex: <http://example.org/> .
                ex:S a sh:NodeShape; sh:class ex:Missing;
                    sh:target [ a sh:SPARQLTarget; sh:select '''
                        SELECT ?this WHERE {
                            ?this <http://example.org/p> ?value .
                            FILTER(REGEX(?value, "a"))
                        }
                    ''' ] .
            "#,
            None,
        )
        .unwrap(),
    ));
    let configuration = preparation.xpath_configuration(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    let capsule = Arc::new(());
    let result = configuration.run(|| {
        preparation
            .bind_shared_dataset(Arc::clone(&data))
            .map_err(|error| (error, Arc::clone(&capsule)))
    });
    assert!(result.is_err(), "the actual target query must be refused");
    assert_eq!(
        Arc::strong_count(&capsule),
        2,
        "the caller's actual execution capsule must survive its paired query diagnostic"
    );
    let error = result
        .unwrap_err()
        .into_public(|(error, _capsule)| XPathValidationError::Shapes(error));
    assert!(
        matches!(error, XPathValidationError::Query(diagnostic) if diagnostic.code == Resource::MatchSteps.code())
    );
    assert_eq!(Arc::strong_count(&capsule), 1);
    assert!(current().is_none());
    assert_eq!(
        preparation
            .with_xpath_regex(Profile::Xpath31, Limits::new())
            .bind_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .results
            .len(),
        1
    );
}

#[test]
fn temporary_pattern_cells_do_not_extend_the_declaration_cache() {
    let prepared =
        PreparedShapes::new(Arc::new(crate::engine::parse_shapes(SHAPES, None).unwrap()));
    let configuration = prepared.xpath_configuration(Profile::Xpath31, Limits::new());
    let declared = declarations::collect(&configuration.shapes).unwrap();
    assert_eq!(declared.len(), 1);
    let owned = configuration.compiled(declared[0].cell, "a", "").unwrap();
    let _scope = enter(configuration.clone());
    for _ in 0..1_000 {
        let temporary = LegacyCell::new();
        assert!(
            facet(&temporary, "b", None)
                .unwrap()
                .matches(Some("b"))
                .unwrap()
        );
    }
    let warm = Configuration {
        limits: Limits::new().with(Resource::CompileSteps, 0),
        ..configuration.clone()
    };
    assert!(Arc::ptr_eq(
        &owned,
        &warm.compiled(declared[0].cell, "a", "").unwrap()
    ));
    let slots = configuration.caches.lock().unwrap();
    assert_eq!(slots.declared.as_ref().unwrap().len(), 1);
}

#[test]
fn source_admission_sees_a_late_function_body_without_extending_its_cache_snapshot() {
    let mut shapes = crate::engine::parse_shapes(SHAPES, None).unwrap();
    let mut body_shape = shapes.node_shapes[0].clone();
    body_shape.constraints = vec![Constraint::Pattern {
        regex: "(?:b)".to_owned(),
        flags: None,
        compiled: Arc::new(LegacyCell::new()),
    }];
    let function = Arc::new(CustomFunction {
        iri: NamedNode::new_unchecked("http://example.org/function"),
        kind: CustomFnKind::ListParameter,
        params: vec![],
        required: 0,
        body: OnceLock::new(),
    });
    shapes.node_shapes[0]
        .constraints
        .push(Constraint::Expression {
            expr: NodeExpr::CustomCall {
                func: Arc::clone(&function),
                args: vec![],
            },
            messages: vec![],
            severity: None,
        });
    let configuration =
        PreparedShapes::new(Arc::new(shapes)).xpath_configuration(Profile::Xpath31, Limits::new());
    configuration.admit_declared_patterns().unwrap();
    function
        .body
        .set(NodeExpr::Filter {
            nodes: Box::new(NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("b"),
            ))),
            shape: Box::new(body_shape),
        })
        .unwrap();
    configuration.admit_declared_patterns().unwrap();
    let warm = Configuration {
        limits: Limits::new().with(Resource::CompileSteps, 0),
        ..configuration.clone()
    };
    assert!(matches!(
        warm.admit_declared_patterns(),
        Err(XPathValidationError::Pattern(xpath::Error::Resource(refusal)))
            if refusal.resource == Resource::CompileSteps
    ));
    assert_eq!(
        configuration
            .caches
            .lock()
            .unwrap()
            .declared
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    configuration.admit_declared_patterns().unwrap();
}

#[test]
fn ambient_context_suspends_and_restores_the_exact_selected_runtime() {
    let prepared =
        PreparedShapes::new(Arc::new(crate::engine::parse_shapes(SHAPES, None).unwrap()));
    let configuration = prepared.xpath_configuration(Profile::Xpath31, Limits::new());
    let original = crate::sparql::replace_ambient_context(crate::sparql::AmbientContext::default());
    {
        let _scope = enter(configuration.clone());
        let suspended =
            crate::sparql::replace_ambient_context(crate::sparql::AmbientContext::default());
        assert!(!suspended.is_idle());
        assert!(current().is_none());
        let _ = crate::sparql::replace_ambient_context(suspended);
        let restored = current().unwrap();
        assert_eq!(restored.profile, configuration.profile);
        assert_eq!(restored.limits, configuration.limits);
        assert!(Arc::ptr_eq(&restored.shapes, &configuration.shapes));
        assert!(Arc::ptr_eq(&restored.caches, &configuration.caches));
        let cause = xpath::compile(Profile::Xpath31, "a", "", Limits::new())
            .unwrap()
            .is_match("a", Limits::new().with(Resource::MatchSteps, 0))
            .unwrap_err();
        capture(Cause::Pattern(cause));
        let saved =
            crate::sparql::replace_ambient_context(crate::sparql::AmbientContext::default());
        let _ = crate::sparql::replace_ambient_context(saved);
        assert!(
            matches!(take_cause(), Some(Cause::Pattern(xpath::Error::Resource(refusal))) if refusal.resource == Resource::MatchSteps)
        );
    }
    assert!(current().is_none());
    let _ = crate::sparql::replace_ambient_context(original);
}

#[test]
fn canonical_focus_error_keeps_its_own_cause_across_actual_worker_geometries() {
    let data = crate::text_ingest::parse_turtle_to_dataset(
        "<http://example.org/b> <http://example.org/p> 1 .\n<http://example.org/a> <http://example.org/p> 2 .",
        None,
    ).unwrap();
    for typed_first in [false, true] {
        let mut shapes = crate::engine::parse_shapes(SHAPES, None).unwrap();
        let pattern_shape = Box::new(shapes.node_shapes[0].clone());
        let typed = NodeExpr::Filter {
            nodes: Box::new(NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("a"),
            ))),
            shape: pattern_shape,
        };
        let ordinary = NodeExpr::InstancesOf(Box::new(NodeExpr::Constant(Term::Literal(
            Literal::new_simple_literal("not an IRI"),
        ))));
        let (then, els) = if typed_first {
            (typed, ordinary)
        } else {
            (ordinary, typed)
        };
        shapes.node_shapes[0].constraints = vec![Constraint::Expression {
            expr: NodeExpr::If {
                cond: Box::new(NodeExpr::Select {
                    query: "SELECT ($this = <http://example.org/a> AS ?result) WHERE {}".to_owned(),
                    variable: "result".to_owned(),
                    key: "sh:select",
                }),
                then: Box::new(then),
                els: Box::new(els),
            },
            messages: vec![],
            severity: None,
        }];
        let selected = PreparedShapes::new(Arc::new(shapes)).with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        );
        let bound = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
        let baseline = {
            let _guard = crate::parallel::force_scheduler_for_test(false, 1);
            bound.validate().unwrap_err()
        };
        match (&baseline, typed_first) {
            (XPathValidationError::Pattern(xpath::Error::Resource(refusal)), true) => {
                assert_eq!(refusal.resource, Resource::MatchSteps);
            }
            (XPathValidationError::Execution(_), false) => {}
            _ => panic!("earliest canonical root returned the wrong cause: {baseline}"),
        }
        for (workers, chunk) in [(2, 1), (4, 1), (4, 2), (4, 7)] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .unwrap();
            let actual = pool.install(|| {
                let _guard = crate::parallel::force_scheduler_for_test(true, chunk);
                bound.validate().unwrap_err()
            });
            assert_eq!(
                actual.to_string(),
                baseline.to_string(),
                "{workers} workers, chunk {chunk}"
            );
            assert_eq!(
                std::mem::discriminant(&actual),
                std::mem::discriminant(&baseline)
            );
        }
    }
}
