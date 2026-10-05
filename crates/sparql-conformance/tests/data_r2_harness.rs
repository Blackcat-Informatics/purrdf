// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native failing-first coverage of the older corpus's actual result carriers.

use purrdf_sparql_conformance::{compare, manifest, run};
use std::path::Path;

fn case(group: &str, local: &str) -> manifest::SparqlTestCase {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("suite/w3c-sparql10")
        .join(group)
        .join("manifest.ttl");
    manifest::load(&path)
        .expect("frozen manifest")
        .into_iter()
        .find(|case| case.iri.ends_with(&format!("#{local}")))
        .expect("exact frozen case")
}

fn passes(group: &str, local: &str) {
    let case = case(group, local);
    let outcome = run::run(&case, None).expect("native evaluation");
    compare::compare(&case, &outcome).unwrap_or_else(|error| panic!("{group}/{local}: {error}"));
}

#[test]
fn turtle_boolean_results_grade_true_and_false_ask_answers() {
    passes("type-promotion", "type-promotion-01");
    passes("type-promotion", "type-promotion-23");
}

#[test]
fn rdf_xml_solutions_and_explicit_indices_grade_the_frozen_order() {
    passes("sort", "dawg-sort-1");
    passes("sort", "dawg-sort-builtin");
    passes("sort", "dawg-sort-function");
    passes("solution-seq", "offset-1");
}

#[test]
fn the_manifest_lax_cardinality_grades_both_reduced_cases() {
    passes("reduced", "reduced-1");
    passes("reduced", "reduced-2");
}

#[test]
fn query_dataset_sources_are_loaded_under_their_declared_iris() {
    for local in ["01", "03", "05", "06", "07", "08", "11", "12b"] {
        passes("dataset", &format!("dawg-dataset-{local}"));
    }
}

#[test]
fn named_graph_blank_scopes_survive_the_global_result_comparison() {
    passes("graph", "dawg-graph-11");
}
