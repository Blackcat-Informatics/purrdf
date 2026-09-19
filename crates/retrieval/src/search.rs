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
//! verified fixed-point law. `search` adds no policy of its own. The value it
//! contributes is the **bridge** from the executor's `(rank, candidate, block)` rows to
//! the fusion protocol: a ranked stream must carry the profile's reciprocal-rank
//! contribution, and the profile is deliberately not a planning input, so the
//! contribution is attached here — from the exact profile in force, never
//! recomputed by a producer.
//!
//! That bridge is [`RankedStreamAdapter`], and it is public. A seam is a place
//! to stop *and* a place to start, so the piece a caller needs in order to
//! resume from `execute` cannot be the one piece it has to re-derive: an adapter
//! written by hand would re-derive the contribution arithmetic, and two
//! derivations of one law drift. The composition-identity tests assemble the
//! pipeline from this exported adapter for exactly that reason — what they
//! compare `search` against is the pipeline a caller can actually write.
//!
//! # A failed stratum is named in the answer, not left behind at `execute`
//!
//! [`execute`] reports a stratum that failed as a status rather than a stream,
//! and fusion consumes streams, so a failed stratum contributes no rows and no
//! fusion trailer entry of its own. That is where the status could die, and §6
//! of the design record says it may not: a fused answer carries the status of
//! every producer that contributed **and of every applicable producer that could
//! not**, never reduced to one aggregate flag.
//!
//! So `search` completes the fused trailer with [`ExecutionResult::statuses`]
//! through [`FusionTrailer::completed_with`] — the strata fusion verified keep
//! their verified status, and the strata fusion never saw (failed, or not
//! weighted) are added by name. "Two strata answered and a third could not" and
//! "two answered" are therefore different answers, readable from one place.
//!
//! The trailer property survives that: it is assembled from a value that exists
//! only once every stream has reached a terminal status — its own receipt, or
//! the contribution bound `top_k` stopped it at — so nothing readable mid-stream
//! gained a completeness claim.
//!
//! # The plan's identity travels with the rows
//!
//! An answer names the plan it descends from, and there are two ways to make it
//! do so. One is to ask the plan at the end, which always succeeds and proves
//! nothing: the name would be right even if the rows had come from somewhere
//! else entirely. The other is to carry the identity along with the rows and
//! read it back off the answer, which is what happens here — [`execute`] tags
//! each stream with the plan its unit was compiled from, the bridge below keeps
//! the tag, and [`fuse`] names it in the trailer only when every stream still
//! agrees on it.
//!
//! `search` therefore takes [`SearchResult::plan_id`] from the trailer and
//! refuses an answer whose streams carried a different identity, or none while
//! streams fused. The flow is load-bearing rather than decorative: break it
//! anywhere between the plan and the trailer and the search fails instead of
//! filing its rows under a plan that did not produce them.
//!
//! # And so does what each index attested about itself
//!
//! The third thing that can only be known where the rows are is what the index
//! behind them attested: which generation answered, and whether it admitted to
//! being short. [`execute`] reads it off the governed receipt of the very run
//! that produced the rows and tags the stream with it; the bridge below carries
//! it into [`RankedStream::attestation`]; and
//! [`FusionStream`](crate::FusionStream) reads it **before pulling a row** and
//! derives the trailer's [`FusionTrailer::attestations`],
//! [`FusionTrailer::exactness`] and [`FusionTrailer::evidence_id`] from it.
//!
//! Losing it on this path would not be a missing field, it would be a wrong
//! claim: with no stratum attesting an incomplete index the trailer says every
//! fused score is [`ScoreExactness::Exact`](crate::ScoreExactness), and a score
//! that is really a lower bound would be published as the whole number. So
//! `search` attaches it rather than letting the adapter's honest default stand
//! in for a producer that did say something.
//!
//! [`SearchResult::evidence_id`] is read back off the trailer for the reason
//! [`SearchResult::plan_id`] is: the value of the evidence is that it is the id
//! the answer actually carries, and one recomputed here from the same
//! attestations would be true of this function rather than of that answer.
//!
//! # So does each producer's declared stream contract
//!
//! The same route carries a second thing the fusion stage is the consumer of:
//! the duplicate handling and the candidate domains each producer declared
//! where it was registered. [`compile`] reads them off the registry it admits
//! against, [`execute`] tags every stream with them, the bridge below reports
//! them, and [`FusionStream`](crate::FusionStream) reads them before it pulls a
//! row — so a producer that declared its repeats are the consumer's to remove
//! is de-duplicated, one that promised there are none is believed and charged
//! nothing for the promise, and one that named the blocks of the candidate
//! universe it draws from lets the fusion stop reading it the moment it can no
//! longer change the answer. `search` chooses none of them; it only refuses to
//! lose the declarations on the way, and the trailer it returns reports the
//! domains that were in force
//! ([`FusionTrailer::domains`](crate::FusionTrailer::domains)) so an answer can
//! be audited against them.
//!
//! # A term that reached no producer is in the answer too
//!
//! Statuses are per producer, and a caller asks per term. A request can carry a
//! modality this registry has no producer for, so the answer names every request
//! term that reached nothing in [`SearchResult::unserved_terms`], typed, derived
//! from the plan that actually ran ([`Plan::unserved_evidence`](crate::Plan::unserved_evidence)). An
//! armed-but-unserved modality is visible as exactly that, never as a term that
//! quietly produced no rows.
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
//! # The profile reaches admission, without becoming a planning input
//!
//! `search` holds both the plan and the law it is about to fuse under, which is
//! the one point in the pipeline where the two meet before any row is read. So
//! it re-forms the admission environment around that law, and each stratum's
//! planned depth is measured against the rank resolution that law actually
//! delivers — recorded as
//! [`PlannedResolution`](crate::PlannedResolution), alongside the registry's row
//! bound, which remains a refusal because a depth the registry cannot fill is a
//! depth no arithmetic can supply. Resolution is not: a plan read past the depth
//! its profile still separates answers correctly and deterministically, more
//! coarsely, so it is reported rather than refused. The plan is unchanged and the
//! planner still never sees a profile — the coupling is measured where it becomes
//! knowable, not carried through a stage that must not know it.
//!
//! That measurement does not stop at the waist. `search` is the one entry point
//! that plans and executes in a single call, so it is also the only one where a
//! caller cannot stop between the two to read the compiled bundle: it carries
//! the waist's own map forward as [`SearchResult::planned_resolution`],
//! unchanged and unmerged. The answer therefore holds both altitudes at once —
//! what the depths a plan records were going to cost, and what the rows
//! actually pulled cost ([`FusionTrailer::resolution`]) — and a caller
//! comparing them is comparing the estimate with the outcome rather than
//! reading one number twice.
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

use std::collections::BTreeMap;

use purrdf_core::DatasetView;
use purrdf_sparql_eval::{PfAttestation, PropertyFunctionRegistry};
use purrdf_text::Fixed;

use crate::admission::{AdmissionEnvironment, AdmissionError};
use crate::compile::{PlannedResolution, compile};
use crate::error::{FusionError, PlanError};
use crate::execute::{ExecutionError, ExecutionResult, RankedStreamImpl, execute};
use crate::fuse::{TopK, fuse};
use crate::fusion_profile::{DecayRule, FusionProfile};
use crate::fusion_stream::{FusedRow, FusionTrailer};
use crate::id::{EvidenceId, FusionProfileId, PlanId};
use crate::iri::{Iri, Term};
use crate::plan::UnservedTerm;
use crate::planner::plan;
use crate::ranked_stream::{
    ProducerReceipt, ProtocolError, RankedRow, RankedStream, StreamContract,
};
use crate::reciprocal_rank::contribution_under;
use crate::request::RetrievalRequest;
use crate::statistics::Statistics;

/// The answer to a search: the top-k fused rows, the terminal trailer, the
/// per-term evidence, and both identities that name exactly which plan and which
/// fusion law produced them.
///
/// The rows are ordered by the profile's declared total tie-break and bounded by
/// the caller's [`TopK`]; the trailer carries every applicable producer's own
/// status, whether it contributed or could not. `plan_id` names the pinned plan
/// the streams descend from and `profile_id` names the law they were fused
/// under, so two answers are comparable only when both agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    /// The fused rows, in the profile's declared final order, at most the
    /// caller's [`TopK`].
    ///
    /// A top-k answer is not a claim that nothing else ranked: reaching the
    /// bound and exhausting the frontier return the same shape. Completeness
    /// belongs to the producers and is in [`Self::trailer`].
    pub rows: Vec<FusedRow>,
    /// The terminal report: every applicable producer's own status, keyed by
    /// stratum, and the profile identity.
    ///
    /// Every stratum the plan reached has an entry — the ones fusion verified
    /// from the rows it pulled, and the ones fusion never saw (a stratum whose
    /// unit failed, or one the profile declares no weight for) carried over from
    /// [`ExecutionResult::statuses`]. Nothing is reduced to an aggregate flag,
    /// so "answered with nothing" and "could not answer" stay distinguishable.
    pub trailer: FusionTrailer,
    /// Every request term that reached no producer at all, ascending by index.
    ///
    /// Derived from the plan that actually ran
    /// ([`Plan::unserved_evidence`](crate::Plan::unserved_evidence)), so
    /// it reports the terms this answer's own bindings leave unanswered. Empty
    /// exactly when every term reached a producer **with its content** — a
    /// producer that renders at least one of its facets into an argument
    /// position, so the query that ran contains it. A modality the request
    /// lattice can express but this registry has no producer for shows up here
    /// as [`UnservedReason::NoProducerAccepts`](crate::UnservedReason); one
    /// whose only acceptor declared nowhere to put it shows up as
    /// [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason);
    /// neither is left as a term that silently contributed no rows.
    pub unserved_terms: Vec<UnservedTerm>,
    /// The canonical identity of the plan the answer descends from.
    ///
    /// Read back off [`Self::trailer`], where it arrived on the streams
    /// themselves: `execute` tags each stream with the plan its unit was
    /// compiled from, and `fuse` names that plan in the trailer when every
    /// stream still agrees on it. So this is the identity that travelled the
    /// pipeline with the rows, not one re-read from the plan after the fact —
    /// and a stream whose identity changed on the way is a refusal
    /// ([`FusionError::PlanIdMismatch`]) rather than an answer filed under a
    /// plan that did not produce it.
    pub plan_id: PlanId,
    /// The identity of the evidence this answer carries: a digest of exactly
    /// the per-stratum attestations in [`Self::trailer`].
    ///
    /// Read back off [`FusionTrailer::evidence_id`], never recomputed here.
    /// Two answers that name the same plan, the same profile and the same
    /// evidence id were assembled from the same indexes in the same state; two
    /// that differ only here were not, and that difference is invisible in every
    /// other field — the dataset snapshot, the query text and the registry
    /// fingerprint are all unchanged by an index rebuild. Recomputing the digest
    /// from the same attestations would produce the same bytes and prove
    /// nothing, because it would be a fact about this function rather than about
    /// the answer in the caller's hand.
    pub evidence_id: EvidenceId,
    /// What each stratum's **planned** depth was going to cost in rank
    /// resolution under this profile, exactly as the admission waist recorded
    /// it, keyed by stratum.
    ///
    /// This is the compiled plan's own map
    /// ([`CompiledRetrieval::resolution`](crate::CompiledRetrieval)) carried
    /// through unchanged, and it answers a different question from the one
    /// [`FusionTrailer::resolution`] answers on [`Self::trailer`]:
    ///
    /// * here, *planned* — the depth the plan recorded for the stratum, against
    ///   the depth this profile still separates. It is knowable before any row
    ///   is read, so a caller that wants to decide whether a search is worth
    ///   paying for reads this one, at the waist, through
    ///   [`compile`](crate::compile). It is on the answer as well so that a
    ///   caller which took the one-call entry point — the only path that plans
    ///   and executes without stopping in between — is not the one caller that
    ///   cannot see it;
    /// * there, *observed* — how deep fusion really pulled and how many adjacent
    ///   ranks it really could not tell apart, counted on the rows themselves.
    ///
    /// The two legitimately disagree: a top-k that certified early never reaches
    /// its planned depth, and a bound a fusion never reached cost it nothing.
    /// Neither number is a correction of the other.
    ///
    /// Keyed by the strata this profile weights **and** the plan gave a depth,
    /// so it is empty for no other reason. A stratum with no weight makes no
    /// contribution to fuse, so there is no resolution it could have; that
    /// stratum is named in [`Self::unweighted_strata`] instead.
    pub planned_resolution: BTreeMap<Iri, PlannedResolution>,
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
/// 4. [`fuse(streams, profile, top_k)`](crate::fuse) — the verified fixed-point
///    fusion, bounded by the caller's `top_k`.
///
/// The parameters read request → data → policy → bound: what is being asked,
/// what it is asked of, the law the answer is composed under, and how much of
/// the answer is wanted. `top_k` is required rather than defaulted because fused
/// enumeration is top-k by construction; see [`TopK`] and §7 of the design
/// record.
///
/// The executor's `(rank, candidate, block)` streams are bridged to the fusion protocol
/// by [`RankedStreamAdapter`], which attaches each row's reciprocal-rank
/// contribution computed from exactly the profile in force. Nothing else is
/// added: `search` is the composition and nothing more.
///
/// A stratum `profile` declares no weight for is fused out of the answer and
/// named in [`SearchResult::unweighted_strata`]; the strata the profile does
/// weight still answer. See this module's header for why that is a report rather
/// than a refusal. Its own execution status is still in the trailer, because it
/// ran.
///
/// The answer reports rank resolution twice, and deliberately:
/// [`SearchResult::planned_resolution`] is the admission waist's estimate of
/// what the depths this plan records would cost under `profile`, and
/// [`FusionTrailer::resolution`] on [`SearchResult::trailer`] is what the rows
/// this run actually pulled did cost. A caller that only wants the first does
/// not have to run a search at all — [`compile`](crate::compile) against an
/// environment naming `profile` answers it without executing anything.
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
    top_k: TopK,
) -> Result<SearchResult, SearchError>
where
    S: Statistics,
    D: DatasetView + Sync,
{
    // 1. Plan. A pure function of the request, the registry and the statistics.
    let plan = plan(request, registry, statistics).map_err(SearchError::PlanError)?;

    // 2. Compile. Admission against the live environment, then emission. The
    //    environment is re-formed around `profile` rather than taken verbatim:
    //    `search` is about to fuse under exactly this law, so the admission
    //    waist is held to it whatever the caller's own environment named. That
    //    is the one place the profile can be known at admission without making
    //    it a planning input.
    let env = AdmissionEnvironment {
        registry: env.registry,
        statistics: env.statistics,
        fusion_profile: Some(profile),
    };
    let compiled = compile(&plan, &env).map_err(SearchError::AdmissionError)?;

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
    let ExecutionResult {
        streams: executed,
        statuses,
    } = execution;
    let mut streams: Vec<(Iri, RankedStreamAdapter)> = Vec::with_capacity(executed.len());
    let mut unweighted_strata: Vec<Iri> = Vec::new();
    for stratum_stream in executed {
        let stratum = stratum_stream.stratum.clone();
        let plan_id = stratum_stream.plan_id;
        let attestation = stratum_stream.attestation.clone();
        // The contract travels with the stream, exactly as the plan identity
        // does: it is the producer's own declaration, read at the admission
        // waist and carried, so the promise fusion holds this stream to is the
        // one its producer made and not one re-read off a registry afterwards.
        let Some(adapter) = RankedStreamAdapter::new(
            stratum_stream.stream,
            stratum_stream.contract,
            profile,
            &stratum,
        ) else {
            unweighted_strata.push(stratum);
            continue;
        };
        // The stream carries the plan it was compiled from into the fusion, so
        // the identity the answer reports is the one that travelled with the
        // rows rather than one read back off the plan at the end — and it
        // carries what the index behind those rows attested for the same
        // reason. Fusion reads the attestation before it pulls a row, which is
        // why it is attached here rather than collected after.
        streams.push((
            stratum,
            adapter.with_plan_id(plan_id).with_attestation(attestation),
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
    let fused_strata = streams.len();
    let fused = fuse::<RankedStreamAdapter, Term>(streams, profile, top_k)
        .await
        .map_err(SearchError::FusionError)?;

    // 5b. Read the plan identity back off the answer rather than off the plan.
    //     Every stream handed to `fuse` was tagged with the compiled plan, and
    //     `fuse` puts the tag in the trailer only if all of them still agreed on
    //     it, so this is the identity that actually travelled plan → compiled
    //     bundle → stream → answer. A trailer that names a different plan, or
    //     none while streams fused, means something replaced a stream on the way
    //     and the answer is not the one this plan produced.
    //
    //     No stream at all is the one case with nothing to read: every unit
    //     failed, so no stream existed to carry the identity. The plan still ran
    //     and the answer still names it.
    let plan_id = match fused.trailer.plan_id {
        Some(carried) if carried == compiled.plan_id => carried,
        carried if fused_strata > 0 => {
            return Err(SearchError::FusionError(FusionError::PlanIdMismatch {
                expected: compiled.plan_id,
                got: carried,
            }));
        }
        _ => compiled.plan_id,
    };

    // 6. Complete the terminal report. Fusion verified the strata it was handed;
    //    the executor holds the rest — a stratum whose unit failed and a stratum
    //    the profile does not weight both ran without ever becoming a stream.
    //    They are applicable producers that could not contribute, and §6 says
    //    the answer carries them beside the ones that did.
    let trailer = fused.trailer.completed_with(statuses);
    // 6b. Read the evidence identity back off the answer, exactly as the plan
    //     identity was read back off it above. Completing the report cannot move
    //     it: a stratum that never became a stream opened no index, so it has no
    //     attestation to digest and gets no entry.
    let evidence_id = trailer.evidence_id;

    Ok(SearchResult {
        rows: fused.rows,
        trailer,
        unserved_terms: plan.unserved_evidence(),
        plan_id,
        evidence_id,
        // 7. Carry the waist's own resolution evidence onto the answer. It is
        //    moved, not recomputed: recomputing it here from the profile and
        //    the plan would be a second derivation of what the admission waist
        //    already decided, and the value of the evidence is that it is the
        //    one the compiled bundle actually holds.
        planned_resolution: compiled.resolution,
        profile_id: profile.id(),
        unweighted_strata,
    })
}

/// The bridge from [`execute`]'s `(rank, candidate, block)` stream to the ranked fusion
/// protocol: the one piece a caller resuming at `execute` would otherwise have
/// to write itself.
///
/// [`execute`] deliberately carries no contribution. A contribution depends on
/// the fusion profile's weights and smoothing constant, the profile is
/// deliberately not a planning input, and keeping the executor profile-free is
/// what lets the unfused rung be consumed with no fusion law in the path.
/// [`fuse`] nonetheless
/// requires a [`RankedRow`], which carries the contribution beside them. This
/// adapter is that conversion, and
/// it is the *only* place the crate performs it.
///
/// It is public because §4 of the design record makes every stage boundary a
/// place to both stop and start. A caller that stops at `execute` — to walk a
/// stratum's rows itself, or to combine the streams its own way — and later decides to
/// fuse after all must not have to re-derive `w * recip(K + rank)`: two
/// derivations of one law drift, silently, in the direction of a plausible
/// order. So the derivation is exported rather than duplicated, and `search` is
/// defined in terms of the same exported value a caller composes with.
///
/// ```text
/// search(request, …, profile, k)
///   ==  fuse(execute(compile(plan(…))) bridged by RankedStreamAdapter, profile, k)
/// ```
///
/// The adapter cannot smuggle in a number the profile did not authorize: fusion
/// recomputes every contribution from the profile and refuses a mismatch with
/// [`ProtocolError::ContributionMismatch`]. Its terminal receipt is the
/// executor's own, passed through unchanged.
#[derive(Debug)]
pub struct RankedStreamAdapter {
    /// The executor's rows, in rank order.
    inner: RankedStreamImpl,
    /// The contract the stratum's producer declared these rows under.
    contract: StreamContract,
    /// The profile's weight for this stream's stratum.
    weight: Fixed,
    /// The profile's decay rule, carrying its smoothing constant.
    decay: DecayRule,
    /// The pinned plan these rows descend from, when the caller named one.
    plan_id: Option<PlanId>,
    /// What the index behind these rows attested, when the caller named it.
    attestation: PfAttestation,
}

impl RankedStreamAdapter {
    /// Bridge `stream` for `stratum` under `profile`, reporting `contract`, or
    /// `None` when `profile` declares no weight for that stratum.
    ///
    /// The weight is read from the profile here rather than taken as an
    /// argument, so a caller cannot attach a weight the profile does not
    /// declare — the contribution is a function of the law in force and of
    /// nothing else.
    ///
    /// The contract goes the other way: it is taken as an argument, because it
    /// is the *producer's* declaration and no profile knows it. A caller
    /// resuming at [`execute`] holds it beside the stream it is bridging
    /// ([`StratumStream::contract`](crate::StratumStream)) and passes it
    /// through; a caller whose stream came from somewhere else states its own,
    /// with [`StreamContract::new`]. It is a parameter rather than a builder
    /// step for the reason [`RankedStream::contract`] has no default — there is
    /// no honest value for a stream that declines to say.
    ///
    /// `None` is the honest answer for an unweighted stratum rather than an
    /// error: a weight is exactly "how much does this count", and a profile that
    /// says nothing about a stratum has not said the request is malformed. The
    /// caller decides what to do with the stream it cannot fuse — [`search`]
    /// names it in [`SearchResult::unweighted_strata`] and fuses the rest.
    #[must_use]
    pub fn new(
        stream: RankedStreamImpl,
        contract: StreamContract,
        profile: &FusionProfile,
        stratum: &Iri,
    ) -> Option<Self> {
        Some(Self {
            inner: stream,
            contract,
            weight: profile.weight(stratum)?,
            decay: profile.decay(),
            plan_id: None,
            attestation: PfAttestation::UNDECLARED,
        })
    }

    /// Name the pinned plan these rows descend from.
    ///
    /// [`execute`] tags every stream it returns with the plan its unit was
    /// compiled from ([`StratumStream::plan_id`](crate::StratumStream)), and
    /// this is how that tag continues into the fusion: [`fuse`] reads it back
    /// through [`RankedStream::plan_id`] and names it in the trailer, so the
    /// identity in a fused answer is the one that travelled with the rows.
    ///
    /// It is attached here rather than taken by [`new`](Self::new) because the
    /// executor's rows and their provenance arrive as two fields of one
    /// `StratumStream`, and a caller resuming at `execute` holds both. A caller
    /// whose stream descends from no plan attaches nothing and fuses anyway; see
    /// [`RankedStream::plan_id`].
    #[must_use]
    pub const fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// Name what the index behind these rows attested.
    ///
    /// [`execute`] reads this off the governed receipt of the run that produced
    /// the rows and tags the stream with it
    /// ([`StratumStream::attestation`](crate::StratumStream)); this is how that
    /// tag continues into the fusion, which reads it through
    /// [`RankedStream::attestation`] before pulling a row and derives the
    /// trailer's attestation map, its
    /// [`ScoreExactness`](crate::ScoreExactness) and its
    /// [`EvidenceId`] from it.
    ///
    /// It is a builder step rather than a parameter of [`new`](Self::new) for
    /// the reason [`with_plan_id`](Self::with_plan_id) is: the rows and their
    /// provenance arrive as separate fields of one `StratumStream`, and a stream
    /// that descends from no index at all attaches nothing and fuses anyway.
    /// Attaching nothing is honest here in a way it never is for a contract —
    /// see [`RankedStream::attestation`], which defaults for exactly that
    /// reason.
    #[must_use]
    pub fn with_attestation(mut self, attestation: PfAttestation) -> Self {
        self.attestation = attestation;
        self
    }
}

impl RankedStream for RankedStreamAdapter {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        let Some((rank, item, block)) = self.inner.next().await? else {
            return Ok(None);
        };
        // Ranks are 1-based, and a contribution at rank zero is undefined rather
        // than large. `execute` never emits one, but this adapter is public and a
        // caller can hand it a stream it built itself, so the violation is
        // reported as the protocol error fusion would raise for the same row —
        // one layer earlier, by the code that noticed it.
        if rank == 0 {
            return Err(ProtocolError::OutOfOrderRanks {
                expected: 1,
                got: rank,
            });
        }
        // With `rank >= 1` checked above and `k >= 1` fixed by the validated
        // profile this adapter read its weight from, `weight * recip(k + rank)`
        // is one checked division and a product strictly below `weight`, so it
        // cannot leave the fixed-point range. That argument is still only an
        // argument, and this is a library path a caller reaches with its own
        // stream and its own profile: the refusal is returned as a typed
        // protocol error rather than asserted with a panic, because a wrong
        // argument here would abort the caller's process instead of failing its
        // request. Fusion re-verifies the value regardless.
        let value = contribution_under(self.decay, self.weight, rank).map_err(|error| {
            ProtocolError::UncomputableContribution {
                rank,
                reason: error.to_string(),
            }
        })?;
        // The block travels through untouched. It is the producer's claim about
        // where this candidate came from, and this bridge adds the contribution
        // and nothing else: a block derived, defaulted or widened here would be
        // this adapter answering a question only the producer can — the same rule
        // that keeps it from inventing a contract.
        Ok(Some(RankedRow::new(rank, value, item, block)))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        self.inner.receipt().await
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    fn plan_id(&self) -> Option<PlanId> {
        self.plan_id
    }

    fn attestation(&self) -> PfAttestation {
        self.attestation.clone()
    }
}
