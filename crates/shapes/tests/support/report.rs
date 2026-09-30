// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading a validation outcome the way the SHACL integration tests compare it:
//! a prepared validation's report as canonical N-Triples, a report's focus nodes,
//! and a data graph merged with the triples a rule run derived.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::Arc;

use purrdf::{RdfDataset, RdfDatasetBuilder};
use purrdf_shapes::engine::PreparedShapes;
use purrdf_shapes::report::ValidationReport;

/// Validate `data` with `prepared` and render the report's canonical N-Triples.
///
/// The report's RDF form is the comparison surface rather than a field-by-field
/// walk: two restores are equal exactly when the graphs they produce are the same
/// bytes.
pub fn report_nt(prepared: &PreparedShapes, data: &Arc<RdfDataset>) -> String {
    prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("binding the data graph")
        .validate()
        .expect("validation runs")
        .to_ntriples()
}

/// Every result's focus node, rendered, sorted (duplicates kept).
pub fn focus_nodes(report: &ValidationReport) -> Vec<String> {
    let mut nodes: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    nodes.sort();
    nodes
}

/// `base` with `derived` pushed after it, frozen into one dataset.
pub fn merge(base: &RdfDataset, derived: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base);
    builder.push_dataset(derived);
    builder.freeze().map_err(|e| e.to_string())
}
