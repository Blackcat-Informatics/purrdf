// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native setup operands retained through the evaluator boundary.

use crate::HnswError;
use purrdf_core::{DistanceMetric, EmbeddingError, FamilyId, TargetSetId, VectorSpaceId};
use purrdf_sparql_eval::{
    EvalError, Kernel, NativeDiagnosticKind, RetainedNativeFailure, WorkspaceCapability,
};

#[derive(Debug)]
pub(super) enum SetupFailure {
    Embedding {
        context: &'static str,
        cause: EmbeddingError,
    },
    Native {
        context: Option<&'static str>,
        cause: HnswError,
    },
    MissingSpace(VectorSpaceId),
    MissingSet(TargetSetId),
    MissingMatrix {
        target_set: TargetSetId,
        vector_space: VectorSpaceId,
    },
    MissingFamily {
        vector_space: VectorSpaceId,
        family: FamilyId,
    },
    UnsupportedMetric {
        vector_space: VectorSpaceId,
        metric: DistanceMetric,
    },
    CandidateBound {
        rows: usize,
        maximum: u64,
    },
    Kernel {
        expected: Kernel,
        actual: Kernel,
    },
    Dimensions {
        actual: usize,
        expected: u32,
    },
    Terms {
        rows: usize,
        terms: usize,
    },
}

impl SetupFailure {
    pub(super) fn into_error(self) -> EvalError {
        let kind = match &self {
            Self::MissingSpace(_)
            | Self::MissingSet(_)
            | Self::MissingMatrix { .. }
            | Self::UnsupportedMetric { .. }
            | Self::CandidateBound { .. }
            | Self::Terms { .. } => NativeDiagnosticKind::Config,
            Self::Embedding { .. }
            | Self::Native { .. }
            | Self::MissingFamily { .. }
            | Self::Kernel { .. }
            | Self::Dimensions { .. } => NativeDiagnosticKind::Data,
        };
        RetainedNativeFailure::error(kind, self, &WorkspaceCapability::resident())
    }
}

impl purrdf_hash::error::StructuredError for SetupFailure {
    purrdf_hash::error_visit! {
        Embedding { context, cause } => "hnsw-setup-embedding" { "context" => Text(context), "cause" => Cause(cause) },
        Native { context, cause } => "hnsw-setup-native" {
            "context" => Value(context.map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Text)), "cause" => Cause(cause)
        },
        MissingSpace(id) => "hnsw-setup-missing-space" { "vector_space" => Record(id) },
        MissingSet(id) => "hnsw-setup-missing-set" { "target_set" => Record(id) },
        MissingMatrix { target_set, vector_space } => "hnsw-setup-missing-matrix" { "target_set" => Record(target_set), "vector_space" => Record(vector_space) },
        MissingFamily { vector_space, family } => "hnsw-setup-missing-family" { "vector_space" => Record(vector_space), "family" => Record(family) },
        UnsupportedMetric { vector_space, metric } => "hnsw-setup-unsupported-metric" { "vector_space" => Record(vector_space), "metric" => Record(metric) },
        CandidateBound { rows, maximum } => "hnsw-setup-candidate-bound" { "rows" => Unsigned(*rows as u128), "maximum" => Unsigned(u128::from(*maximum)) },
        Kernel { expected, actual } => "hnsw-setup-kernel" { "expected" => Record(expected), "actual" => Record(actual) },
        Dimensions { actual, expected } => "hnsw-setup-dimensions" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(u128::from(*expected)) },
        Terms { rows, terms } => "hnsw-setup-terms" { "rows" => Unsigned(*rows as u128), "terms" => Unsigned(*terms as u128) },
    }
}

impl core::fmt::Display for SetupFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Embedding { context, cause } => write!(f, "{context}: {cause}"),
            Self::Native { context, cause } => {
                if let Some(context) = context {
                    write!(f, "{context}: ")?;
                }
                cause.fmt(f)
            }
            Self::MissingSpace(id) => write!(f, "the artifact declares no vector space {id}"),
            Self::MissingSet(id) => write!(f, "the artifact declares no target set {id}"),
            Self::MissingMatrix {
                target_set,
                vector_space,
            } => write!(
                f,
                "the artifact holds no matrix joining target set {target_set} to vector space {vector_space}"
            ),
            Self::MissingFamily {
                vector_space,
                family,
            } => write!(
                f,
                "vector space {vector_space} names family {family}, which the artifact does not hold"
            ),
            Self::UnsupportedMetric {
                vector_space,
                metric,
            } => write!(
                f,
                "vector space {vector_space} declares the caller-defined distance metric {metric:?}, whose parameters are opaque bytes this engine cannot evaluate"
            ),
            Self::CandidateBound { rows, maximum } => write!(
                f,
                "this space holds {rows} row(s), which is more than the {maximum} candidate(s) the configured guard admits"
            ),
            Self::Kernel {
                expected,
                actual: _,
            } => write!(
                f,
                "the HNSW payload was built under a different distance kernel than the family contract declares ({expected:?})"
            ),
            Self::Dimensions { actual, expected } => write!(
                f,
                "the HNSW payload indexes {actual} dimension(s) but the vector space declares {expected}"
            ),
            Self::Terms { rows, terms } => write!(
                f,
                "the HNSW graph holds {rows} row(s) but {terms} term(s) were bound; every row must stand for exactly one RDF term"
            ),
        }
    }
}

impl std::error::Error for SetupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Embedding { cause, .. } => Some(cause),
            Self::Native { cause, .. } => Some(cause),
            Self::MissingSpace(_)
            | Self::MissingSet(_)
            | Self::MissingMatrix { .. }
            | Self::MissingFamily { .. }
            | Self::UnsupportedMetric { .. }
            | Self::CandidateBound { .. }
            | Self::Kernel { .. }
            | Self::Dimensions { .. }
            | Self::Terms { .. } => None,
        }
    }
}
