// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original payload refusals carried through graph and streaming diagnostics.

use crate::codec::CodecError;
use core::fmt;
use purrdf_lex::cbor::{DecodeError, Value};

#[derive(Debug)]
pub(super) enum PayloadError {
    Unavailable {
        reason: &'static str,
        detail: PayloadCause,
    },
    Damaged(PayloadCause),
    Budget(PayloadCause),
}

#[derive(Debug)]
pub(super) enum PayloadCause {
    Codec(CodecError),
    MissingCatalog(Value),
    TransformedNotBytes,
    Cbor(DecodeError),
    EncodedBlobLimit { actual: usize, limit: usize },
    DecodedBlobLimit { actual: usize, limit: usize },
    SnapshotBlobLimit { actual: usize, limit: usize },
}

impl From<CodecError> for PayloadError {
    fn from(error: CodecError) -> Self {
        match error {
            error @ CodecError::Unavailable { reason, .. } => Self::Unavailable {
                reason,
                detail: PayloadCause::Codec(error),
            },
            error @ CodecError::Failed(_) => Self::Damaged(PayloadCause::Codec(error)),
            error @ CodecError::Limit { .. } => Self::Budget(PayloadCause::Codec(error)),
        }
    }
}

impl PayloadError {
    pub(super) fn opaque_reason(&self) -> &'static str {
        match self {
            Self::Unavailable { reason, .. } => reason,
            Self::Damaged(_) => "damaged",
            Self::Budget(_) => "over-budget",
        }
    }

    pub(super) fn diagnostic_code(&self, budget_code: &'static str) -> &'static str {
        match self {
            Self::Unavailable { reason, .. } => super::diag_code_for(reason),
            Self::Damaged(_) => "DamagedFrame",
            Self::Budget(_) => budget_code,
        }
    }

    pub(super) fn diagnostic_detail(&self) -> String {
        match self {
            Self::Damaged(_) => format!("payload decode failed: {self}"),
            Self::Unavailable { .. } | Self::Budget(_) => self.to_string(),
        }
    }
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { detail, .. } | Self::Damaged(detail) | Self::Budget(detail) => {
                detail.fmt(f)
            }
        }
    }
}

impl std::error::Error for PayloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Unavailable { detail, .. } | Self::Damaged(detail) | Self::Budget(detail) => {
                detail
            }
        })
    }
}

impl fmt::Display for PayloadCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Codec(CodecError::Unavailable { detail, .. }) => detail.fmt(f),
            Self::Codec(error) => error.fmt(f),
            Self::MissingCatalog(value) => write!(f, "codec id {value:?} not in catalog"),
            Self::TransformedNotBytes => f.write_str("transformed frame 'd' must be a byte string"),
            Self::Cbor(error) => error.fmt(f),
            Self::EncodedBlobLimit { limit, .. } => write!(f, "encoded blob exceeds {limit} bytes"),
            Self::DecodedBlobLimit { limit, .. } => write!(f, "decoded blob exceeds {limit} bytes"),
            Self::SnapshotBlobLimit { limit, .. } => {
                write!(f, "snapshot blob exceeds {limit} bytes")
            }
        }
    }
}

purrdf_lex::error_source!(PayloadCause { Codec, Cbor });

impl purrdf_hash::error::StructuredError for PayloadError {
    purrdf_hash::error_visit! {
        Unavailable { reason, detail } => "gts-payload-unavailable" {
            "reason" => Code(reason), "detail" => Cause(detail),
        },
        Damaged(detail) => "gts-payload-damaged" { "detail" => Cause(detail) },
        Budget(detail) => "gts-payload-budget" { "detail" => Cause(detail) },
    }
}

impl purrdf_hash::error::StructuredError for PayloadCause {
    purrdf_hash::error_visit! {
        Codec(error) => "gts-payload-codec" { "cause" => Cause(error) },
        MissingCatalog(value) => "gts-payload-missing-catalog" { "id" => Record(value) },
        TransformedNotBytes => "gts-payload-transformed-not-bytes" {},
        Cbor(error) => "gts-payload-cbor" { "cause" => Cause(error) },
        EncodedBlobLimit { actual, limit } => "gts-payload-encoded-blob-limit" {
            "actual" => ByteCount(*actual as u128), "limit" => ByteCount(*limit as u128),
        },
        DecodedBlobLimit { actual, limit } => "gts-payload-decoded-blob-limit" {
            "actual" => ByteCount(*actual as u128), "limit" => ByteCount(*limit as u128),
        },
        SnapshotBlobLimit { actual, limit } => "gts-payload-snapshot-blob-limit" {
            "actual" => ByteCount(*actual as u128), "limit" => ByteCount(*limit as u128),
        },
    }
}
