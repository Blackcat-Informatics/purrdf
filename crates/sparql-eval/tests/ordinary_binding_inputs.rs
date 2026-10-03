// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical relation seeds preserve bags and the interner's complete term identity.

mod support;

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlRequest};
use purrdf_sparql_eval::{EvalOptions, NativeSparqlEngine, QueryGovernors, QueryOptions};
use support::{EX, render_cell, row, sorted_rows};

#[test]
fn duplicate_nullable_and_absent_drivers_keep_their_exact_join_bags() {
    let mut builder = RdfDatasetBuilder::new();
    let [a, b, p] = ["a", "b", "p"].map(|local| builder.intern_iri(&format!("{EX}{local}")));
    for (subject, value) in [(a, "1"), (a, "2"), (b, "3")] {
        let object =
            builder.intern_literal(RdfLiteral::typed(value, purrdf_xsd::datatype::XSD_INTEGER));
        builder.push_quad(subject, p, object, None);
    }
    let dataset = builder.freeze().expect("nullable bag fixture");
    let a = format!("<{EX}a>");
    let b = format!("<{EX}b>");
    let missing = format!("<{EX}missing>");
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        for optional in [false, true] {
            let body = if optional {
                "OPTIONAL { ?x ex:p ?v }"
            } else {
                "?x ex:p ?v"
            };
            let query = format!(
                "PREFIX ex: <{EX}> SELECT ?x ?v ?mark WHERE {{ \
                 VALUES ?x {{ ex:a ex:a ex:missing UNDEF }} BIND(\"fresh\" AS ?mark) {body} }}"
            );
            let outcome = engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED.with_max_intermediate_cells(128),
                )
                .expect("native bag outcome");
            let result = outcome.into_complete().expect("bounded bag completes");
            let mut expected = Vec::new();
            for _ in 0..3 {
                for value in ["1", "2"] {
                    expected.push(row(&[("x", &a), ("v", value), ("mark", "fresh")]));
                }
            }
            expected.push(row(&[("x", &b), ("v", "3"), ("mark", "fresh")]));
            if optional {
                expected.push(row(&[("x", &missing), ("v", "UNBOUND"), ("mark", "fresh")]));
            }
            expected.sort();
            assert_eq!(
                sorted_rows(&result, render_cell),
                expected,
                "optional={optional}"
            );
        }
    }
}

#[test]
fn computed_existing_missing_and_quoted_slots_use_dataset_identity_before_scanning() {
    let mut builder = RdfDatasetBuilder::new();
    let [a, p, q, o, record, has, noise] = ["a", "p", "q", "o", "record", "has", "noise"]
        .map(|local| builder.intern_iri(&format!("{EX}{local}")));
    let value = builder.intern_literal(RdfLiteral::typed("1", purrdf_xsd::datatype::XSD_INTEGER));
    builder.push_quad(a, p, value, None);
    let quote = builder.intern_triple(a, q, o);
    builder.push_quad(record, has, quote, None);
    for index in 0..5_000 {
        let subject = builder.intern_iri(&format!("{EX}noise{index}"));
        builder.push_quad(subject, noise, o, None);
    }
    let dataset = builder.freeze().expect("term identity fixture");
    let cases = [
        (
            "?x ?v",
            "VALUES ?name { \"a\" \"missing\" } \
             BIND(IRI(CONCAT(\"http://example.org/\", ?name)) AS ?x) \
             BIND(\"fresh\" AS ?mark) ?x ex:p ?v",
            row(&[("x", &format!("<{EX}a>")), ("v", "1")]),
        ),
        (
            "?record",
            "VALUES ?x { <<( ex:a ex:q ex:o )>> <<( ex:missing ex:q ex:o )>> } \
             BIND(\"fresh\" AS ?mark) ?record ex:has ?x",
            row(&[("record", &format!("<{EX}record>"))]),
        ),
        (
            "?record",
            "VALUES ?s { ex:a ex:missing } BIND(\"fresh\" AS ?mark) \
             ?record ex:has <<( ?s ex:q ex:o )>>",
            row(&[("record", &format!("<{EX}record>"))]),
        ),
    ];
    for (projection, body, expected) in cases {
        let query = format!("PREFIX ex: <{EX}> SELECT {projection} WHERE {{ {body} }}");
        let outcome = NativeSparqlEngine::new()
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(128),
            )
            .expect("term identity outcome");
        assert!(
            outcome.evidence().consumed_in(ResourceDimension::Fuel) < 128,
            "{query}: {outcome:?}"
        );
        let result = outcome
            .into_complete()
            .expect("bounded term identity relation");
        assert_eq!(sorted_rows(&result, render_cell), vec![expected], "{query}");
    }
}
