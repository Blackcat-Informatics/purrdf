// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fold's view of the statement layer, indexed by reifier id.
//!
//! A `reifies` row asks three questions of everything folded before it: which
//! triple terms take their components from this reifier id (its anchors), what
//! the id's first binding is, and whether the identical row is already
//! recorded. Answering any of them by scanning `Graph::terms` or
//! `Graph::reifiers` makes a fold quadratic in its rows, so the fold keeps this
//! index beside the graph, fed by the same pushes, and each row costs what its
//! own anchors cost.

use super::term_decode::TermReferences;
use crate::model::{Graph, ReifierRow, Term, Triple3};
use crate::{FastMap, FastSet};

/// Indexes kept in step with one segment's terms and accepted bindings.
/// Full row membership is retained only when constructing a graph.
#[derive(Default)]
pub(super) struct ReifierIndex {
    /// Reifier id → the triple terms naming it through `rf`, in term-id order.
    /// A triple term with no `rf` is anchored on nothing.
    anchors: FastMap<usize, Vec<usize>>,
    anchor_entries: usize,
    /// Reifier id → its FIRST recorded binding, as [`Graph::reifier`] answers.
    first: FastMap<usize, Triple3>,
    /// Every materialized row, so a repeated row is recorded once, as
    /// [`Graph::set_reifier`] does. Streaming reads leave this empty.
    rows: FastSet<ReifierRow>,
}

impl ReifierIndex {
    pub(super) fn retained_entries(&self) -> usize {
        self.anchors
            .len()
            .saturating_add(self.anchor_entries)
            .saturating_add(self.first.len())
            .saturating_add(self.rows.len())
    }

    /// Note a term the fold has just appended as `term_id`.
    pub(super) fn observe_term(&mut self, term_id: usize, term: &Term) {
        self.observe_references(term_id, TermReferences::from_term(term));
    }

    pub(super) fn observe_references(&mut self, term_id: usize, references: TermReferences) {
        let Some(anchored_on) = references.anchor() else {
            return;
        };
        self.anchors.entry(anchored_on).or_default().push(term_id);
        self.anchor_entries = self.anchor_entries.saturating_add(1);
    }

    /// The triple terms anchored on `rid`.
    pub(super) fn anchors(&self, rid: usize) -> &[usize] {
        self.anchors.get(&rid).map_or(&[], Vec::as_slice)
    }

    /// The first binding recorded for `rid`.
    pub(super) fn first_binding(&self, rid: usize) -> Option<Triple3> {
        self.first.get(&rid).copied()
    }

    /// Keep the first binding for validation. Only a materializing read keeps
    /// every distinct statement row; sink callbacks already carry those rows.
    pub(super) fn record(&mut self, graph: &mut Graph, row: ReifierRow, materialize: bool) {
        if !materialize || self.rows.insert(row) {
            self.first.entry(row.0).or_insert(row.1);
            if materialize {
                graph.reifiers.push(row);
            }
        }
    }
}
