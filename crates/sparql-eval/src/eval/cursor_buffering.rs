// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A fixed, admitted record of the buffering strategies execution actually selected.

use crate::workspace::SharedWorkspace;
use crate::{EvalError, WorkspaceCapability};
use std::sync::atomic::{AtomicU16, Ordering};

/// An original operator unit retained while the cursor delivers its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CursorBufferReason {
    /// ORDER BY retains its input rows and ordering keys.
    OrderedInput,
    /// GROUP BY or an aggregate retains its grouping/input state.
    AggregateState,
    /// DISTINCT/REDUCED retains alternatives until their evidence is coalesced.
    EvidenceCoalescing,
    /// DISTINCT/REDUCED retains the identities it has already emitted.
    DistinctKeys,
    /// Stateful expressions retain their original operator-major input unit.
    StatefulExpression,
    /// A selected join driver is retained before evaluating its driven relation.
    DriverRelation,
    /// A join/OPTIONAL/MINUS probe relation is retained for matching rows.
    ProbeRelation,
    /// A compiled BGP retains an earlier stage's intermediate relation.
    IntermediateRelation,
    /// Property-path traversal retains its reachability sets and work state.
    PathReachability,
    /// Coupled finite governors or a stop signal require a settled rollback unit.
    GovernorSettlement,
    /// An explicit semantic verdict preserves the original producer's commit unit.
    VerdictCommitUnit,
    /// A host relation invocation retains its original invocation unit.
    HostInvocation,
    /// A selected certified physical plan retains its native execution unit.
    PhysicalPlan,
}

impl CursorBufferReason {
    const ALL: [Self; 13] = [
        Self::OrderedInput,
        Self::AggregateState,
        Self::EvidenceCoalescing,
        Self::DistinctKeys,
        Self::StatefulExpression,
        Self::DriverRelation,
        Self::ProbeRelation,
        Self::IntermediateRelation,
        Self::PathReachability,
        Self::GovernorSettlement,
        Self::VerdictCommitUnit,
        Self::HostInvocation,
        Self::PhysicalPlan,
    ];
}

/// Buffering selected so far, copied without allocating or advancing a source.
/// Each reason appears at most once, regardless of rows or operator re-entry.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorBufferingReport(u16);

impl CursorBufferingReport {
    /// Whether execution has selected no retaining operator unit so far.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether this retaining strategy was actually selected.
    #[must_use]
    pub const fn contains(self, reason: CursorBufferReason) -> bool {
        self.0 & (1 << reason as u8) != 0
    }

    /// Selected reasons in a fixed presentation order.
    pub fn reasons(self) -> impl Iterator<Item = CursorBufferReason> {
        CursorBufferReason::ALL
            .into_iter()
            .filter(move |reason| self.contains(*reason))
    }
}

impl std::fmt::Debug for CursorBufferingReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_set().entries(self.reasons()).finish()
    }
}

#[derive(Clone)]
pub(super) struct BufferingTrace(SharedWorkspace<AtomicU16>);

impl BufferingTrace {
    pub(super) fn new(workspace: &WorkspaceCapability) -> Result<Self, EvalError> {
        SharedWorkspace::new_admitted(AtomicU16::new(0), workspace).map(Self)
    }
    pub(super) fn note(&self, reason: CursorBufferReason) {
        self.0.fetch_or(1 << reason as u8, Ordering::Relaxed);
    }
    pub(super) fn report(&self) -> CursorBufferingReport {
        CursorBufferingReport(self.0.load(Ordering::Relaxed))
    }
}
