// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original payload availability and commitment refusals.

use purrdf_core::{ContentDigest, IndexId};

/// A guard's unavailable or incorrectly committed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardPayloadRefusal {
    /// An inline payload has no borrowed byte section.
    InlineMissing {
        /// Original guard identity.
        guard: IndexId,
    },
    /// The payload lives in an external binding.
    Detached {
        /// Original guard identity.
        guard: IndexId,
    },
    /// Byte length cannot be represented by the wire's counter.
    LengthPlatform {
        /// Original guard identity.
        guard: IndexId,
        /// Original byte length.
        actual: usize,
        /// Maximum wire length.
        maximum: u64,
    },
    /// Observed and committed lengths disagree.
    Length {
        /// Original guard identity.
        guard: IndexId,
        /// Committed byte length.
        expected: u64,
        /// Observed byte length.
        actual: u64,
    },
    /// Observed and committed content digests disagree.
    Digest {
        /// Original guard identity.
        guard: IndexId,
        /// Committed content digest.
        expected: ContentDigest,
        /// Observed content digest.
        actual: ContentDigest,
    },
}

impl purrdf_hash::error::StructuredError for GuardPayloadRefusal {
    purrdf_hash::error_visit! {
        InlineMissing { guard } => "hnsw-payload-inline-missing" { "guard" => Record(guard) },
        Detached { guard } => "hnsw-payload-detached" { "guard" => Record(guard) },
        LengthPlatform { guard, actual, maximum } => "hnsw-payload-length-platform" {
            "guard" => Record(guard), "actual" => ByteCount(*actual as u128), "maximum" => ByteCount(u128::from(*maximum)),
        },
        Length { guard, expected, actual } => "hnsw-payload-length-mismatch" {
            "guard" => Record(guard), "expected" => ByteCount(u128::from(*expected)), "actual" => ByteCount(u128::from(*actual)),
        },
        Digest { guard, expected, actual } => "hnsw-payload-digest-mismatch" {
            "guard" => Record(guard), "expected" => Bytes(expected.as_bytes()), "actual" => Bytes(actual.as_bytes()),
        },
    }
}

impl core::fmt::Display for GuardPayloadRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InlineMissing { guard } => write!(
                f,
                "guard {guard} declares an inline payload but the artifact lends no bytes"
            ),
            Self::Detached { guard } => write!(
                f,
                "guard {guard} stores its payload in an external binding; resolve that binding and verify it before searching"
            ),
            Self::LengthPlatform {
                guard,
                actual: _,
                maximum: _,
            } => write!(f, "guard {guard} payload length does not fit u64"),
            Self::Length {
                guard,
                expected,
                actual,
            } => write!(
                f,
                "guard {guard} commits {expected} payload byte(s) but {actual} are present"
            ),
            Self::Digest {
                guard,
                expected,
                actual,
            } => write!(
                f,
                "guard {guard} commits payload SHA-256 {expected} but the bytes hash to {actual}"
            ),
        }
    }
}

impl std::error::Error for GuardPayloadRefusal {}
