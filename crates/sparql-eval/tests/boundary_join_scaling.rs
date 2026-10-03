// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Full-scale ordinary-join and OPTIONAL regressions under one cell allowance.

mod support;

use purrdf_core::{ResourceDimension, SparqlRequest};
use purrdf_sparql_eval::{EvalOptions, NativeSparqlEngine, QueryGovernors, QueryOptions};
use support::{EX, boundary_joins, render_cell, row, row_count, sorted_rows};

fn selective_patterns_across_boundaries_preserve_bags_with_bounded_work() {
    let dataset = boundary_joins::dataset();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(boundary_joins::CELLS);
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        for (label, original, equivalent, expected_rows) in boundary_joins::QUERIES {
            let mut results = Vec::new();
            let mut fuel = Vec::new();
            for body in [original, equivalent] {
                let text = boundary_joins::query(body);
                let outcome = engine
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
                    .expect("scaled query returns an outcome");
                assert!(outcome.is_complete(), "{label}: {outcome:?}");
                let evidence = outcome.evidence();
                assert!(
                    evidence.consumed_in(ResourceDimension::IntermediateCells)
                        <= boundary_joins::CELLS,
                    "{label}: {evidence:?}"
                );
                fuel.push(evidence.consumed_in(ResourceDimension::Fuel));
                let result = outcome.into_complete().expect("scaled query completes");
                assert_eq!(row_count(&result), expected_rows, "{label}");
                results.push(sorted_rows(&result, render_cell));
            }
            assert_eq!(results[0], results[1], "{label}: exact bag parity");
            let mut expected = match label {
                "after_optional" => (0..4_000)
                    .filter(|index| *index < 80 || index % 8 != 0)
                    .map(|index| row(&[("item", &format!("<{EX}item{index}>"))]))
                    .collect::<Vec<_>>(),
                "after_bind" => (0..4_000)
                    .map(|index| {
                        let item = format!("<{EX}item{index}>");
                        row(&[("item", &item), ("copy", &item)])
                    })
                    .collect(),
                "optional_body" => (0..10)
                    .flat_map(|anchor| {
                        (0..100).map(move |member| {
                            row(&[
                                ("a", &format!("<{EX}anchor{anchor}>")),
                                ("flag", if member % 2 == 0 { "false" } else { "true" }),
                            ])
                        })
                    })
                    .collect(),
                _ => unreachable!("the fixture names its three query forms"),
            };
            expected.sort();
            assert_eq!(results[0], expected, "{label}: independent expected bag");
            // A physical boundary can retain extra columns/driver rows, but must not
            // multiply unrelated dataset regions into millions of intermediates.
            assert!(fuel[0] <= fuel[1].saturating_mul(10), "{label}: {fuel:?}");
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("{label}: rows={expected_rows}, measured fuel={fuel:?}");
        }
    }
}

fn explain_measures_the_full_optional_join_without_a_cartesian_reservation() {
    let dataset = boundary_joins::dataset();
    let query = boundary_joins::query(boundary_joins::QUERIES[0].1);
    let explanation = NativeSparqlEngine::new()
        .explain_query(&dataset, &query, None)
        .expect("full-scale EXPLAIN returns a measured explanation");
    assert_eq!(explanation.evidence().tripped, None);
    assert_eq!(
        explanation.ledger().iter().fold(0_u64, |total, node| {
            total.saturating_add(node.fuel_total())
        }),
        explanation.evidence().consumed_in(ResourceDimension::Fuel),
        "the physical seed work remains attributed to the source plan",
    );
    assert!(explanation.evidence().consumed_in(ResourceDimension::Fuel) > 0);
    assert!(
        explanation
            .evidence()
            .consumed_in(ResourceDimension::IntermediateCells)
            < boundary_joins::CELLS
    );
}

fn negative_exists_probes_the_selected_anchor_without_an_independent_product() {
    let dataset = boundary_joins::dataset();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(boundary_joins::CELLS);
    for repeated_anchor in ["", "?a a ex:Anchor ."] {
        let text = boundary_joins::query(&format!(
            "SELECT ?a WHERE {{ ?a a ex:Anchor . FILTER NOT EXISTS {{ {repeated_anchor} \
             ?a ex:at ?n . ?m ex:at ?n . ?m ex:of ?dev . ?dev ex:flag true }} }}"
        ));
        let outcome = NativeSparqlEngine::new()
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
            .expect("negative EXISTS returns an outcome");
        assert!(outcome.is_complete(), "{outcome:?}");
        assert!(outcome.evidence().consumed_in(ResourceDimension::Fuel) < 100_000);
        assert_eq!(row_count(&outcome.into_complete().expect("complete")), 0);
    }
}

purrdf_testkit::harness_main!(
    selective_patterns_across_boundaries_preserve_bags_with_bounded_work,
    explain_measures_the_full_optional_join_without_a_cartesian_reservation,
    negative_exists_probes_the_selected_anchor_without_an_independent_product,
);
