// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Filesystem-free CSVW input resources and diagnostics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Explicit CSVW processing entry point selected by the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum CsvwAction {
    /// Process a table, optionally with a host-selected metadata document.
    Table {
        /// Absolute table URL.
        table_iri: String,
        /// Absolute metadata URL, or `None` for an embedded default description.
        metadata_iri: Option<String>,
    },
    /// Process a table-group or table metadata document.
    Metadata {
        /// Absolute metadata URL.
        metadata_iri: String,
    },
}

/// Complete in-memory resource set for one CSVW operation.
///
/// The host performs HTTP, `Link`-header, or filesystem discovery and places all
/// selected resources here by absolute IRI. The engine never performs I/O and
/// never silently substitutes a different discovered metadata document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvwInput {
    action: CsvwAction,
    resources: BTreeMap<String, Vec<u8>>,
}

impl CsvwInput {
    /// Construct and validate a bounded resource package.
    ///
    /// # Errors
    ///
    /// Returns a package/configuration failure for malformed IRIs, missing action
    /// resources, duplicate keys (before construction), or configured size/count
    /// breaches.
    pub fn new(
        action: CsvwAction,
        resources: BTreeMap<String, Vec<u8>>,
        limits: ProjectionLimits,
    ) -> Result<Self, ProjectionError> {
        if resources.is_empty() {
            return Err(ProjectionError::package(
                "CSVW input must contain at least one resource",
            ));
        }
        if resources.len() > limits.max_artifacts() {
            return Err(ProjectionError::limit(format!(
                "CSVW input contains more than {} resources",
                limits.max_artifacts()
            )));
        }
        let mut total = 0usize;
        for (iri, bytes) in &resources {
            validate_absolute_iri(iri, "CSVW resource")?;
            if bytes.len() > limits.max_artifact_bytes() {
                return Err(ProjectionError::limit(
                    "CSVW input resource exceeds the configured artifact limit",
                )
                .at_path(iri));
            }
            total = total
                .checked_add(bytes.len())
                .ok_or_else(|| ProjectionError::limit("CSVW input byte count overflow"))?;
        }
        if total > limits.max_total_bytes() {
            return Err(ProjectionError::limit(
                "CSVW input exceeds the configured total-byte limit",
            ));
        }
        for iri in action_iris(&action) {
            validate_absolute_iri(iri, "CSVW action")?;
            if !resources.contains_key(iri) {
                return Err(ProjectionError::package(format!(
                    "CSVW action resource `{iri}` is absent"
                )));
            }
        }
        Ok(Self { action, resources })
    }

    /// Explicit processing action.
    pub const fn action(&self) -> &CsvwAction {
        &self.action
    }

    /// Fetch a resource by exact absolute IRI.
    pub fn get(&self, iri: &str) -> Option<&[u8]> {
        self.resources.get(iri).map(Vec::as_slice)
    }

    /// Deterministically ordered resources.
    pub fn resources(&self) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
        self.resources
            .iter()
            .map(|(iri, bytes)| (iri.as_str(), bytes.as_slice()))
    }
}

fn action_iris(action: &CsvwAction) -> impl Iterator<Item = &str> {
    let (first, second) = match action {
        CsvwAction::Table {
            table_iri,
            metadata_iri,
        } => (table_iri.as_str(), metadata_iri.as_deref()),
        CsvwAction::Metadata { metadata_iri } => (metadata_iri.as_str(), None),
    };
    std::iter::once(first).chain(second)
}

/// Stable severity for a non-fatal CSVW metadata or row diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvwWarningKind {
    /// Invalid metadata value ignored according to the Recommendation.
    InvalidValue,
    /// Unrecognized property ignored according to the Recommendation.
    UnknownProperty,
    /// Data row does not satisfy a non-fatal datatype or schema constraint.
    Validation,
}

/// Deterministic non-fatal CSVW diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwWarning {
    /// Stable diagnostic category.
    pub kind: CsvwWarningKind,
    /// Resource containing the problem.
    pub resource: String,
    /// JSON path, row/cell coordinate, or empty string for the whole resource.
    pub location: String,
    /// Human-readable detail.
    pub message: String,
}

impl CsvwWarning {
    pub(crate) fn new(
        kind: CsvwWarningKind,
        resource: impl Into<String>,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            resource: resource.into(),
            location: location.into(),
            message: message.into(),
        }
    }
}
