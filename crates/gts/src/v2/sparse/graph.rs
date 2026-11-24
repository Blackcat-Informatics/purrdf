// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::ops::Range;
use std::io;

use crate::v2::fields::push;
use crate::v2::rdf::dictionary::{ByReferenceTerm, ByReferenceTerms, RowScope};
use crate::v2::references::{
    EntryAdmission, FrameOwners, LocalTerms, Matches, SuppressionClaim, decode_suppressions,
};
use crate::v2::{
    FormedBudget, FrameType, RdfFold, Row, RowDisposition, RowHeader, SegmentDictionary, Term,
    TermId,
};

use super::claims::{GraphEntry, GraphRange};
use super::frame::LocalFrame;
use super::{Covered, PlacedIndex, QueryRead, RangeReason, Stop};

/// A row admitted by the dictionary at its own frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RangeRow {
    /// Item number placed by the segment's offsets list.
    pub position: u64,
    /// Zero-based encoded row index in that frame.
    pub index: u64,
    /// The typed RDF row, with IDs resolved through the answer's dictionary.
    pub row: Row,
    /// A term in the row depends on a value declared by reference.
    pub by_reference: bool,
    /// Suppression's visibility verdict, when the query requested it.
    pub visible: Option<bool>,
}

/// A term in the segment-local dictionary returned alongside sparse rows.
#[derive(Debug)]
pub enum RangeTerm<'a> {
    /// A complete RDF term.
    Resolved(&'a Term<'a>),
    /// A term that retains its typed value declaration.
    ByReference(&'a ByReferenceTerm),
}

/// Rows from the requested graph and their verified dictionary.
#[derive(Debug)]
#[non_exhaustive]
pub struct RangeGraph {
    /// Returned rows in physical item and row order.
    pub rows: Vec<RangeRow>,
    dictionary: Dictionary,
}

impl RangeGraph {
    /// Look up an encoded dictionary ID in the answer's selected segment.
    pub fn term(&self, id: u64) -> Option<RangeTerm<'_>> {
        self.dictionary.term(id)
    }

    /// Resolve an arena handle from a datatype or a resolved triple.
    pub fn resolved_term(&self, id: TermId) -> Option<&Term<'_>> {
        self.dictionary.arena.term(id)
    }
}

#[derive(Debug)]
struct Dictionary {
    arena: RdfFold,
    observed: SegmentDictionary,
    references: ByReferenceTerms,
    scopes: Vec<(usize, RowScope)>,
    formed: FormedBudget,
}

impl Dictionary {
    fn new(formed_bytes: u64) -> Self {
        Self {
            arena: RdfFold::default(),
            observed: SegmentDictionary::new(0),
            references: ByReferenceTerms::default(),
            scopes: Vec::new(),
            formed: FormedBudget::new(formed_bytes),
        }
    }

    fn terms(&mut self, item: usize, frame: &LocalFrame) -> Result<(), Stop> {
        let registry = core::num::NonZeroU64::MIN;
        let reservation =
            crate::v2::terms::reserve_verified(frame.content(), self.observed.next_id(), registry)
                .map_err(Stop::Structure)?;
        let header = reservation
            .content
            .as_ref()
            .and_then(|content| content.as_ref().ok())
            .ok_or(Stop::Unknown(RangeReason::Payload))?;
        let terms = header
            .decode(frame.payload(), &mut self.formed)
            .map_err(Stop::content_error)?;
        self.observed
            .observe_referenced_terms(
                &mut self.arena,
                &reservation,
                terms,
                &mut self.references,
                |_| true,
            )
            .map_err(Stop::fold_error)?;
        push(&mut self.scopes, (item, self.observed.row_scope()))?;
        Ok(())
    }

    fn scope(&self, item: usize) -> RowScope {
        let before = self.scopes.partition_point(|&(terms, _)| terms < item);
        if before == 0 {
            RowScope::EMPTY
        } else {
            self.scopes[before - 1].1
        }
    }

    fn term(&self, id: u64) -> Option<RangeTerm<'_>> {
        if let Some(reference) = self.references.get(id, self.observed.row_scope()) {
            return Some(RangeTerm::ByReference(reference));
        }
        self.arena
            .term(self.observed.readable_term(id)?)
            .map(RangeTerm::Resolved)
    }

    fn names_graph(&self, id: Option<u64>, target: Option<&str>) -> bool {
        match (id, target) {
            (None, None) => true,
            (Some(id), Some(target)) => matches!(
                self.term(id),
                Some(RangeTerm::Resolved(Term::Iri(text))) if text == target
            ),
            _ => false,
        }
    }
}

impl<R> QueryRead<'_, R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    pub(super) fn graph(
        &mut self,
        placed: &mut PlacedIndex,
        target: Option<&str>,
    ) -> Result<RangeGraph, Stop> {
        let mut dictionary = Dictionary::new(self.capacity.formed_bytes);
        for index in 0..placed.covered.len() {
            let covered = placed.covered[index];
            if covered.kind == FrameType::Terms {
                let frame = self.fetch(placed, covered)?;
                dictionary.terms(covered.item, &frame)?;
            }
        }
        let mut claims = GraphClaims::new(&placed.claims.graphs, &placed.covered, |id| {
            dictionary.names_graph(id, target)
        })?;
        let mut rows = Vec::new();
        for ordinal in 0..placed.covered.len() {
            let covered = placed.covered[ordinal];
            if !claims.selects(ordinal, covered.kind)? {
                continue;
            }
            let frame = self.fetch(placed, covered)?;
            let header =
                RowHeader::read_verified(covered.kind, frame.content(), core::num::NonZeroU64::MIN)
                    .ok_or(Stop::Unknown(RangeReason::Payload))?
                    .map_err(Stop::content_error)?;
            let decoded = header
                .decode(frame.payload())
                .map_err(Stop::content_error)?;
            let scope = dictionary.scope(covered.item);
            dictionary
                .observed
                .check_scoped_annotations(&dictionary.arena, &decoded.rows, scope)
                .map_err(Stop::fold_error)?;
            let ranges = claims.rows(ordinal, covered.kind, decoded.rows.len() as u64)?;
            let mut assigned = ranges.into_iter().flatten().peekable();
            for (index, row) in decoded.rows.into_iter().enumerate() {
                let index = index as u64;
                let claimed = assigned.next_if_eq(&index).is_some();
                if claimed != dictionary.names_graph(row.graph_id(), target) {
                    return Err(Stop::Unknown(RangeReason::Contradiction));
                }
                if claimed
                    && let Ok(by_reference) =
                        dictionary
                            .observed
                            .admit_referenced_row(row, scope, &dictionary.references)
                {
                    push(
                        &mut rows,
                        RangeRow {
                            position: covered.item as u64,
                            index,
                            row,
                            by_reference,
                            visible: None,
                        },
                    )?;
                }
            }
        }
        if self.query.suppression {
            self.apply_suppression(placed, &mut dictionary, &mut rows)?;
        }
        Ok(RangeGraph { rows, dictionary })
    }

    fn apply_suppression(
        &mut self,
        placed: &mut PlacedIndex,
        dictionary: &mut Dictionary,
        rows: &mut [RangeRow],
    ) -> Result<(), Stop> {
        let registry = core::num::NonZeroU64::MIN;
        let owners = FrameOwners::default();
        let mut claims = Vec::new();
        let mut by_reference = false;
        for index in 0..placed.covered.len() {
            let covered = placed.covered[index];
            if covered.kind != FrameType::Suppress {
                continue;
            }
            let frame = self.fetch(placed, covered)?;
            let mut content = frame.content();
            content.record(1)?;
            let count = content.uint()?;
            let entries = decode_suppressions(frame.payload(), count, registry)
                .map_err(Stop::content_error)?;
            let scope = dictionary.scope(covered.item);
            for entry in entries {
                let terms = LocalTerms::new(&dictionary.observed, scope, &dictionary.arena);
                let admission = terms
                    .suppression_with_references(entry, &dictionary.references)
                    .map_err(Stop::fold_error)?;
                match admission {
                    EntryAdmission::Accepted(Some(entry)) => {
                        let entry = entry
                            .try_map(|leaf| leaf.resolve(0, &owners, &mut dictionary.arena))
                            .map_err(Stop::fold_error)?;
                        push(
                            &mut claims,
                            SuppressionClaim {
                                offset: covered.item,
                                entry,
                            },
                        )?;
                    }
                    EntryAdmission::Accepted(None) => by_reference = true,
                    _ => {}
                }
            }
        }
        if by_reference || (!claims.is_empty() && rows.iter().any(|row| row.by_reference)) {
            return Err(Stop::Unknown(RangeReason::Value));
        }
        let mut matches = Matches::new(&claims, &dictionary.arena).map_err(Stop::fold_error)?;
        for row in rows {
            let hidden = if claims.is_empty() {
                false
            } else {
                let scope = dictionary.scope(row.position as usize);
                let RowDisposition::Accepted(line) =
                    dictionary.observed.resolve_scoped_row(row.row, scope)
                else {
                    unreachable!("the answer admitted this row without value references")
                };
                matches.line(line)
            };
            row.visible = Some(!hidden);
        }
        Ok(())
    }
}

struct ActiveRange {
    first_row: u64,
    count: u64,
}

pub(super) struct GraphClaims {
    remaining: std::vec::IntoIter<GraphRange>,
    started: Option<GraphRange>,
    active: [Vec<ActiveRange>; 4],
    last: [Option<usize>; 4],
}

impl GraphClaims {
    /// Select directory entries by dictionary name and validate every start
    /// position before the first target frame is fetched.
    pub(super) fn new(
        directory: &[GraphEntry],
        covered: &[Covered],
        names_target: impl Fn(Option<u64>) -> bool,
    ) -> Result<Self, Stop> {
        let mut ranges = Vec::new();
        for range in directory
            .iter()
            .filter(|entry| names_target(entry.graph))
            .flat_map(|entry| &entry.ranges)
        {
            let index = usize::try_from(range.first_frame)
                .ok()
                .and_then(|position| covered.get(position));
            if !index.is_some_and(|frame| frame.kind == range.kind) {
                return Err(Stop::Unknown(RangeReason::IndexClaims));
            }
            push(&mut ranges, *range)?;
        }
        ranges.sort_unstable_by_key(|range| (range.first_frame, range.first_row));
        let mut remaining = ranges.into_iter();
        let started = remaining.next();
        let mut last = [None; 4];
        for (index, frame) in covered.iter().enumerate() {
            if let Some(slot) = slot(frame.kind) {
                last[slot] = Some(index);
            }
        }
        Ok(Self {
            remaining,
            started,
            active: [const { Vec::new() }; 4],
            last,
        })
    }

    /// A range that continues advances to the next covered frame of its
    /// type, independent of interleaved frames of other types.
    pub(super) fn selects(&mut self, index: usize, kind: FrameType) -> Result<bool, Stop> {
        let Some(slot) = slot(kind) else {
            return Ok(false);
        };
        while let Some(range) = self
            .started
            .filter(|range| range.first_frame == index as u64)
        {
            push(
                &mut self.active[slot],
                ActiveRange {
                    first_row: range.first_row,
                    count: range.row_count,
                },
            )?;
            self.started = self.remaining.next();
        }
        Ok(!self.active[slot].is_empty())
    }

    /// Assign claims using the verified frame's actual row count. Call after
    /// content admission so a malformed row frame precedes a contradiction.
    pub(super) fn rows(
        &mut self,
        index: usize,
        kind: FrameType,
        rows: u64,
    ) -> Result<Vec<Range<u64>>, Stop> {
        let slot = slot(kind).expect("only a row frame can have a graph claim");
        let active = &mut self.active[slot];
        let mut assigned = Vec::new();
        assigned.try_reserve_exact(active.len())?;
        for range in active.iter_mut() {
            if range.first_row >= rows {
                return Err(Stop::Unknown(RangeReason::Contradiction));
            }
            let length = range.count.min(rows - range.first_row);
            assigned.push(range.first_row..range.first_row + length);
            range.count -= length;
            range.first_row = 0;
        }
        active.retain(|range| range.count != 0);
        if self.last[slot] == Some(index) && !active.is_empty() {
            return Err(Stop::Unknown(RangeReason::Contradiction));
        }
        assigned.sort_unstable_by_key(|range| range.start);
        if assigned.windows(2).any(|pair| pair[0].end > pair[1].start) {
            return Err(Stop::Unknown(RangeReason::Contradiction));
        }
        Ok(assigned)
    }
}

fn slot(kind: FrameType) -> Option<usize> {
    Some(match kind {
        FrameType::Quads => 0,
        FrameType::ReifierBindings => 1,
        FrameType::Annotations => 2,
        FrameType::EmptyGraphs => 3,
        _ => return None,
    })
}
