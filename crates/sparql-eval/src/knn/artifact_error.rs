// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-bound failures while creating a space from a PURREMB artifact.

use core::fmt;

use purrdf_core::EmbeddingError;

use crate::{EvalError, NativeDiagnosticKind, RetainedNativeFailure, WorkspaceCapability};

#[derive(Clone, Copy, Debug)]
pub(super) enum ArtifactStage {
    Open,
    Verify,
    Metric,
    EffectiveMatrix,
    ScalarType,
    Row(usize),
}

impl purrdf_hash::error::StructuredError for ArtifactStage {
    purrdf_hash::error_visit! {
        Open => "knn-artifact-open" {},
        Verify => "knn-artifact-verify" {},
        Metric => "knn-artifact-metric" {},
        EffectiveMatrix => "knn-artifact-effective-matrix" {},
        ScalarType => "knn-artifact-scalar-type" {},
        Row(row) => "knn-artifact-row" { "row" => Unsigned(*row as u128) },
    }
}

#[derive(Debug)]
struct ArtifactReadFailure {
    stage: ArtifactStage,
    cause: EmbeddingError,
}

impl ArtifactStage {
    pub(super) fn legacy(self, cause: EmbeddingError) -> EvalError {
        EvalError::data(ArtifactReadFailure { stage: self, cause }.to_string())
    }

    pub(super) fn retained(self, cause: EmbeddingError) -> EvalError {
        RetainedNativeFailure::error(
            NativeDiagnosticKind::Data,
            ArtifactReadFailure { stage: self, cause },
            &WorkspaceCapability::resident(),
        )
    }
}

impl purrdf_hash::error::StructuredError for ArtifactReadFailure {
    purrdf_hash::error_visit! {
        @record self { stage, cause } => "knn-artifact-read-failure" {
            "stage" => Record(&self.stage),
            "cause" => Cause(&self.cause),
        }
    }
}

impl fmt::Display for ArtifactReadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.stage {
            ArtifactStage::Open => f.write_str("the PURREMB artifact is unreadable")?,
            ArtifactStage::Verify => f.write_str("the PURREMB artifact does not verify")?,
            ArtifactStage::Metric => f.write_str("the family's declared metric is unusable")?,
            ArtifactStage::EffectiveMatrix => f.write_str("the effective matrix is unreadable")?,
            ArtifactStage::ScalarType => f.write_str("the matrix's scalar type is unreadable")?,
            ArtifactStage::Row(row) => write!(f, "row {row} of the matrix is unreadable")?,
        }
        write!(f, ": {}", self.cause)
    }
}

impl std::error::Error for ArtifactReadFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
