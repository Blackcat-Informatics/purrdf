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
    Stream(purrdf_hash::error::IoFailure),
    MissingCatalog(Value),
    TransformedNotBytes,
    BlobNotBytes,
    BlobMissingIdentity,
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

impl From<std::io::Error> for PayloadError {
    fn from(error: std::io::Error) -> Self {
        let class = stream_classification(&error);
        classified(PayloadCause::Stream(error.into()), class)
    }
}

impl From<DecodeError> for PayloadError {
    fn from(error: DecodeError) -> Self {
        let class = stream_classification(&error);
        classified(PayloadCause::Cbor(error), class)
    }
}

fn stream_classification(
    error: &(dyn std::error::Error + 'static),
) -> (bool, Option<&'static str>) {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    let mut budget = false;
    let mut unavailable = None;
    while let Some(error) = source {
        match error.downcast_ref::<CodecError>() {
            Some(CodecError::Limit { .. }) => {
                budget = true;
                break;
            }
            Some(CodecError::Unavailable { reason, .. }) => {
                unavailable = Some(*reason);
                break;
            }
            _ => source = error.source(),
        }
    }
    (budget, unavailable)
}

fn classified(
    detail: PayloadCause,
    (budget, unavailable): (bool, Option<&'static str>),
) -> PayloadError {
    if budget {
        PayloadError::Budget(detail)
    } else if let Some(reason) = unavailable {
        PayloadError::Unavailable { reason, detail }
    } else {
        PayloadError::Damaged(detail)
    }
}

impl PayloadError {
    pub(super) fn blob_refusal_kind(&self) -> Option<super::BlobRefusalKind> {
        match self {
            Self::Budget(_) => Some(super::BlobRefusalKind::ByteBudget),
            Self::Unavailable {
                detail: PayloadCause::Codec(error),
                ..
            } if error.is_capability_unavailable() => {
                Some(super::BlobRefusalKind::UnavailableCapability)
            }
            Self::Unavailable {
                detail: PayloadCause::Stream(_),
                ..
            } => Some(super::BlobRefusalKind::UnavailableCapability),
            _ => None,
        }
    }

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
            Self::Stream(error) => error.fmt(f),
            Self::MissingCatalog(value) => write!(f, "codec id {value:?} not in catalog"),
            Self::TransformedNotBytes => f.write_str("transformed frame 'd' must be a byte string"),
            Self::BlobNotBytes => f.write_str("blob frame 'd' must be a byte string"),
            Self::BlobMissingIdentity => {
                f.write_str("external blob frame must declare a valid public digest")
            }
            Self::Cbor(error) => error.fmt(f),
            Self::EncodedBlobLimit { limit, .. } => write!(f, "encoded blob exceeds {limit} bytes"),
            Self::DecodedBlobLimit { limit, .. } => write!(f, "decoded blob exceeds {limit} bytes"),
            Self::SnapshotBlobLimit { limit, .. } => {
                write!(f, "snapshot blob exceeds {limit} bytes")
            }
        }
    }
}

purrdf_lex::error_source!(PayloadCause {
    Codec,
    Stream,
    Cbor
});

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
        Stream(error) => "gts-payload-stream" { "cause" => Cause(error) },
        MissingCatalog(value) => "gts-payload-missing-catalog" { "id" => Record(value) },
        TransformedNotBytes => "gts-payload-transformed-not-bytes" {},
        BlobNotBytes => "gts-payload-blob-not-bytes" {},
        BlobMissingIdentity => "gts-payload-blob-missing-identity" {},
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
