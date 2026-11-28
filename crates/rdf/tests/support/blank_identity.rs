// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Blank-node identity, read straight off the IR, for the blank-label and
//! composite-view tests.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_rdf::{CompositeDatasetView, CompositeSource, RdfDataset, TermRef, ViewLimits};

/// The distinct blank `(label, scope)` pairs among a dataset's quad subjects
/// and objects, read straight off the IR (never through the owned rendering,
/// which would re-encode them).
pub fn blank_nodes(dataset: &RdfDataset) -> BTreeSet<(String, u32)> {
    let mut blanks = BTreeSet::new();
    for quad in dataset.quads() {
        for id in [quad.s, quad.o] {
            if let TermRef::Blank { label, scope } = dataset.resolve(id) {
                blanks.insert((label.to_owned(), scope.ordinal()));
            }
        }
    }
    blanks
}

/// A single-source composite view over `dataset`, in an explicitly SHARED blank
/// identity space so the view reports the source's own `(label, scope)` pairs
/// unchanged.
pub fn composite_over(dataset: &Arc<RdfDataset>) -> CompositeDatasetView {
    CompositeDatasetView::from_shared_sources(
        vec![CompositeSource::new(Arc::clone(dataset))],
        ViewLimits::default(),
    )
    .expect("a single retained source composes")
}
