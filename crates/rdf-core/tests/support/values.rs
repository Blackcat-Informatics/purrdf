// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading a view's quads back as dataset-independent values, so two views that mint
//! unrelated id spaces (the reference `RdfDataset`, a `PackView`, a `PagedDataset`) can be
//! compared by value.

// Included by `#[path]` into several integration-test binaries, and no single binary uses
// every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_core::{DatasetView, GraphMatch, TermValue};

/// The dataset-independent value of the term `id` names in `v`: the view's own
/// [`DatasetView::term_value`], which a test reads through for every view it compares.
pub fn to_value<V: DatasetView>(v: &V, id: V::Id) -> TermValue {
    v.term_value(id)
        .expect("an id the view minted resolves to a value")
}

/// A deterministic sort key for a value row (`TermValue` is not `Ord`; its `Debug` form
/// is total and dataset-independent).
pub fn row_key(row: &[TermValue]) -> String {
    format!("{row:?}")
}

/// Every quad of the view as sorted `[s, p, o, g]` value rows, through the generic trait
/// surface only. The default graph is rendered as the IRI `urn:default-graph`, so
/// named-graph quads stay distinguishable.
pub fn collect_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .map(|q| {
            let mut row = vec![to_value(v, q.s), to_value(v, q.p), to_value(v, q.o)];
            row.push(q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)));
            row
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}
