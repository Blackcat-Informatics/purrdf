// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The top-level `fuse` entry point: run a fusion to completion.
//!
//! `fuse` is a thin driver over [`FusionStream`]: it validates the streams are
//! tagged with distinct strata the profile declares, pulls every certified row,
//! and returns them with the terminal trailer. It is one pass over the streams;
//! a caller that wants to stop early or inspect the frontier uses
//! [`FusionStream`] directly.

use core::fmt;
use std::collections::BTreeSet;
use std::marker::PhantomData;

use crate::error::FusionError;
use crate::fusion_profile::FusionProfile;
use crate::fusion_stream::{FusedRow, FusionStream, FusionTrailer};
use crate::iri::{Iri, Term};
use crate::ranked_stream::RankedStream;

/// The complete result of a fusion: the ordered rows and the terminal trailer.
///
/// `T` is the input item type; it is carried only so the result is typed by the
/// streams that produced it, matching the signature of [`fuse`].
pub struct FusionResult<T> {
    /// The fused rows, in final order.
    pub rows: Vec<FusedRow>,
    /// The terminal report: every producer's status and both identities.
    pub trailer: FusionTrailer,
    marker: PhantomData<fn() -> T>,
}

impl<T> FusionResult<T> {
    /// The fused rows, in final order.
    #[must_use]
    pub fn rows(&self) -> &[FusedRow] {
        &self.rows
    }

    /// The terminal trailer.
    #[must_use]
    pub const fn trailer(&self) -> &FusionTrailer {
        &self.trailer
    }
}

impl<T> fmt::Debug for FusionResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FusionResult")
            .field("rows", &self.rows)
            .field("trailer", &self.trailer)
            .finish()
    }
}

/// Fuse `streams` under `profile`, returning the complete ordered answer.
///
/// Every stream must be tagged with a stratum the profile declares a weight for,
/// and no two streams may share a stratum. The streams are consumed in rank
/// order; each producer's contribution is re-verified against the profile, and
/// its terminal receipt is preserved in the trailer.
///
/// # Errors
///
/// [`FusionError::DuplicateStratum`] when two streams share a stratum;
/// [`FusionError::UnknownStratum`] when a stream's stratum has no declared
/// weight; [`FusionError::Protocol`] when a stream violates the input protocol;
/// and [`FusionError::Overflow`] when a checked sum leaves the fixed-point
/// range.
pub async fn fuse<S, T>(
    streams: Vec<(Iri, S)>,
    profile: &FusionProfile,
) -> Result<FusionResult<T>, FusionError>
where
    S: RankedStream<Item = T>,
    T: Ord + Clone + Into<Term>,
{
    let mut seen = BTreeSet::new();
    for (stratum, _) in &streams {
        if !seen.insert(stratum.clone()) {
            return Err(FusionError::DuplicateStratum {
                stratum: stratum.as_str().to_owned(),
            });
        }
        if profile.weight(stratum).is_none() {
            return Err(FusionError::UnknownStratum {
                stratum: stratum.as_str().to_owned(),
            });
        }
    }

    let mut fusion = FusionStream::new(streams, profile.clone());
    let mut rows = Vec::new();
    while let Some(row) = fusion.next().await? {
        rows.push(row);
    }
    let trailer = fusion.trailer().await?;
    Ok(FusionResult {
        rows,
        trailer,
        marker: PhantomData,
    })
}
