// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit carrier retention admission and non-identity work counters.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{RdfDataset, RdfDiagnostic};

/// Finite admission ceilings for one immutable view. Owners may select larger
/// ceilings explicitly; exceeding any ceiling fails before publishing a view.
#[derive(Debug, Clone, Copy)]
pub struct ViewLimits {
    /// Maximum number of retained source handles.
    pub max_sources: usize,
    /// Maximum aggregate source term count, including aliases.
    pub max_terms: usize,
    /// Maximum aggregate RDF record count across all three tables.
    pub max_rows: usize,
    /// Maximum retained native RDF payload bytes (not allocator/index/sidecar bytes).
    pub max_payload_bytes: usize,
    /// Maximum conservative charge for view construction and retained bookkeeping,
    /// including transformed literal text and caller-owned graph placement.
    pub max_auxiliary_bytes: usize,
}

impl Default for ViewLimits {
    fn default() -> Self {
        Self {
            max_sources: 64,
            max_terms: 16_777_216,
            max_rows: 67_108_864,
            max_payload_bytes: 1_073_741_824,
            max_auxiliary_bytes: 536_870_912,
        }
    }
}

/// Successful copy/publication work. Counters never participate in RDF identity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewWork {
    /// Distinct terms in newly published dictionaries or rebound literals.
    pub copied_terms: usize,
    /// RDF records replayed at a dataset materialization boundary.
    pub copied_rows: usize,
    /// Published UTF-8 payload bytes in new dictionaries or rebound literals.
    /// Temporary importer buffers and allocator overhead are not counted.
    pub copied_text_bytes: usize,
    /// ID payload bytes written into view mappings and suppression sets.
    /// This excludes hash-table capacity and source-owned indexes.
    pub copied_index_bytes: usize,
    /// Successful native dataset freezes.
    pub freezes: usize,
    /// Successful complete view materializations.
    pub materializations: usize,
}

/// Retained payload and view-owned bookkeeping, separate from operational work.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewStats {
    /// Retained source handles. The view owns each handle until its final owner drops it.
    pub retained_sources: usize,
    /// Aggregate source term count, before cross-source aliasing.
    pub retained_terms: usize,
    /// Aggregate source RDF record count, before suppression and deduplication.
    pub retained_rows: usize,
    /// Immutable RDF payload bytes; source indexes and sidecars remain source-owned.
    pub retained_payload_bytes: usize,
    /// Conservative admission charge for retained and construction bookkeeping.
    /// Hash entries use four times their payload size for table slack; transient
    /// lookup maps are included. This is not an allocator or RSS measurement.
    pub auxiliary_bytes: usize,
    /// Work accumulated through this view and its clones.
    pub work: ViewWork,
}

impl ViewStats {
    pub(crate) fn retain(&mut self, dataset: &RdfDataset) {
        self.retained_sources = self.retained_sources.saturating_add(1);
        self.retained_terms = self.retained_terms.saturating_add(dataset.term_count());
        self.retained_rows = self.retained_rows.saturating_add(dataset.rdf_row_count());
        self.retained_payload_bytes = self
            .retained_payload_bytes
            .saturating_add(dataset.rdf_payload_bytes());
    }
}

impl ViewLimits {
    pub(crate) fn check(self, stats: &ViewStats) -> Result<(), RdfDiagnostic> {
        for (name, actual, limit) in [
            ("sources", stats.retained_sources, self.max_sources),
            ("terms", stats.retained_terms, self.max_terms),
            ("rows", stats.retained_rows, self.max_rows),
            (
                "payload bytes",
                stats.retained_payload_bytes,
                self.max_payload_bytes,
            ),
            (
                "auxiliary bytes",
                stats.auxiliary_bytes,
                self.max_auxiliary_bytes,
            ),
        ] {
            if actual > limit {
                return Err(RdfDiagnostic::error(
                    "view-retention-limit",
                    format!("view retains {actual} {name}, limit is {limit}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkCounter {
    terms: AtomicUsize,
    rows: AtomicUsize,
    text: AtomicUsize,
    indexes: AtomicUsize,
    freezes: AtomicUsize,
    materializations: AtomicUsize,
}

impl WorkCounter {
    pub(crate) fn add(&self, work: ViewWork) {
        for (counter, value) in [
            (&self.terms, work.copied_terms),
            (&self.rows, work.copied_rows),
            (&self.text, work.copied_text_bytes),
            (&self.indexes, work.copied_index_bytes),
            (&self.freezes, work.freezes),
            (&self.materializations, work.materializations),
        ] {
            let mut old = counter.load(Ordering::Relaxed);
            loop {
                match counter.compare_exchange_weak(
                    old,
                    old.saturating_add(value),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(actual) => old = actual,
                }
            }
        }
    }

    pub(crate) fn get(&self) -> ViewWork {
        ViewWork {
            copied_terms: self.terms.load(Ordering::Relaxed),
            copied_rows: self.rows.load(Ordering::Relaxed),
            copied_text_bytes: self.text.load(Ordering::Relaxed),
            copied_index_bytes: self.indexes.load(Ordering::Relaxed),
            freezes: self.freezes.load(Ordering::Relaxed),
            materializations: self.materializations.load(Ordering::Relaxed),
        }
    }
}
