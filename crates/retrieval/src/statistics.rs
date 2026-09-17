// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The statistics a plan is planned against.
//!
//! Planning consults statistics — cardinalities for the strata it places, and
//! selectivities for the request terms a producer receives — because that is
//! what makes planning more than structural matching. Those statistics are an
//! **explicit input**, not something [`plan`](crate::plan) reaches into a store
//! for: the planner is a pure function, so the only statistics it can see are
//! the ones the caller hands it through this trait.
//!
//! # No clock, no store, no fabricated default
//!
//! The trait exposes no revision minted from a clock: [`Statistics::revision`]
//! is the *provider's* declaration of which snapshot it is, and the planner
//! records it verbatim. Staleness is therefore a comparison of caller-declared
//! revisions, never a timestamp the planner read. A provider that knows nothing
//! returns `None` from the query methods and says so through its source label;
//! the planner never invents a value to fill the gap.
//!
//! # Why the query methods return `Option`
//!
//! An unknown cardinality is not a zero one. Returning `Option` lets the
//! planner distinguish "the provider measured nothing" from "the provider
//! measured empty": the former falls back to the producer's declared row bound
//! (except where that bound is genuinely unbounded — see
//! [`PlanError::StatisticsUnavailable`](crate::PlanError::StatisticsUnavailable)),
//! the latter caps the stratum at zero.

use crate::iri::Iri;
use crate::request::RequestTerm;

/// A caller-supplied source of the cardinalities and selectivities planning
/// consults.
///
/// Implementations are pure lookups over caller-owned data: they hold no store
/// handle, open no file and read no clock, so a plan built through this trait is
/// reproducible across processes and targets for the same inputs.
pub trait Statistics {
    /// A caller-supplied label for the provider, recorded in the plan snapshot.
    ///
    /// It names *which* statistics were consulted, so a plan replayed against a
    /// different provider is distinguishable rather than silently re-planned.
    fn source(&self) -> &str;

    /// The provider-declared revision of the snapshot, recorded verbatim.
    ///
    /// Comparing this value is how a later admission stage detects a plan
    /// replayed against moved statistics. The planner never generates it — a
    /// revision read from a clock would make the plan depend on the wall clock.
    fn revision(&self) -> &str;

    /// The cardinality the provider reports for `predicate`, if known.
    ///
    /// The planner consults this for each stratum it places: a measured
    /// cardinality may lower the depth below the producer's declared row bound,
    /// never raise it.
    fn cardinality(&self, predicate: &Iri) -> Option<u64>;

    /// The selectivity the provider reports for `term` under `predicate`, in
    /// `[0, 1]`, if known.
    ///
    /// A value outside the unit interval is not a statistic; the planner clamps
    /// it at the boundary rather than letting a provider fault reorder a plan.
    fn selectivity(&self, predicate: &Iri, term: &RequestTerm) -> Option<f64>;
}
