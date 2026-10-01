// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Predicate sequences and inverses use the same connected joins as their SPARQL
//! triple-pattern translation, through the public governed and ordinary entries.

mod support;

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlEngine, SparqlRequest,
    SparqlResult, TrippedGovernor,
};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_sparql_algebra::{
    Child, GraphPattern, NamedNode, PropertyPathExpression, Query, SparqlParser, TermPattern,
    Variable,
};
use purrdf_sparql_eval::{
    InProcessServiceResolver, NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use support::{EX, iri, local_dataset, render_cell, row, run_prefixed, solutions, sorted_rows};

const PREFIX: &str = "PREFIX ex: <http://example.org/>\n";
const PAIRS: usize = 64;
// A four-pattern connected join has 64 answers and five working columns. This ceiling
// leaves room for its linear intermediates while excluding a 128-by-128 typed product.
const CELL_CEILING: u64 = 4_096;

/// Independent typed endpoints, each linked through its own two-predicate chain.
fn typed_pairs() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(TYPE);
    let class = builder.intern_iri(&format!("{EX}Node"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for index in 0..PAIRS {
        let x = builder.intern_iri(&format!("{EX}x{index}"));
        let middle = builder.intern_iri(&format!("{EX}middle{index}"));
        let y = builder.intern_iri(&format!("{EX}y{index}"));
        builder.push_quad(x, rdf_type, class, None);
        builder.push_quad(y, rdf_type, class, None);
        builder.push_quad(x, p, middle, None);
        builder.push_quad(middle, q, y, None);
    }
    builder.freeze().expect("typed pairs freeze")
}

#[test]
fn linked_typed_endpoints_complete_under_a_linear_cell_ceiling() {
    let engine = NativeSparqlEngine::new();
    let dataset = typed_pairs();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(CELL_CEILING);

    // Keep the untranslated shape as an independently constructed control. Preparing
    // public algebra does not parse or translate it, so this remains the disconnected
    // BGP joined to a separate path even after the text parser is repaired.
    let Query::Select {
        pattern: GraphPattern::Project { inner, variables },
        dataset: query_dataset,
        base_iri,
        version,
    } = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT ?x ?y ?tx ?ty WHERE {{ ?x a ?tx . ?y a ?ty }}"
        ))
        .expect("the disconnected control parses")
    else {
        panic!("the control is a projected SELECT");
    };
    let untranslated = Query::Select {
        pattern: GraphPattern::Project {
            inner: Child::new(GraphPattern::Join {
                left: inner,
                right: Child::new(GraphPattern::Path {
                    subject: TermPattern::Variable(Variable::new("x")),
                    path: PropertyPathExpression::sequence(
                        PropertyPathExpression::NamedNode(
                            NamedNode::new(format!("{EX}p")).expect("absolute IRI"),
                        ),
                        PropertyPathExpression::NamedNode(
                            NamedNode::new(format!("{EX}q")).expect("absolute IRI"),
                        ),
                    ),
                    object: TermPattern::Variable(Variable::new("y")),
                }),
            }),
            variables,
        },
        dataset: query_dataset,
        base_iri,
        version,
    };
    let prepared = engine
        .prepare_algebra(untranslated, QueryOptions::EMPTY)
        .expect("the control is valid public algebra");
    let refused = engine
        .query_prepared_governed_view(&*dataset, &prepared, &[], QueryOptions::EMPTY, &governors)
        .expect("admission refusal is an outcome");
    let exhausted = refused.exhausted().expect("the cross product is refused");
    assert!(matches!(
        exhausted.tripped,
        TrippedGovernor::Refused {
            dimension: ResourceDimension::IntermediateCells,
            limit: CELL_CEILING,
            estimate,
        } if estimate > CELL_CEILING
    ));
    for dimension in ResourceDimension::ALL {
        assert_eq!(exhausted.evidence.consumed_in(dimension), 0);
    }

    let mut answers = Vec::new();
    for body in [
        "?x ex:p/ex:q ?y . ?x a ?tx . ?y a ?ty",
        "?x ex:p ?middle . ?middle ex:q ?y . ?x a ?tx . ?y a ?ty",
    ] {
        let text = format!("{PREFIX}SELECT ?x ?y ?tx ?ty WHERE {{ {body} }}");
        let complete = engine
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &governors,
            )
            .expect("the connected query evaluates");
        assert!(complete.is_complete(), "{text}: {complete:?}");
        assert!(
            complete
                .evidence()
                .consumed_in(ResourceDimension::IntermediateCells)
                <= CELL_CEILING
        );
        let result = complete.into_complete().expect("the result is complete");
        answers.push(sorted_rows(&result, render_cell));
    }
    let mut expected = (0..PAIRS)
        .map(|index| {
            row(&[
                ("x", &format!("<{EX}x{index}>")),
                ("y", &format!("<{EX}y{index}>")),
                ("tx", &format!("<{EX}Node>")),
                ("ty", &format!("<{EX}Node>")),
            ])
        })
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(answers[0], expected);
    assert_eq!(answers[1], expected);
}

#[test]
fn each_path_occurrence_has_fresh_hidden_witnesses_and_preserves_the_bag() {
    let dataset = local_dataset([
        ("s", "p", "a"),
        ("s", "p", "b"),
        ("a", "q", "o"),
        ("b", "q", "o"),
        ("s", "r", "c"),
        ("s", "r", "d"),
        ("s", "r", "e"),
        ("c", "t", "u"),
        ("d", "t", "u"),
        ("e", "t", "u"),
        ("unrelated", "marker", "marked"),
    ]);
    for (body, expanded, count, y) in [
        (
            "ex:s ex:p/ex:q ?x, ?y",
            "ex:s ex:p ?left . ?left ex:q ?x . ex:s ex:p ?right . ?right ex:q ?y",
            4,
            "o",
        ),
        (
            "ex:s ex:p/ex:q ?x ; ex:r/ex:t ?y",
            "ex:s ex:p ?left . ?left ex:q ?x . ex:s ex:r ?right . ?right ex:t ?y",
            6,
            "u",
        ),
    ] {
        let result = run_prefixed(&dataset, PREFIX, &format!("SELECT * WHERE {{ {body} }}"));
        let expected = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?x ?y WHERE {{ {expanded} }}"),
        );
        assert_eq!(
            sorted_rows(&result, render_cell),
            sorted_rows(&expected, render_cell)
        );
        let (variables, rows) = solutions(result);
        assert_eq!(variables, ["x", "y"]);
        assert_eq!(rows, vec![vec![Some(iri("o")), Some(iri(y))]; count]);
    }
    let result = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT * WHERE {
            _:__purrdf_anon_0 ex:marker ex:marked . ex:s ex:p/ex:q ?o
        }",
    );
    let (variables, rows) = solutions(result);
    assert_eq!(variables, ["o"]);
    assert_eq!(rows, vec![vec![Some(iri("o"))]; 2]);
}

#[test]
fn inverse_groups_reverse_both_step_order_and_directions() {
    let dataset = local_dataset([
        ("s", "p", "middle"),
        ("middle", "q", "o"),
        ("x", "r", "middle"),
        ("y", "t", "o"),
    ]);
    for (path, from, to) in [
        ("(ex:p/ex:q)", "s", "o"),
        ("^(ex:p/ex:q)", "o", "s"),
        ("^(^(ex:p/ex:q))", "s", "o"),
        ("ex:p/^ex:r", "s", "x"),
        ("^(ex:p/^ex:r)", "x", "s"),
        ("^(ex:p/(ex:q/^ex:t))", "y", "s"),
    ] {
        let result = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?s ?o WHERE {{ ?s {path} ?o }}"),
        );
        let (variables, rows) = solutions(result);
        assert_eq!(variables, ["s", "o"]);
        assert_eq!(rows, vec![vec![Some(iri(from)), Some(iri(to))]], "{path}");
    }
}

#[test]
fn an_inverse_join_witness_can_be_a_literal() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let o = builder.intern_iri(&format!("{EX}o"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    let witness = builder.intern_literal(RdfLiteral::simple("shared value"));
    builder.push_quad(s, p, witness, None);
    builder.push_quad(o, q, witness, None);
    let dataset = builder.freeze().expect("literal witness fixture");
    let result = run_prefixed(&dataset, PREFIX, "SELECT * WHERE { ?s ex:p/^ex:q ?o }");
    let (variables, rows) = solutions(result);
    assert_eq!(variables, ["s", "o"]);
    assert_eq!(rows, vec![vec![Some(iri("s")), Some(iri("o"))]]);
}

/// An RDF 1.2 triple term occupies object positions on both edges. The inverse
/// makes it the shared witness without putting it into an RDF subject position.
#[test]
fn an_inverse_join_witness_can_be_an_rdf12_triple_term() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let o = builder.intern_iri(&format!("{EX}o"));
    let unrelated = builder.intern_iri(&format!("{EX}unrelated"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    let quoted_subject = builder.intern_iri(&format!("{EX}quotedSubject"));
    let quoted_predicate = builder.intern_iri(&format!("{EX}quotedPredicate"));
    let quoted_object = builder.intern_literal(RdfLiteral::simple("shared value"));
    let witness = builder.intern_triple(quoted_subject, quoted_predicate, quoted_object);
    let different_object = builder.intern_literal(RdfLiteral::simple("different value"));
    let different_witness =
        builder.intern_triple(quoted_subject, quoted_predicate, different_object);
    builder.push_quad(s, p, witness, None);
    builder.push_quad(o, q, witness, None);
    builder.push_quad(unrelated, q, different_witness, None);
    let dataset = builder.freeze().expect("RDF 1.2 witness fixture");

    let result = run_prefixed(&dataset, PREFIX, "SELECT * WHERE { ?s ex:p/^ex:q ?o }");
    let expanded = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT ?s ?o WHERE { ?s ex:p ?witness . ?o ex:q ?witness }",
    );
    assert_eq!(
        sorted_rows(&result, render_cell),
        sorted_rows(&expanded, render_cell)
    );
    let (variables, rows) = solutions(result);
    assert_eq!(variables, ["s", "o"]);
    assert_eq!(rows, vec![vec![Some(iri("s")), Some(iri("o"))]]);
}

#[test]
fn group_operators_keep_the_translated_paths_in_their_own_scope() {
    let dataset = local_dataset([
        ("s", "p", "a"),
        ("s", "p", "b"),
        ("a", "q", "o"),
        ("b", "q", "o"),
    ]);
    for (body, expanded, count) in [
        (
            "VALUES ?s { ex:s ex:none } OPTIONAL { ?s ex:p/ex:q ?o }",
            "VALUES ?s { ex:s ex:none } OPTIONAL { ?s ex:p ?middle . ?middle ex:q ?o }",
            3,
        ),
        (
            "{ ?s ex:p/ex:q ?o } UNION { ?o ^(ex:p/ex:q) ?s }",
            "{ ?s ex:p ?left . ?left ex:q ?o } UNION { ?s ex:p ?right . ?right ex:q ?o }",
            4,
        ),
        (
            "VALUES ?s { ex:s ex:none } FILTER EXISTS { ?s ex:p/ex:q ?o }",
            "VALUES ?s { ex:s ex:none } FILTER EXISTS { ?s ex:p ?middle . ?middle ex:q ?o }",
            1,
        ),
        (
            "{ SELECT ?s ?o WHERE { ?s ex:p/ex:q ?o } }",
            "{ SELECT ?s ?o WHERE { ?s ex:p ?middle . ?middle ex:q ?o } }",
            2,
        ),
    ] {
        let result = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?s ?o WHERE {{ {body} }}"),
        );
        let expected = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?s ?o WHERE {{ {expanded} }}"),
        );
        assert_eq!(support::row_count(&result), count, "{body}");
        assert_eq!(
            sorted_rows(&result, render_cell),
            sorted_rows(&expected, render_cell),
            "{body}"
        );
    }
}

#[test]
fn graph_scope_and_service_forwarding_keep_the_same_solution_bags() {
    let mut builder = RdfDatasetBuilder::new();
    for (graph, end) in [
        (None, "default"),
        (Some("g1"), "first"),
        (Some("g2"), "second"),
    ] {
        let graph = graph.map(|name| builder.intern_iri(&format!("{EX}{name}")));
        for (s, p, o) in [("s", "p", "middle"), ("middle", "q", end)] {
            let s = builder.intern_iri(&format!("{EX}{s}"));
            let p = builder.intern_iri(&format!("{EX}{p}"));
            let o = builder.intern_iri(&format!("{EX}{o}"));
            builder.push_quad(s, p, o, graph);
        }
    }
    let dataset = builder.freeze().expect("scoped chains freeze");
    let result = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT * WHERE { GRAPH ?g { ?s ex:p/ex:q ?o } }",
    );
    assert_eq!(
        sorted_rows(&result, render_cell),
        vec![
            row(&[
                ("g", "<http://example.org/g1>"),
                ("s", "<http://example.org/s>"),
                ("o", "<http://example.org/first>")
            ]),
            row(&[
                ("g", "<http://example.org/g2>"),
                ("s", "<http://example.org/s>"),
                ("o", "<http://example.org/second>")
            ]),
        ]
    );

    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(&dataset));
    for body in [
        "SERVICE ex:svc { ?s ex:p/ex:q ?o }",
        "SERVICE SILENT ex:svc { ?s ex:p/ex:q ?o }",
        "SERVICE ex:svc { GRAPH ?g { ?s ex:p/ex:q ?o } }",
    ] {
        let query = format!("{PREFIX}SELECT * WHERE {{ {body} }}");
        let result = NativeSparqlEngine::new()
            .query_with_source(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &resolver,
                QueryOptions::EMPTY,
            )
            .expect("the service reparses translated triples");
        let local = if body.contains("GRAPH") {
            "SELECT * WHERE { GRAPH ?g { ?s ex:p ?middle . ?middle ex:q ?o } }"
        } else {
            "SELECT ?s ?o WHERE { ?s ex:p ?middle . ?middle ex:q ?o }"
        };
        let local = run_prefixed(&dataset, PREFIX, local);
        let mut local_rows = sorted_rows(&local, render_cell);
        for row in &mut local_rows {
            row.remove("middle");
        }
        assert_eq!(sorted_rows(&result, render_cell), local_rows, "{body}");
    }
}

#[test]
fn construct_and_update_where_accept_paths_without_accepting_path_templates() {
    let mut dataset = local_dataset([("s", "p", "middle"), ("middle", "q", "o")]);
    let engine = NativeSparqlEngine::new();
    let result = run_prefixed(
        &dataset,
        PREFIX,
        "CONSTRUCT { ?s ex:reachable ?o } WHERE { ?s ex:p/ex:q ?o }",
    );
    let SparqlResult::Graph(graph) = result else {
        panic!("CONSTRUCT returns its graph");
    };
    let (variables, rows) = solutions(run_prefixed(
        &graph,
        PREFIX,
        "SELECT ?s ?o WHERE { ?s ex:reachable ?o }",
    ));
    assert_eq!(variables, ["s", "o"]);
    assert_eq!(rows, vec![vec![Some(iri("s")), Some(iri("o"))]]);

    let update = format!("{PREFIX}INSERT {{ ?s ex:reachable ?o }} WHERE {{ ?o ^(ex:p/ex:q) ?s }}");
    let outcome = engine
        .update_governed(
            &mut dataset,
            SparqlRequest {
                query: &update,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(64),
        )
        .expect("an update WHERE translates the inverse chain");
    assert!(outcome.is_applied(), "{outcome:?}");
    assert_eq!(
        sorted_rows(
            &run_prefixed(
                &dataset,
                PREFIX,
                "SELECT ?s ?o WHERE { ?s ex:reachable ?o }"
            ),
            render_cell
        ),
        sorted_rows(
            &run_prefixed(&graph, PREFIX, "SELECT ?s ?o WHERE { ?s ex:reachable ?o }"),
            render_cell
        )
    );

    let before = Arc::clone(&dataset);
    for body in [
        "INSERT DATA { ex:s ex:p/ex:q ex:o }",
        "INSERT { ?s ^ex:p ?o } WHERE { ?s ex:reachable ?o }",
        "DELETE WHERE { ?s ex:p/ex:q ?o }",
    ] {
        let text = format!("{PREFIX}{body}");
        let diagnostic = engine
            .update(
                &mut dataset,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect_err("a path cannot be an update template");
        assert!(
            diagnostic.to_string().contains("property paths"),
            "{diagnostic:?}"
        );
        assert!(
            Arc::ptr_eq(&dataset, &before),
            "a refused template changed the dataset"
        );
    }
}
