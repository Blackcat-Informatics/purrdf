// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed failures of construction, validation, search, and payload decoding.
//!
//! Every failure is a value a caller can match on and act upon. There is no panic path
//! for malformed input and no silent fallback: a stale or corrupt index payload is an
//! error naming the coordinate, not an empty search result that would be
//! indistinguishable from an honest one.

use std::fmt;

mod arithmetic;
pub use arithmetic::ArithmeticRefusal;
mod matrix;
pub use matrix::MatrixRefusal;
mod payload;
pub use payload::PayloadRefusal;
mod guard_payload;
pub use guard_payload::GuardPayloadRefusal;
mod profile;
pub use profile::{CoordinateValue, ProfileRefusal};

/// A result whose error is an [`HnswError`].
pub type Result<T> = std::result::Result<T, HnswError>;

/// One matrix component's original IEEE 754 width and complete bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentValue {
    /// Original `binary32` bits, including sign and any NaN payload.
    Binary32Bits(u32),
    /// Original `binary64` bits, including sign and any NaN payload.
    Binary64Bits(u64),
}

impl purrdf_hash::error::StructuredError for ComponentValue {
    purrdf_hash::error_visit! {
        Binary32Bits(bits) => "hnsw-component-binary32" { "value" => Float32Bits(*bits) },
        Binary64Bits(bits) => "hnsw-component-binary64" { "value" => Float64Bits(*bits) },
    }
}

/// Original container refusal or unresolved coordinates at the HNSW adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingFailure {
    /// Original container failure.
    Container(purrdf_core::EmbeddingError),
    /// Undeclared vector space.
    MissingVectorSpace(purrdf_core::VectorSpaceId),
    /// Undeclared target set.
    MissingTargetSet(purrdf_core::TargetSetId),
    /// No matrix joins these coordinates.
    MissingMatrix {
        /// Requested target set.
        target_set: purrdf_core::TargetSetId,
        /// Requested vector space.
        vector_space: purrdf_core::VectorSpaceId,
    },
    /// Row count exceeds this target's index range.
    RowCount {
        /// Original row count.
        value: u64,
        /// Largest addressable index.
        maximum: usize,
    },
    /// Effective dimension exceeds this target's index range.
    Dimension {
        /// Original dimension.
        value: u32,
        /// Largest addressable index.
        maximum: usize,
    },
}

impl purrdf_hash::error::StructuredError for EmbeddingFailure {
    purrdf_hash::error_visit! {
        Container(cause) => "hnsw-embedding-container" { "cause" => Cause(cause) },
        MissingVectorSpace(id) => "hnsw-embedding-missing-vector-space" { "id" => Record(id) },
        MissingTargetSet(id) => "hnsw-embedding-missing-target-set" { "id" => Record(id) },
        MissingMatrix { target_set, vector_space } => "hnsw-embedding-missing-matrix" {
            "target_set" => Record(target_set), "vector_space" => Record(vector_space),
        },
        RowCount { value, maximum } => "hnsw-embedding-row-count" {
            "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128),
        },
        Dimension { value, maximum } => "hnsw-embedding-dimension" {
            "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128),
        },
    }
}

impl fmt::Display for EmbeddingFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(cause) => cause.fmt(f),
            Self::MissingVectorSpace(id) => write!(f, "the artifact declares no vector space {id}"),
            Self::MissingTargetSet(id) => write!(f, "the artifact declares no target set {id}"),
            Self::MissingMatrix {
                target_set,
                vector_space,
            } => write!(
                f,
                "the artifact holds no matrix joining target set {target_set} to vector space {vector_space}"
            ),
            Self::RowCount {
                value: _,
                maximum: _,
            } => f.write_str("the matrix row count does not fit this platform's index range"),
            Self::Dimension {
                value: _,
                maximum: _,
            } => f.write_str("the effective dimension does not fit this platform's index range"),
        }
    }
}

purrdf_lex::error_source!(EmbeddingFailure { Container });

/// Everything that can go wrong building, decoding, or searching an [`crate::HnswIndex`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HnswError {
    /// Native traversal allocation or admission failed under the query account.
    Workspace(purrdf_sparql_eval::EvalError),

    /// A parameter was outside its admitted range.
    InvalidParameter {
        /// Which parameter (`M`, `M0`, `ef_construction`, `ef_search`).
        name: &'static str,
        /// The original supplied value.
        value: usize,
        /// Inclusive lower bound at the failing comparison.
        minimum: usize,
        /// Why the value is refused.
        reason: &'static str,
    },

    /// The parameter set is well-formed on its own but cannot describe this matrix.
    ParameterValidation {
        /// Original matrix or connectivity refusal.
        cause: MatrixRefusal,
    },

    /// A vector component is not finite.
    NonFiniteComponent {
        /// The row holding the component.
        row: usize,
        /// The component's column.
        column: usize,
        /// The original component, before widening or formatting.
        value: ComponentValue,
    },

    /// A vector has zero L2 norm under a metric that divides by it.
    ZeroNorm {
        /// The offending row.
        row: usize,
    },

    /// A distance left the finite `binary64` range.
    NonFiniteDistance {
        /// The candidate row whose distance overflowed.
        row: usize,
    },

    /// A row index is outside the matrix.
    RowOutOfBounds {
        /// The requested index.
        index: usize,
        /// The largest valid index.
        max: usize,
    },

    /// The requested distance metric has no kernel this crate can evaluate.
    UnsupportedMetric {
        /// Original metric identity, parameter encoding and canonical bytes.
        metric: purrdf_core::DistanceMetric,
    },

    /// A search or payload operation needed a graph that holds no nodes.
    EmptyGraph,

    /// A payload is structurally malformed.
    InvalidPayload {
        /// What is wrong with the bytes.
        reason: PayloadRefusal,
    },

    /// A payload declares a format version this decoder does not implement.
    VersionMismatch {
        /// The version this decoder implements.
        expected: u32,
        /// The version found in the bytes.
        actual: u32,
    },

    /// A payload's header records an arithmetic code this index type does not compute
    /// with.
    ///
    /// The code is the arithmetic the recorded distances were produced under. Reading
    /// them as another arithmetic's would compare numbers from two different laws, so
    /// the payload is refused by name rather than decoded; a zero (the reserved value
    /// of the previous image version) names no arithmetic at all.
    ArithmeticMismatch {
        /// The identifier of the arithmetic this index computes with.
        arithmetic: &'static str,
        /// The code found in the bytes.
        actual: u32,
    },

    /// A payload's header records a dispatch path of its arithmetic that this process
    /// cannot run.
    ///
    /// A reassociated arithmetic's bits depend on the compilation that produced them, so
    /// an image records the path its distances came from, and only that path can decode
    /// it, verify its rebuild or search it: another path would recompute every distance
    /// with different last bits. Any process that can run the recorded path does so,
    /// even where a wider one is available; this is raised only when the recorded path's
    /// compilation is not in this build (it belongs to another target) or the processor
    /// does not report a feature it needs. That is not tampering and not an honest "no",
    /// so it is refused by name rather than answered `false`. An arithmetic whose bits
    /// are the same on every path records one code and never raises this.
    ArithmeticPathUnavailable {
        /// The image code the payload records.
        recorded: u32,
        /// The image code of the widest dispatch path this process runs: the one a new
        /// build here would record.
        available: u32,
    },

    /// A payload's header records a build shape for its arithmetic that is not this
    /// build's.
    ///
    /// A reassociated arithmetic's bits depend on what its dispatch path compiled to, and
    /// that depends on the build as well as the path: its target architecture and target
    /// features, and its [`BuildIdentity`](purrdf_core::distance::BuildIdentity) --
    /// compiler release and LLVM version, target CPU, optimisation level, debug assertions
    /// and codegen flags. The same path contracts to fused multiply-add in one build and not
    /// in another. So the image records the shape of the build that computed it, and a
    /// build of another shape refuses it by name rather than searching it, or rebuilding it
    /// and answering `false`, with bits another compilation would produce. Within a build
    /// of the recorded shape a rebuild that differs is the honest `false`.
    ArithmeticBuildMismatch {
        /// The shape the payload records.
        recorded: purrdf_core::distance::BuildShape,
        /// This build's shape.
        here: purrdf_core::distance::BuildShape,
    },

    /// The calling thread's floating-point environment is not the IEEE-754 one the
    /// distance arithmetic defines its results under.
    ///
    /// A build, rebuild or search computed under a flush-to-zero or re-rounding
    /// environment would produce different distances, and a different graph, with
    /// nothing in the result to say so.
    FloatEnvironment(purrdf_core::distance::FloatEnvironmentError),

    /// A checked arithmetic operation overflowed.
    ArithmeticOverflow(ArithmeticRefusal),

    /// The index footprint would exceed the target's addressable memory.
    AddressSpaceExceeded {
        /// Bytes the index would need.
        required: u64,
        /// Bytes the target can address.
        maximum: u64,
    },

    /// A PURREMB artifact held no derived index naming this profile.
    MissingIndexGuard {
        /// What was searched and what was expected.
        implementations: [&'static str; 2],
    },

    /// A PURREMB artifact held more than one derived index naming this profile.
    ///
    /// Two HNSW guards in one artifact cannot be told apart by a query that names the
    /// profile alone, and choosing one silently would answer from an index the caller did
    /// not name, so the ambiguity is refused rather than resolved.
    AmbiguousIndexGuard {
        /// How many matching guards were found.
        count: usize,
    },

    /// A guard claims this profile but its contents disagree with the profile spec.
    GuardProfile {
        /// Which field disagrees and how. The cold payload is boxed at the
        /// physically fallible resident profile-construction boundary.
        cause: Box<ProfileRefusal>,
    },

    /// The index payload is detached or absent, so it cannot be verified or searched.
    PayloadUnavailable {
        /// Why the bytes could not be obtained.
        cause: GuardPayloadRefusal,
    },

    /// The inline payload does not match the guard's committed SHA-256 and length.
    PayloadCommitment {
        /// What the guard committed and what the bytes actually are.
        cause: GuardPayloadRefusal,
    },

    /// A PURREMB container operation failed.
    Embedding {
        /// Original container or adapter cause.
        cause: EmbeddingFailure,
    },
}

impl purrdf_hash::error::StructuredError for HnswError {
    purrdf_hash::error_visit! {
        Workspace(cause) => "hnsw-workspace" { "cause" => Cause(cause) },
        InvalidParameter { name, value, minimum, reason } => "hnsw-invalid-parameter" {
            "name" => Code(name), "value" => Unsigned(*value as u128),
            "minimum" => Unsigned(*minimum as u128), "reason" => Text(reason),
        },
        ParameterValidation { cause } => "hnsw-parameter-validation" { "cause" => Cause(cause) },
        NonFiniteComponent { row, column, value } => "hnsw-non-finite-component" {
            "row" => Unsigned(*row as u128), "column" => Unsigned(*column as u128),
            "value" => Record(value),
        },
        ZeroNorm { row } => "hnsw-zero-norm" { "row" => Unsigned(*row as u128) },
        NonFiniteDistance { row } => "hnsw-non-finite-distance" { "row" => Unsigned(*row as u128) },
        RowOutOfBounds { index, max } => "hnsw-row-out-of-bounds" { "index" => Unsigned(*index as u128), "max" => Unsigned(*max as u128) },
        UnsupportedMetric { metric } => "hnsw-unsupported-metric" { "metric" => Record(metric) },
        EmptyGraph => "hnsw-empty-graph" {},
        InvalidPayload { reason } => "hnsw-invalid-payload" { "reason" => Cause(reason) },
        VersionMismatch { expected, actual } => "hnsw-version-mismatch" { "expected" => Unsigned(u128::from(*expected)), "actual" => Unsigned(u128::from(*actual)) },
        ArithmeticMismatch { arithmetic, actual } => "hnsw-arithmetic-mismatch" { "arithmetic" => Code(arithmetic), "actual" => Unsigned(u128::from(*actual)) },
        ArithmeticPathUnavailable { recorded, available } => "hnsw-arithmetic-path-unavailable" {
            "recorded" => Unsigned(u128::from(*recorded)), "available" => Unsigned(u128::from(*available)),
        },
        ArithmeticBuildMismatch { recorded, here } => "hnsw-arithmetic-build-mismatch" { "recorded" => Record(recorded), "here" => Record(here) },
        FloatEnvironment(cause) => "hnsw-float-environment" { "cause" => Cause(cause) },
        ArithmeticOverflow(cause) => "hnsw-arithmetic-overflow" { "cause" => Cause(cause) },
        AddressSpaceExceeded { required, maximum } => "hnsw-address-space-exceeded" {
            "required" => ByteCount(u128::from(*required)), "maximum" => ByteCount(u128::from(*maximum)),
        },
        MissingIndexGuard { implementations } => "hnsw-missing-index-guard" {
            "implementations" => List(implementations.iter().map(|value| purrdf_hash::error::ErrorValue::Code(value))),
        },
        AmbiguousIndexGuard { count } => "hnsw-ambiguous-index-guard" { "count" => Unsigned(*count as u128) },
        GuardProfile { cause } => "hnsw-guard-profile" { "cause" => Cause(cause.as_ref()) },
        PayloadUnavailable { cause } => "hnsw-payload-unavailable" { "cause" => Cause(cause) },
        PayloadCommitment { cause } => "hnsw-payload-commitment" { "cause" => Cause(cause) },
        Embedding { cause } => "hnsw-embedding" { "cause" => Cause(cause) },
    }
}

impl fmt::Display for HnswError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace(error) => error.fmt(f),
            Self::InvalidParameter {
                name,
                value,
                minimum: _,
                reason,
            } => write!(f, "invalid parameter {name} = {value}: {reason}"),
            Self::ParameterValidation { cause } => {
                write!(f, "parameters do not describe this matrix: {cause}")
            }
            Self::NonFiniteComponent {
                row,
                column,
                value: _,
            } => write!(
                f,
                "row {row} component {column} is not finite; a distance over it has no \
                 defined ranking"
            ),
            Self::ZeroNorm { row } => write!(
                f,
                "row {row} has zero L2 norm and the declared metric divides by it"
            ),
            Self::NonFiniteDistance { row } => write!(
                f,
                "the distance to row {row} left the finite binary64 range"
            ),
            Self::RowOutOfBounds { index, max } => {
                write!(
                    f,
                    "row {index} is out of bounds (largest valid row is {max})"
                )
            }
            Self::UnsupportedMetric { metric } => write!(
                f,
                "the distance metric {metric:?} has no evaluable kernel; only cosine, \
                 negative dot, and squared Euclidean are ranked here"
            ),
            Self::EmptyGraph => write!(f, "the graph holds no nodes"),
            Self::InvalidPayload { reason } => write!(f, "invalid payload: {reason}"),
            Self::VersionMismatch { expected, actual } => write!(
                f,
                "payload version {actual} is not the implemented version {expected}"
            ),
            Self::ArithmeticMismatch { arithmetic, actual } => write!(
                f,
                "the payload's arithmetic field is {actual}, which is not a code of the \
                 {arithmetic} distance arithmetic this index computes with"
            ),
            Self::ArithmeticPathUnavailable {
                recorded,
                available,
            } => write!(
                f,
                "the payload's distances were computed on the dispatch path recorded as \
                 arithmetic code {recorded} ({}), which this process cannot run: its \
                 compilation is not in this build, or the processor lacks a feature it needs \
                 (the widest path this process runs is code {available}, {}); an image whose \
                 arithmetic depends on its dispatch path is reproducible only on the path \
                 that built it",
                crate::profile::path_label(*recorded),
                crate::profile::path_label(*available)
            ),
            Self::ArithmeticBuildMismatch { recorded, here } => write!(
                f,
                "the payload's distances were computed by another build ({recorded}) than this \
                 one ({here}): {}; what a dispatch path compiles to depends on the build's \
                 target architecture and features, compiler, target CPU, optimisation level and \
                 codegen flags, so the image is reproducible only by a build of its own shape",
                if recorded.bits() == here.bits() {
                    "the target and features agree and the build identity differs"
                } else if recorded.identity() == here.identity() {
                    "the build identity agrees and the target or features differ"
                } else {
                    "the target or features and the build identity differ"
                }
            ),
            Self::FloatEnvironment(error) => write!(
                f,
                "the floating-point environment cannot run the distance arithmetic: {error}"
            ),
            Self::ArithmeticOverflow(cause) => cause.fmt(f),
            Self::AddressSpaceExceeded { required, maximum } => write!(
                f,
                "the index would need {required} bytes, which exceeds the {maximum} \
                 addressable on this target"
            ),
            Self::MissingIndexGuard {
                implementations: [exact, reassociated],
            } => {
                write!(
                    f,
                    "no HNSW derived index: the artifact holds no derived index whose implementation is `{exact}` or `{reassociated}`; register one with `guard::derived_index`"
                )
            }
            Self::AmbiguousIndexGuard { count } => write!(
                f,
                "the artifact holds {count} HNSW derived indexes; a query that names the \
                 profile alone cannot choose one without answering from an index the caller \
                 did not name"
            ),
            Self::GuardProfile { cause } => {
                write!(f, "the HNSW guard disagrees with its profile: {cause}")
            }
            Self::PayloadUnavailable { cause } => {
                write!(f, "the HNSW payload is unavailable: {cause}")
            }
            Self::PayloadCommitment { cause } => {
                write!(f, "the HNSW payload commitment fails: {cause}")
            }
            Self::Embedding { cause } => {
                write!(f, "the PURREMB container refused the HNSW index: {cause}")
            }
        }
    }
}

impl std::error::Error for HnswError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Workspace(error) => Some(error),
            Self::FloatEnvironment(error) => Some(error),
            Self::ArithmeticOverflow(cause) => Some(cause),
            Self::Embedding { cause } => Some(cause),
            Self::ParameterValidation { cause } => Some(cause),
            Self::InvalidPayload { reason } => Some(reason),
            Self::PayloadUnavailable { cause } | Self::PayloadCommitment { cause } => Some(cause),
            Self::GuardProfile { cause } => Some(cause.as_ref()),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(HnswError {
    Workspace(purrdf_sparql_eval::EvalError),
    FloatEnvironment(purrdf_core::distance::FloatEnvironmentError),
    ArithmeticOverflow(ArithmeticRefusal),
});

impl From<purrdf_core::EmbeddingError> for HnswError {
    fn from(error: purrdf_core::EmbeddingError) -> Self {
        // A refused float environment is the same refusal whichever layer met it, and
        // keeps its name rather than becoming an artifact defect.
        if let purrdf_core::EmbeddingError::FloatEnvironment(refusal) = error {
            return Self::FloatEnvironment(refusal);
        }
        Self::Embedding {
            cause: EmbeddingFailure::Container(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::distance::{BuildIdentity, BuildShape};

    /// An `x86_64` shape with `features`, under `identity`.
    fn shape(features: &[&str], identity: BuildIdentity) -> BuildShape {
        BuildShape::encode("x86_64", features, identity)
    }

    #[test]
    fn every_variant_renders_without_its_debug_representation() {
        // A message that fell back to `{:?}` would leak a Rust type name into a user-facing
        // diagnostic, so each arm is asserted to produce a distinct, human sentence.
        let cases = [
            HnswError::InvalidParameter {
                name: "M",
                value: 1,
                minimum: 2,
                reason: "must be at least 2",
            },
            HnswError::ParameterValidation {
                cause: MatrixRefusal::ZeroRows { dimensions: 8 },
            },
            HnswError::NonFiniteComponent {
                row: 0,
                column: 1,
                value: ComponentValue::Binary32Bits(f32::NAN.to_bits()),
            },
            HnswError::ZeroNorm { row: 2 },
            HnswError::NonFiniteDistance { row: 3 },
            HnswError::RowOutOfBounds { index: 9, max: 8 },
            HnswError::UnsupportedMetric {
                metric: purrdf_core::DistanceMetric::Extension {
                    identifier: "https://example.org/metric".to_owned(),
                    parameter_encoding: "application/cbor".to_owned(),
                    parameters: vec![1, 2, 3],
                },
            },
            HnswError::EmptyGraph,
            HnswError::InvalidPayload {
                reason: PayloadRefusal::Truncated {
                    available: 0,
                    at: 0,
                    count: 8,
                },
            },
            HnswError::VersionMismatch {
                expected: 1,
                actual: 2,
            },
            HnswError::ArithmeticMismatch {
                arithmetic: "binary64-lane16-tree-v1",
                actual: 0,
            },
            HnswError::ArithmeticPathUnavailable {
                recorded: 2,
                available: 3,
            },
            HnswError::ArithmeticBuildMismatch {
                recorded: shape(&["sse2"], BuildIdentity::here()),
                here: shape(&["sse2", "fma"], BuildIdentity::here()),
            },
            HnswError::ArithmeticBuildMismatch {
                recorded: shape(&["sse2"], BuildIdentity::from_digest(7)),
                here: shape(&["sse2"], BuildIdentity::here()),
            },
            HnswError::ArithmeticBuildMismatch {
                recorded: shape(&["sse2"], BuildIdentity::from_digest(7)),
                here: shape(&["sse2", "fma"], BuildIdentity::here()),
            },
            HnswError::FloatEnvironment(
                purrdf_core::distance::FloatEnvironmentError::FlushToZero {
                    evidence: purrdf_core::distance::FloatEnvironmentEvidence::Register {
                        name: "MXCSR",
                        bits: 0x9fc0,
                    },
                },
            ),
            HnswError::ArithmeticOverflow(ArithmeticRefusal::MatrixElements {
                rows: usize::MAX,
                dimensions: 2,
            }),
            HnswError::AddressSpaceExceeded {
                required: 10,
                maximum: 5,
            },
            HnswError::MissingIndexGuard {
                implementations: [
                    crate::IMPLEMENTATION_ID,
                    crate::IMPLEMENTATION_ID_REASSOCIATED,
                ],
            },
            HnswError::AmbiguousIndexGuard { count: 2 },
            crate::profile::profile_error(ProfileRefusal::MissingParameter { name: "M0" }),
            HnswError::PayloadUnavailable {
                cause: GuardPayloadRefusal::Detached {
                    guard: purrdf_core::IndexId::from_raw([1; 32]),
                },
            },
            HnswError::PayloadCommitment {
                cause: GuardPayloadRefusal::Length {
                    guard: purrdf_core::IndexId::from_raw([1; 32]),
                    expected: 10,
                    actual: 9,
                },
            },
            HnswError::Embedding {
                cause: EmbeddingFailure::Container(purrdf_core::EmbeddingError::Malformed(
                    "structure refused",
                )),
            },
        ];
        // The build refusal names both shapes as feature lists and both identities, and
        // says which of the two parts differs.
        let mismatches: Vec<String> = cases
            .iter()
            .filter(|case| matches!(case, HnswError::ArithmeticBuildMismatch { .. }))
            .map(ToString::to_string)
            .collect();
        let here = BuildIdentity::here().to_string();
        let foreign = BuildIdentity::from_digest(7).to_string();
        assert!(
            mismatches[0].contains("x86_64 with sse2, under")
                && mismatches[0].contains("x86_64 with sse2, fma, under")
                && mismatches[0].contains(&here)
                && mismatches[0].contains("the build identity agrees and the target"),
            "{}",
            mismatches[0]
        );
        assert!(
            mismatches[1].contains(&foreign)
                && mismatches[1].contains(&here)
                && mismatches[1].contains("the target and features agree and the build identity"),
            "{}",
            mismatches[1]
        );
        assert!(
            mismatches[2].contains("the target or features and the build identity differ"),
            "{}",
            mismatches[2]
        );
        let mut seen = purrdf_core::FastSet::default();
        for error in cases {
            let message = error.to_string();
            assert!(
                !message.contains("HnswError"),
                "leaked type name: {message}"
            );
            assert!(seen.insert(message.clone()), "duplicate message: {message}");
        }
    }
}
