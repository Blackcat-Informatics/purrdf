// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Turtle fixtures for the SHACL integration tests: a shapes graph or a data graph
//! read from a snippet under the including test's own `@prefix` header.
//!
//! Every helper takes that header explicitly. Each test binary declares its own
//! `PREFIXES`, and the prefix set a snippet is read under is part of what the test
//! states, so the helpers never supply one.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::expression::NodeExpr;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::{Constraint, Shapes};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

/// The shapes graph `prefixes` + `shapes_ttl` reads as, or the load error's text.
pub fn load(prefixes: &str, shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{prefixes}{shapes_ttl}"), None).map_err(String::from)
}

/// The shapes graph `prefixes` + `shapes_ttl` reads as; a load error fails the test.
#[track_caller]
pub fn loads(prefixes: &str, shapes_ttl: &str) -> Shapes {
    load(prefixes, shapes_ttl).unwrap_or_else(|error| panic!("the shapes graph must load: {error}"))
}

/// [`loads`] for the valid neighbour of a refused shapes graph: the loading
/// counterpart a refusal test pairs with each [`refused`].
#[track_caller]
pub fn loads_neighbour(prefixes: &str, shapes_ttl: &str) -> Shapes {
    load(prefixes, shapes_ttl)
        .unwrap_or_else(|error| panic!("the valid neighbour must load: {error}"))
}

/// Require the shapes graph `prefixes` + `shapes_ttl` to be refused at load with
/// an error that mentions `needle`.
#[track_caller]
pub fn refused(prefixes: &str, shapes_ttl: &str, needle: &str) {
    let error = load(prefixes, shapes_ttl).expect_err("the shapes graph must be refused at load");
    assert!(
        error.contains(needle),
        "refusal must mention {needle:?}: {error}"
    );
}

/// The data graph `prefixes` + `data_ttl` reads as.
pub fn data(prefixes: &str, data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{prefixes}{data_ttl}"), None).expect("data parses")
}

/// Validate the data graph `data_ttl` against the shapes graph `shapes_ttl`, both
/// read under `prefixes`.
#[track_caller]
pub fn validate(prefixes: &str, shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    validate_dataset_with_shapes_graph(
        &data(prefixes, data_ttl),
        &loads(prefixes, shapes_ttl),
        None,
    )
    .expect("validation runs")
}

/// The one `sh:expression` the shapes graph `prefixes` + `shapes_ttl` declares;
/// a fixture that declares none or several fails the test.
pub fn expression_of(prefixes: &str, shapes_ttl: &str) -> NodeExpr {
    let shapes = parse_shapes(&format!("{prefixes}{shapes_ttl}"), None).expect("shapes parse");
    let mut found: Vec<NodeExpr> = shapes
        .node_shapes
        .iter()
        .flat_map(|shape| &shape.constraints)
        .filter_map(|c| match c {
            Constraint::Expression { expr, .. } => Some(expr.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "the fixture must declare exactly one sh:expression"
    );
    found.remove(0)
}

/// The rendered error the shapes graph `prefixes` + `shapes_ttl` is refused
/// with at shapes-load; a graph that loads fails the test.
pub fn load_error(prefixes: &str, shapes_ttl: &str) -> String {
    parse_shapes(&format!("{prefixes}{shapes_ttl}"), None)
        .expect_err("the fixture must be refused at shapes-load")
        .to_string()
}

/// The shapes-graph lint report of the Turtle document `prefixes` + `shapes_ttl`,
/// read with its document prefix map and no imports.
pub fn lint_of(prefixes: &str, shapes_ttl: &str) -> purrdf_shapes::lint::LintReport {
    let document =
        purrdf_shapes::text_ingest::parse_turtle_document(&format!("{prefixes}{shapes_ttl}"), None)
            .expect("parses");
    purrdf_shapes::lint::lint(
        &document.dataset,
        &document.prefixes,
        None,
        None,
        &purrdf_shapes::ShapesImports::new(),
    )
    .expect("lint runs")
}
