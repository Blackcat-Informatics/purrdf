// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shared lint diagnostic carrier for the surviving `purrdf-slice` lints.
//!
//! [`ProjectionDiagnostic`] is the `{severity, code, message, check, instance}` (+
//! optional alignment-row CURIEs) shape every slice lint emits and the PyO3 binding
//! packs into a Python dict. The correspondence-soundness checks moved to the
//! oxigraph-free `purrdf-logic-compile` pass (which redeclares its own equivalent
//! struct); the surviving [`crate::prefix_lint`] still emits this carrier, so it lives
//! here in a small dedicated home.

/// One lint problem. The `check`/`instance` convention mirrors the native SSSOM
/// validator's diagnostic dict (`purrdf.validate_sssom`) so the PyO3 binding packs
/// both into the same `{severity, code, message, check, instance}` shape the Python
/// finding leg consumes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProjectionDiagnostic {
    /// Severity token: `"ERROR"`, `"WARNING"`, or `"INFO"`.
    pub severity: String,
    /// The drift family (e.g. `prefix-consistency`). The finding leg maps this to the
    /// canonical code `mapping-compile.<check>`.
    pub check: String,
    /// A stable per-check code (same value as `check`); carried for dict parity with
    /// the SSSOM validator's `code` slot.
    pub code: String,
    /// The human-readable problem.
    pub message: String,
    /// The most-specific RDF node the problem concerns, or `None`.
    pub instance: Option<String>,
    /// For alignment-direction findings, the SSSOM row CURIEs that the finding is
    /// about. These are `None` for the prefix-consistency lint.
    pub subject_id: Option<String>,
    pub predicate_id: Option<String>,
    pub object_id: Option<String>,
}

impl ProjectionDiagnostic {
    /// Severity-first ordering used for stable, deterministic lint output:
    /// ERROR < WARNING < INFO < everything else, then check, then instance.
    pub fn cmp_severity_check_instance(&self, other: &Self) -> std::cmp::Ordering {
        let order = |s: &str| match s {
            "ERROR" => 0,
            "WARNING" => 1,
            "INFO" => 2,
            _ => 3,
        };
        order(&self.severity)
            .cmp(&order(&other.severity))
            .then_with(|| self.check.cmp(&other.check))
            .then_with(|| self.instance.cmp(&other.instance))
    }
}
