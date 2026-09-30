// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The accounting observation every shared-view bench reports, and the round-trip
//! delta it measures a delta carrier with.
//!
//! This is bench support, not API: it is hidden from the documentation and carries
//! no stability promise. It lives in the library because the carrier benches of
//! this crate and of `purrdf-rdf` are separate crates that can share code only
//! through a library they both reach, and one line format read by two benches is
//! one definition of what the line says.

use std::sync::Arc;
use std::time::Duration;

use crate::{
    DatasetMut as _, DeltaDatasetView, MutableDataset, QuadValues, RdfDataset, TermValue,
    ViewAccountingReport,
};

/// Every field an observation line must carry.
const OBSERVATION_FIELDS: [&str; 10] = [
    "shape=",
    "variant=",
    "profile=",
    "elapsed_ns=",
    "peak_accounted_bytes=",
    "retained_bytes=",
    "incremental_bytes=",
    "copies=",
    "freezes=",
    "materializations=",
];

/// The build profile this run was taken under.
///
/// Derived, never asserted as a literal: a debug-profile figure is not comparable
/// with a release one, and a hardcoded `release` would quietly claim it was. The
/// line is a record, so it has to say which of the two it is.
#[must_use]
pub const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// The accounting families one measured variant reports, in the units the carrier
/// itself keeps. `peak_accounted_bytes` is an ACCOUNTED figure, not an allocator or
/// RSS measurement.
#[derive(Debug, Clone, Copy, Default)]
pub struct Observation {
    /// The peak of accounted bytes.
    pub peak_accounted_bytes: usize,
    /// The retention ledger's deduplicated retained bytes.
    pub retained_bytes: usize,
    /// The view's own incremental charge.
    pub incremental_bytes: usize,
    /// Rows copied.
    pub copies: usize,
    /// Datasets frozen.
    pub freezes: usize,
    /// Materializations.
    pub materializations: usize,
}

impl Observation {
    /// A carrier's own accounting report, read verbatim, with `extra_bytes` (bytes
    /// the measured path accounted outside the ledger) added to the peak.
    #[must_use]
    pub fn from_report(report: &ViewAccountingReport, extra_bytes: usize) -> Self {
        Self {
            peak_accounted_bytes: report.total_accounted_bytes().saturating_add(extra_bytes),
            retained_bytes: report
                .retained
                .retained_payload_bytes
                .saturating_add(report.retained.memo_bytes),
            incremental_bytes: report.incremental_auxiliary_bytes,
            copies: report.incremental_work.copied_rows,
            freezes: report.incremental_work.freezes,
            materializations: report.incremental_work.materializations,
        }
    }
}

/// Print one observation line, self-asserting that every named field survived the
/// formatting. This is the acceptance mechanism: it runs in a harness's `--test`
/// smoke mode too, so a shape that stopped reporting fails the bench.
///
/// # Panics
///
/// If the formatted line lost a field.
pub fn observe(shape: &str, variant: &str, elapsed: Duration, observed: Observation) {
    let line = format!(
        "observation shape={shape} variant={variant} profile={profile} \
         elapsed_ns={elapsed} peak_accounted_bytes={peak} retained_bytes={retained} \
         incremental_bytes={incremental} copies={copies} freezes={freezes} \
         materializations={materializations}",
        profile = build_profile(),
        elapsed = elapsed.as_nanos(),
        peak = observed.peak_accounted_bytes,
        retained = observed.retained_bytes,
        incremental = observed.incremental_bytes,
        copies = observed.copies,
        freezes = observed.freezes,
        materializations = observed.materializations,
    );
    for field in OBSERVATION_FIELDS {
        assert!(
            line.contains(field),
            "the observation line lost {field:?}: {line}"
        );
    }
    println!("{line}");
}

/// A delta view whose EFFECTIVE content is exactly `base`'s, reached through a real
/// mutation round trip rather than an untouched passthrough — the scratch row
/// `<{namespace}scratch> <{namespace}p> <{namespace}o>` is inserted and taken back
/// out — so the delta machinery (suppression rows, delta-only ids) is genuinely in
/// the read path.
///
/// # Panics
///
/// If the round trip does not insert, remove and publish as it must.
#[must_use]
pub fn round_trip_delta(base: &Arc<RdfDataset>, namespace: &str) -> Arc<DeltaDatasetView> {
    let mut mutable = MutableDataset::new(Arc::clone(base));
    let scratch = QuadValues::triple(
        TermValue::iri(format!("{namespace}scratch")),
        TermValue::iri(format!("{namespace}p")),
        TermValue::iri(format!("{namespace}o")),
    );
    assert!(
        mutable
            .insert(scratch.clone())
            .expect("the scratch row inserts")
    );
    assert!(mutable.remove(&scratch), "and is taken back out again");
    Arc::new(mutable.snapshot_view().expect("the delta publishes"))
}
