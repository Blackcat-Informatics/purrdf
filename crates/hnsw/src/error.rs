// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed failures of construction, validation, search, and payload decoding.
//!
//! Every failure is a value a caller can match on and act upon. There is no panic path
//! for malformed input and no silent fallback: a stale or corrupt index payload is an
//! error naming the coordinate, not an empty search result that would be
//! indistinguishable from an honest one.

use std::fmt;

/// A result whose error is an [`HnswError`].
pub type Result<T> = std::result::Result<T, HnswError>;

/// Everything that can go wrong building, decoding, or searching an [`crate::HnswIndex`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HnswError {
    /// A parameter was outside its admitted range.
    InvalidParameter {
        /// Which parameter (`M`, `M0`, `ef_construction`, `ef_search`).
        name: &'static str,
        /// The value that was supplied, rendered.
        value: String,
        /// Why the value is refused.
        reason: &'static str,
    },

    /// The parameter set is well-formed on its own but cannot describe this matrix.
    ParameterValidation {
        /// What the parameters and the matrix disagree about.
        description: String,
    },

    /// A vector component is not finite.
    NonFiniteComponent {
        /// The row holding the component.
        row: usize,
        /// The component's column.
        column: usize,
    },

    /// A vector has zero L2 norm under a metric that divides by it.
    ZeroNorm {
        /// The offending row.
        row: usize,
    },

    /// A distance left the finite binary64 range.
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
        /// The metric's stable identifier, rendered.
        metric: String,
    },

    /// A search or payload operation needed a graph that holds no nodes.
    EmptyGraph,

    /// A payload is structurally malformed.
    InvalidPayload {
        /// What is wrong with the bytes.
        reason: String,
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
    ArithmeticOverflow,

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
        description: String,
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
        /// Which field disagrees and how.
        description: String,
    },

    /// The index payload is detached or absent, so it cannot be verified or searched.
    PayloadUnavailable {
        /// Why the bytes could not be obtained.
        description: String,
    },

    /// The inline payload does not match the guard's committed SHA-256 and length.
    PayloadCommitment {
        /// What the guard committed and what the bytes actually are.
        description: String,
    },

    /// A PURREMB container operation failed.
    Embedding {
        /// The container error, rendered.
        description: String,
    },
}

impl fmt::Display for HnswError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter {
                name,
                value,
                reason,
            } => write!(f, "invalid parameter {name} = {value}: {reason}"),
            Self::ParameterValidation { description } => {
                write!(f, "parameters do not describe this matrix: {description}")
            }
            Self::NonFiniteComponent { row, column } => write!(
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
                "the distance metric {metric} has no evaluable kernel; only cosine, \
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
            Self::ArithmeticOverflow => write!(f, "index arithmetic overflowed"),
            Self::AddressSpaceExceeded { required, maximum } => write!(
                f,
                "the index would need {required} bytes, which exceeds the {maximum} \
                 addressable on this target"
            ),
            Self::MissingIndexGuard { description } => {
                write!(f, "no HNSW derived index: {description}")
            }
            Self::AmbiguousIndexGuard { count } => write!(
                f,
                "the artifact holds {count} HNSW derived indexes; a query that names the \
                 profile alone cannot choose one without answering from an index the caller \
                 did not name"
            ),
            Self::GuardProfile { description } => {
                write!(
                    f,
                    "the HNSW guard disagrees with its profile: {description}"
                )
            }
            Self::PayloadUnavailable { description } => {
                write!(f, "the HNSW payload is unavailable: {description}")
            }
            Self::PayloadCommitment { description } => {
                write!(f, "the HNSW payload commitment fails: {description}")
            }
            Self::Embedding { description } => {
                write!(
                    f,
                    "the PURREMB container refused the HNSW index: {description}"
                )
            }
        }
    }
}

impl std::error::Error for HnswError {}

impl From<purrdf_core::distance::FloatEnvironmentError> for HnswError {
    fn from(error: purrdf_core::distance::FloatEnvironmentError) -> Self {
        Self::FloatEnvironment(error)
    }
}

impl From<purrdf_core::EmbeddingError> for HnswError {
    fn from(error: purrdf_core::EmbeddingError) -> Self {
        // A refused float environment is the same refusal whichever layer met it, and
        // keeps its name rather than becoming an artifact defect.
        if let purrdf_core::EmbeddingError::FloatEnvironment(refusal) = error {
            return Self::FloatEnvironment(refusal);
        }
        Self::Embedding {
            description: error.to_string(),
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
                value: "1".to_owned(),
                reason: "must be at least 2",
            },
            HnswError::ParameterValidation {
                description: "zero rows".to_owned(),
            },
            HnswError::NonFiniteComponent { row: 0, column: 1 },
            HnswError::ZeroNorm { row: 2 },
            HnswError::NonFiniteDistance { row: 3 },
            HnswError::RowOutOfBounds { index: 9, max: 8 },
            HnswError::UnsupportedMetric {
                metric: "extension".to_owned(),
            },
            HnswError::EmptyGraph,
            HnswError::InvalidPayload {
                reason: "short".to_owned(),
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
            HnswError::ArithmeticOverflow,
            HnswError::AddressSpaceExceeded {
                required: 10,
                maximum: 5,
            },
            HnswError::MissingIndexGuard {
                description: "no guard names the profile".to_owned(),
            },
            HnswError::AmbiguousIndexGuard { count: 2 },
            HnswError::GuardProfile {
                description: "parameter encoding differs".to_owned(),
            },
            HnswError::PayloadUnavailable {
                description: "the payload is detached".to_owned(),
            },
            HnswError::PayloadCommitment {
                description: "the length differs".to_owned(),
            },
            HnswError::Embedding {
                description: "structure refused".to_owned(),
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
        let mut seen = std::collections::HashSet::new();
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
