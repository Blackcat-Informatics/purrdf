// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// The pre-binding lane is deprecated and inert; these tests still name it.
#![allow(deprecated)]

//! Host-only scope invariants, with hand-derived bags and identity partitions.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    BlankNode, Chain, Child, GraphPattern, GroundTerm, NamedNode, NamedNodePattern,
    PropertyFunctionCall, Query, QueryDataset, SparqlParser, TermPattern, TriplePattern, Variable,
    try_pattern_to_select_query,
};
use purrdf_sparql_eval::{
    ExtensionEnv, GraphResolveRequest, GraphResolver, InProcessServiceResolver, LoadError,
    MemoryRelation, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions, ShaclPrebinding,
};
use support::{EX, iri, local_dataset, render_cell, row, solutions, sorted_rows};

const PREFIX: &str = "PREFIX ex: <http://example.org/> ";
const PATH: &str = "(ex:p|ex:q)/ex:r";

fn witness_data() -> Arc<RdfDataset> {
    local_dataset([
        ("s", "p", "a"),
        ("s", "p", "b"),
        ("s", "q", "a"),
        ("a", "r", "o"),
        ("b", "r", "o"),
        ("wrong", "r", "other"),
    ])
}

fn binding_join(binding: TermPattern, disconnect: bool, project_early: bool) -> Query {
    let edge = |subject, predicate: &str, object| GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(
                NamedNode::new(format!("{EX}{predicate}")).expect("fixture predicate"),
            ),
            object,
        }],
    };
    let start = TermPattern::NamedNode(NamedNode::new(format!("{EX}s")).unwrap());
    let alternative = GraphPattern::Union {
        arms: Chain::new(
            edge(start.clone(), "p", binding.clone()),
            edge(start, "q", binding.clone()),
            [],
        ),
    };
    let left = if project_early {
        GraphPattern::Project {
            inner: Child::new(alternative),
            variables: vec![],
        }
    } else {
        alternative
    };
    let following_binding = if disconnect {
        TermPattern::Variable(Variable::hidden("disconnected"))
    } else {
        binding
    };
    Query::Select {
        pattern: GraphPattern::Project {
            inner: Child::new(GraphPattern::Join {
                left: Child::new(left),
                right: Child::new(edge(
                    following_binding,
                    "r",
                    TermPattern::Variable(Variable::new("o")),
                )),
            }),
            variables: vec![Variable::new("o")],
        },
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    }
}

#[test]
fn lost_connections_and_early_projection_have_independently_distinct_bags() {
    let dataset = witness_data();
    let engine = NativeSparqlEngine::new();
    for (binding, disconnect, early, o_count, other_count) in [
        (
            TermPattern::Variable(Variable::hidden("path")),
            false,
            false,
            3,
            0,
        ),
        (
            TermPattern::Variable(Variable::hidden("path")),
            true,
            false,
            6,
            3,
        ),
        (
            TermPattern::Variable(Variable::hidden("path")),
            false,
            true,
            6,
            3,
        ),
        (
            TermPattern::BlankNode(BlankNode::new("spelling")),
            false,
            false,
            6,
            3,
        ),
    ] {
        // Three first-edge derivations, and three independent r edges. Losing
        // their shared binding yields nine products: six o and three other.
        let query = binding_join(binding, disconnect, early);
        query
            .validate()
            .expect("each tree is structurally admissible");
        let carrier = try_pattern_to_select_query(query.pattern()).expect("carrier");
        let mut expected = vec![row(&[("o", "<http://example.org/o>")]); o_count];
        expected.extend(vec![
            row(&[("o", "<http://example.org/other>")]);
            other_count
        ]);
        expected.sort();
        let plan = engine
            .prepare_algebra(query, QueryOptions::EMPTY)
            .expect("raw preparation");
        for result in [
            engine
                .query_prepared(&dataset, &plan, &[], QueryOptions::EMPTY)
                .expect("raw execution"),
            engine
                .query(
                    &dataset,
                    SparqlRequest {
                        query: &carrier,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("carrier execution"),
        ] {
            assert_eq!(sorted_rows(&result, render_cell), expected);
            assert_eq!(solutions(result).0, ["o"], "witness remains hidden");
        }
    }
}

#[test]
fn copied_raw_blank_sites_keep_independent_graph_owners() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(&format!("{EX}s"));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let graph = builder.intern_iri(&format!("{EX}g"));
    for object in ["a", "b"] {
        let object = builder.intern_iri(&format!("{EX}{object}"));
        builder.push_quad(subject, predicate, object, Some(graph));
    }
    let dataset = builder.freeze().expect("two named-graph edges");
    let engine = NativeSparqlEngine::new();
    let copied = GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(Variable::new("s")),
            predicate: NamedNodePattern::NamedNode(NamedNode::new(format!("{EX}p")).unwrap()),
            object: TermPattern::BlankNode(BlankNode::new("same")),
        }],
    };
    let in_graph = |inner: GraphPattern| GraphPattern::Graph {
        name: NamedNodePattern::NamedNode(NamedNode::new(format!("{EX}g")).unwrap()),
        inner: inner.into(),
    };
    for (pattern, body, count) in [
        (
            GraphPattern::Join {
                left: in_graph(copied.clone()).into(),
                right: in_graph(copied.clone()).into(),
            },
            "GRAPH ex:g { ?s ex:p _:left } GRAPH ex:g { ?s ex:p _:right }",
            4,
        ),
        (
            in_graph(GraphPattern::Join {
                left: copied.clone().into(),
                right: copied.into(),
            }),
            "GRAPH ex:g { ?s ex:p _:same . ?s ex:p _:same }",
            2,
        ),
    ] {
        // Independent GRAPH owners choose a or b separately: four products.
        // One positive owner shares the existential: only the two equal pairs.
        let query = Query::Select {
            pattern: GraphPattern::Project {
                inner: pattern.into(),
                variables: vec![Variable::new("s")],
            },
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        query.validate().expect("raw source-owner control");
        let carrier = try_pattern_to_select_query(query.pattern()).expect("owner-aware carrier");
        let source = format!("{PREFIX}SELECT ?s WHERE {{ {body} }}");
        let raw = engine.prepare_algebra(query, QueryOptions::EMPTY).unwrap();
        let prepared = engine.prepare_query(&source, None).unwrap();
        let carried = engine.prepare_query(&carrier, None).unwrap();
        let expected = vec![row(&[("s", "<http://example.org/s>")]); count];
        for _ in 0..3 {
            for plan in [&raw, &prepared, &carried] {
                let result = engine
                    .query_prepared(&dataset, plan, &[], QueryOptions::EMPTY)
                    .expect("source-owner execution");
                assert_eq!(sorted_rows(&result, render_cell), expected);
                assert_eq!(solutions(result).0, ["s"]);
            }
        }
    }
}

#[test]
fn property_functions_share_existentials_without_leaking_witness_columns() {
    let dataset = witness_data();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{EX}relation"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![iri("a"), iri("yes")], vec![iri("b"), iri("no")]],
            )
            .expect("fixture relation"),
        ),
    );
    let env = ExtensionEnv::over_relations(registry).expect("relation environment");
    let options = QueryOptions::new().with_env(&env);
    let engine = NativeSparqlEngine::new();
    for (body, count, raw_join) in [
        (
            "?s (ex:p|ex:q) _:shared . (_:shared) ex:relation (ex:yes)",
            2,
            false,
        ),
        (
            "(_:shared) ex:relation (ex:yes) . ?s (ex:p|ex:q) _:shared",
            2,
            false,
        ),
        (
            "?s (ex:p|ex:q) _:shared . FILTER(true) (_:shared) ex:relation (ex:yes)",
            2,
            false,
        ),
        (
            "?s ex:p _:shared . (_:shared) ex:relation (ex:yes)",
            1,
            true,
        ),
    ] {
        let text = format!("{PREFIX}SELECT * WHERE {{ {body} }}");
        let prepared = engine
            .prepare_query_with_options(&text, None, options)
            .expect("registry-aware source admission");
        let raw_input = if raw_join {
            raw_relation_join()
        } else {
            prepared.query().clone()
        };
        let carrier =
            try_pattern_to_select_query(raw_input.pattern()).expect("property-function carrier");
        if raw_join {
            assert!(
                carrier.matches('{').count() >= 2,
                "PF operand retains required braces"
            );
            assert!(
                !carrier.contains("_:shared"),
                "shared endpoint crosses braces as an alias"
            );
        }
        let raw = engine
            .prepare_algebra(raw_input, options)
            .expect("registry-aware raw admission");
        let expected = vec![row(&[("s", "<http://example.org/s>")]); count];
        for repetition in 0..3 {
            for result in [
                engine
                    .query_with_options_view(
                        &*dataset,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &[],
                        },
                        options,
                    )
                    .expect("native relation query"),
                engine
                    .query_prepared(&dataset, &prepared, &[], options)
                    .expect("prepared relation query"),
                engine
                    .query_prepared(&dataset, &raw, &[], options)
                    .expect("raw relation query"),
                engine
                    .query_with_options_view(
                        &*dataset,
                        SparqlRequest {
                            query: &carrier,
                            base_iri: None,
                            substitutions: &[],
                        },
                        options,
                    )
                    .expect("property-function carrier reparse"),
            ] {
                // Only a satisfies the relation. The p-only raw join has one
                // derivation; p|q has two. The b/no row cannot cross-product in.
                assert_eq!(
                    sorted_rows(&result, render_cell),
                    expected,
                    "run {repetition}"
                );
                assert_eq!(solutions(result).0, ["s"]);
            }
        }
    }
}

fn raw_relation_join() -> Query {
    let shared = TermPattern::BlankNode(BlankNode::new("shared"));
    Query::Select {
        pattern: GraphPattern::Project {
            inner: GraphPattern::Join {
                left: GraphPattern::Bgp {
                    patterns: vec![TriplePattern {
                        subject: TermPattern::Variable(Variable::new("s")),
                        predicate: NamedNodePattern::NamedNode(
                            NamedNode::new(format!("{EX}p")).unwrap(),
                        ),
                        object: shared.clone(),
                    }],
                }
                .into(),
                right: GraphPattern::PropertyFunction(PropertyFunctionCall {
                    iri: format!("{EX}relation"),
                    subject_args: vec![shared],
                    object_args: vec![TermPattern::NamedNode(
                        NamedNode::new(format!("{EX}yes")).unwrap(),
                    )],
                })
                .into(),
            }
            .into(),
            variables: vec![Variable::new("s")],
        },
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    }
}

#[test]
fn concrete_blank_prebindings_preserve_scope_and_never_become_existentials() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(&format!("{EX}p"));
    for (scope, object) in [(7, "a"), (8, "b")] {
        let subject = builder.intern_blank("same", BlankScope(scope));
        let object = builder.intern_iri(&format!("{EX}{object}"));
        builder.push_quad(subject, predicate, object, None);
    }
    let subject = builder.intern_iri(&format!("{EX}s"));
    let object = builder.intern_iri(&format!("{EX}c"));
    builder.push_quad(subject, predicate, object, None);
    let dataset = builder.freeze().expect("concrete-identity data");
    let text = format!("{PREFIX}SELECT ?focus ?o WHERE {{ ?focus ex:p ?o }}");
    let engine = NativeSparqlEngine::new();
    let plan = engine
        .prepare_query(&text, None)
        .expect("parameterized query");
    for (scope, object) in [(7, Some("a")), (8, Some("b")), (9, None)] {
        let focus = TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(scope),
        };
        let substitutions = [("focus".into(), focus.clone())];
        for options in [
            QueryOptions::EMPTY,
            QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied),
        ] {
            for result in [
                engine
                    .query_with_options_view(
                        &*dataset,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &substitutions,
                        },
                        options,
                    )
                    .expect("concrete text prebinding"),
                engine
                    .query_prepared(&dataset, &plan, &substitutions, options)
                    .expect("concrete prepared prebinding"),
            ] {
                let (variables, rows) = solutions(result);
                assert_eq!(variables, ["focus", "o"]);
                let expected = object.map(|name| vec![Some(focus.clone()), Some(iri(name))]);
                assert_eq!(rows, expected.into_iter().collect::<Vec<_>>());
            }
        }
        let raw = SparqlParser::new()
            .parse_query(&text)
            .expect("source query")
            .substitute_variable(
                &Variable::new("focus"),
                GroundTerm::BlankNode(BlankNode::new(BlankScope(scope).qualify_label("same"))),
            );
        assert!(
            try_pattern_to_select_query(raw.pattern()).is_err(),
            "an injection-only dataset blank cannot be carried as source VALUES syntax"
        );
        let injected = engine
            .prepare_algebra(raw, QueryOptions::EMPTY)
            .expect("injection admission");
        let (_, rows) = solutions(
            engine
                .query_prepared(&dataset, &injected, &[], QueryOptions::EMPTY)
                .expect("injection-only raw execution"),
        );
        assert_eq!(rows.len(), usize::from(object.is_some()));
        if let Some(name) = object {
            assert_eq!(rows, [vec![Some(focus), Some(iri(name))]]);
        }
    }
    let existential = format!("{PREFIX}SELECT ?o WHERE {{ _:same ex:p ?o }}");
    let result = engine
        .query(
            &dataset,
            SparqlRequest {
                query: &existential,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("a pattern blank is an existential");
    assert_eq!(
        sorted_rows(&result, render_cell),
        vec![
            row(&[("o", "<http://example.org/a>")]),
            row(&[("o", "<http://example.org/b>")]),
            row(&[("o", "<http://example.org/c>")]),
        ]
    );
    assert!(
        SparqlParser::new()
            .parse_query(&format!(
                "{PREFIX}SELECT ?focus WHERE {{ VALUES ?focus {{ _:same }} }}"
            ))
            .is_err(),
        "concrete blank VALUES are injection-only, never source syntax"
    );
}

#[test]
fn federation_names_preserve_caller_lookalikes_and_zero_column_bags() {
    let dataset = witness_data();
    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(&dataset));
    let engine = NativeSparqlEngine::new();
    for (body, expected_vars, count) in [
        (format!("ex:s {PATH} ex:o"), vec![], 3),
        (
            format!("{{ SELECT DISTINCT * WHERE {{ ex:s {PATH} ex:o }} }}"),
            vec![],
            1,
        ),
        (
            format!("VALUES ?__purrdf_hidden_0 {{ \"caller\" }} ex:s {PATH} ?o"),
            vec!["__purrdf_hidden_0", "o"],
            3,
        ),
    ] {
        let text = format!("{PREFIX}SELECT * WHERE {{ SERVICE ex:svc {{ {body} }} }}");
        let prepared = engine.prepare_query(&text, None).expect("prepared SERVICE");
        let options = QueryOptions::EMPTY.with_remote(Some(&resolver));
        let raw = engine
            .prepare_algebra(prepared.query().clone(), options)
            .expect("raw SERVICE admission");
        for _ in 0..3 {
            for result in [
                engine
                    .query_with_source(
                        &dataset,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &[],
                        },
                        &resolver,
                        QueryOptions::EMPTY,
                    )
                    .expect("in-process SERVICE carrier"),
                engine
                    .query_prepared(&dataset, &prepared, &[], options)
                    .expect("prepared SERVICE reuse"),
                engine
                    .query_prepared(&dataset, &raw, &[], options)
                    .expect("raw SERVICE reuse"),
            ] {
                let (variables, rows) = solutions(result);
                assert_eq!(variables, expected_vars);
                assert_eq!(rows.len(), count);
                for row in rows {
                    if expected_vars.is_empty() {
                        assert_eq!(row, []);
                    } else {
                        assert_eq!(
                            row,
                            [Some(TermValue::simple_literal("caller")), Some(iri("o"))]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn concrete_blank_identity_inside_rdf12_triple_terms_is_structural() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("same", BlankScope(7));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let object = builder.intern_iri(&format!("{EX}a"));
    let triple = builder.intern_triple(subject, predicate, object);
    let report = builder.intern_iri(&format!("{EX}report"));
    let said = builder.intern_iri(&format!("{EX}said"));
    builder.push_quad(report, said, triple, None);
    builder.push_quad(subject, predicate, object, None);
    let dataset = builder.freeze().expect("blank-bearing triple-term data");
    let text = format!("{PREFIX}SELECT ?o WHERE {{ ex:report ex:said <<( ?focus ex:p ?o )>> }}");
    let engine = NativeSparqlEngine::new();
    let plan = engine
        .prepare_query(&text, None)
        .expect("triple-term query");
    for scope in [7, 8] {
        let substitutions = [(
            "focus".into(),
            TermValue::Blank {
                label: "same".into(),
                scope: BlankScope(scope),
            },
        )];
        for result in [
            engine
                .query(
                    &dataset,
                    SparqlRequest {
                        query: &text,
                        base_iri: None,
                        substitutions: &substitutions,
                    },
                )
                .expect("triple-term text prebinding"),
            engine
                .query_prepared(&dataset, &plan, &substitutions, QueryOptions::EMPTY)
                .expect("triple-term prepared prebinding"),
        ] {
            let (_, rows) = solutions(result);
            assert_eq!(
                rows,
                if scope == 7 {
                    vec![vec![Some(iri("a"))]]
                } else {
                    vec![]
                }
            );
        }
    }
    let existential = format!(
        "{PREFIX}SELECT ?o WHERE {{ ex:report ex:said <<( _:shared ex:p ?o )>> . _:shared ex:p ?o }}"
    );
    let query = SparqlParser::new()
        .parse_query(&existential)
        .expect("existential inside/outside triple");
    let carrier = try_pattern_to_select_query(query.pattern()).expect("existential carrier");
    let raw = engine
        .prepare_algebra(query, QueryOptions::EMPTY)
        .expect("raw quoted-pattern preparation");
    let (_, rows) = solutions(
        engine
            .query_prepared(&dataset, &raw, &[], QueryOptions::EMPTY)
            .expect("raw shared quoted-pattern existential"),
    );
    assert_eq!(rows, [vec![Some(iri("a"))]]);
    for text in [&existential, &carrier] {
        let (_, rows) = solutions(
            engine
                .query(
                    &dataset,
                    SparqlRequest {
                        query: text,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("one shared pattern existential"),
        );
        assert_eq!(rows, [vec![Some(iri("a"))]]);
    }
}

fn template_partition(
    dataset: &RdfDataset,
    expected_subjects: usize,
    context: &str,
) -> BTreeSet<(String, BlankScope)> {
    let mut subjects: BTreeMap<(String, BlankScope), BTreeSet<String>> = BTreeMap::new();
    for quad in dataset.quads() {
        let predicate = dataset.term_value(quad.p);
        if predicate == iri("left") || predicate == iri("right") {
            let subject = dataset.term_value(quad.s);
            let (label, scope) = subject.as_blank().expect("a fresh template subject");
            assert_ne!(
                (label, scope),
                ("c1", BlankScope::DEFAULT),
                "never the data blank: {context}"
            );
            assert_eq!(dataset.term_value(quad.o), iri("o"));
            subjects
                .entry((label.to_owned(), scope))
                .or_default()
                .insert(predicate.as_iri().unwrap().to_owned());
        }
    }
    assert_eq!(
        subjects.len(),
        expected_subjects,
        "one allocation per solution row"
    );
    assert!(
        subjects.values().all(|predicates| predicates.len() == 2),
        "the two uses of one template label co-refer within their row"
    );
    subjects.into_keys().collect()
}

#[test]
fn construct_and_insert_allocate_per_duplicate_row_and_per_execution() {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&witness_data());
    let seed = builder.intern_iri(&format!("{EX}seed"));
    let has = builder.intern_iri(&format!("{EX}has"));
    let data_blank = builder.intern_blank("c1", BlankScope::DEFAULT);
    builder.push_quad(seed, has, data_blank, None);
    let dataset = builder.freeze().expect("template input");
    let engine = NativeSparqlEngine::new();
    let construct = format!(
        "{PREFIX}CONSTRUCT {{ _:fresh ex:left ?o ; ex:right ?o . ex:carry ex:data ?data }} \
         WHERE {{ ex:s {PATH} ?o . ex:seed ex:has ?data }}"
    );
    let plan = engine
        .prepare_query(&construct, None)
        .expect("prepared CONSTRUCT");
    for _ in 0..3 {
        for result in [
            engine
                .query(
                    &dataset,
                    SparqlRequest {
                        query: &construct,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("native CONSTRUCT"),
            engine
                .query_prepared(&dataset, &plan, &[], QueryOptions::EMPTY)
                .expect("prepared CONSTRUCT reuse"),
        ] {
            let SparqlResult::Graph(graph) = result else {
                panic!("CONSTRUCT graph")
            };
            assert_eq!(
                graph.quad_count(),
                7,
                "six allocated triples and one carried triple"
            );
            template_partition(&graph, 3, "CONSTRUCT");
            let carried: Vec<_> = graph
                .quads()
                .filter(|quad| graph.term_value(quad.p) == iri("data"))
                .map(|quad| graph.term_value(quad.o))
                .collect();
            assert_eq!(
                carried,
                [TermValue::Blank {
                    label: "c1".into(),
                    scope: BlankScope::DEFAULT
                }]
            );
        }
    }
    // A shared publication builder gives repeated CONSTRUCT executions one
    // identity space, so freshness across calls can be observed directly.
    let mut accumulated = RdfDatasetBuilder::new();
    accumulated.push_dataset(&dataset);
    for _ in 0..3 {
        engine
            .construct_prepared_into_view(
                &*dataset,
                &plan,
                &[],
                QueryOptions::EMPTY,
                &mut accumulated,
            )
            .expect("repeated CONSTRUCT append");
    }
    let accumulated = accumulated.freeze().expect("shared result publication");
    template_partition(&accumulated, 9, "three CONSTRUCT appends");
    let update =
        format!("{PREFIX}INSERT {{ _:fresh ex:left ?o ; ex:right ?o }} WHERE {{ ex:s {PATH} ?o }}");
    let mut target = Arc::clone(&dataset);
    let mut old_allocations = BTreeSet::new();
    for run in 1..=3 {
        engine
            .update_with_options(
                &mut target,
                SparqlRequest {
                    query: &update,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .expect("INSERT template applies atomically");
        let allocations = template_partition(&target, 3 * run, &format!("INSERT execution {run}"));
        assert!(old_allocations.is_subset(&allocations));
        assert_eq!(
            allocations.difference(&old_allocations).count(),
            3,
            "each execution allocates three new nodes in the same dataset"
        );
        old_allocations = allocations;
    }
}

#[test]
fn bnode_allocation_is_fresh_against_default_scope_dataset_blank() {
    let mut builder = RdfDatasetBuilder::new();
    let existing = builder.intern_blank("bnode1", BlankScope::DEFAULT);
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let object = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(existing, predicate, object, None);
    let dataset = builder.freeze().expect("existing default-scope blank");
    let engine = NativeSparqlEngine::new();
    let text = format!(
        "{PREFIX}SELECT ?existing WHERE {{ ?existing ex:p ex:o \
         BIND(BNODE() AS ?fresh) FILTER(?fresh = ?existing) }}"
    );
    let result = engine
        .query(
            &dataset,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("BNODE allocation query");
    assert_eq!(
        solutions(result).1.len(),
        0,
        "BNODE() must allocate an identity distinct from existing dataset blanks"
    );
}

#[test]
fn bnode_memo_and_prepared_reuse_preserve_scoped_identity_partitions() {
    let engine = NativeSparqlEngine::new();
    let text = format!(
        "{PREFIX}SELECT ?existing (BNODE(\"same\") AS ?first) \
         (BNODE(\"same\") AS ?again) (BNODE(\"other\") AS ?other) \
         WHERE {{ ?existing ex:p ex:o }}"
    );
    let prepared = engine
        .prepare_query(&text, None)
        .expect("prepared memo query");
    for prefix in [None, Some("caller_")] {
        for scope in [BlankScope::DEFAULT, BlankScope(7)] {
            let mut builder = RdfDatasetBuilder::new();
            let predicate = builder.intern_iri(&format!("{EX}p"));
            let object = builder.intern_iri(&format!("{EX}o"));
            for ordinal in [1, 2] {
                let label = format!("{}bnode{ordinal}", prefix.unwrap_or_default());
                let blank = builder.intern_blank(&label, scope);
                builder.push_quad(blank, predicate, object, None);
            }
            let dataset = builder.freeze().expect("occupied mint spellings");
            let mut previous = None;
            for _ in 0..3 {
                let result = engine
                    .query_prepared(
                        &dataset,
                        &prepared,
                        &[],
                        QueryOptions::EMPTY.with_bnode_mint_prefix(prefix),
                    )
                    .expect("scoped memo execution");
                let (variables, rows) = solutions(result);
                assert_eq!(variables, ["existing", "first", "again", "other"]);
                assert_eq!(rows.len(), 2);
                let mut allocations = BTreeSet::new();
                for cells in &rows {
                    assert_eq!(
                        cells[1], cells[2],
                        "same argument in one solution shares identity"
                    );
                    assert_ne!(
                        cells[1], cells[3],
                        "different arguments allocate independently"
                    );
                    for index in [1, 3] {
                        let Some(TermValue::Blank {
                            label,
                            scope: allocated_scope,
                        }) = &cells[index]
                        else {
                            panic!("BNODE produces a concrete blank");
                        };
                        assert_eq!(*allocated_scope, BlankScope::DEFAULT);
                        if scope == BlankScope::DEFAULT {
                            assert_ne!(
                                cells[index], cells[0],
                                "dataset identity is never captured"
                            );
                            assert!(![1, 2].into_iter().any(|ordinal| label
                                == &format!("{}bnode{ordinal}", prefix.unwrap_or_default())));
                        }
                        assert!(
                            allocations.insert((label.clone(), *allocated_scope)),
                            "different solutions allocate distinct identities"
                        );
                    }
                }
                let first = if scope == BlankScope::DEFAULT { 3 } else { 1 };
                assert!(
                    allocations.contains(&(
                        format!("{}bnode{first}", prefix.unwrap_or_default()),
                        BlankScope::DEFAULT
                    )),
                    "no-collision labels retain their exact bytes"
                );
                if let Some(previous) = &previous {
                    assert_eq!(
                        &rows, previous,
                        "independent result executions retain deterministic spelling"
                    );
                }
                previous = Some(rows);
            }
        }
    }
}

#[test]
fn bnode_allocation_avoids_dataset_blanks_retained_only_inside_triple_terms() {
    let engine = NativeSparqlEngine::new();
    let text = format!(
        "{PREFIX}SELECT ?existing ?fresh WHERE {{ ex:holder ex:term ?quoted \
         BIND(SUBJECT(?quoted) AS ?existing) BIND(BNODE() AS ?fresh) }}"
    );
    let prepared = engine
        .prepare_query(&text, None)
        .expect("quoted identity query");
    for prefix in [None, Some("caller_")] {
        for scope in [BlankScope::DEFAULT, BlankScope(7)] {
            let mut builder = RdfDatasetBuilder::new();
            let blank =
                builder.intern_blank(&format!("{}bnode1", prefix.unwrap_or_default()), scope);
            let predicate = builder.intern_iri(&format!("{EX}p"));
            let object = builder.intern_iri(&format!("{EX}o"));
            let quoted = builder.intern_triple(blank, predicate, object);
            let holder = builder.intern_iri(&format!("{EX}holder"));
            let term = builder.intern_iri(&format!("{EX}term"));
            builder.push_quad(holder, term, quoted, None);
            let dataset = builder.freeze().expect("quoted dataset blank");
            let (_, rows) = solutions(
                engine
                    .query_prepared(
                        &dataset,
                        &prepared,
                        &[],
                        QueryOptions::EMPTY.with_bnode_mint_prefix(prefix),
                    )
                    .expect("quoted identity execution"),
            );
            assert_eq!(rows.len(), 1);
            assert_ne!(rows[0][0], rows[0][1]);
            assert_eq!(
                rows[0][1],
                Some(TermValue::Blank {
                    label: format!(
                        "{}bnode{}",
                        prefix.unwrap_or_default(),
                        if scope == BlankScope::DEFAULT { 2 } else { 1 }
                    ),
                    scope: BlankScope::DEFAULT,
                })
            );
        }
    }
}

#[test]
fn raw_late_values_cannot_capture_an_earlier_bnode_allocation() {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty raw input");
    for prefix in [None, Some("caller_")] {
        let source = format!(
            "{PREFIX}SELECT ?fresh ?existing WHERE {{ BIND(BNODE() AS ?fresh) \
             FILTER(?fresh = ?existing) }}"
        );
        let mut query = SparqlParser::new()
            .parse_query(&source)
            .expect("allocation/filter source");
        let Query::Select {
            pattern: GraphPattern::Project { inner, .. },
            ..
        } = &mut query
        else {
            panic!("SELECT projection");
        };
        let GraphPattern::Filter { inner, .. } = &mut **inner else {
            panic!("FILTER body");
        };
        *inner = GraphPattern::Join {
            left: inner.clone(),
            right: GraphPattern::Values {
                variables: vec![Variable::new("existing")],
                bindings: vec![vec![Some(GroundTerm::BlankNode(BlankNode::new(format!(
                    "{}bnode1",
                    prefix.unwrap_or_default()
                ))))]],
            }
            .into(),
        }
        .into();
        let options = QueryOptions::EMPTY.with_bnode_mint_prefix(prefix);
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_algebra(query.clone(), options)
            .expect("raw concrete input");
        for _ in 0..3 {
            assert_eq!(
                solutions(
                    engine
                        .query_prepared(&dataset, &prepared, &[], options)
                        .expect("late VALUES execution")
                )
                .1
                .len(),
                0
            );
        }
        let mut context = purrdf_sparql_eval::EvalCtx::new(&*dataset);
        if let Some(prefix) = prefix {
            context = context
                .with_bnode_mint_prefix(prefix)
                .expect("caller prefix");
        }
        assert!(
            purrdf_sparql_eval::eval(query.pattern(), &mut context)
                .expect("raw eval entry")
                .is_empty()
        );
    }
}

#[test]
fn ordinary_construct_allocation_is_distinct_from_a_carried_dataset_blank() {
    let mut builder = RdfDatasetBuilder::new();
    let existing = builder.intern_blank("c1", BlankScope::DEFAULT);
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let object = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(existing, predicate, object, None);
    let dataset = builder
        .freeze()
        .expect("existing default-scope template spelling");
    let text = format!(
        "{PREFIX}CONSTRUCT {{ ?existing ex:kept ex:o . _:fresh ex:allocated ex:o }} \
         WHERE {{ ?existing ex:p ex:o }}"
    );
    let result = NativeSparqlEngine::new()
        .query(
            &dataset,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("ordinary CONSTRUCT");
    let SparqlResult::Graph(graph) = result else {
        panic!("CONSTRUCT graph");
    };
    let subjects: BTreeSet<_> = graph
        .quads()
        .map(|quad| {
            let TermValue::Blank { label, scope } = graph.term_value(quad.s) else {
                panic!("both CONSTRUCT subjects are blank identities");
            };
            (label, scope)
        })
        .collect();
    assert_eq!(
        subjects.len(),
        2,
        "carried and allocated identities must remain distinct"
    );
}

#[test]
fn service_bnode_results_have_response_identity_separation() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("empty local dataset");
    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(&dataset));
    let text = format!(
        "{PREFIX}SELECT ?local ?remote WHERE {{ BIND(BNODE() AS ?local) \
         SERVICE ex:svc {{ BIND(BNODE() AS ?remote) }} FILTER(?local = ?remote) }}"
    );
    let result = NativeSparqlEngine::new()
        .query_with_source(
            &dataset,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
            &resolver,
            QueryOptions::EMPTY,
        )
        .expect("independent SERVICE BNODE result");
    assert_eq!(
        solutions(result).1.len(),
        0,
        "endpoint result blanks are distinct from local allocations"
    );
}

#[test]
fn update_allocations_avoid_reserved_namespaces_and_honor_caller_prefix() {
    let engine = NativeSparqlEngine::new();
    for prefix in [None, Some("caller_")] {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri(&format!("{EX}seed"));
        let predicate = builder.intern_iri(&format!("{EX}retained"));
        let namespace = prefix.unwrap_or_default();
        let retained: BTreeSet<_> = [
            "c1".to_owned(),
            "append0_c1".to_owned(),
            "append1_c1".to_owned(),
            format!("{namespace}c1"),
            format!("{namespace}append0_c1"),
            format!("{namespace}append1_c1"),
        ]
        .into_iter()
        .collect();
        for label in &retained {
            let blank = builder.intern_blank(label, BlankScope::DEFAULT);
            builder.push_quad(subject, predicate, blank, None);
        }
        let initial = builder.freeze().expect("retained destination namespaces");
        for body in [
            "INSERT DATA { _:fresh ex:left ex:o ; ex:right ex:o }",
            "INSERT { _:fresh ex:left ex:o ; ex:right ex:o } WHERE {}",
            "INSERT { ?fresh ex:left ex:o ; ex:right ex:o } WHERE { BIND(BNODE() AS ?fresh) }",
        ] {
            let mut target = Arc::clone(&initial);
            let text = format!("{PREFIX}{body}");
            let mut previous = BTreeSet::new();
            for execution in 1..=3 {
                engine
                    .update_with_options(
                        &mut target,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &[],
                        },
                        QueryOptions::EMPTY.with_bnode_mint_prefix(prefix),
                    )
                    .expect("fresh update publication");
                let current = template_partition(&target, execution, body);
                let new: Vec<_> = current.difference(&previous).collect();
                assert_eq!(new.len(), 1, "each execution adds exactly one identity");
                let (label, scope) = new[0];
                assert_eq!(*scope, BlankScope::DEFAULT);
                assert!(!retained.contains(label), "existing identity never reused");
                if !body.starts_with("INSERT DATA") {
                    assert!(label.starts_with(namespace), "caller mint prefix preserved");
                    assert!(label.starts_with(&format!("{namespace}append")));
                }
                assert!(previous.is_subset(&current));
                assert_eq!(target.quad_count(), retained.len() + 2 * execution);
                previous = current;
            }
        }
    }
}

#[test]
fn later_update_operations_avoid_blanks_retained_only_inside_new_terms() {
    let engine = NativeSparqlEngine::new();
    for (first, embedded_label) in [
        (
            "INSERT DATA { ex:holder ex:term <<( _:embedded ex:p ex:o )>> }".to_owned(),
            "c1",
        ),
        (
            format!(
                "PREFIX cdt: <{}> INSERT {{ ex:holder ex:term ?list }} \
                 WHERE {{ BIND(cdt:List(BNODE()) AS ?list) }}",
                purrdf_cdt::CDT_NS
            ),
            "bnode1",
        ),
    ] {
        let text = format!(
            "{PREFIX}{first} ; INSERT {{ _:fresh ex:left ex:o ; ex:right ex:o }} WHERE {{}}"
        );
        let mut target = RdfDatasetBuilder::new()
            .freeze()
            .expect("empty destination");
        engine
            .update_with_options(
                &mut target,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .expect("sequential operations preserve nested identities");
        assert_eq!(target.quad_count(), 3);
        let fresh = template_partition(&target, 1, &first);
        let held: Vec<_> = target
            .quads()
            .filter(|quad| target.term_value(quad.p) == iri("term"))
            .map(|quad| target.term_value(quad.o))
            .collect();
        assert_eq!(held.len(), 1);
        let mut embedded = BTreeSet::new();
        let complete: std::ops::ControlFlow<()> = held[0].visit_terms(|term| {
            match term {
                TermValue::Blank { label, scope } => {
                    embedded.insert((label.to_owned(), *scope));
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    ..
                } => {
                    embedded.extend(purrdf_core::cdt_blank::cdt_embedded_blanks(
                        lexical_form,
                        datatype,
                    ));
                }
                TermValue::Iri(_) | TermValue::Triple { .. } => {}
            }
            std::ops::ControlFlow::Continue(())
        });
        assert_eq!(complete, std::ops::ControlFlow::Continue(()));
        assert_eq!(embedded.len(), 1, "one retained, nested identity");
        assert!(embedded.is_disjoint(&fresh));
        assert_eq!(
            embedded,
            BTreeSet::from([(embedded_label.to_owned(), BlankScope::DEFAULT)])
        );
        assert!(fresh.iter().all(|(label, _)| label.starts_with("append0_")));
    }
}

#[test]
fn load_then_insert_preserves_a_nested_identity_without_advancing_the_mint_counter() {
    struct Document(Arc<RdfDataset>);
    impl GraphResolver for Document {
        fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
            assert_eq!(request.iri, format!("{EX}document"));
            Ok(Arc::clone(&self.0))
        }
    }
    let engine = NativeSparqlEngine::new();
    for composite in [false, true] {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri(&format!("{EX}holder"));
        let predicate = builder.intern_iri(&format!("{EX}term"));
        let object = if composite {
            builder.intern_literal(purrdf_core::RdfLiteral::typed(
                "[_:c1]",
                purrdf_cdt::CDT_LIST,
            ))
        } else {
            let blank = builder.intern_blank("c1", BlankScope::DEFAULT);
            let nested_predicate = builder.intern_iri(&format!("{EX}p"));
            let nested_object = builder.intern_iri(&format!("{EX}o"));
            builder.intern_triple(blank, nested_predicate, nested_object)
        };
        builder.push_quad(subject, predicate, object, None);
        let document = Document(builder.freeze().expect("nested source document"));
        let held = document.0.term_value(object);
        let mut target = RdfDatasetBuilder::new().freeze().expect("empty target");
        let text = format!(
            "{PREFIX}LOAD ex:document ; \
             INSERT {{ _:fresh ex:left ex:o ; ex:right ex:o }} WHERE {{}}"
        );
        engine
            .update_with_options(
                &mut target,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY.with_load(Some(&document)),
            )
            .expect("LOAD and INSERT apply in request order");
        assert_eq!(target.quad_count(), 3);
        let fresh = template_partition(&target, 1, "LOAD did not allocate template blanks");
        assert!(fresh.is_disjoint(&BTreeSet::from([("c1".to_owned(), BlankScope::DEFAULT,)])));
        let retained: Vec<_> = target
            .quads()
            .filter(|quad| target.term_value(quad.p) == iri("term"))
            .map(|quad| target.term_value(quad.o))
            .collect();
        assert_eq!(
            retained,
            [held],
            "LOAD keeps the source's exact structural value"
        );
    }
}
