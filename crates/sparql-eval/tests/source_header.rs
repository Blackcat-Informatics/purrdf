// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// The pre-binding lane is deprecated and inert; these tests still name it.
#![allow(deprecated)]

//! Observable compiler headers survive topology and physical driver changes.

mod support;

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDatasetBuilder, SparqlResult};
use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser, Variable};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{
    GovernedOutcome, InternedOutcome, NativeSparqlEngine, PartialAnswers, PreparedQuery,
    QueryGovernors, QueryOptions, ShaclPrebinding,
};
use support::{iri, local_dataset, render_cell, sorted_rows};

const TEXT: &str = "PREFIX ex: <http://example.org/> SELECT * WHERE { \
    ?s ex:p ?a . ?x ex:t ?z . { ?s ex:q ?b } UNION { ?s ex:r ?b } }";
const HEADER: [&str; 5] = ["s", "a", "x", "z", "b"];

fn projectless(text: &str) -> Query {
    let mut query = SparqlParser::new().parse_query(text).unwrap();
    let Query::Select { pattern, .. } = &mut query else {
        panic!("SELECT");
    };
    let GraphPattern::Project { inner, .. } = std::mem::replace(pattern, GraphPattern::empty_bgp())
    else {
        panic!("projection");
    };
    *pattern = inner.into_inner();
    query
}

fn header(result: &SparqlResult) -> &[String] {
    let SparqlResult::Solutions { variables, .. } = result else {
        panic!("solutions");
    };
    variables
}

#[test]
fn compiler_and_rewritten_plans_preserve_source_columns_and_duplicate_bags() {
    let data = local_dataset([
        ("s", "p", "a"),
        ("s", "q", "b"),
        ("s", "r", "b"),
        ("x1", "t", "z1"),
        ("x2", "t", "z2"),
    ]);
    let engine = NativeSparqlEngine::new();
    let typed = engine
        .prepare_algebra(projectless(TEXT), QueryOptions::EMPTY)
        .unwrap();
    let rewritten = PreparedQuery::rewritten(projectless(TEXT), QueryOptions::EMPTY).unwrap();
    assert_ne!(
        typed.query(),
        &projectless(TEXT),
        "the fixture must exercise topology repair"
    );
    let Query::Select { pattern, .. } = typed.query() else {
        panic!("SELECT");
    };
    assert!(
        !matches!(pattern, GraphPattern::Project { .. }),
        "layout is metadata, not a scope barrier"
    );
    let control = engine.prepare_query(TEXT, None).unwrap();
    let control = engine
        .query_prepared(&data, &control, &[], QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(header(&control), HEADER);
    assert_eq!(sorted_rows(&control, render_cell).len(), 4);
    for prepared in [&*typed, &rewritten] {
        for _ in 0..3 {
            let result = engine
                .query_prepared(&data, prepared, &[], QueryOptions::EMPTY)
                .unwrap();
            assert_eq!(header(&result), HEADER);
            assert_eq!(
                sorted_rows(&result, render_cell),
                sorted_rows(&control, render_cell)
            );
        }
        let result = engine
            .query_prepared(
                &data,
                prepared,
                &[("s".into(), iri("s")), ("extra".into(), iri("extra"))],
                QueryOptions::EMPTY,
            )
            .unwrap();
        assert_eq!(header(&result), ["s", "a", "x", "z", "b", "extra"]);
        assert_eq!(sorted_rows(&result, render_cell).len(), 4);
        let shacl = engine
            .query_prepared(
                &data,
                prepared,
                &[("s".into(), iri("s"))],
                QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied),
            )
            .unwrap();
        assert_eq!(header(&shacl), HEADER);
        assert_eq!(
            sorted_rows(&shacl, render_cell),
            sorted_rows(&control, render_cell)
        );
    }
}

#[test]
fn empty_and_governed_results_keep_the_compiler_header() {
    let data = local_dataset([
        ("s", "p", "a"),
        ("s", "q", "b"),
        ("s", "r", "b"),
        ("x", "t", "z"),
    ]);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_algebra(projectless(TEXT), QueryOptions::EMPTY)
        .unwrap();
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let result = engine
        .query_prepared(&empty, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(header(&result), HEADER);
    assert_eq!(sorted_rows(&result, render_cell), Vec::new());
    let mut checked_partial = false;
    for fuel in [0, 1, 2, 4, 8, 16, 32, 64, 128, 256] {
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED.with_fuel(fuel)));
        match engine
            .query_prepared_governed_in_operation(
                &*data,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &state,
            )
            .unwrap()
        {
            GovernedOutcome::Complete { result, .. } => assert_eq!(header(&result), HEADER),
            GovernedOutcome::BudgetExhausted(exhausted) => {
                if let PartialAnswers::Certain(partial) = &exhausted.partial {
                    assert_eq!(header(partial.result()), HEADER);
                    checked_partial = true;
                }
            }
        }
    }
    assert!(
        checked_partial,
        "the fixture must exercise the certified egress"
    );
}

#[test]
fn retained_source_header_is_charged_once_and_released_with_the_plan() {
    let engine = NativeSparqlEngine::new();
    let observer = engine.plan_memory_observer();
    let mut extra = String::new();
    for index in 0..12 {
        write!(extra, " ; ex:u{index} ?v{index}").unwrap();
    }
    let text = TEXT.replace("?x ex:t ?z .", &format!("?x ex:t ?z{extra} ."));
    let prepared = engine
        .prepare_algebra(projectless(&text), QueryOptions::EMPTY)
        .unwrap();
    let bytes = prepared.retained_size_bytes();
    assert!(bytes > prepared.query().retained_size_bytes());
    assert_eq!(observer.stats().live_bytes, bytes);
    let clone = Arc::clone(&prepared);
    assert_eq!(observer.stats().live_plans, 1);
    drop(prepared);
    assert_eq!(observer.stats().live_bytes, bytes);
    drop(clone);
    assert_eq!(observer.stats().live_bytes, 0);
    assert_eq!(observer.stats().live_plans, 0);
}

#[test]
fn parameterized_interned_egress_keeps_textual_projection_order() {
    let engine = NativeSparqlEngine::new();
    let data = local_dataset([
        ("s", "p", "a"),
        ("s", "q", "b"),
        ("s", "r", "b"),
        ("x", "t", "z"),
    ]);
    let mut execution = engine
        .prepare_execution(TEXT, None, &["s"], QueryOptions::EMPTY)
        .unwrap();
    execution.bind_named("s", iri("s")).unwrap();
    for _ in 0..3 {
        engine
            .execute(&mut execution, &*data, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("solutions");
                };
                assert_eq!(
                    rows.variables()
                        .iter()
                        .map(Variable::as_str)
                        .collect::<Vec<_>>(),
                    HEADER
                );
            })
            .unwrap();
    }
}
