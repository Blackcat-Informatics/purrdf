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
use crate::dictionary::DictionaryError;
use crate::model::{Graph, ReifierRow, Term, TermKind, Triple3};
use crate::term_table::TermMetadata;
use crate::{FastMap, FastSet};

/// Indexes kept in step with one segment's terms and accepted bindings.
/// Full row membership is retained only when constructing a graph.
#[derive(Default)]
pub(super) struct ReifierIndex {
    /// Reifier id → the triple terms whose components that id supplies, in
    /// term-id order: every triple term naming it through `rf`, and the
    /// implicit self-bound triple term (no `rf`, no `tt`) whose own id it is.
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
        let Some(anchored_on) = references.anchor(term_id) else {
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

    /// Would recording `(rid, triple)` let some quoted-triple term contain
    /// itself?
    ///
    /// The anchors are the terms whose components this row would supply: those
    /// naming `rid` through `rf`, and — because a self-bound triple term may
    /// leave `rf` implicit — the term whose own id IS `rid`. Miss an anchor and
    /// the loop is recorded rather than refused.
    pub(super) fn binding_is_recursive(
        &self,
        terms: impl TermMetadata,
        rid: usize,
        triple: Triple3,
    ) -> Result<bool, DictionaryError> {
        let mut seen: FastSet<usize> = FastSet::default();
        for &anchor in self.anchors(rid) {
            for component in <[usize; 3]>::from(triple) {
                // Each probe starts from an empty set; clearing the hoisted
                // set reuses its table instead of allocating one per component.
                seen.clear();
                if self.depends_on_anchor(terms, component, anchor, (rid, triple), &mut seen)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn depends_on_anchor(
        &self,
        terms: impl TermMetadata,
        term_id: usize,
        anchor: usize,
        pending: (usize, Triple3),
        seen: &mut FastSet<usize>,
    ) -> Result<bool, DictionaryError> {
        let mut work = purrdf_lex::walk::WorkList::<usize, 16>::new();
        work.push(term_id);
        while let Some(term_id) = work.pop() {
            if term_id == anchor {
                return Ok(true);
            }
            if !seen.insert(term_id) {
                continue;
            }
            if let Some(limit) = terms.traversal_limit() {
                let required = seen.len().saturating_add(work.len()).saturating_add(3);
                if required > limit {
                    return Err(DictionaryError::Limit {
                        resource: "reifier-traversal-entries",
                        required,
                        limit,
                    });
                }
            }
            if let Some((s, p, o)) = self.pending_binding(terms, term_id, pending)? {
                work.extend([o, p, s]);
            }
        }
        Ok(false)
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
        terms: impl TermMetadata,
        term_id: usize,
        pending: (usize, Triple3),
    ) -> Result<Option<Triple3>, DictionaryError> {
        let Some(term) = terms.references(term_id)? else {
            return Ok(None);
        };
        if term.kind != TermKind::Triple {
            return Ok(None);
        }
        // A self-describing triple term (`tt`) can only name already-introduced
        // components, so it is acyclic by construction and takes precedence.
        if let Some(triple) = term.triple {
            return Ok(Some(triple));
        }
        // Otherwise the components come from the statement layer, through the
        // reifier id the term names — or, when it names none, through its OWN
        // id (§7.1: a self-bound triple term may leave `rf` implicit). The row
        // being read is not recorded yet, so substitute it for the id it binds.
        let rid = term.reifier.unwrap_or(term_id);
        if rid == pending.0 {
            return Ok(Some(pending.1));
        }
        Ok(self.first_binding(rid))
    }
}
