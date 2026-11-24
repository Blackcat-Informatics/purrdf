// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::v2::{LinkState, PayloadRefusal, PayloadState, StoredState};

/// The independently checked envelope of a retained generation 2 frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameWireResult {
    /// File-relative segment ordinal.
    pub segment: u64,
    /// Item position within the segment, with its opener at zero.
    pub position: usize,
    /// File-relative byte offset.
    pub offset: usize,
    /// Length of the original encoded frame in bytes.
    pub length: usize,
    /// Deterministic frame type code as read, including unknown codes.
    pub frame_type: Option<u64>,
    /// Link verdict, after checking a final witness where applicable.
    pub link: LinkState,
    /// The payload verdict without retaining the decoded bytes.
    pub payload: PayloadOutcome,
    /// Verdict of the stored check when the storage form provides one.
    pub stored: Option<StoredState>,
}

/// The payload outcome recorded separately from the link and stored checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PayloadOutcome {
    /// The payload satisfies the length and digest checks.
    Verified,
    /// A grammar, decoding, length or digest check failed.
    Damaged,
    /// The reader could not check the payload.
    Unchecked(PayloadUnchecked),
}

/// The two causes of an unchecked payload in the typed result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PayloadUnchecked {
    /// The reader does not implement the codec or storage form.
    UnknownCodec,
    /// A stored or decoded byte capacity refused the payload check.
    ExceedsCapacity,
}

impl From<&PayloadState<'_>> for PayloadOutcome {
    fn from(state: &PayloadState<'_>) -> Self {
        match state {
            PayloadState::Verified(_) => Self::Verified,
            PayloadState::Damaged(_) => Self::Damaged,
            PayloadState::Unchecked(
                PayloadRefusal::UnknownCodec(_) | PayloadRefusal::UnknownStorage(_),
            ) => Self::Unchecked(PayloadUnchecked::UnknownCodec),
            PayloadState::Unchecked(
                PayloadRefusal::StoredSize(_)
                | PayloadRefusal::DecodedSize(_)
                | PayloadRefusal::TotalDecoded(_),
            ) => Self::Unchecked(PayloadUnchecked::ExceedsCapacity),
        }
    }
}
