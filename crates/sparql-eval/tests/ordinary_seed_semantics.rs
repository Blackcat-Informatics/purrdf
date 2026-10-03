// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordinary physical join seeds preserve operand scope and graph context.

mod support;

use purrdf_core::{RdfDatasetBuilder, ResourceDimension, SparqlRequest};
use purrdf_sparql_eval::{
    EvalOptions, NativeSparqlEngine, PartialAnswers, QueryGovernors, QueryOptions,
};
use support::{EX, render_cell, row, sorted_rows, two_integer_objects};

#[test]
fn ordinary_joins_preserve_expression_scope_and_subquery_modifiers() {
    let dataset = two_integer_objects();
    let left = "VALUES ?x { ex:s2 } BIND(1 AS ?mark)";
    let s1 = format!("<{EX}s1>");
    let s2 = format!("<{EX}s2>");
    let cases = [
        (
            "a nested filter cannot read an outer-only binding",
            "?x ?s ?v",
            "{ ?s ex:p ?v FILTER(?s = ?x) }",
            Vec::new(),
        ),
        (
            "a nested BIND cannot read an outer-only binding",
            "?x ?s ?v ?seen",
            "{ ?s ex:p ?v BIND(BOUND(?x) AS ?seen) }",
            vec![
                row(&[("x", &s2), ("s", &s1), ("v", "1"), ("seen", "false")]),
                row(&[("x", &s2), ("s", &s2), ("v", "2"), ("seen", "false")]),
            ],
        ),
        (
            "a seed cannot create an overlapping MINUS domain",
            "?x ?s ?v",
            "{ ?s ex:p ?v MINUS { VALUES ?x { ex:s2 } } }",
            vec![
                row(&[("x", &s2), ("s", &s1), ("v", "1")]),
                row(&[("x", &s2), ("s", &s2), ("v", "2")]),
            ],
        ),
        (
            "a subquery selects its global first row before the outer join",
            "?x ?v",
            "{ SELECT ?x ?v WHERE { ?x ex:p ?v } ORDER BY ?v LIMIT 1 }",
            Vec::new(),
        ),
        (
            "an ungrouped subquery aggregate counts the independent bag",
            "?x ?n",
            "{ SELECT (COUNT(*) AS ?n) WHERE { ?x ex:p ?v } }",
            vec![row(&[("x", &s2), ("n", "2")])],
        ),
        (
            "a subquery grouping expression cannot read an outer-only binding",
            "?x ?seen ?n",
            "{ SELECT ?seen (COUNT(*) AS ?n) WHERE { ?s ex:p ?v } \
             GROUP BY (BOUND(?x) AS ?seen) }",
            vec![row(&[("x", &s2), ("seen", "false"), ("n", "2")])],
        ),
    ];
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        for (label, projection, right, expected) in &cases {
            let query = format!("PREFIX ex: <{EX}> SELECT {projection} WHERE {{ {left} {right} }}");
            let result = engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                )
                .expect("ordinary boundary query returns an outcome")
                .into_complete()
                .expect("ordinary boundary query completes");
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(sorted_rows(&result, render_cell), expected, "{label}");
        }
    }
}

#[test]
fn every_seeded_optional_fuel_cut_is_a_subbag_without_fabricated_padding() {
    let dataset = two_integer_objects();
    let query = format!(
        "PREFIX ex: <{EX}> SELECT ?x ?v WHERE {{ \
         VALUES ?x {{ ex:s1 ex:s1 ex:missing }} BIND(1 AS ?mark) \
         OPTIONAL {{ ?x ex:p ?v }} }}"
    );
    let s1 = format!("<{EX}s1>");
    let missing = format!("<{EX}missing>");
    let mut expected = vec![
        row(&[("x", &s1), ("v", "1")]),
        row(&[("x", &s1), ("v", "1")]),
        row(&[("x", &missing), ("v", "UNBOUND")]),
    ];
    expected.sort();
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        let request = SparqlRequest {
            query: &query,
            base_iri: None,
            substitutions: &[],
        };
        let measured = engine
            .query_governed(
                &dataset,
                request,
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("measure tiny ordinary OPTIONAL");
        let fuel = measured.evidence().consumed_in(ResourceDimension::Fuel);
        assert_eq!(
            sorted_rows(
                &measured
                    .into_complete()
                    .expect("unlimited OPTIONAL completes"),
                render_cell,
            ),
            expected,
        );
        for allowance in 0..=fuel {
            let outcome = engine
                .query_governed(
                    &dataset,
                    request,
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED.with_fuel(allowance),
                )
                .expect("fuel cut returns an outcome");
            let actual = if let Some(exhausted) = outcome.exhausted() {
                let PartialAnswers::Certain(partial) = &exhausted.partial else {
                    panic!("fuel {allowance}: OPTIONAL must retain a certain lower bag");
                };
                sorted_rows(partial.result(), render_cell)
            } else {
                sorted_rows(
                    &outcome.into_complete().expect("uncut OPTIONAL completes"),
                    render_cell,
                )
            };
            let mut remaining = expected.clone();
            for answer in actual {
                let occurrence = remaining
                    .iter()
                    .position(|candidate| *candidate == answer)
                    .unwrap_or_else(|| {
                        panic!("fuel {allowance}: fabricated or repeated row {answer:?}")
                    });
                remaining.remove(occurrence);
            }
        }
    }
}

#[test]
fn graph_boundaries_preserve_outer_compatibility_and_restore_the_active_graph() {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for index in 1..=2 {
        let graph = builder.intern_iri(&format!("{EX}g{index}"));
        let subject = builder.intern_iri(&format!("{EX}s{index}"));
        let value = builder.intern_iri(&format!("{EX}v{index}"));
        builder.push_quad(subject, p, value, Some(graph));
        let pick = builder.intern_iri(&format!("{EX}pick{index}"));
        builder.push_quad(
            subject,
            q,
            pick,
            if index == 1 { Some(graph) } else { None },
        );
    }
    let dataset = builder.freeze().expect("graph boundary fixture freezes");
    let [g2, s2, v2, pick2, missing] =
        ["g2", "s2", "v2", "pick2", "missing"].map(|local| format!("<{EX}{local}>"));
    let cases = [
        (
            "an outer graph binding restricts the independent GRAPH bag",
            "SELECT ?g ?s ?v WHERE { VALUES ?g { ex:g2 } BIND(1 AS ?mark) \
             GRAPH ?g { ?s ex:p ?v } }",
            vec![row(&[("g", &g2), ("s", &s2), ("v", &v2)])],
        ),
        (
            "OPTIONAL cannot replace an unmatched outer graph binding",
            "SELECT ?g ?s ?v WHERE { VALUES ?g { ex:g2 ex:missing } BIND(1 AS ?mark) \
             OPTIONAL { GRAPH ?g { ?s ex:p ?v } } }",
            vec![
                row(&[("g", &g2), ("s", &s2), ("v", &v2)]),
                row(&[("g", &missing), ("s", "UNBOUND"), ("v", "UNBOUND")]),
            ],
        ),
        (
            "a GRAPH filter cannot read an outer-only binding",
            "SELECT ?x ?g ?s ?v WHERE { VALUES ?x { ex:s2 } BIND(1 AS ?mark) \
             GRAPH ?g { ?s ex:p ?v FILTER(?s = ?x) } }",
            Vec::new(),
        ),
        (
            "a seeded pattern after GRAPH and BIND reads the default graph",
            "SELECT ?g ?s ?v ?picked WHERE { GRAPH ?g { ?s ex:p ?v } \
             BIND(?s AS ?copy) ?copy ex:q ?picked }",
            vec![row(&[
                ("g", &g2),
                ("s", &s2),
                ("v", &v2),
                ("picked", &pick2),
            ])],
        ),
    ];
    for (label, body, mut expected) in cases {
        let result = support::run_prefixed(&dataset, &format!("PREFIX ex: <{EX}> "), body);
        expected.sort();
        assert_eq!(sorted_rows(&result, render_cell), expected, "{label}");
    }
}
