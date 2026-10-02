// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bag, witness scope and federation contracts for translated predicate alternatives.

mod support;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest};
use purrdf_sparql_algebra::{
    Child, GraphPattern, PropertyPathExpression, Query, SparqlParser, pattern_to_select_query,
};
use purrdf_sparql_eval::{
    InProcessServiceResolver, NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use std::sync::Arc;
use support::{EX, local_dataset, render_cell, row_count, run_prefixed, solutions, sorted_rows};

const PREFIX: &str = "PREFIX ex: <http://example.org/> ";

fn branching_dataset() -> Arc<RdfDataset> {
    local_dataset([
        ("s", "p", "a"),
        ("s", "p", "b"),
        ("s", "p", "o"),
        ("s", "q", "a"),
        ("s", "q", "o"),
        ("a", "r", "o"),
        ("b", "r", "o"),
        ("a", "t", "o"),
        ("b", "t", "o"),
        ("wrong", "r", "other"),
    ])
}

/// Keep the native non-repeating Path as an independent bag control. A temporary
/// optional quantifier prevents text translation; removing just that wrapper in
/// public algebra leaves the exact original property-path expression untouched.
fn native_path(dataset: &Arc<RdfDataset>, path: &str) -> purrdf_core::SparqlResult {
    let mut query = SparqlParser::new()
        .parse_query(&format!("{PREFIX}SELECT ?s ?o WHERE {{ ?s ({path})? ?o }}"))
        .expect("the native control parses");
    let Query::Select {
        pattern: GraphPattern::Project { inner, .. },
        ..
    } = &mut query
    else {
        panic!("SELECT");
    };
    let GraphPattern::Path { path, .. } = &mut **inner else {
        panic!("untranslated path");
    };
    let PropertyPathExpression::ZeroOrOne(original) = path else {
        panic!("optional wrapper");
    };
    *path = (**original).clone();
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_algebra(query, QueryOptions::EMPTY)
        .expect("valid native control");
    engine
        .query_prepared_governed_view(
            &**dataset,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("native path evaluates")
        .into_complete()
        .expect("native control completes")
}

#[test]
fn alternatives_preserve_each_arm_and_each_sequence_witness() {
    let dataset = branching_dataset();
    for (path, count) in [
        ("ex:p|ex:q", 5),
        ("ex:p|ex:p", 6),
        ("(ex:p|ex:q)/(ex:r|ex:t)", 6),
        ("(ex:p|ex:p)/(ex:r|ex:r)", 8),
        ("^( (ex:p|ex:q)/(ex:r|ex:t) )", 6),
        ("^(^( (ex:p|ex:q)/(ex:r|ex:t) ))", 6),
        ("ex:p/ex:r|ex:q/ex:r", 3),
    ] {
        let translated = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?s ?o WHERE {{ ?s {path} ?o }}"),
        );
        let native = native_path(&dataset, path);
        assert_eq!(row_count(&translated), count, "{path}");
        assert_eq!(
            sorted_rows(&translated, render_cell),
            sorted_rows(&native, render_cell),
            "{path}"
        );
    }
}

#[test]
fn source_blank_endpoints_keep_their_bgp_identity_across_filters_and_alternatives() {
    let dataset = local_dataset([
        ("a", "p", "o"),
        ("b", "q", "o"),
        ("a", "t", "x"),
        ("wrong", "t", "y"),
    ]);
    for body in [
        "_:r (ex:p|ex:q) ?o . _:r ex:t ?x",
        "_:r (ex:p|ex:q) ?o . FILTER(true) _:r ex:t ?x",
        "_:r ex:t ?x . FILTER(true) _:r (ex:p|ex:q) ?o",
    ] {
        let result = run_prefixed(
            &dataset,
            PREFIX,
            &format!("SELECT ?o ?x WHERE {{ {body} }}"),
        );
        let expected = run_prefixed(
            &dataset,
            PREFIX,
            "SELECT ?o ?x WHERE { ?s (ex:p|ex:q) ?o . ?s ex:t ?x }",
        );
        assert_eq!(row_count(&result), 1, "{body}");
        assert_eq!(
            sorted_rows(&result, render_cell),
            sorted_rows(&expected, render_cell),
            "{body}"
        );
    }
}

#[test]
fn hidden_witnesses_stay_out_of_star_and_distinct_solution_identity() {
    let dataset = branching_dataset();
    let result = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT * WHERE { ?s (ex:p|ex:q)/(ex:r|ex:t) ?o }",
    );
    let (variables, rows) = solutions(result);
    assert_eq!(variables, ["s", "o"]);
    assert_eq!(rows.len(), 6);
    for form in ["SELECT DISTINCT *", "SELECT REDUCED *"] {
        let result = run_prefixed(
            &dataset,
            PREFIX,
            &format!("{form} WHERE {{ ?s (ex:p|ex:q)/(ex:r|ex:t) ?o }}"),
        );
        assert_eq!(row_count(&result), 1);
    }
    let result = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT (COUNT(DISTINCT *) AS ?n) WHERE { ?s (ex:p|ex:q)/(ex:r|ex:t) ?o }",
    );
    let expected = run_prefixed(
        &dataset,
        PREFIX,
        "SELECT (COUNT(DISTINCT *) AS ?n) WHERE { ?s ex:p ?o FILTER(?o = ex:o) }",
    );
    assert_eq!(
        sorted_rows(&result, render_cell),
        sorted_rows(&expected, render_cell)
    );
}

#[test]
fn serialized_queries_keep_distinctness_and_stable_projection_names() {
    let dataset = branching_dataset();
    for form in [
        "SELECT *",
        "SELECT DISTINCT *",
        "SELECT REDUCED *",
        "SELECT (COUNT(DISTINCT *) AS ?n)",
    ] {
        let text = format!("{PREFIX}{form} WHERE {{ ?s (ex:p|ex:q)/(ex:r|ex:t) ?o }}");
        let Query::Select { pattern, .. } = SparqlParser::new()
            .parse_query(&text)
            .expect("source query")
        else {
            panic!("SELECT");
        };
        let expected = run_prefixed(&dataset, "", &text);
        let carrier = pattern_to_select_query(&pattern);
        let actual = run_prefixed(&dataset, "", &carrier);
        assert_eq!(
            sorted_rows(&actual, render_cell),
            sorted_rows(&expected, render_cell),
            "{carrier}"
        );
        let Query::Select { pattern, .. } = SparqlParser::new()
            .parse_query(&carrier)
            .expect("carrier parses")
        else {
            panic!("SELECT");
        };
        assert_eq!(pattern_to_select_query(&pattern), carrier);
    }
}

#[test]
fn raw_distinct_and_reduced_carriers_hide_witnesses_before_deduplication() {
    let dataset = branching_dataset();
    let Query::Select {
        pattern: GraphPattern::Project { inner, .. },
        ..
    } = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT * WHERE {{ ?s (ex:p|ex:q)/(ex:r|ex:t) ?o }}"
        ))
        .expect("translated body")
    else {
        panic!("projection");
    };
    for pattern in [
        GraphPattern::Distinct {
            inner: Child::new((*inner).clone()),
        },
        GraphPattern::Reduced { inner },
    ] {
        let text = pattern_to_select_query(&pattern);
        let result = run_prefixed(&dataset, "", &text);
        let (variables, rows) = solutions(result);
        assert_eq!(variables, ["s", "o"], "{text}");
        assert_eq!(rows.len(), 1, "{text}");
    }
}

#[test]
fn service_carriers_keep_zero_column_bags_and_distinct_star_counts() {
    let dataset = branching_dataset();
    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(&dataset));
    for (body, count) in [
        ("ex:s (ex:p|ex:q)/(ex:r|ex:t) ex:o", 6),
        (
            "{ SELECT DISTINCT * WHERE { ex:s (ex:p|ex:q)/(ex:r|ex:t) ex:o } }",
            1,
        ),
        (
            "{ SELECT (COUNT(DISTINCT *) AS ?n) WHERE { ?s (ex:p|ex:q)/(ex:r|ex:t) ?o } }",
            1,
        ),
    ] {
        let query = format!("{PREFIX}SELECT * WHERE {{ SERVICE ex:svc {{ {body} }} }}");
        let actual = NativeSparqlEngine::new()
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
            .expect("the carrier evaluates remotely");
        let expected = run_prefixed(&dataset, PREFIX, &format!("SELECT * WHERE {{ {body} }}"));
        assert_eq!(row_count(&actual), count, "{query}");
        assert_eq!(
            sorted_rows(&actual, render_cell),
            sorted_rows(&expected, render_cell),
            "{query}"
        );
        let (actual_vars, _) = solutions(actual);
        let (expected_vars, _) = solutions(expected);
        assert_eq!(
            actual_vars, expected_vars,
            "carrier column must remain unobservable"
        );
    }
}

#[test]
fn named_graph_and_literal_or_quoted_endpoints_survive_service_roundtrips() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    let literal = builder.intern_literal(RdfLiteral::simple("value"));
    let quoted = builder.intern_triple(s, p, literal);
    for graph in [None, Some(builder.intern_iri(&format!("{EX}g")))] {
        for predicate in [p, q] {
            for object in [literal, quoted] {
                builder.push_quad(s, predicate, object, graph);
            }
        }
    }
    let dataset = builder.freeze().expect("RDF1.2 fixture");
    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(&dataset));
    for (body, count) in [
        ("?s (ex:p|ex:q) \"value\"", 2),
        ("?s ^(ex:p|ex:q) \"value\"", 0),
        ("?s (ex:p|ex:q) <<( ex:s ex:p \"value\" )>>", 2),
        ("?s ^(ex:p|ex:q) <<( ex:s ex:p \"value\" )>>", 0),
        ("GRAPH ?g { ?s (ex:p|ex:q) ?o }", 4),
    ] {
        let query = format!("{PREFIX}SELECT * WHERE {{ SERVICE ex:svc {{ {body} }} }}");
        let actual = NativeSparqlEngine::new()
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
            .expect("valid positional matcher carrier");
        let expected = run_prefixed(&dataset, PREFIX, &format!("SELECT * WHERE {{ {body} }}"));
        assert_eq!(row_count(&actual), count, "{body}");
        assert_eq!(
            sorted_rows(&actual, render_cell),
            sorted_rows(&expected, render_cell),
            "{body}"
        );
    }
}

#[test]
fn public_raw_doors_refuse_hidden_observers_without_restricting_ordinary_names() {
    use purrdf_sparql_algebra::{Expression, Variable};
    use purrdf_sparql_eval::{EvalCtx, eval, evaluate_query};
    let dataset = branching_dataset();
    let hidden = Variable::hidden("witness");
    let unit = GraphPattern::empty_bgp();
    for pattern in [
        GraphPattern::Project {
            inner: Child::new(unit.clone()),
            variables: vec![hidden.clone()],
        },
        GraphPattern::Group {
            inner: Child::new(unit.clone()),
            variables: vec![hidden.clone()],
            aggregates: vec![],
        },
        GraphPattern::Filter {
            inner: Child::new(unit.clone()),
            expr: Expression::Bound(hidden.clone()),
        },
        GraphPattern::Filter {
            inner: Child::new(unit.clone()),
            expr: Expression::Variable(hidden),
        },
    ] {
        let mut ctx = EvalCtx::new(&*dataset);
        assert!(
            eval(&pattern, &mut ctx)
                .expect_err("raw pattern refuses hidden observation")
                .to_string()
                .contains("non-distinguished")
        );
        let mut ctx = EvalCtx::new(&*dataset);
        assert!(
            evaluate_query(&support::ask(pattern), &mut ctx)
                .expect_err("raw query refuses hidden observation")
                .to_string()
                .contains("non-distinguished")
        );
    }
    let ordinary = GraphPattern::Project {
        inner: Child::new(unit),
        variables: vec![Variable::new("__purrdf_hidden_0")],
    };
    let mut ctx = EvalCtx::new(&*dataset);
    assert_eq!(
        eval(&ordinary, &mut ctx)
            .expect("legal caller name")
            .schema
            .vars(),
        [Variable::new("__purrdf_hidden_0")]
    );
}

#[test]
fn preparing_already_scoped_source_blanks_is_idempotent() {
    let dataset = local_dataset([
        ("a", "p", "o"),
        ("b", "q", "o"),
        ("a", "t", "x"),
        ("wrong", "t", "y"),
    ]);
    let query = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT * WHERE {{ _:r (ex:p|ex:q) ?o . FILTER(true) _:r ex:t ?x }}"
        ))
        .expect("a source block parses");
    let engine = NativeSparqlEngine::new();
    let original = engine
        .prepare_algebra(query, QueryOptions::EMPTY)
        .expect("the source block is admitted");
    for _ in 0..3 {
        let prepared = engine
            .prepare_algebra(original.query().clone(), QueryOptions::EMPTY)
            .expect("the canonical match identities remain valid");
        assert_eq!(prepared.query(), original.query());
        let result = engine
            .query_prepared_governed_view(
                &*dataset,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("prepared evaluation succeeds")
            .into_complete()
            .expect("the prepared block completes");
        assert_eq!(row_count(&result), 1);
        assert_eq!(solutions(result).0, vec!["o", "x"]);
    }
}

#[test]
fn serialized_updates_never_bind_template_only_variables_to_path_witnesses() {
    let dataset = branching_dataset();
    let engine = NativeSparqlEngine::new();
    for template in [
        "?__purrdf_hidden_0 ex:new ?o",
        "?o ?__purrdf_hidden_0 ex:new",
        "<<( ?__purrdf_hidden_0 ex:new ?o )>> ex:marked true",
        "<<( ?o ?__purrdf_hidden_0 ex:new )>> ex:marked true",
        "<<( ?o ex:new ?__purrdf_hidden_0 )>> ex:marked true",
        "GRAPH ?__purrdf_hidden_0 { ?o ex:marked true }",
    ] {
        let source = format!("{PREFIX}INSERT {{ {template} }} WHERE {{ ?s (ex:p|ex:q)/ex:r ?o }}");
        let parsed = SparqlParser::new()
            .parse_update(&source)
            .expect("UPDATE parses");
        for text in [source, parsed.to_string()] {
            let mut data = Arc::clone(&dataset);
            engine
                .update_with_options(
                    &mut data,
                    SparqlRequest {
                        query: &text,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                )
                .expect("unbound output causes no insertion");
            assert_eq!(
                data.owned_quads().collect::<Vec<_>>(),
                dataset.owned_quads().collect::<Vec<_>>(),
                "{text}"
            );
            assert_eq!(
                data.owned_named_graphs().collect::<Vec<_>>(),
                dataset.owned_named_graphs().collect::<Vec<_>>(),
                "{text}"
            );
        }
    }
}
