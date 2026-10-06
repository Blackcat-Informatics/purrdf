// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Temporary correlated and ENF trees retain native positive execution.

mod support;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, ResourceDimension, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, PartialAnswers, QueryGovernors, QueryOptions};
use std::sync::Arc;
use support::{Row, render_cell, row, sorted_rows};

const EX: &str = "http://example.org/";
const VALUES: &str = "VALUES ?outer { ex:s0 ex:s1 ex:missing }";
const BODY: &str = "{ ?s ex:edgeA ?t } UNION { ?s ex:edgeB ?t } ?s ex:a ?a . ?t ex:b ?b .";
const CELLS: u64 = 64;

fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let [edge_a, edge_b, a, b] =
        ["edgeA", "edgeB", "a", "b"].map(|local| builder.intern_iri(&format!("{EX}{local}")));
    for index in 0..256 {
        let [s, t, av, bv] =
            ["s", "t", "a", "b"].map(|local| builder.intern_iri(&format!("{EX}{local}{index}")));
        builder.push_quad(s, a, av, None);
        builder.push_quad(t, b, bv, None);
        if index < 2 {
            builder.push_quad(s, if index == 0 { edge_a } else { edge_b }, t, None);
        }
    }
    builder.freeze().expect("fixture dataset")
}

fn expected() -> Vec<Row> {
    (0..2)
        .map(|index| {
            row(&[
                ("outer", &format!("<{EX}s{index}>")),
                ("s", &format!("<{EX}s{index}>")),
                ("a", &format!("<{EX}a{index}>")),
                ("b", &format!("<{EX}b{index}>")),
            ])
        })
        .collect()
}

fn lateral(body: &str) -> String {
    format!(
        "PREFIX ex: <{EX}> SELECT ?outer ?s ?a ?b WHERE {{ {VALUES} LATERAL {{ {body} FILTER(?s = ?outer) }} }}"
    )
}

#[test]
fn copied_lateral_union_keeps_native_bindings_and_source_columns() {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    let distributed = "{ ?s ex:edgeA ?t . ?s ex:a ?a . ?t ex:b ?b } UNION { ?s ex:edgeB ?t . ?s ex:a ?a . ?t ex:b ?b }";
    for body in [BODY, distributed] {
        let text = lateral(body);
        let result = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("native correlated schedule")
            .into_complete()
            .expect("complete bag");
        let SparqlResult::Solutions { variables, .. } = &result else {
            panic!("SELECT")
        };
        assert_eq!(variables, &["outer", "s", "a", "b"]);
        assert_eq!(sorted_rows(&result, render_cell), expected());
    }
}

#[test]
fn correlated_and_uncorrelated_exists_copies_preserve_their_row_scopes() {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    for (filter, names) in [
        ("", vec!["s0", "s1", "missing"]),
        ("FILTER(?s = ?outer)", vec!["s0", "s1"]),
    ] {
        let text = format!(
            "PREFIX ex: <{EX}> SELECT ?outer WHERE {{ {VALUES} FILTER EXISTS {{ {BODY} {filter} }} }}"
        );
        let result = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(CELLS),
            )
            .expect("native EXISTS schedule")
            .into_complete()
            .expect("complete bag");
        let mut expected = names
            .into_iter()
            .map(|name| row(&[("outer", &format!("<{EX}{name}>"))]))
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(sorted_rows(&result, render_cell), expected);
    }
}

#[test]
fn copied_positive_schedule_fuel_cuts_remain_complete_binding_lower_bounds() {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    let text = lateral(BODY);
    let run = |fuel| {
        engine
            .query_governed(
                &data,
                SparqlRequest {
                    query: &text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED
                    .with_max_intermediate_cells(CELLS)
                    .with_fuel(fuel),
            )
            .expect("governed correlated schedule")
    };
    let complete = run(u64::MAX);
    let spend = complete.evidence().consumed_in(ResourceDimension::Fuel);
    assert_eq!(
        sorted_rows(
            &complete.into_complete().expect("complete bag"),
            render_cell
        ),
        expected()
    );
    for fuel in 0..=spend {
        let outcome = run(fuel);
        let rows = if let Some(exhausted) = outcome.exhausted() {
            let PartialAnswers::Certain(partial) = &exhausted.partial else {
                panic!("fuel {fuel}: correlated cuts remain certain lower bounds");
            };
            sorted_rows(partial.result(), render_cell)
        } else {
            sorted_rows(
                &outcome.into_complete().expect("completed cut"),
                render_cell,
            )
        };
        let mut available = expected();
        for answer in rows {
            let at = available
                .iter()
                .position(|candidate| *candidate == answer)
                .unwrap_or_else(|| panic!("fuel {fuel}: {answer:?}"));
            available.remove(at);
        }
    }
}
