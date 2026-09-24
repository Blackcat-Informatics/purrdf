// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! enumeration carries no cross-stratum accounting — each producer emits in its
//! own rank order over its own materialized result, no cross-producer state
//! exists, and a caller can walk a whole stream for what that stratum's result
//! already cost. Fused enumeration cannot work that way: §5 sums contributions
//! across strata, so no candidate
//! may be emitted until it is known not to reappear in another stratum and raise
//! its total, and the threshold over the stream heads is what bounds how deep
//! fusion must look to certify its next row. That bound is what makes memory
//! proportional to the frontier rather than to the input.
//!
//! *Complete* fused enumeration would need to remember everything already
//! emitted — complete enumeration may not discard, so it must retain, and
//! retention is linear wherever it is put — and this surface does not offer it
//! as though it were free. So `k` is
//! a required argument rather than a default: a caller states how many rows it
//! wants, and a caller that wants every row of a stratum wants the unfused
//! rung — not because walking there is free, but because that stratum's whole
//! result is materialized either way, so the walk adds nothing to what
//! producing it already cost.
//!
//! Nothing about the returned rows is a completeness claim — reaching `k` and
//! exhausting the frontier are the same return value — and nothing needs to be.
//! Completeness belongs to the producers, and every producer's own terminal
//! status is in the trailer.
//!
//! # How far the bound reaches into the reading
//!
//! Reaching `k` stops the reading *from that point on*: every stream that still
//! holds rows is closed at the contribution it was read down to —
//! [`ProducerStatus::CeilingReached`](crate::ProducerStatus::CeilingReached) —
//! rather than drained to make it say `Exhausted`. Draining would read every
//! row of every stream to produce a report, which is precisely the memory bound
//! §7 says fused enumeration exists to keep, and it would report each stratum as
//! complete when the answer deliberately was not. Every producer still gets a
//! status; the status is just the true one.
//!
//! What that leaves open is how much reading it took to *reach* `k`, and the
//! honest answer depends on what the producers declared, so it is stated here
//! rather than promised away.
//!
//! A fused score is exact only once every stream that could still name a
//! candidate has named it, and a stream that has told the consumer nothing and
//! answers nothing offers no random access: [`RankedStream`]'s `next` and
//! `receipt` are then the whole of it, so the only way to learn that such a
//! stream will not name `x` is to read it until it does or until it ends. Where
//! the strata overlap — the case a fused answer is usually wanted for — the
//! confirmations arrive early and `k` rows cost a few rows per stratum. Where
//! they do not overlap, and the producers have neither declared nor answered,
//! they never arrive: every candidate waits on a stratum that was never going
//! to mention it, and the reading runs to the end of the streams even though
//! the *rows* are still bounded by `k`. That is not a defect of the bound, it
//! is the price of an exact score over a stream that will say nothing about its
//! own candidates.
//!
//! There are two ways out and a producer may sell either.
//!
//! [`StreamContract::domains`](crate::StreamContract::domains) — supplied by
//! the host at registration, carried with the stream — says which blocks of the
//! candidate universe a producer may name, and fusion skips exactly the streams
//! that provably cannot name the candidate it is certifying. Under such
//! declarations the reading is bounded by the same argument as the rows: `k`
//! rows plus the lookahead the threshold needs, per stratum, however long the
//! streams are. Under [`CandidateDomains`](crate::CandidateDomains)'s
//! `Unrestricted` — the honest default-shaped value, and the widest promise —
//! nothing is skipped on this account.
//!
//! [`StreamContract::exclusion`](crate::StreamContract::exclusion) is the
//! other, and it is the one that reaches the case a declaration cannot: two
//! producers over one block, both declaring the truth, whose results never
//! overlap. A producer that declared a basis can be **asked** whether it holds
//! one named candidate, and an `Excluded` answer retires that stream's claim on
//! that candidate exactly as a declaration would have. The reading then stops
//! where the fused threshold licenses it to — a property of the decay law and
//! the weights, flat in the streams' length — rather than at the end of the
//! streams. The asking is itself bounded: only a candidate that is blocked and
//! could still win is looked up, and one `(candidate, stream)` pair costs at
//! most one lookup for the whole read.
//!
//! The scores are identical under all of it. What a producer declares or
//! answers changes how much is read, never what is returned.
//!
//! # The bound that reaches here has already been spent
//!
//! Everything above is about how much of a *materialized* stream a bound reads.
//! How much gets materialized in the first place is a different question, and it
//! is not this stage's to answer: the same declarations let the **planner** derive
//! each stratum's depth from the caller's bound, so a top-five request over
//! disjoint strata compiles to a `LIMIT` of five per stratum rather than to the
//! declaration's own row count. See [`plan`](crate::plan) for the rule and its
//! proof. `k` still bounds the reading here, over a read that is already the size
//! the answer needs.
//!
//! That is also why `top_k` is checked against what the streams were planned for.
//! A stream whose depth was narrowed to five rows cannot answer a fusion for six,
//! and the rows give no sign of it.

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
///
/// It is serializable because it is part of a request
/// ([`ReadBound`](crate::ReadBound)) and therefore part of a plan, and a plan is
/// a value a caller stores, ships and hands back. The wire form is the row count
/// itself.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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
/// # The bound the streams were planned for travels with them too
///
/// A stream that names the row bound its depth was derived for
/// ([`RankedStream::fused_bound`](crate::RankedStream::fused_bound)) is held to
/// it: `top_k` must be that bound, or the fusion is refused
/// ([`FusionError::ReadBoundMismatch`]). This is the lower-level entry, so it
/// still takes the bound as an argument — a caller assembling its own streams has
/// no plan to read one from — but a caller resuming from a planned, compiled,
/// executed bundle cannot silently fuse it at a depth the read cannot serve.
/// Streams that name no bound fuse at whatever `top_k` says.
///
/// # Errors
///
/// [`FusionError::DuplicateStratum`] when two streams share a stratum;
/// [`FusionError::UnknownStratum`] when a stream's stratum has no declared
/// weight; [`FusionError::PlanIdMismatch`] when two streams name different
/// pinned plans; [`FusionError::ReadBoundMismatch`] when a stream was planned for
/// a different bound than `top_k`; [`FusionError::Protocol`] when a stream violates the input
/// protocol; [`FusionError::Overflow`] when a checked sum leaves the fixed-point
/// range; [`FusionError::MaxContributionsExceeded`] when a candidate is
/// contributed to more times than there are strata, which this entry point's
/// own duplicate-stratum refusal makes unreachable from a conforming stream.
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
        // A stream that says what bound its depth was derived for is held to it,
        // before a row is pulled and for the reason the plan identity is: the
        // depth behind these rows is honest for one bound only. A stream that says
        // nothing is bounded by nothing and fuses at whatever the caller named.
        if let Some(planned) = stream.fused_bound()
            && planned != top_k
        {
            return Err(FusionError::ReadBoundMismatch {
                planned,
                requested: top_k,
            });
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
