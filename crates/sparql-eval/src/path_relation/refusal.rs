// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original path construction and invocation failures, before presentation.

use super::PathDirection;
use crate::{EvalError, NativeDiagnosticKind, PfArity, RetainedNativeFailure, WorkspaceCapability};
use purrdf_core::TermValue;

#[derive(Debug)]
pub(super) enum PathRefusal {
    EmptyStep,
    Predicate {
        index: usize,
        actual: TermValue,
    },
    Duplicate {
        index: usize,
        predicate: TermValue,
        direction: PathDirection,
    },
    ZeroMinimum,
    ReversedBounds {
        minimum: u32,
        maximum: u32,
    },
    ZeroPaths,
    ZeroExpansions,
    DenseNodes {
        count: usize,
    },
    DenseStatements {
        count: usize,
    },
    SnapshotMismatch {
        expected_digest: [u8; 32],
        expected_count: usize,
        observed_digest: [u8; 32],
        observed_count: usize,
    },
    Arity {
        shortest: bool,
        expected: PfArity,
        supplied: PfArity,
    },
    ExpansionLimit {
        relation: &'static str,
        limit: u64,
        observed: u64,
        seed: TermValue,
    },
    CandidateLimit {
        relation: &'static str,
        limit: u64,
        observed: u64,
        seed: TermValue,
    },
}

impl PathRefusal {
    pub(super) fn error(self) -> EvalError {
        let kind = match &self {
            Self::EmptyStep
            | Self::Predicate { .. }
            | Self::Duplicate { .. }
            | Self::ZeroMinimum
            | Self::ReversedBounds { .. }
            | Self::ZeroPaths
            | Self::ZeroExpansions => NativeDiagnosticKind::Config,
            Self::DenseNodes { .. }
            | Self::DenseStatements { .. }
            | Self::SnapshotMismatch { .. } => NativeDiagnosticKind::Data,
            Self::Arity { .. } => NativeDiagnosticKind::Function,
            Self::ExpansionLimit { .. } | Self::CandidateLimit { .. } => {
                NativeDiagnosticKind::FunctionOperational
            }
        };
        RetainedNativeFailure::error(kind, self, &WorkspaceCapability::resident())
    }
}

impl purrdf_hash::error::StructuredError for PathDirection {
    purrdf_hash::error_visit! {
        Forward => "path-forward" {},
        Inverse => "path-inverse" {},
    }
}

impl purrdf_hash::error::StructuredError for PathRefusal {
    purrdf_hash::error_visit! {
        EmptyStep => "path-empty-step" {},
        ExpansionLimit { relation, limit, observed, seed } => "path-expansion-limit" {
            "relation" => Code(relation), "limit" => Unsigned(u128::from(*limit)),
            "observed" => Unsigned(u128::from(*observed)), "seed" => Term(seed),
        },
        CandidateLimit { relation, limit, observed, seed } => "path-candidate-limit" {
            "relation" => Code(relation), "limit" => Unsigned(u128::from(*limit)),
            "observed" => Unsigned(u128::from(*observed)), "seed" => Term(seed),
        },
        Predicate { index, actual } => "path-non-iri-predicate" {
            "index" => Unsigned(*index as u128), "actual" => Term(actual),
        },
        Duplicate { index, predicate, direction } => "path-duplicate-alternative" {
            "index" => Unsigned(*index as u128), "predicate" => Term(predicate),
            "direction" => Record(direction),
        },
        ZeroMinimum => "path-zero-minimum" {},
        ReversedBounds { minimum, maximum } => "path-reversed-bounds" {
            "minimum" => Unsigned(u128::from(*minimum)),
            "maximum" => Unsigned(u128::from(*maximum)),
        },
        ZeroPaths => "path-zero-path-limit" {},
        ZeroExpansions => "path-zero-expansion-limit" {},
        DenseNodes { count } => "path-node-index-exhausted" {
            "count" => Unsigned(*count as u128),
        },
        DenseStatements { count } => "path-statement-index-exhausted" {
            "count" => Unsigned(*count as u128),
        },
        SnapshotMismatch { expected_digest, expected_count, observed_digest, observed_count }
            => "path-snapshot-mismatch" {
            "expected_digest" => Bytes(expected_digest),
            "expected_count" => Unsigned(*expected_count as u128),
            "observed_digest" => Bytes(observed_digest),
            "observed_count" => Unsigned(*observed_count as u128),
        },
        Arity { shortest, expected, supplied } => "path-invocation-arity" {
            "shortest" => Boolean(*shortest), "expected" => Record(expected),
            "supplied" => Record(supplied),
        },
    }
}

impl std::error::Error for PathRefusal {}

impl core::fmt::Display for PathRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ExpansionLimit {
                relation,
                limit,
                observed: _,
                seed,
            } => write!(
                f,
                "{relation} traversal exceeded max_expansions_per_invocation ({limit}) while exploring from \
                 seed {seed:?}; this is a resource guard over edges actually traversed, so whether \
                 it fires depends on the row ceiling the engine granted — the same query under \
                 a LIMIT may stop before it"
            ),
            Self::CandidateLimit {
                relation,
                limit,
                observed: _,
                seed,
            } => write!(
                f,
                "{relation} traversal exceeded max_paths_per_seed ({limit}) at seed {seed:?}; this is a resource \
                 guard over candidate walks actually enumerated, so whether it fires depends on \
                 the row ceiling the engine granted — the same query under a LIMIT may stop \
                 before it"
            ),
            Self::EmptyStep => f.write_str(
                "a path step needs at least one (predicate, direction) alternative; an empty \
                 alternation traverses nothing and so defines no hop",
            ),
            Self::Predicate { index, actual } => write!(
                f,
                "path step alternative {index} is {actual:?}; a predicate must be an IRI"
            ),
            Self::Duplicate {
                index,
                predicate,
                direction,
            } => write!(
                f,
                "path step alternative {index} repeats {predicate:?} in the {direction:?} direction; a \
                 duplicated alternative doubles every walk that traverses it, with no \
                 observable difference at the call site"
            ),
            Self::ZeroMinimum => f.write_str(
                "min_hops must be at least 1; a zero-hop path is the identity and has no \
                 witness — no statement was traversed, so there is no hop to bind",
            ),
            Self::ReversedBounds { minimum, maximum } => write!(
                f,
                "min_hops ({minimum}) exceeds max_hops ({maximum}); the accepted walk-length \
                 interval would be empty"
            ),
            Self::ZeroPaths => f.write_str(
                "max_paths_per_seed must be at least 1; a guard of zero fails on the first \
                 candidate walk and so can only ever produce an error",
            ),
            Self::ZeroExpansions => f.write_str(
                "max_expansions_per_invocation must be at least 1; a guard of zero fails on the \
                 first traversed edge and so can only ever produce an error",
            ),
            Self::DenseNodes { count } => write!(
                f,
                "path step spans {count} distinct nodes, which exceeds the dense u32 index space"
            ),
            Self::DenseStatements { count } => write!(
                f,
                "path step spans {count} distinct statements, which exceeds the dense u32 index space"
            ),
            Self::SnapshotMismatch {
                expected_digest,
                expected_count,
                observed_digest,
                observed_count,
            } => write!(
                f,
                "this path snapshot records a different edge set than the dataset presented \
                 carries under the same step: the snapshot digests to {} over {expected_count} edge(s), and \
                 the dataset digests to {} over {observed_count} edge(s). The property-function seam hands a \
                 relation no dataset at evaluation time, so an unnoticed mismatch is answered \
                 silently about the snapshot's edges; rebuild the snapshot from the dataset \
                 being queried",
                purrdf_hash::hex::encode(expected_digest),
                purrdf_hash::hex::encode(observed_digest)
            ),
            Self::Arity {
                shortest,
                expected,
                supplied,
            } => write!(
                f,
                "{}path witness relation expects {expected} argument(s), got {supplied}",
                if *shortest { "shortest " } else { "" }
            ),
        }
    }
}
