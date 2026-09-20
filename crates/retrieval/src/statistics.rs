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
//! measured empty": the former falls back to the producer's declared row bound —
//! at the deepest depth a read can be taken to, where that bound is larger than a
//! read can reach or is the genuinely unbounded `u64::MAX` — while the latter
//! narrows the stratum as far as a statistic is allowed to narrow
//! anything, which is to one row and no further.
//!
//! So a provider that measures nothing is never why a plan is refused. It leaves
//! the declaration standing, and a declaration past the read range is recorded at
//! the ceiling with the probe row one past it, so the read's own ending names the
//! planned depth as the stopper — see [`plan`](crate::plan).
//!
//! # A statistic narrows a read; it never eliminates one
//!
//! Every bound derived here is floored at one. A measurement of zero — a
//! cardinality of zero, a selectivity of zero, or both — is an honest report and
//! is taken as one, but what follows from it is the shallowest read there is,
//! not the absence of a read. A depth of zero would compile to `LIMIT 0`, invoke
//! no relation at all, and then report the stratum exhausted with no rows, which
//! is the strongest completeness claim this layer can make and would have been
//! minted from an estimate rather than from data. Emptiness is the producer's to
//! report, in the receipt fusion verifies against the rows it actually pulled,
//! so the planner's job is to ask the shallowest honest question and let the
//! producer answer it.
//!
//! # Why selectivity is an integer, in parts per million
//!
//! A selectivity is a ratio, and every other ratio this layer carries — a
//! stratum weight, a spatial maximum distance, a fused score — is an exact
//! base-10 [`Fixed`](crate::Fixed) rather than a binary float, for the reason
//! [`RequestTerm::NumericRange`](crate::RequestTerm::NumericRange) spells out:
//! a plan's identity is a digest over its recorded fields, and a type with two
//! spellings of one value (`0.0` and `-0.0`) and a value that is not equal to
//! itself (`NaN`) has no place in an identity.
//!
//! Parts per million rather than `Fixed` because that is the resolution the
//! plan actually records
//! ([`StatisticsEntry::selectivity_ppm`](crate::StatisticsEntry::selectivity_ppm)):
//! a provider's value is written into the snapshot **verbatim**, with no
//! rounding step between what was reported and what was recorded, so the
//! snapshot explains the depth the planner derived from it exactly. It also
//! closes the last arithmetic path a float could have entered by — the crate
//! root denies `clippy::float_arithmetic`, so none can.

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

    /// The cardinality the provider reports for `subject`, if known.
    ///
    /// The planner consults this for each stratum it places: a measured
    /// cardinality may lower the depth below the producer's declared row bound,
    /// never raise it.
    fn cardinality(&self, subject: &Iri) -> Option<u64>;

    /// The selectivity the provider reports for `term` under `subject`, in
    /// parts per million of the rows `subject` carries, if known.
    ///
    /// `1_000_000` is "every row matches" and `0` is "none does". A value above
    /// `1_000_000` is not a statistic — it claims a term matches more rows than
    /// exist — and the planner clamps it at unity rather than letting a provider
    /// fault narrow a stratum below what its producer can answer.
    ///
    /// # What the planner does with it
    ///
    /// The subject a depth is derived for is a **stratum**, so a selectivity
    /// reported under a stratum is the one that bounds it: a term matching a
    /// tenth of a stratum's rows cannot be read a stratum-deep, and recording a
    /// depth no term can fill would license reading rows that are not there.
    /// The bound is applied the way a cardinality is — it lowers a depth, never
    /// raises one — and it is rounded **up**, so a bound derived from a ratio
    /// can never fall below the row count the ratio describes.
    ///
    /// Zero is where that rounding stops helping, so the derived depth is also
    /// floored at one. Reporting zero is not a provider fault: it is the correct
    /// answer for a provider that measured no matching rows, and the plan records
    /// the value verbatim in its snapshot. What the planner declines to do is
    /// turn it into a depth of zero, because that depth compiles to `LIMIT 0`,
    /// which hands back no row whatever the index holds, and then reports the
    /// stratum exhausted having emitted nothing — an emptiness claim the
    /// provider's estimate would have made on the producer's behalf. The
    /// relation is still opened; what it is never allowed to do is answer. So the
    /// exhaustion is the bound's claim rather than the data's, and it is
    /// indistinguishable in every trailer field from an honestly empty answer.
    /// Floored at one, the relation is asked, and its answer is the thing that
    /// says whether anything was there.
    ///
    /// A provider is free to report under a request predicate instead, or as
    /// well; a plan records every selectivity it was told, whatever the subject
    /// (see [`StatisticsSnapshot`](crate::StatisticsSnapshot)). Only a stratum's
    /// own selectivity bounds a stratum's own depth, because only that one is a
    /// statement about the rows the depth counts.
    fn selectivity_ppm(&self, subject: &Iri, term: &RequestTerm) -> Option<u64>;
}
