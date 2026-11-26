// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free subject seeks over resident tables and warm exact index runs.

use std::cmp::Ordering;

use crate::{GraphMatch, StatisticsStream, SubjectOrderedScan};

use super::super::triple::{masked_ordinal_range, ordinal_range};
use super::{AnnotationRow, OrdinalRows, QuadIds, QuadRow, RdfDataset, ReifierRow, TermId};

enum Rows<'a> {
    Asserted(OrdinalRows<'a, QuadRow>),
    Reifiers(OrdinalRows<'a, ReifierRow>, TermId),
    Annotations(OrdinalRows<'a, AnnotationRow>),
    Empty,
}

impl Rows<'_> {
    fn last_subject(&self) -> Option<TermId> {
        match self {
            Self::Asserted(rows) => rows.last_row().map(|last| last.s),
            Self::Reifiers(rows, _) => rows.last_row().map(|last| last.0),
            Self::Annotations(rows) => rows.last_row().map(|last| last.0),
            Self::Empty => None,
        }
    }

    fn keep_through(&mut self, last: TermId) {
        match self {
            Self::Asserted(rows) => rows.keep_prefix(|row| row.s <= last),
            Self::Reifiers(rows, _) => rows.keep_prefix(|row| row.0 <= last),
            Self::Annotations(rows) => rows.keep_prefix(|row| row.0 <= last),
            Self::Empty => {}
        }
    }
}

struct SubjectRows<'a> {
    rows: Rows<'a>,
    graph: GraphMatch,
}

impl Iterator for SubjectRows<'_> {
    type Item = QuadIds;

    fn next(&mut self) -> Option<QuadIds> {
        let graph = self.graph;
        match &mut self.rows {
            Rows::Asserted(rows) => rows.find(|row| graph.matches(row.g)).map(QuadIds::from),
            Rows::Reifiers(rows, predicate) => {
                rows.find(|row| graph.matches(row.2))
                    .map(|(s, o, g)| QuadIds {
                        s,
                        p: *predicate,
                        o,
                        g,
                    })
            }
            Rows::Annotations(rows) => rows
                .find(|row| graph.matches(row.3))
                .map(|(s, p, o, g)| QuadIds { s, p, o, g }),
            Rows::Empty => None,
        }
    }
}

impl SubjectOrderedScan<TermId> for SubjectRows<'_> {
    fn advance_to(&mut self, minimum: TermId) {
        match &mut self.rows {
            Rows::Asserted(rows) => rows.skip_prefix(|row| row.s < minimum),
            Rows::Reifiers(rows, _) => rows.skip_prefix(|row| row.0 < minimum),
            Rows::Annotations(rows) => rows.skip_prefix(|row| row.0 < minimum),
            Rows::Empty => {}
        }
    }
}

const STREAMS: [StatisticsStream; 3] = [
    StatisticsStream::Asserted,
    StatisticsStream::Reifiers,
    StatisticsStream::Annotations,
];

impl RdfDataset {
    fn asserted_subject_rows(
        &self,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> Option<Rows<'_>> {
        if self.quads.is_empty() {
            return Some(Rows::Empty);
        }
        if predicate.is_none() && object.is_none() {
            return Some(Rows::Asserted(ordinal_range(&self.quads, None, |_| {
                Ordering::Equal
            })));
        }
        // Borrow the same exact prefix as the ordinary graph-bound or
        // graph-unbound probe. A selective graph must not scan other graphs.
        // Both permutations order subjects within that fixed prefix.
        let predicate = predicate?;
        let object = object?;
        let ordinals = match graph {
            GraphMatch::Any => self.indexes.pos.get()?,
            GraphMatch::Default | GraphMatch::Named(_) => self.indexes.gpos.get()?,
        };
        Some(Rows::Asserted(ordinal_range(
            &self.quads,
            Some(ordinals),
            |row| {
                match graph {
                    GraphMatch::Any => Ordering::Equal,
                    GraphMatch::Default => row.g.cmp(&None),
                    GraphMatch::Named(graph) => row.g.cmp(&Some(graph)),
                }
                .then_with(|| (row.p, row.o).cmp(&(predicate, object)))
            },
        )))
    }

    fn subject_rows(
        &self,
        stream: StatisticsStream,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> Option<Rows<'_>> {
        match stream {
            StatisticsStream::Asserted => self.asserted_subject_rows(predicate, object, graph),
            StatisticsStream::Reifiers => {
                if self.reifiers.is_empty() {
                    return Some(Rows::Empty);
                }
                self.assert_reifier_table_invariant();
                let reifies = self.rdf_reifies_id()?;
                if predicate.is_some_and(|predicate| predicate != reifies) {
                    return Some(Rows::Empty);
                }
                let rows = if let Some(object) = object {
                    let ordinals = self.reifier_targets.get()?;
                    ordinal_range(&self.reifiers, Some(ordinals), |row| row.1.cmp(&object))
                } else {
                    ordinal_range(&self.reifiers, None, |_| Ordering::Equal)
                };
                Some(Rows::Reifiers(rows, reifies))
            }
            StatisticsStream::Annotations => {
                if self.annotations.is_empty() {
                    return Some(Rows::Empty);
                }
                let pattern = [None, predicate, object];
                let mask = super::component_mask(pattern) >> 1;
                let ordinals = if let Some(index) = mask.checked_sub(1) {
                    Some(self.annotation_orders[index].get()?.as_ref())
                } else {
                    None
                };
                Some(Rows::Annotations(masked_ordinal_range(
                    &self.annotations,
                    ordinals,
                    pattern,
                    super::annotation_components,
                )))
            }
        }
    }

    pub(crate) fn native_subject_order_available(
        &self,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> bool {
        // Both non-subject terms must be fixed so reordering equal-subject
        // products cannot reorder distinct bindings. A single stream also
        // preserves the indexed path's subject order on a cold source. The
        // indexed path appends virtual streams, which is observable in
        // order-sensitive aggregates even without a SELECT order clause.
        if predicate.is_none() || object.is_none() {
            return false;
        }
        let mut seen = false;
        for stream in STREAMS {
            let Some(rows) = self.subject_rows(stream, predicate, object, graph) else {
                return false;
            };
            if rows.last_subject().is_some() && std::mem::replace(&mut seen, true) {
                return false;
            }
        }
        true
    }

    pub(crate) fn native_subject_upper_bound(
        &self,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> Option<TermId> {
        STREAMS.into_iter().try_fold(None, |upper, stream| {
            Some(
                upper.max(
                    self.subject_rows(stream, predicate, object, graph)?
                        .last_subject(),
                ),
            )
        })?
    }

    pub(crate) fn native_ordered_subject_quads(
        &self,
        stream: StatisticsStream,
        first: Option<TermId>,
        last: Option<TermId>,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> Option<impl SubjectOrderedScan<TermId> + '_> {
        let mut rows = SubjectRows {
            rows: self.subject_rows(stream, predicate, object, graph)?,
            graph,
        };
        if let Some(first) = first {
            rows.advance_to(first);
        }
        if let Some(last) = last {
            rows.rows.keep_through(last);
        }
        Some(rows)
    }
}
