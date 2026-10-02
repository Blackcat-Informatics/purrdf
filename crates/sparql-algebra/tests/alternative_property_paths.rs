// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Predicate-only paths become compact bag unions and connected joins.

#[path = "support/patterns.rs"]
mod patterns;

use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
use purrdf_sparql_algebra::{
    GraphPattern, ParserOptions, Query, SparqlParser, Variable, pattern_to_select_query,
    pattern_to_select_query_with_options,
};

const PREFIX: &str = "PREFIX ex: <http://example.org/> ";

fn parsed(body: &str) -> GraphPattern {
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query(&format!("{PREFIX}SELECT * WHERE {{ {body} }}"))
        .expect("a predicate path parses")
    else {
        panic!("SELECT");
    };
    pattern
}

#[test]
fn branching_compositions_stay_compact_and_keep_repeated_arms() {
    let pattern = parsed("?s (ex:p|ex:p)/(ex:q|ex:r) ?o");
    let GraphPattern::Project { inner, variables } = pattern else {
        panic!("projection");
    };
    assert_eq!(variables, [Variable::new("s"), Variable::new("o")]);
    let GraphPattern::Join { left, right } = &*inner else {
        panic!("composition");
    };
    let GraphPattern::Union { arms: left } = &**left else {
        panic!("left alternative");
    };
    let GraphPattern::Union { arms: right } = &**right else {
        panic!("right alternative");
    };
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 2);
    assert_eq!(left[0], left[1]);
    let mut hidden = std::collections::BTreeSet::new();
    walk_pre_post(NodeRef::Pattern(&inner), |visit, node| {
        if visit == Visit::Enter {
            assert!(!matches!(node, NodeRef::Pattern(GraphPattern::Path { .. })));
            node.for_each_variable(|variable| {
                if variable.is_hidden() {
                    hidden.insert(variable.clone());
                }
            });
        }
        Flow::Descend
    });
    assert_eq!(hidden.len(), 1);
}

#[test]
fn every_object_has_a_hygienic_witness_and_only_real_variables_are_projected() {
    let pattern = parsed(
        "?s (ex:p|ex:q)/ex:r ?o, ?z . \
        ?__purrdf_hidden_0 ex:t _:__purrdf_anon_0",
    );
    let GraphPattern::Project { variables, inner } = pattern else {
        panic!("projection");
    };
    assert_eq!(
        variables,
        ["__purrdf_hidden_0", "s", "o", "z"].map(Variable::new)
    );
    let mut hidden = std::collections::BTreeSet::new();
    walk_pre_post(NodeRef::Pattern(&inner), |visit, node| {
        if visit == Visit::Enter {
            node.for_each_variable(|variable| {
                if variable.is_hidden() {
                    hidden.insert(variable.clone());
                }
            });
        }
        Flow::Descend
    });
    assert_eq!(hidden.len(), 2);
    let rendered = pattern_to_select_query(&inner);
    assert!(!rendered.contains('\0'));
    assert!(rendered.starts_with("SELECT ?__purrdf_hidden_0 ?s ?o ?z WHERE"));
    assert!(rendered.contains("?___purrdf_hidden_0"));
    SparqlParser::new()
        .parse_query(&rendered)
        .expect("collision-safe carrier");
}

#[test]
fn serializers_reparse_inverse_literal_and_quoted_endpoints_under_registry_collisions() {
    let options = ParserOptions {
        property_fn_namespaces: vec!["http://example.org/".to_owned()],
        ..ParserOptions::default()
    };
    for body in [
        "?s (ex:p|ex:q) \"value\"",
        "?s ^(ex:p|ex:q) \"value\"",
        "?s (ex:p|ex:q) <<( ex:s ex:t \"value\" )>>",
        "?s ^(ex:p|ex:q) <<( ex:s ex:t \"value\" )>>",
        "GRAPH ?g { ?s ^(^(ex:p/(ex:q|ex:r))) ?o }",
    ] {
        let pattern = patterns::where_body(&parsed(body));
        for text in [
            pattern_to_select_query(&pattern),
            pattern_to_select_query_with_options(&pattern, &options),
        ] {
            assert!(!text.contains('\0'), "{text}");
            let query = SparqlParser::new()
                .parse_query_with(&text, &options)
                .unwrap_or_else(|error| panic!("{text}: {error}"));
            let Query::Select { pattern, .. } = query else {
                panic!("SELECT");
            };
            let canonical = pattern_to_select_query(&pattern);
            let Query::Select { pattern, .. } = SparqlParser::new()
                .parse_query_with(&canonical, &options)
                .expect("the complete carrier reparses")
            else {
                panic!("SELECT");
            };
            assert_eq!(pattern_to_select_query(&pattern), canonical);
        }
    }
}

#[test]
fn many_alternatives_do_not_expand_sequence_branches_exponentially() {
    let text = std::iter::repeat_n("(ex:p|ex:q)", 40)
        .collect::<Vec<_>>()
        .join("/");
    let pattern = parsed(&format!("?s {text} ?o"));
    let mut triples = 0;
    let mut unions = 0;
    walk_pre_post(NodeRef::Pattern(&pattern), |visit, node| {
        if visit == Visit::Enter {
            match node {
                NodeRef::Triple(_) => triples += 1,
                NodeRef::Pattern(GraphPattern::Union { .. }) => unions += 1,
                _ => {}
            }
        }
        Flow::Descend
    });
    assert_eq!(triples, 80);
    assert_eq!(unions, 40);
}

#[test]
fn deeply_nested_inverse_alternatives_translate_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let body = format!(
                "?s {}ex:p|ex:q{} ?o",
                "^(".repeat(100_000),
                ")".repeat(100_000)
            );
            let pattern = parsed(&body);
            let GraphPattern::Project { inner, .. } = pattern else {
                panic!("projection");
            };
            assert!(matches!(&*inner, GraphPattern::Union { .. }));
        })
        .expect("small-stack worker")
        .join()
        .expect("iterative translation");
}

#[test]
fn unrelated_property_function_blanks_remain_unobserved_existentials() {
    let options = ParserOptions {
        property_fn_namespaces: vec!["http://example.org/functions/".to_owned()],
        ..ParserOptions::default()
    };
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query_with(
            &format!(
                "{PREFIX}PREFIX pf: <http://example.org/functions/> \
            SELECT * WHERE {{ ?s (ex:p|ex:q)/ex:r ?o . [] pf:relation [] }}"
            ),
            &options,
        )
        .expect("mixed triples block")
    else {
        panic!("SELECT");
    };
    let mut found = false;
    walk_pre_post(NodeRef::Pattern(&pattern), |visit, node| {
        if visit == Visit::Enter
            && let NodeRef::Pattern(GraphPattern::PropertyFunction(call)) = node
        {
            found = true;
            assert!(
                call.subject_args
                    .iter()
                    .chain(&call.object_args)
                    .all(|term| matches!(term, purrdf_sparql_algebra::TermPattern::BlankNode(_)))
            );
        }
        Flow::Descend
    });
    assert!(found);
}

#[test]
fn non_distinguished_identity_is_admitted_only_in_match_roles() {
    use purrdf_sparql_algebra::{
        AggregateExpression, AggregateFunction, Child, Expression, GraphUpdateOperation,
        TermPattern, Update,
    };
    let hidden = Variable::hidden("witness");
    let Query::Select {
        pattern: GraphPattern::Project { inner, .. },
        dataset,
        base_iri,
        version,
    } = SparqlParser::new()
        .parse_query(&format!("{PREFIX}SELECT * WHERE {{ ?s ex:p ?o }}"))
        .expect("ordinary control")
    else {
        panic!("SELECT");
    };
    let mut matched = (*inner).clone();
    let GraphPattern::Bgp { patterns } = &mut matched else {
        panic!("BGP");
    };
    patterns[0].object = TermPattern::Variable(hidden.clone());
    let query = |pattern| Query::Select {
        pattern,
        dataset: dataset.clone(),
        base_iri: base_iri.clone(),
        version: version.clone(),
    };
    query(matched.clone())
        .validate()
        .expect("opaque match identity");
    query(GraphPattern::Values {
        variables: vec![hidden.clone()],
        bindings: vec![vec![None]],
    })
    .validate()
    .expect("internal match VALUES");
    let aggregate = AggregateExpression::new(
        AggregateFunction::Count,
        vec![Expression::Variable(hidden.clone())],
        vec![],
        vec![],
        true,
    )
    .expect("aggregate arity");
    for pattern in [
        GraphPattern::Project {
            inner: Child::new(matched.clone()),
            variables: vec![hidden.clone()],
        },
        GraphPattern::Group {
            inner: Child::new(matched.clone()),
            variables: vec![hidden.clone()],
            aggregates: vec![],
        },
        GraphPattern::Group {
            inner: Child::new(matched.clone()),
            variables: vec![],
            aggregates: vec![(Variable::new("count"), aggregate)],
        },
        GraphPattern::Extend {
            inner: Child::new(matched.clone()),
            variable: hidden.clone(),
            expression: Expression::Variable(Variable::new("s")),
        },
        GraphPattern::Filter {
            inner: Child::new(matched.clone()),
            expr: Expression::Bound(hidden.clone()),
        },
        GraphPattern::Filter {
            inner: Child::new(matched.clone()),
            expr: Expression::Variable(hidden),
        },
    ] {
        assert!(
            query(pattern.clone())
                .validate()
                .expect_err("non-distinguished observation refused")
                .to_string()
                .contains("non-distinguished")
        );
        assert!(purrdf_sparql_algebra::try_pattern_to_select_query(&pattern).is_err());
        assert!(
            purrdf_sparql_algebra::try_pattern_to_select_query_with_options(
                &pattern,
                &ParserOptions::default()
            )
            .is_err()
        );
        let expected = pattern
            .validate_hidden_variables()
            .expect_err("explicit hidden observation");
        let operation = GraphUpdateOperation::DeleteInsert {
            delete: vec![],
            insert: vec![],
            with: None,
            using: vec![],
            pattern: Box::new(pattern),
        };
        assert_eq!(
            operation
                .try_to_sparql()
                .expect_err("checked operation refuses hidden WHERE observers"),
            expected
        );
        assert_eq!(
            Update {
                operations: vec![operation],
                base_iri: None,
                version: None,
            }
            .try_to_sparql()
            .expect_err("checked request refuses hidden WHERE observers"),
            expected
        );
    }
    query(GraphPattern::Project {
        inner: Child::new(matched),
        variables: vec![Variable::new("__purrdf_hidden_0")],
    })
    .validate()
    .expect("ordinary caller lookalike is distinguished");
}

#[test]
fn hidden_graph_outputs_and_update_carriers_are_refused_in_every_template_slot() {
    use purrdf_sparql_algebra::{
        Child, GraphUpdateOperation, NamedNode, NamedNodePattern, TermPattern, Update,
    };
    use std::fmt::Write as _;
    let Query::Construct {
        template,
        pattern,
        dataset,
        base_iri,
        version,
    } = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}CONSTRUCT {{ ?s ex:p ?o }} WHERE {{ ?s ex:p ?o }}"
        ))
        .expect("template control")
    else {
        panic!("CONSTRUCT");
    };
    for position in 0..6 {
        let mut quad = template[0].clone();
        let hidden = Variable::hidden("output");
        match position {
            0 => quad.triple.subject = TermPattern::Variable(hidden),
            1 => quad.triple.predicate = NamedNodePattern::Variable(hidden),
            2 => {
                let mut nested = quad.triple.clone();
                nested.object = TermPattern::Variable(hidden);
                quad.triple.object = TermPattern::Triple(Child::new(nested));
            }
            3 => quad.graph = Some(NamedNodePattern::Variable(hidden)),
            4 => {
                let mut nested = quad.triple.clone();
                nested.predicate = NamedNodePattern::Variable(hidden);
                quad.triple.object = TermPattern::Triple(Child::new(nested));
            }
            5 => {
                let mut nested = quad.triple.clone();
                nested.subject = TermPattern::Variable(hidden);
                quad.triple.object = TermPattern::Triple(Child::new(nested));
            }
            _ => unreachable!(),
        }
        let query = Query::Construct {
            template: vec![quad.clone()],
            pattern: pattern.clone(),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        };
        assert!(query.validate().is_err());
        let expected = query
            .validate_hidden_variables()
            .expect_err("hidden graph output");
        for operation in [
            GraphUpdateOperation::InsertData {
                data: vec![quad.clone()],
            },
            GraphUpdateOperation::DeleteData {
                data: vec![quad.clone()],
            },
            GraphUpdateOperation::DeleteInsert {
                delete: vec![quad.clone()],
                insert: vec![],
                with: None,
                using: vec![],
                pattern: Box::new(pattern.clone()),
            },
            GraphUpdateOperation::DeleteInsert {
                delete: vec![],
                insert: vec![quad],
                with: None,
                using: vec![],
                pattern: Box::new(pattern.clone()),
            },
        ] {
            let request = Update {
                operations: vec![
                    GraphUpdateOperation::Create {
                        silent: false,
                        graph: NamedNode::new_unchecked("http://example.org/g"),
                    },
                    operation.clone(),
                ],
                base_iri: Some(NamedNode::new_unchecked("http://example.org/")),
                version: None,
            };
            let mut out = String::new();
            assert_eq!(
                request
                    .try_to_sparql()
                    .expect_err("typed refusal preflights the whole request"),
                expected
            );
            assert_eq!(
                operation
                    .try_to_sparql()
                    .expect_err("typed refusal before rendering"),
                expected
            );
            assert!(write!(&mut out, "{request}").is_err());
            assert!(
                out.is_empty(),
                "the whole request is preflighted before bytes"
            );
            assert!(write!(&mut out, "{operation}").is_err());
            assert!(
                out.is_empty(),
                "invalid output is refused before carrier bytes"
            );
        }
    }
    let describe = Query::Describe {
        pattern,
        targets: vec![NamedNodePattern::Variable(Variable::hidden("target"))],
        dataset,
        base_iri,
        version,
    };
    assert!(describe.validate().is_err());
    assert!(describe.validate_hidden_variables().is_err());
}

#[test]
fn checked_update_carriers_preserve_display_bytes_for_every_operation() {
    for text in [
        "INSERT DATA { GRAPH <http://example.org/g> { <http://example.org/s> <http://example.org/p> <<( <http://example.org/a> <http://example.org/b> \"v\"@en--ltr )>> } }",
        "DELETE DATA { <http://example.org/s> <http://example.org/p> \"v\" }",
        "DELETE { ?s ex:p ?o } INSERT { ?s ex:result ?o } WHERE { ?s ex:p ?o }",
        "INSERT { ?s ex:result ?o } WHERE { ?s (ex:p|ex:q)/ex:r ?o }",
        "LOAD SILENT <http://example.org/source> INTO GRAPH <http://example.org/g>",
        "CLEAR SILENT NAMED",
        "DROP SILENT ALL",
        "CREATE SILENT GRAPH <http://example.org/g>",
        "ADD SILENT DEFAULT TO GRAPH <http://example.org/g>",
        "MOVE SILENT GRAPH <http://example.org/g> TO DEFAULT",
        "COPY SILENT GRAPH <http://example.org/g> TO GRAPH <http://example.org/h>",
    ] {
        let request = SparqlParser::new()
            .parse_update(&format!("{PREFIX}{text}"))
            .expect("valid operation");
        let rendered = request.try_to_sparql().expect("checked request");
        assert_eq!(rendered, request.to_string(), "{text}");
        for operation in &request.operations {
            assert_eq!(
                operation.try_to_sparql().expect("checked operation"),
                operation.to_string(),
                "{text}"
            );
        }
        SparqlParser::new()
            .parse_update(&rendered)
            .expect("checked carrier remains valid SPARQL");
        assert_eq!(request.try_to_sparql().expect("repeat"), rendered);
    }

    let canonical =
        "BASE <http://example.org/> CREATE SILENT GRAPH <http://example.org/g> ; DROP SILENT ALL";
    let request = SparqlParser::new()
        .parse_update(canonical)
        .expect("whole-request byte control");
    assert_eq!(request.try_to_sparql().expect("checked request"), canonical);
    assert_eq!(request.to_string(), canonical);

    let canonical = "INSERT DATA { <http://example.org/s> <http://example.org/p> \"v\" . }";
    let request = SparqlParser::new()
        .parse_update(canonical)
        .expect("template byte control");
    assert_eq!(request.try_to_sparql().expect("checked request"), canonical);
    assert_eq!(
        request.operations[0].try_to_sparql().expect("operation"),
        canonical
    );
}
