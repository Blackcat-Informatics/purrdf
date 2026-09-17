// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The composition entry point: search is the four stages, run as one.
//!
//! [`search`] is defined *as* the composition of the pipeline, never as a second
//! implementation of it:
//!
//! ```text
//! search = fuse ∘ execute ∘ compile ∘ plan
//! ```
//!
//! It plans the request, admits and compiles the plan, executes the units, and
//! fuses the surviving streams under the caller's profile — each step a call to
//! the stage function that already owns it. A caller that wants to stop between
//! stages calls those functions directly; a caller that wants one answer calls
//! `search`.
//!
//! # Why this is a thin driver
//!
//! Every stage's semantics live in its own module: the planner is pure, admission
//! is the narrow waist, execution isolates a stratum's failure, and fusion is the
//! verified fixed-point law. `search` adds no policy of its own. The only value it
//! contributes is the **bridge** from the executor's `(rank, candidate)` rows to
//! the fusion protocol: a ranked stream must carry the profile's reciprocal-rank
//! contribution, and the profile is deliberately not a planning input, so the
//! contribution is attached here — from the exact profile in force, never
//! recomputed by a producer.
//!
//! # Failed strata are not fused
//!
//! [`execute`] reports a stratum that failed as a status rather than a stream.
//! Fusion consumes streams, so a failed stratum contributes no rows and no
//! trailer entry; its own status remains in [`ExecutionResult::statuses`]. A
//! caller that must distinguish "answered with nothing" from "could not answer"
//! stops at `execute` and reads those statuses directly.
//!
//! # A stratum the profile does not weight
//!
//! The fusion profile is deliberately **not** a planning input: the plan is
//! pinned against a registry, and the law an answer is composed under is chosen
//! later and independently. So the strata a plan reaches and the strata a
//! profile weights are two separate lists, and they can legitimately disagree —
//! a caller can fuse the same plan under a profile that scores only the text
//! stratum today and one that scores text and vectors tomorrow.
//!
//! A stratum the profile declares no weight for therefore has no contribution to
//! make: a weight is exactly "how much does this count", and saying nothing is
//! not the same as saying the request is malformed. `search` fuses the strata
//! the profile *does* weight and names the rest in
//! [`SearchResult::unweighted_strata`]. Failing the whole request instead would
//! throw away every stratum the profile did weight in order to report one it did
//! not — the over-refusal the repository treats as exactly as severe as a wrong
//! answer.
//!
//! The skipped strata are **reported, never dropped**: they arrive as a typed,
//! stratum-named list on the answer, in stratum order, so a caller can see
//! precisely which part of its plan its profile did not score.
//!
//! One case is still a refusal: when the profile weights *none* of the strata
//! the plan reached, there is no answer to keep working. The plan and the
//! profile are disjoint, every row that ran would be discarded, and an empty
//! result would be indistinguishable from a search that legitimately found
//! nothing. That is [`FusionError::UnknownStratum`], naming the first executed
//! stratum the profile does not weight.
//!
//! [`fuse`] itself keeps refusing an unweighted stratum outright, and that is
//! not a contradiction: its caller hands it `(stratum, stream)` pairs directly
//! and thereby asserts that each one belongs in this fusion. `search` makes no
//! such assertion on the caller's behalf — it decides which streams to hand over
//! and reports what it left out.

use purrdf_core::DatasetView;
use purrdf_sparql_eval::PropertyFunctionRegistry;
use purrdf_text::Fixed;

use crate::admission::{AdmissionEnvironment, AdmissionError};
use crate::compile::compile;
use crate::error::{FusionError, PlanError};
use crate::execute::{ExecutionError, RankedStreamImpl, execute};
use crate::fuse::fuse;
use crate::fusion_profile::FusionProfile;
use crate::fusion_stream::{FusedRow, FusionTrailer};
use crate::id::{FusionProfileId, PlanId};
use crate::iri::{Iri, Term};
use crate::planner::plan;
use crate::ranked_stream::{ProducerReceipt, ProtocolError, RankedStream};
use crate::reciprocal_rank::contribution;
use crate::request::RetrievalRequest;
use crate::statistics::Statistics;

/// The complete answer to a search: the fused rows, the terminal trailer, and
/// both identities that name exactly which plan and which fusion law produced
/// them.
///
/// The rows are ordered by the profile's declared total tie-break; the trailer
/// carries every contributing producer's own status. `plan_id` names the pinned
/// plan the streams descend from and `profile_id` names the law they were fused
/// under, so two answers are comparable only when both agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    /// The fused rows, in the profile's declared final order.
    pub rows: Vec<FusedRow>,
    /// The terminal report: every contributing producer's status and the
    /// profile identity.
    pub trailer: FusionTrailer,
    /// The canonical identity of the plan the answer descends from.
    pub plan_id: PlanId,
    /// The identity of the fusion profile the answer was fused under.
    pub profile_id: FusionProfileId,
    /// Every stratum that ran but that the profile declares no weight for, in
    /// ascending stratum order.
    ///
    /// These strata produced rows and those rows are **not** in
    /// [`Self::rows`]: with no weight there is no contribution they could make.
    /// They are named here rather than dropped, because "the profile did not
    /// score this part of the plan" and "this part of the plan found nothing"
    /// are different facts and a caller must be able to tell them apart.
    ///
    /// Empty for the ordinary case, where the profile weights every stratum the
    /// plan reached. It can never be the *only* thing here: a profile that
    /// weights none of the executed strata is refused outright, so a non-empty
    /// list always accompanies at least one fused stratum. See this module's
    /// header.
    pub unweighted_strata: Vec<Iri>,
}

/// A failure of any stage of the search composition, tagged with the stage that
/// refused.
///
/// The variants are the four stages in execution order. Search adds no failure
/// of its own: every refusal is a stage's typed error, carried unchanged, so a
/// caller can switch on the stage that failed and then on that stage's own exact
/// dimension without losing information to an aggregate error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SearchError {
    /// The planner refused the request.
    #[error("search planning failed: {0}")]
    PlanError(#[from] PlanError),

    /// Admission refused the planned request.
    #[error("search admission failed: {0}")]
    AdmissionError(#[from] AdmissionError),

    /// Execution refused to run the compiled units as a whole.
    ///
    /// A per-stratum failure is not this error: it is reported by `execute` as
    /// that stratum's status.
    #[error("search execution failed: {0}")]
    ExecutionError(#[from] ExecutionError),

    /// Fusion refused the surviving streams or the profile.
    #[error("search fusion failed: {0}")]
    FusionError(#[from] FusionError),
}

/// Run the whole retrieval ladder for one request and return one fused answer.
///
/// This is the composition of the four stages, in order:
///
/// 1. [`plan(request, registry, statistics)`](crate::plan) — the pure planner;
/// 2. [`compile(&plan, env)`](crate::compile) — semantic admission and emission;
/// 3. [`execute(&compiled, registry, dataset)`](crate::execute) — one run per
///    stratum, against the caller's data;
/// 4. [`fuse(streams, profile)`](crate::fuse) — the verified fixed-point fusion.
///
/// The parameters read request → data → policy: what is being asked, what it is
/// asked of, and the law the answer is composed under.
///
/// The executor's `(rank, candidate)` streams are bridged to the fusion protocol
/// by attaching each row's reciprocal-rank contribution, computed from exactly
/// the profile in force. Nothing else is added: `search` is the composition and
/// nothing more.
///
/// A stratum `profile` declares no weight for is fused out of the answer and
/// named in [`SearchResult::unweighted_strata`]; the strata the profile does
/// weight still answer. See this module's header for why that is a report rather
/// than a refusal.
///
/// # Errors
///
/// The corresponding [`SearchError`] variant of whichever stage refuses:
/// [`SearchError::PlanError`], [`SearchError::AdmissionError`],
/// [`SearchError::ExecutionError`], or [`SearchError::FusionError`]. Each carries
/// the stage's own typed error unchanged.
///
/// [`FusionError::UnknownStratum`] additionally names `search`'s one refusal of
/// its own: `profile` weights none of the strata that ran, so no row it produced
/// could contribute to the answer.
// The composition is awaited in one task and never crosses a thread boundary, so
// the `Send` bound this lint wants to express would buy nothing and would force a
// `Sync` statistics provider and environment into every caller. The
// `RankedStream` trait carries the same reasoning for its own futures.
#[allow(clippy::future_not_send)]
pub async fn search<S, D>(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    statistics: &S,
    dataset: &D,
    env: &AdmissionEnvironment<'_>,
    profile: &FusionProfile,
) -> Result<SearchResult, SearchError>
where
    S: Statistics,
    D: DatasetView + Sync,
{
    // 1. Plan. A pure function of the request, the registry and the statistics.
    let plan = plan(request, registry, statistics).map_err(SearchError::PlanError)?;

    // 2. Compile. Admission against the live environment, then emission.
    let compiled = compile(&plan, env).map_err(SearchError::AdmissionError)?;

    // 3. Execute. Each stratum runs independently against the caller's dataset; a
    //    failed stratum is a status, not a stream.
    let execution = execute(&compiled, registry, dataset)
        .await
        .map_err(SearchError::ExecutionError)?;

    // 4. Bridge each surviving stream to the fusion protocol. The weight is read
    //    from the profile so the contribution attached here is exactly the one
    //    fusion re-verifies; a stratum the profile does not weight has no
    //    contribution to make, so it is set aside by name rather than taking the
    //    whole request down with it. See this module's header.
    let mut streams: Vec<(Iri, RankedStreamAdapter)> = Vec::with_capacity(execution.streams.len());
    let mut unweighted_strata: Vec<Iri> = Vec::new();
    for stratum_stream in execution.streams {
        let stratum = stratum_stream.stratum.clone();
        let Some(weight) = profile.weight(&stratum) else {
            unweighted_strata.push(stratum);
            continue;
        };
        streams.push((
            stratum,
            RankedStreamAdapter::new(stratum_stream.stream, weight, profile.k_parameter()),
        ));
    }
    // `execute` yields its streams in the compiler's stratum order, but the
    // report is sorted rather than inherited: it is part of the answer, and an
    // answer's fields do not depend on an upstream stage's ordering.
    unweighted_strata.sort();
    if streams.is_empty() {
        // Disjoint plan and profile: nothing that ran can contribute, so there
        // is no partial answer to keep working and an empty one would claim the
        // search found nothing. The refusal names the stratum, in the same
        // order the report uses.
        if let Some(stratum) = unweighted_strata.first() {
            return Err(SearchError::FusionError(FusionError::UnknownStratum {
                stratum: stratum.as_str().to_owned(),
            }));
        }
    }

    // 5. Fuse. `Term` is the item type: the executor's candidate terms.
    let fused = fuse::<RankedStreamAdapter, Term>(streams, profile)
        .await
        .map_err(SearchError::FusionError)?;

    Ok(SearchResult {
        rows: fused.rows,
        trailer: fused.trailer,
        plan_id: plan.id(),
        profile_id: profile.id(),
        unweighted_strata,
    })
}

/// Bridges the executor's materialized `(rank, candidate)` stream to the ranked
/// fusion protocol.
///
/// The executor deliberately carries no contribution — the profile is not a plan
/// input — so this adapter attaches the profile's reciprocal-rank value for each
/// rank as it is pulled. Fusion recomputes that value and refuses a mismatch, so
/// the adapter cannot smuggle in a contribution the profile did not authorize.
struct RankedStreamAdapter {
    /// The executor's rows, in rank order.
    inner: RankedStreamImpl,
    /// The profile's weight for this stream's stratum.
    weight: Fixed,
    /// The profile's smoothing constant.
    k: u32,
}

impl RankedStreamAdapter {
    /// Bridge `inner` using the profile's `weight` and smoothing constant `k`.
    fn new(inner: RankedStreamImpl, weight: Fixed, k: u32) -> Self {
        Self { inner, weight, k }
    }
}

impl RankedStream for RankedStreamAdapter {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        let Some((rank, item)) = self.inner.next().await? else {
            return Ok(None);
        };
        // A validated profile fixes `k >= 1` and the executor emits 1-based
        // ranks, so `weight * recip(k + rank)` is one checked division and a
        // product strictly below `weight`; it cannot leave the fixed-point
        // range. Fusion re-verifies the value regardless.
        let value = contribution(self.weight, rank, self.k)
            .expect("a validated profile and a 1-based rank cannot overflow");
        Ok(Some((rank, value, item)))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        self.inner.receipt().await
    }
}
