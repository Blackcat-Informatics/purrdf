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

use crate::model::{Graph, ReifierRow, Term, TermKind, Triple3};
use crate::{FastMap, FastSet};

/// Indexes kept in step with one segment's `Graph::terms` and
/// `Graph::reifiers` as the fold appends to them.
#[derive(Default)]
pub(super) struct ReifierIndex {
    /// Reifier id → the triple terms whose components that id supplies, in
    /// term-id order: every triple term naming it through `rf`, and the
    /// implicit self-bound triple term (no `rf`, no `tt`) whose own id it is.
    anchors: FastMap<usize, Vec<usize>>,
    /// Reifier id → its FIRST recorded binding, as [`Graph::reifier`] answers.
    first: FastMap<usize, Triple3>,
    /// Every recorded row, so a repeated row is recorded once, as
    /// [`Graph::set_reifier`] does.
    rows: FastSet<ReifierRow>,
}

impl ReifierIndex {
    /// Note a term the fold has just appended as `term_id`.
    pub(super) fn observe_term(&mut self, term_id: usize, term: &Term) {
        if term.kind != TermKind::Triple {
            return;
        }
        let anchored_on = match (term.reifier, term.triple) {
            (Some(rid), _) => rid,
            (None, None) => term_id,
            (None, Some(_)) => return,
        };
        self.anchors.entry(anchored_on).or_default().push(term_id);
    }

    /// The triple terms anchored on `rid`.
    pub(super) fn anchors(&self, rid: usize) -> &[usize] {
        self.anchors.get(&rid).map_or(&[], Vec::as_slice)
    }

    /// The first binding recorded for `rid`.
    pub(super) fn first_binding(&self, rid: usize) -> Option<Triple3> {
        self.first.get(&rid).copied()
    }

    /// Record `row` in `graph` unless the identical row is already there.
    pub(super) fn record(&mut self, graph: &mut Graph, row: ReifierRow) {
        if self.rows.insert(row) {
            self.first.entry(row.0).or_insert(row.1);
            graph.reifiers.push(row);
        }
    }

    /// Would recording `(rid, triple)` let some quoted-triple term contain
    /// itself?
    ///
    /// The anchors are the terms whose components this row would supply: those
    /// naming `rid` through `rf`, and — because a self-bound triple term may
    /// leave `rf` implicit — the term whose own id IS `rid`. Miss an anchor and
    /// the loop is recorded rather than refused.
    pub(super) fn binding_is_recursive(&self, graph: &Graph, rid: usize, triple: Triple3) -> bool {
        let mut seen: FastSet<usize> = FastSet::default();
        self.anchors(rid).iter().any(|&anchor| {
            <[usize; 3]>::from(triple).into_iter().any(|component| {
                // Each probe starts from an empty set; clearing the hoisted
                // set reuses its table instead of allocating one per component.
                seen.clear();
                self.depends_on_anchor(graph, component, anchor, (rid, triple), &mut seen)
            })
        })
    }

    fn depends_on_anchor(
        &self,
        graph: &Graph,
        term_id: usize,
        anchor: usize,
        pending: (usize, Triple3),
        seen: &mut FastSet<usize>,
    ) -> bool {
        if term_id == anchor {
            return true;
        }
        if !seen.insert(term_id) {
            return false;
        }
        let Some(triple) = self.pending_binding(graph, term_id, pending) else {
            return false;
        };
        <[usize; 3]>::from(triple)
            .into_iter()
            .any(|component| self.depends_on_anchor(graph, component, anchor, pending, seen))
    }

    /// The `(s, p, o)` a triple TERM would resolve to once `pending` is
    /// recorded.
    ///
    /// This MUST mirror [`Graph::triple_of`] spelling for spelling. `triple_of`
    /// is the one place a folded quoted triple's components are resolved, and
    /// every walker of those components — the segment union, the canonical
    /// writer, every projection to text — recurses through it. A spelling this
    /// function fails to follow is a loop the fold admits and the first such
    /// walker then dies on.
    fn pending_binding(
        &self,
        graph: &Graph,
        term_id: usize,
        pending: (usize, Triple3),
    ) -> Option<Triple3> {
        let term = graph.terms.get(term_id)?;
        if term.kind != TermKind::Triple {
            return None;
        }
        // A self-describing triple term (`tt`) can only name already-introduced
        // components, so it is acyclic by construction and takes precedence.
        if let Some(triple) = term.triple {
            return Some(triple);
        }
        // Otherwise the components come from the statement layer, through the
        // reifier id the term names — or, when it names none, through its OWN
        // id (§7.1: a self-bound triple term may leave `rf` implicit). The row
        // being read is not recorded yet, so substitute it for the id it binds.
        let rid = term.reifier.unwrap_or(term_id);
        if rid == pending.0 {
            return Some(pending.1);
        }
        self.first_binding(rid)
    }
}
