// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The explicit geometric span-selection law, independent of physical matching.

use crate::solution::{RetainedRow, RowBag};
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceCapability};
use purrdf_core::ViewTermId;

/// One half-open token span in a caller-declared logical occurrence domain.
/// Coordinates use the original index's unsigned position representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanGeometry {
    /// Equal domain ordinals identify the same document/language (or the
    /// explicit domain selected by a query-result consumer).
    pub domain: usize,
    /// Inclusive original token position.
    pub start: u64,
    /// Exclusive original token position; an adjacent span is disjoint.
    pub end: u64,
}

/// Longest first, then ascending start/end; preserve duplicate derivations of
/// a selected geometry and return ordinals in the original input bag order.
/// This is not the strictly-longer-CONTAINING predicate of FILTER NOT EXISTS.
///
/// # Errors
/// Refuses inverted coordinates and actual checked-layout/storage failures.
pub fn longest_first_indices(
    spans: &[SpanGeometry],
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<usize>, EvalError> {
    let mut ordered = AdmittedVec::with_capacity(spans.len(), workspace)?;
    for (ordinal, span) in spans.iter().enumerate() {
        if span.end < span.start {
            return Err(crate::NativeDiagnostic::error(
                crate::NativeDiagnosticKind::Data,
                "span end precedes its start",
                workspace,
            ));
        }
        ordered.push(ordinal)?;
    }
    ordered.as_mut_slice().sort_unstable_by(|&a, &b| {
        let a = spans[a];
        let b = spans[b];
        a.domain
            .cmp(&b.domain)
            .then_with(|| (b.end - b.start).cmp(&(a.end - a.start)))
            .then_with(|| a.start.cmp(&b.start))
            .then_with(|| a.end.cmp(&b.end))
    });
    // Coordinate compression is one preparation over the authored geometry.
    // Two native prefix counters test overlap in logarithmic time; they contain
    // only originally admitted integer arrays, not opaque tree-node allocations.
    let mut coordinates = AdmittedVec::new(workspace);
    for span in spans {
        coordinates.push((span.domain, span.start))?;
        coordinates.push((span.domain, span.end))?;
    }
    coordinates.as_mut_slice().sort_unstable();
    let mut previous = None;
    coordinates.retain(|coordinate| {
        if previous == Some(*coordinate) {
            return false;
        }
        previous = Some(*coordinate);
        true
    });
    let width = coordinates
        .len()
        .checked_add(1)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    let mut starts = AdmittedVec::with_capacity(width, workspace)?;
    let mut ends = AdmittedVec::with_capacity(width, workspace)?;
    for _ in 0..width {
        starts.push(0usize)?;
        ends.push(0usize)?;
    }
    let mut geometries = AdmittedMap::default();
    let mut selected = AdmittedVec::new(workspace);
    for ordinal in ordered {
        let candidate = spans[ordinal];
        let geometry = (candidate.domain, candidate.start, candidate.end);
        if geometries.get(&geometry).is_some() {
            selected.push(ordinal)?;
            continue;
        }
        // A zero-length half-open span is empty and therefore disjoint.
        if candidate.start == candidate.end {
            selected.push(ordinal)?;
            continue;
        }
        let before_end = coordinates
            .partition_point(|coordinate| *coordinate < (candidate.domain, candidate.end));
        let through_start = coordinates
            .partition_point(|coordinate| *coordinate <= (candidate.domain, candidate.start));
        let overlap = prefix(&starts, before_end)?
            .checked_sub(prefix(&ends, through_start)?)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if overlap != 0 {
            continue;
        }
        let start = coordinates
            .binary_search(&(candidate.domain, candidate.start))
            .expect("authored coordinate");
        let end = coordinates
            .binary_search(&(candidate.domain, candidate.end))
            .expect("authored coordinate");
        increment(starts.as_mut_slice(), start)?;
        increment(ends.as_mut_slice(), end)?;
        geometries.insert_admitted(geometry, (), workspace)?;
        selected.push(ordinal)?;
    }
    selected.as_mut_slice().sort_unstable();
    Ok(selected)
}

fn prefix(values: &[usize], mut index: usize) -> Result<usize, EvalError> {
    let mut sum = 0usize;
    while index != 0 {
        sum = sum
            .checked_add(values[index])
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        index &= index - 1;
    }
    Ok(sum)
}
fn increment(values: &mut [usize], ordinal: usize) -> Result<(), EvalError> {
    let mut index = ordinal
        .checked_add(1)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    while index < values.len() {
        values[index] = values[index]
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        index = index
            .checked_add(index.isolate_lowest_one())
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
    }
    Ok(())
}

/// A borrowed actual query result plus admitted selection scratch. Native row,
/// dictionary and evidence owners remain in the original query result.
#[derive(Debug)]
pub struct SpanSelection<'a, I: ViewTermId> {
    pub(crate) rows: &'a RowBag<I>,
    pub(crate) ordinals: AdmittedVec<usize>,
}
impl<I: ViewTermId> SpanSelection<'_, I> {
    /// Selected rows in original stable query-result order.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = &RetainedRow<I>> {
        self.ordinals.iter().map(|&ordinal| &self.rows[ordinal])
    }
    /// Original selected row ordinals, including duplicates.
    pub fn ordinals(&self) -> &[usize] {
        &self.ordinals
    }
    /// Forward the whole reached-invocation search scope, including zero rows.
    pub fn search_evidence(&self) -> &crate::RowEvidence {
        self.rows.search_evidence()
    }
}
