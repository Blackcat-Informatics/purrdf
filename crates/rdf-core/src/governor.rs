// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one execution-governance vocabulary shared by every PurRDF tier.
//!
//! A governor never changes an answer, only an outcome: it decides whether a caller
//! receives the complete answer or a certified subset plus a typed cause. The rows
//! themselves are never different. That is what distinguishes a resource ceiling from
//! semantic optionality, and it is why the vocabulary is a kernel type rather than a
//! per-tier invention.
//!
//! The demand-paging tier ([`crate::ir::PagedQueryError`]) and the compute tier both
//! name these types, so a consumer writes exactly one budget renderer and one stop-cause
//! renderer. Adding a governed resource is a row in [`ResourceDimension`], not a new
//! taxonomy.
//!
//! Nothing here reads a clock, draws randomness, allocates a thread, or performs I/O.
//! A deadline is always host-owned and arrives as a [`StopCause::Deadline`] report; the
//! module stays `wasm32-unknown-unknown` compatible.

use std::ops::Index;

/// Why an operation stopped before it could complete on its own terms.
///
/// Cancellation and a deadline are the same primitive — a host-supplied stop signal —
/// and differ only in which cause the host reports. Keeping them in one enum is what
/// lets the paging tier and the compute tier share a single renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopCause {
    /// The caller or host cancelled the operation.
    Cancelled,
    /// A host-owned deadline expired. PurRDF itself never reads a clock.
    Deadline,
}

impl StopCause {
    /// A stable diagnostic label for this stop cause.
    ///
    /// These strings are shared with [`crate::ir::PageFaultKind::label`]; changing one
    /// changes the other's rendered diagnostics.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Deadline => "deadline exceeded",
        }
    }
}

impl std::fmt::Display for StopCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// One governed resource.
///
/// Each dimension is charged at a documented, deterministic counting point, so the same
/// query over the same data with the same ceilings consumes exactly the same amount.
///
/// The set spans both governed tiers: the demand-paging tier bounds I/O ([`Pages`] and
/// [`Bytes`]), the evaluation tier bounds compute (everything else). They are two
/// projections of one vector, not two budget systems — see
/// [`PagedQueryLimits::to_resource_vector`](crate::ir::PagedQueryLimits::to_resource_vector).
///
/// [`Pages`]: ResourceDimension::Pages
/// [`Bytes`]: ResourceDimension::Bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceDimension {
    /// Abstract execution steps charged at the evaluator's counting points.
    Fuel,
    /// Rows committed to the final answer sequence, after every modifier. This is an
    /// operational ceiling, never `LIMIT`: `LIMIT` is query semantics.
    AnswerRows,
    /// Intermediate solution cells (`rows * columns`), which is what actually bounds
    /// allocation: a two-column and a forty-column bag of the same row count are a
    /// twentyfold different allocation.
    IntermediateCells,
    /// Bytes minted into a scratch arena by value-constructing operations, which grow
    /// independently of any row or cell count.
    ScratchBytes,
    /// Requests issued to a remote or federated endpoint.
    RemoteRequests,
    /// Nesting depth of user-defined function invocation.
    UdfDepth,
    /// Distinct pages a demand-paging operation may admit. Cached re-reads of an
    /// already-admitted page charge nothing further.
    Pages,
    /// Provider-reported byte charges of admitted pages.
    Bytes,
}

impl ResourceDimension {
    /// Every governed dimension, in declaration order. Iterate this rather than
    /// hand-listing variants, so a new dimension reaches every consumer.
    pub const ALL: [Self; 8] = [
        Self::Fuel,
        Self::AnswerRows,
        Self::IntermediateCells,
        Self::ScratchBytes,
        Self::RemoteRequests,
        Self::UdfDepth,
        Self::Pages,
        Self::Bytes,
    ];

    /// The number of governed dimensions, and the width of a [`ResourceVector`].
    pub const COUNT: usize = Self::ALL.len();

    /// A stable kebab-case label for this dimension.
    ///
    /// These strings are a pinned contract: a frozen conformance corpus records them as
    /// case discriminants, so a consumer may match on them. Renaming one is a breaking
    /// change to that corpus, not a cosmetic edit.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fuel => "fuel",
            Self::AnswerRows => "answer-rows",
            Self::IntermediateCells => "intermediate-cells",
            Self::ScratchBytes => "scratch-bytes",
            Self::RemoteRequests => "remote-requests",
            Self::UdfDepth => "udf-depth",
            Self::Pages => "pages",
            Self::Bytes => "bytes",
        }
    }

    /// The dense slot this dimension occupies in a [`ResourceVector`].
    const fn index(self) -> usize {
        match self {
            Self::Fuel => 0,
            Self::AnswerRows => 1,
            Self::IntermediateCells => 2,
            Self::ScratchBytes => 3,
            Self::RemoteRequests => 4,
            Self::UdfDepth => 5,
            Self::Pages => 6,
            Self::Bytes => 7,
        }
    }
}

impl std::fmt::Display for ResourceDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A fixed-width value keyed by [`ResourceDimension`], used both for ceilings and for
/// consumption.
///
/// `u64::MAX` in a slot means "no practical ceiling on this dimension". That is what
/// [`ResourceVector::is_bounded`] tests, and it is how an unbounded dimension costs an
/// ungoverned query nothing: the charge site short-circuits before touching a counter.
///
/// There is deliberately **no** `Default` implementation. A derived default would be
/// [`ResourceVector::ZERO`], which as a ceiling vector means "every governor trips
/// immediately" — a silent trap. Name [`ResourceVector::UNBOUNDED`] or
/// [`ResourceVector::ZERO`] explicitly at every construction site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceVector([u64; ResourceDimension::COUNT]);

impl ResourceVector {
    /// Zero on every dimension. As a consumption vector this is "nothing spent yet"; as
    /// a ceiling vector it is a valid hard limit that admits no charged work at all.
    pub const ZERO: Self = Self([0; ResourceDimension::COUNT]);

    /// No practical ceiling on any dimension. Stop signals and genuine evaluation
    /// failures remain fully checked.
    pub const UNBOUNDED: Self = Self([u64::MAX; ResourceDimension::COUNT]);

    /// The value recorded for `dimension`.
    #[must_use]
    pub const fn get(self, dimension: ResourceDimension) -> u64 {
        self.0[dimension.index()]
    }

    /// Overwrite the value recorded for `dimension`.
    pub const fn set(&mut self, dimension: ResourceDimension, value: u64) {
        self.0[dimension.index()] = value;
    }

    /// Return a copy with `dimension` set to `value`.
    #[must_use]
    pub const fn with(mut self, dimension: ResourceDimension, value: u64) -> Self {
        self.set(dimension, value);
        self
    }

    /// Add `amount` to `dimension`, saturating at `u64::MAX` rather than overflowing.
    ///
    /// Charging is the hot path and runs under `overflow-checks`, so it saturates: an
    /// arithmetic panic in a resource meter would convert an exhausted budget into a
    /// crash.
    pub const fn saturating_add_assign(&mut self, dimension: ResourceDimension, amount: u64) {
        let slot = &mut self.0[dimension.index()];
        *slot = slot.saturating_add(amount);
    }

    /// Raise `dimension` to `observed` when `observed` is larger.
    ///
    /// Peak-tracking dimensions — the intermediate-cell ceiling in particular — are
    /// compared against the maximum committed value of any single operator instance,
    /// not against a running sum.
    pub const fn max_assign(&mut self, dimension: ResourceDimension, observed: u64) {
        let slot = &mut self.0[dimension.index()];
        if observed > *slot {
            *slot = observed;
        }
    }

    /// Whether `dimension` carries a practical ceiling.
    #[must_use]
    pub const fn is_bounded(self, dimension: ResourceDimension) -> bool {
        self.get(dimension) != u64::MAX
    }

    /// Whether any dimension carries a practical ceiling.
    ///
    /// A vector for which this is `false` engages no charge site at all.
    #[must_use]
    pub const fn any_bounded(self) -> bool {
        let mut index = 0;
        while index < ResourceDimension::COUNT {
            if self.0[index] != u64::MAX {
                return true;
            }
            index += 1;
        }
        false
    }
}

impl Index<ResourceDimension> for ResourceVector {
    type Output = u64;

    fn index(&self, dimension: ResourceDimension) -> &Self::Output {
        &self.0[dimension.index()]
    }
}

/// The governor that stopped an operation.
///
/// [`TrippedGovernor::Budget`] is field-for-field the shape of
/// [`crate::ir::PagedQueryError::PageBudgetExceeded`], so a consumer writes exactly one
/// budget renderer for both tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrippedGovernor {
    /// A resource ceiling was reached.
    Budget {
        /// The dimension whose ceiling was reached.
        dimension: ResourceDimension,
        /// The inclusive ceiling in force.
        limit: u64,
        /// Consumption charged before the refused work.
        consumed: u64,
    },
    /// A host-supplied stop signal fired.
    Stopped {
        /// Which stop signal fired.
        cause: StopCause,
    },
}

impl TrippedGovernor {
    /// The stable discriminant string for this governor.
    ///
    /// These strings are a pinned contract: a frozen conformance corpus records them as
    /// outcome discriminants and a consumer may match on them, so renaming one breaks
    /// that corpus. They are kebab-case and distinct from the prose of [`Display`].
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Budget { dimension, .. } => match dimension {
                ResourceDimension::Fuel => "fuel-exhausted",
                ResourceDimension::AnswerRows => "answer-cap-exhausted",
                ResourceDimension::IntermediateCells => "cardinality-exhausted",
                ResourceDimension::ScratchBytes => "scratch-exhausted",
                ResourceDimension::RemoteRequests => "remote-exhausted",
                ResourceDimension::UdfDepth => "udf-depth-exhausted",
                // These two spell the paged tier's own variant names
                // (`PagedQueryError::PageBudgetExceeded` / `ByteBudgetExceeded`), so a
                // consumer reading either tier sees one vocabulary.
                ResourceDimension::Pages => "page-budget-exceeded",
                ResourceDimension::Bytes => "byte-budget-exceeded",
            },
            Self::Stopped { cause } => match cause {
                StopCause::Cancelled => "cancelled",
                StopCause::Deadline => "deadline-exceeded",
            },
        }
    }
}

impl std::fmt::Display for TrippedGovernor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Budget {
                dimension,
                limit,
                consumed,
            } => write!(
                f,
                "{} budget exceeded: consumed {consumed}, limit {limit}",
                dimension.label()
            ),
            Self::Stopped { cause } => f.write_str(cause.label()),
        }
    }
}

/// Deterministic evidence accumulated by one governed operation.
///
/// Evidence is returned on the complete path as well as the exhausted one: "completed,
/// cost N fuel, peak M cells" is how a consumer sizes a budget in the first place.
///
/// Consumption of `Fuel`, `AnswerRows`, and `IntermediateCells` is exact and
/// deterministic in the same sense as the paged tier's page and byte evidence: the same
/// snapshot, query, and ceilings produce the same totals and the same terminal status,
/// independent of worker count or scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GovernorEvidence {
    /// Consumption charged per dimension.
    pub consumed: ResourceVector,
    /// The ceilings in force, echoed so the evidence is self-describing.
    pub limits: ResourceVector,
    /// The governor that stopped the operation, or `None` on the complete path.
    pub tripped: Option<TrippedGovernor>,
}

impl GovernorEvidence {
    /// Start fresh evidence for an operation running under `limits`.
    ///
    /// Build this per execution. Governor state is operation-local; sharing it across
    /// queries would drain one query's budget into the next.
    #[must_use]
    pub const fn new(limits: ResourceVector) -> Self {
        Self {
            consumed: ResourceVector::ZERO,
            limits,
            tripped: None,
        }
    }

    /// Consumption charged per dimension.
    #[must_use]
    pub const fn consumed(&self) -> ResourceVector {
        self.consumed
    }

    /// The ceilings in force for this operation.
    #[must_use]
    pub const fn limits(&self) -> ResourceVector {
        self.limits
    }

    /// The governor that stopped the operation, or `None` if it completed.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        self.tripped
    }

    /// Consumption charged on one dimension.
    #[must_use]
    pub const fn consumed_in(&self, dimension: ResourceDimension) -> u64 {
        self.consumed.get(dimension)
    }

    /// The inclusive ceiling in force on one dimension.
    #[must_use]
    pub const fn limit_for(&self, dimension: ResourceDimension) -> u64 {
        self.limits.get(dimension)
    }

    /// Whether the operation completed without any governor tripping.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.tripped.is_none()
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{PageFault, PageFaultKind, PageId, PagedQueryLimits};

    use super::*;

    const ALL_STOP_CAUSES: [StopCause; 2] = [StopCause::Cancelled, StopCause::Deadline];

    #[test]
    fn resource_vector_unbounded_is_not_bounded_on_any_dimension() {
        let vector = ResourceVector::UNBOUNDED;
        for dimension in ResourceDimension::ALL {
            assert_eq!(vector.get(dimension), u64::MAX, "{}", dimension.label());
            assert!(!vector.is_bounded(dimension), "{}", dimension.label());
        }
        assert!(!vector.any_bounded());

        let one_bound = vector.with(ResourceDimension::Fuel, 7);
        assert!(one_bound.any_bounded());
        assert!(one_bound.is_bounded(ResourceDimension::Fuel));
        assert!(!one_bound.is_bounded(ResourceDimension::AnswerRows));
        assert_eq!(one_bound[ResourceDimension::Fuel], 7);
    }

    #[test]
    fn resource_vector_zero_is_bounded_on_every_dimension() {
        let vector = ResourceVector::ZERO;
        for dimension in ResourceDimension::ALL {
            assert_eq!(vector.get(dimension), 0, "{}", dimension.label());
            assert!(vector.is_bounded(dimension), "{}", dimension.label());
        }
        assert!(vector.any_bounded());
    }

    #[test]
    fn saturating_add_does_not_overflow_at_u64_max() {
        let mut vector = ResourceVector::ZERO;
        vector.saturating_add_assign(ResourceDimension::Fuel, u64::MAX);
        vector.saturating_add_assign(ResourceDimension::Fuel, 1);
        assert_eq!(vector.get(ResourceDimension::Fuel), u64::MAX);

        vector.set(ResourceDimension::IntermediateCells, u64::MAX - 1);
        vector.saturating_add_assign(ResourceDimension::IntermediateCells, 9);
        assert_eq!(vector.get(ResourceDimension::IntermediateCells), u64::MAX);

        let mut peak = ResourceVector::ZERO;
        peak.max_assign(ResourceDimension::IntermediateCells, 40);
        peak.max_assign(ResourceDimension::IntermediateCells, 12);
        assert_eq!(peak.get(ResourceDimension::IntermediateCells), 40);
        peak.max_assign(ResourceDimension::IntermediateCells, 41);
        assert_eq!(peak.get(ResourceDimension::IntermediateCells), 41);
    }

    #[test]
    fn every_dimension_label_is_unique_and_stable() {
        let labels: Vec<&'static str> = ResourceDimension::ALL
            .iter()
            .map(|dimension| dimension.label())
            .collect();
        assert_eq!(
            labels,
            vec![
                "fuel",
                "answer-rows",
                "intermediate-cells",
                "scratch-bytes",
                "remote-requests",
                "udf-depth",
                "pages",
                "bytes",
            ]
        );
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ResourceDimension::COUNT);

        let mut indexes: Vec<usize> = ResourceDimension::ALL
            .iter()
            .map(|dimension| dimension.index())
            .collect();
        indexes.sort_unstable();
        assert_eq!(indexes, (0..ResourceDimension::COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn every_tripped_governor_label_is_unique_and_stable() {
        let mut labels: Vec<&'static str> = ResourceDimension::ALL
            .iter()
            .map(|&dimension| {
                TrippedGovernor::Budget {
                    dimension,
                    limit: 0,
                    consumed: 0,
                }
                .label()
            })
            .collect();
        assert_eq!(
            labels,
            vec![
                "fuel-exhausted",
                "answer-cap-exhausted",
                "cardinality-exhausted",
                "scratch-exhausted",
                "remote-exhausted",
                "udf-depth-exhausted",
                // Spelled exactly like the paged tier's own error variants.
                "page-budget-exceeded",
                "byte-budget-exceeded",
            ]
        );
        for cause in ALL_STOP_CAUSES {
            labels.push(TrippedGovernor::Stopped { cause }.label());
        }
        assert_eq!(labels[ResourceDimension::COUNT], "cancelled");
        assert_eq!(labels[ResourceDimension::COUNT + 1], "deadline-exceeded");

        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len(), "labels must be distinct");

        assert_eq!(
            TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 10,
                consumed: 10,
            }
            .to_string(),
            "fuel budget exceeded: consumed 10, limit 10"
        );
        assert_eq!(
            TrippedGovernor::Stopped {
                cause: StopCause::Deadline,
            }
            .to_string(),
            "deadline exceeded"
        );
    }

    #[test]
    fn paged_limits_round_trip_through_the_resource_vector() {
        for limits in [PagedQueryLimits::UNBOUNDED, PagedQueryLimits::new(3, 4096)] {
            let vector = limits.to_resource_vector();
            assert_eq!(
                PagedQueryLimits::from_resource_vector(vector),
                limits,
                "paged limits must survive the projection unchanged"
            );
        }

        let bounded = PagedQueryLimits::new(3, 4096).to_resource_vector();
        assert_eq!(bounded.get(ResourceDimension::Pages), 3);
        assert_eq!(bounded.get(ResourceDimension::Bytes), 4096);
        assert!(bounded.is_bounded(ResourceDimension::Pages));
        assert!(bounded.is_bounded(ResourceDimension::Bytes));

        let unbounded = PagedQueryLimits::UNBOUNDED.to_resource_vector();
        assert_eq!(unbounded, ResourceVector::UNBOUNDED);
        assert!(!unbounded.any_bounded());
    }

    #[test]
    fn paged_limits_projection_leaves_compute_dimensions_unbounded() {
        let vector = PagedQueryLimits::new(3, 4096).to_resource_vector();
        for dimension in [
            ResourceDimension::Fuel,
            ResourceDimension::AnswerRows,
            ResourceDimension::IntermediateCells,
            ResourceDimension::ScratchBytes,
            ResourceDimension::RemoteRequests,
            ResourceDimension::UdfDepth,
        ] {
            assert_eq!(
                vector.get(dimension),
                u64::MAX,
                "projecting an I/O ceiling must not impose a {} ceiling",
                dimension.label()
            );
            assert!(!vector.is_bounded(dimension), "{}", dimension.label());
        }
    }

    #[test]
    fn stop_cause_round_trips_through_page_fault_kind() {
        for cause in ALL_STOP_CAUSES {
            let kind = PageFaultKind::Stopped(cause);
            let PageFaultKind::Stopped(recovered) = kind else {
                panic!("stopped fault kind must carry a stop cause");
            };
            assert_eq!(recovered, cause);
            assert_eq!(kind.label(), cause.label());
        }
    }

    #[test]
    fn page_fault_constructors_still_build_the_expected_stopped_variant() {
        let page = PageId(3);
        assert_eq!(
            PageFault::cancelled(page, "cancelled by host").kind,
            PageFaultKind::Stopped(StopCause::Cancelled)
        );
        assert_eq!(
            PageFault::deadline_exceeded(page, "host deadline elapsed").kind,
            PageFaultKind::Stopped(StopCause::Deadline)
        );
        assert_eq!(
            PageFault::cancelled(page, "cancelled by host").to_string(),
            "cancelled materializing page 3: cancelled by host"
        );
        assert_eq!(
            PageFault::deadline_exceeded(page, "host deadline elapsed").to_string(),
            "deadline exceeded materializing page 3: host deadline elapsed"
        );
    }

    #[test]
    fn evidence_echoes_its_limits_and_starts_complete() {
        let limits = ResourceVector::UNBOUNDED
            .with(ResourceDimension::Fuel, 100)
            .with(ResourceDimension::AnswerRows, 10);
        let mut evidence = GovernorEvidence::new(limits);
        assert_eq!(evidence.limits(), limits);
        assert_eq!(evidence.consumed(), ResourceVector::ZERO);
        assert_eq!(evidence.tripped(), None);
        assert!(evidence.is_complete());
        assert_eq!(evidence.limit_for(ResourceDimension::Fuel), 100);

        evidence
            .consumed
            .saturating_add_assign(ResourceDimension::Fuel, 101);
        evidence.tripped = Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            limit: 100,
            consumed: 101,
        });
        assert!(!evidence.is_complete());
        assert_eq!(evidence.consumed_in(ResourceDimension::Fuel), 101);
        assert_eq!(
            evidence.tripped().map(TrippedGovernor::label),
            Some("fuel-exhausted")
        );
    }
}
