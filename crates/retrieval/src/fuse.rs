// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The top-level `fuse` entry point: certify the top `k` rows and report.
//!
//! `fuse` is a thin driver over [`FusionStream`]: it validates the streams are
//! tagged with distinct strata the profile declares, certifies rows until it
//! holds the caller's [`TopK`] or the frontier empties, and returns them with the
//! terminal trailer. A caller that wants to inspect the frontier row by row uses
//! [`FusionStream`] directly.
//!
//! # `k` is a parameter because fused enumeration is top-k
//!
//! §7 of the design record fixes the asymmetry between the two rungs. Unfused
//! enumeration is unbounded — each producer emits in its own rank order, no
//! cross-producer state exists, and a caller can walk a whole stream. Fused
//! enumeration cannot be: §5 sums contributions across strata, so no candidate
//! may be emitted until it is known not to reappear in another stratum and raise
//! its total, and the threshold over the stream heads is what bounds how deep
//! fusion must look to certify its next row. That bound is what makes memory
//! proportional to the frontier rather than to the input.
//!
//! *Complete* fused enumeration would need to remember everything already
//! emitted, and this surface does not offer it as though it were free. So `k` is
//! a required argument rather than a default: a caller states how many rows it
//! wants, and a caller that wants to walk everything wants the unfused rung,
//! which is built for exactly that.
//!
//! Nothing about the returned rows is a completeness claim — reaching `k` and
//! exhausting the frontier are the same return value — and nothing needs to be.
//! Completeness belongs to the producers, and every producer's own terminal
//! status is in the trailer.
//!
//! # The bound is a bound on the reading, not only on the rows
//!
//! Reaching `k` stops the reading. Every stream that still holds rows is closed
//! at the contribution it was read down to —
//! [`ProducerStatus::CeilingReached`](crate::ProducerStatus::CeilingReached) —
//! rather than drained to make it say `Exhausted`. Draining would read every
//! row of every stream to produce a report, which is precisely the memory bound
//! §7 says fused enumeration exists to keep, and it would report each stratum as
//! complete when the answer deliberately was not. Every producer still gets a
//! status; the status is just the true one.

use core::fmt;
use std::collections::BTreeSet;
use std::marker::PhantomData;

use crate::error::FusionError;
use crate::fusion_profile::FusionProfile;
use crate::fusion_stream::{FusedRow, FusionStream, FusionTrailer};
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::RankedStream;

/// The largest row count `fuse` reserves for up front.
///
/// [`TopK`] is a caller-chosen bound and may be far larger than anything the
/// streams can yield, so it is a request rather than a measurement: reserving it
/// verbatim would let one argument allocate without any producer having emitted
/// a row. The vector grows past this on its own if the rows are really there.
const MAX_PREALLOCATED_ROWS: usize = 1024;

/// How many fused rows a caller asks for: the `k` of top-k.
///
/// A newtype rather than a bare `usize` for one specific reason: this crate
/// already has a `k`. [`FusionProfile::k_parameter`] is the reciprocal-rank
/// smoothing constant `K` of `w * recip(K + rank)`, a property of the fusion law
/// and part of the profile's identity. This `k` is a row count the caller
/// chooses per call and that changes no identity at all. Two unrelated `k`s of
/// the same primitive type, passed to the same function, is exactly the argument
/// swap a type can rule out, so it does.
///
/// A bound of zero is admitted, not refused: "certify no rows and tell me how
/// every producer ended" is a coherent request, and the trailer it returns is
/// the whole answer to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopK(usize);

impl TopK {
    /// A bound of `rows` fused rows.
    #[must_use]
    pub const fn new(rows: usize) -> Self {
        Self(rows)
    }

    /// The bound, as a row count.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for TopK {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The result of a fusion: the certified rows and the terminal trailer.
///
/// `T` is the input item type; it is carried only so the result is typed by the
/// streams that produced it, matching the signature of [`fuse`].
pub struct FusionResult<T> {
    /// The fused rows, in final order, at most the caller's [`TopK`].
    ///
    /// This is a top-k answer, never a claim that nothing else ranked: a run
    /// that reached the bound and a run that emptied the frontier return the
    /// same shape. Completeness is a property of the producers, and each one's
    /// is in [`Self::trailer`].
    pub rows: Vec<FusedRow>,
    /// The terminal report: every producer's status and both identities.
    pub trailer: FusionTrailer,
    marker: PhantomData<fn() -> T>,
}

impl<T> FusionResult<T> {
    /// The fused rows, in final order, at most the caller's [`TopK`].
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

/// Fuse `streams` under `profile`, returning the top `top_k` rows in order.
///
/// Every stream must be tagged with a stratum the profile declares a weight for,
/// and no two streams may share a stratum. The streams are consumed in rank
/// order; each producer's contribution is re-verified against the profile, and
/// its terminal receipt is preserved in the trailer.
///
/// Certification stops as soon as `top_k` rows are held, and so does the
/// reading: a bound smaller than the frontier is the bounded work §7 describes,
/// in rows pulled as well as in rows returned. A `top_k` larger than what the
/// streams can yield is not an error and not truncated: fusion runs until the
/// frontier empties and every row it certified is returned.
///
/// The trailer is produced either way, and it names every stream this fusion was
/// handed: the ones that reached their own receipt, and the ones this bound
/// stopped, at the contribution they were read down to. The row list is
/// therefore never a completeness claim and the trailer always is.
///
/// # The pinned plan travels with the streams
///
/// If every stream names the same plan through
/// [`RankedStream::plan_id`](crate::RankedStream::plan_id), the trailer names it
/// too, so a fused answer's identity is the one that came up the pipeline with
/// the rows. Streams that name no plan fuse into an answer that names no plan.
/// Streams that name *different* plans are refused, because one answer cannot
/// honestly carry two provenances.
///
/// # Errors
///
/// [`FusionError::DuplicateStratum`] when two streams share a stratum;
/// [`FusionError::UnknownStratum`] when a stream's stratum has no declared
/// weight; [`FusionError::PlanIdMismatch`] when two streams name different
/// pinned plans; [`FusionError::Protocol`] when a stream violates the input
/// protocol; [`FusionError::Overflow`] when a checked sum leaves the fixed-point
/// range; [`FusionError::MaxContributionsExceeded`] when a candidate's
/// contribution count leaves the profile's declared bound; and
/// [`FusionError::CeilingExceeded`] when a candidate's accumulated score
/// leaves the profile's declared ceiling.
pub async fn fuse<S, T>(
    streams: Vec<(Iri, S)>,
    profile: &FusionProfile,
    top_k: TopK,
) -> Result<FusionResult<T>, FusionError>
where
    S: RankedStream<Item = T>,
    T: Ord + Clone + Into<Term>,
{
    let mut seen = BTreeSet::new();
    // The pinned plan is read off the streams before any of them is consumed,
    // so a disagreement is refused before a single row is pulled. `pinned` is
    // the plan the streams seen so far agree on; `unpinned` records that at
    // least one named none, which is not a disagreement but does mean no single
    // plan produced the whole answer.
    let mut pinned: Option<PlanId> = None;
    let mut unpinned = false;
    for (stratum, stream) in &streams {
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
        match (stream.plan_id(), pinned) {
            (None, _) => unpinned = true,
            (Some(id), None) => pinned = Some(id),
            (Some(id), Some(expected)) if id != expected => {
                return Err(FusionError::PlanIdMismatch {
                    expected,
                    got: Some(id),
                });
            }
            (Some(_), Some(_)) => {}
        }
    }
    let pinned = if unpinned { None } else { pinned };

    let mut fusion = FusionStream::new(streams, profile.clone());
    if let Some(plan_id) = pinned {
        fusion = fusion.with_plan_id(plan_id);
    }
    let mut rows = Vec::with_capacity(top_k.get().min(MAX_PREALLOCATED_ROWS));
    while rows.len() < top_k.get() {
        let Some(row) = fusion.next().await? else {
            break;
        };
        rows.push(row);
    }
    // Always, and after the rows: the trailer closes whatever the bound left
    // unread at the contribution it was read down to, so every producer has a
    // status and none of them is read further to obtain one.
    let trailer = fusion.trailer().await?;
    Ok(FusionResult {
        rows,
        trailer,
        marker: PhantomData,
    })
}
