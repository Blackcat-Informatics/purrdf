// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The streaming fusion engine: verified-protocol Fagin NRA over exact fixed
//! point.
//!
//! Each stream contributes a monotone, non-increasing contribution sequence to
//! the candidates it carries. A candidate's fused score is the checked sum of
//! its contributions across strata; its lower bound `L(x)` is the sum of the
//! contributions seen so far and its upper bound `U(x)` adds the current head of
//! every stream that has not yet contributed to `x`. Fusion pulls greedily from
//! the highest-contribution head, maintains the threshold `T` over all heads,
//! and emits a candidate only when its score is final (`U(x) == L(x)`) and it
//! provably outranks every other candidate and every not-yet-seen item. Emitted
//! candidates are removed, so memory is proportional to the un-emitted frontier.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use purrdf_text::Fixed;

use crate::error::FusionError;
use crate::fusion_profile::FusionProfile;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{ProducerReceipt, ProtocolError, RankedStream};

/// A candidate's identity in the frontier: its canonical term.
pub type CandidateId = Term;

/// The status of one producer in a fused trailer.
///
/// This mirrors [`ProducerReceipt`] but belongs to the answer: every producer
/// that contributed — and every applicable one that could not — keeps its own
/// status, so "all producers answered" and "one could not" stay distinguishable.
///
/// # Where each variant comes from
///
/// Every variant is the producer's own declaration, converted from the receipt
/// it returned through [`RankedStream::receipt`] — plus, for the strata that
/// never became a stream, the executor's report carried in by
/// [`FusionTrailer::completed_with`].
///
/// Two of them this crate's own executor cannot mint, and that is a fact about
/// the executor rather than about the vocabulary.
/// [`RankedStreamImpl`](crate::RankedStreamImpl) materializes its rows and is
/// always drained, so it declares [`Self::Exhausted`]; a unit that cannot run at
/// all becomes [`Self::ExecutionFailed`]. It never declares
/// [`Self::CeilingReached`], because it stops at no *score* bound — the row
/// bound a caller passes to [`fuse`](crate::fuse) belongs to the fusion, not to
/// a producer. And it never declares [`Self::TermsRejected`], because a producer
/// that accepts none of the request's terms is rejected at planning
/// ([`RejectionReason::NoAcceptedTerm`](crate::RejectionReason)) and so compiles
/// no unit to run: it has no stratum in the answer to report under, and the
/// per-term fact that nothing served a term is reported per term instead, in
/// [`Plan::unserved_terms`](crate::Plan::unserved_terms).
///
/// Both remain reachable through the public protocol, which is the point of
/// having them: [`RankedStream`] is implemented by caller-supplied producers,
/// and a producer that stops at its own declared score bound or declines the
/// terms it was handed reports exactly these, with the fusion engine validating
/// the claim against the rows it actually emitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerStatus {
    /// The producer emitted every row it had.
    Exhausted {
        /// How many rows it emitted.
        rows_emitted: u64,
    },
    /// The producer stopped at a declared score bound rather than at
    /// exhaustion. Declared by the producer itself; see this type's header.
    CeilingReached {
        /// The inclusive bound at which it stopped.
        bound: Fixed,
    },
    /// The producer could not run.
    ExecutionFailed {
        /// A human-readable reason, carried verbatim.
        reason: String,
    },
    /// The producer declined the request terms it was handed. Declared by the
    /// producer itself; see this type's header.
    TermsRejected,
}

impl From<ProducerReceipt> for ProducerStatus {
    fn from(receipt: ProducerReceipt) -> Self {
        match receipt {
            ProducerReceipt::Exhausted { rows_emitted } => Self::Exhausted { rows_emitted },
            ProducerReceipt::CeilingReached { bound } => Self::CeilingReached { bound },
            ProducerReceipt::ExecutionFailed { reason } => Self::ExecutionFailed { reason },
            ProducerReceipt::TermsRejected => Self::TermsRejected,
        }
    }
}

/// One fused row: a candidate with its exact score and its provenance.
///
/// `contributions` names, for every stratum the candidate surfaced in, the
/// 1-based rank and the contribution that stratum made. Their checked sum is
/// `score`. `threshold_witness` is the global threshold in force when the row
/// was certified, so a reader can replay the certification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusedRow {
    /// The candidate, in its canonical term lexical — `<http://example.org/doc>`,
    /// `"lex"@en`, `<<( s p o )>>`.
    ///
    /// That is the same spelling
    /// [`RequestTerm::EntitySeed`](crate::RequestTerm::EntitySeed) takes, so a
    /// fused row can be handed straight back as the seed of a follow-up request.
    pub entity: Term,
    /// The exact fused score.
    pub score: Fixed,
    /// Per-stratum provenance: `(stratum, rank, contribution)`.
    pub contributions: Vec<(Iri, u64, Fixed)>,
    /// The threshold in force when this row was certified.
    pub threshold_witness: Fixed,
}

/// The terminal report of a fusion: every producer's status and both identities.
///
/// The trailer is the only place completeness may be asserted. A consumer that
/// read a prefix and stopped holds evidence the answer is incomplete; nothing
/// mid-stream entitles it to claim otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionTrailer {
    /// Every producer's own status, keyed by its stratum.
    pub statuses: BTreeMap<Iri, ProducerStatus>,
    /// The pinned plan the fused rows came from, when one is attached.
    pub plan_id: Option<PlanId>,
    /// The fusion profile in force.
    pub profile_id: crate::id::FusionProfileId,
}

impl FusionTrailer {
    /// Add the status of every producer that never became a stream, and return
    /// the completed trailer.
    ///
    /// A fusion knows only the streams it was handed, and that is strictly fewer
    /// producers than a request reaches: a stratum whose unit failed to run has
    /// no stream to fuse, and one the profile declares no weight for is set
    /// aside before fusion sees it. Both are "an applicable producer that could
    /// not contribute", which §6 of the design record says the answer must carry
    /// alongside the ones that did — so the stage that *does* know them hands
    /// them here, rather than the answer reducing them to a count of strata that
    /// happened to fuse.
    ///
    /// A status this fusion verified is never replaced. Those were checked
    /// against the rows actually pulled ([`ProtocolError::ForgedReceipt`]), and
    /// an unverified report of the same stratum cannot be allowed to overwrite
    /// one that was; `statuses` fills only the strata the fusion never saw.
    ///
    /// The result does not depend on `statuses`' iteration order: its keys are
    /// distinct and each is inserted only where nothing stands.
    ///
    /// This is a terminal operation on a terminal value — the trailer exists
    /// only after every stream reached its receipt — so nothing here makes a
    /// status readable mid-stream.
    #[must_use]
    pub fn completed_with<I>(mut self, statuses: I) -> Self
    where
        I: IntoIterator<Item = (Iri, ProducerStatus)>,
    {
        for (stratum, status) in statuses {
            self.statuses.entry(stratum).or_insert(status);
        }
        self
    }
}

/// The current head of one stream, already validated and contribution-checked.
#[derive(Clone, Debug)]
struct Head {
    rank: u64,
    contribution: Fixed,
    item: Term,
}

/// A candidate's accumulated NRA state.
#[derive(Clone, Debug)]
struct CandidateState {
    lower_bound: Fixed,
    contributions: Vec<(Iri, u64, Fixed)>,
    seen_streams: BTreeSet<usize>,
}

impl CandidateState {
    fn new() -> Self {
        Self {
            lower_bound: Fixed::ZERO,
            contributions: Vec::new(),
            seen_streams: BTreeSet::new(),
        }
    }

    /// The best (minimum) rank this candidate holds in any stratum.
    fn best_rank(&self) -> u64 {
        self.contributions
            .iter()
            .map(|(_, rank, _)| *rank)
            .min()
            .unwrap_or(u64::MAX)
    }
}

/// The NRA fusion engine over a set of verified ranked streams.
///
/// `FusionStream` is generic over the stream type and produces [`FusedRow`]s on
/// demand. Call [`next`](FusionStream::next) until it returns `Ok(None)`, then
/// [`trailer`](FusionStream::trailer) for the producers' statuses.
pub struct FusionStream<S: RankedStream> {
    streams: Vec<(Iri, S)>,
    profile: FusionProfile,
    plan_id: Option<PlanId>,
    initialized: bool,
    heads: Vec<Option<Head>>,
    exhausted: Vec<bool>,
    next_ranks: Vec<u64>,
    rows_pulled: Vec<u64>,
    last_contribution: Vec<Option<Fixed>>,
    seen_items: Vec<BTreeSet<Term>>,
    statuses: BTreeMap<Iri, ProducerStatus>,
    frontier: BTreeMap<CandidateId, CandidateState>,
    threshold: Fixed,
}

impl<S: RankedStream> fmt::Debug for FusionStream<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let strata: Vec<&str> = self.streams.iter().map(|(iri, _)| iri.as_str()).collect();
        f.debug_struct("FusionStream")
            .field("strata", &strata)
            .field("profile", &self.profile)
            .field("frontier_len", &self.frontier.len())
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl<S: RankedStream> FusionStream<S> {
    /// Build a fusion engine over `streams` under `profile`.
    ///
    /// No plan identity is attached; use [`with_plan_id`](Self::with_plan_id) to
    /// name the pinned plan the streams came from. No stream is pulled until the
    /// first [`next`](Self::next) call.
    #[must_use]
    pub fn new(streams: Vec<(Iri, S)>, profile: FusionProfile) -> Self {
        let count = streams.len();
        Self {
            streams,
            profile,
            plan_id: None,
            initialized: false,
            heads: (0..count).map(|_| None).collect(),
            exhausted: vec![false; count],
            next_ranks: vec![1; count],
            rows_pulled: vec![0; count],
            last_contribution: vec![None; count],
            seen_items: (0..count).map(|_| BTreeSet::new()).collect(),
            statuses: BTreeMap::new(),
            frontier: BTreeMap::new(),
            threshold: Fixed::ZERO,
        }
    }

    /// Attach the pinned plan identity these streams came from.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// The next certified fused row, or `Ok(None)` when the frontier is empty.
    ///
    /// # Errors
    ///
    /// [`FusionError::Overflow`] when a checked sum leaves the fixed-point
    /// range; [`FusionError::Protocol`] when a stream violates the input
    /// protocol; [`FusionError::MaxContributionsExceeded`] when a candidate's
    /// contribution count leaves the profile's declared bound; and
    /// [`FusionError::CeilingExceeded`] when a candidate's accumulated score
    /// leaves the profile's declared ceiling.
    pub async fn next(&mut self) -> Result<Option<FusedRow>, FusionError> {
        self.ensure_initialized().await?;
        loop {
            self.threshold = self.compute_threshold()?;
            if let Some(id) = self.select_emittable()? {
                let state = self.frontier.remove(&id).ok_or_else(|| {
                    FusionError::MalformedProfile(
                        "selected candidate vanished from the frontier".to_owned(),
                    )
                })?;
                let mut contributions = state.contributions;
                contributions.sort_by(|left, right| {
                    left.0
                        .as_str()
                        .cmp(right.0.as_str())
                        .then(left.1.cmp(&right.1))
                });
                return Ok(Some(FusedRow {
                    entity: id,
                    score: state.lower_bound,
                    contributions,
                    threshold_witness: self.threshold,
                }));
            }
            match self.best_head_index() {
                Some(index) => self.pull(index).await?,
                None => return Ok(None),
            }
        }
    }

    /// Drain every remaining stream and return the terminal trailer.
    ///
    /// Draining discards rows but forces every producer to its terminal receipt,
    /// so the trailer's statuses cover every stream this fusion was handed —
    /// including the ones a caller's row bound stopped reading — and cannot be
    /// forged. A producer that never became a stream is not here and cannot be;
    /// the stage that ran it adds it through
    /// [`FusionTrailer::completed_with`].
    ///
    /// # Errors
    ///
    /// [`FusionError::Protocol`] when a stream violates the protocol while
    /// draining, and [`FusionError::Overflow`] on a checked sum overflow.
    pub async fn trailer(&mut self) -> Result<FusionTrailer, FusionError> {
        self.ensure_initialized().await?;
        for index in 0..self.streams.len() {
            while self.heads[index].is_some() {
                self.heads[index] = self.fetch(index).await?;
            }
        }
        Ok(FusionTrailer {
            statuses: self.statuses.clone(),
            plan_id: self.plan_id,
            profile_id: self.profile.id(),
        })
    }

    /// Pull the first head of every stream.
    async fn ensure_initialized(&mut self) -> Result<(), FusionError> {
        if self.initialized {
            return Ok(());
        }
        for index in 0..self.streams.len() {
            self.heads[index] = self.fetch(index).await?;
        }
        self.initialized = true;
        Ok(())
    }

    /// Pull and validate one row from stream `index`.
    ///
    /// Validation is the whole point of the boundary: rank contiguity, monotone
    /// contributions, per-stream uniqueness, the profile's contribution value,
    /// and a consistent terminal receipt.
    async fn fetch(&mut self, index: usize) -> Result<Option<Head>, FusionError> {
        let pulled = {
            let (_, stream) = &mut self.streams[index];
            stream.next().await
        };
        let Some((rank, producer_contribution, item)) = pulled? else {
            let receipt = {
                let (_, stream) = &mut self.streams[index];
                stream.receipt().await
            }?;
            self.finish(index, receipt)?;
            return Ok(None);
        };

        let expected_rank = self.next_ranks[index];
        if rank < expected_rank {
            return Err(ProtocolError::OutOfOrderRanks {
                expected: expected_rank,
                got: rank,
            }
            .into());
        }
        if rank > expected_rank {
            return Err(ProtocolError::NonContiguousRanks {
                gap: rank - expected_rank,
            }
            .into());
        }
        if let Some(previous) =
            self.last_contribution[index].filter(|previous| producer_contribution > *previous)
        {
            return Err(ProtocolError::NonMonotoneContribution {
                previous,
                got: producer_contribution,
            }
            .into());
        }

        let item: Term = item.into();
        if self.seen_items[index].contains(&item) {
            return Err(ProtocolError::DuplicateItem {
                item: item.as_str().to_owned(),
            }
            .into());
        }

        let stratum = self.streams[index].0.clone();
        let weight = self
            .profile
            .weight(&stratum)
            .ok_or_else(|| FusionError::UnknownStratum {
                stratum: stratum.as_str().to_owned(),
            })?;
        let expected =
            crate::reciprocal_rank::contribution(weight, rank, self.profile.k_parameter())?;
        if producer_contribution != expected {
            return Err(ProtocolError::ContributionMismatch {
                expected,
                got: producer_contribution,
            }
            .into());
        }

        self.seen_items[index].insert(item.clone());
        self.last_contribution[index] = Some(producer_contribution);
        self.next_ranks[index] = rank + 1;
        self.rows_pulled[index] += 1;

        Ok(Some(Head {
            rank,
            contribution: expected,
            item,
        }))
    }

    /// Record a terminal receipt, refusing one the rows contradict.
    fn finish(&mut self, index: usize, receipt: ProducerReceipt) -> Result<(), FusionError> {
        let actual = self.rows_pulled[index];
        match &receipt {
            ProducerReceipt::Exhausted { rows_emitted } if *rows_emitted != actual => {
                return Err(ProtocolError::ForgedReceipt {
                    declared: *rows_emitted,
                    actual,
                }
                .into());
            }
            ProducerReceipt::TermsRejected | ProducerReceipt::ExecutionFailed { .. }
                if actual > 0 =>
            {
                return Err(ProtocolError::ForgedReceipt {
                    declared: 0,
                    actual,
                }
                .into());
            }
            _ => {}
        }
        let stratum = self.streams[index].0.clone();
        self.exhausted[index] = true;
        self.heads[index] = None;
        self.statuses.insert(stratum, ProducerStatus::from(receipt));
        Ok(())
    }

    /// The index of the stream whose head has the highest contribution.
    fn best_head_index(&self) -> Option<usize> {
        let mut best: Option<(usize, Fixed)> = None;
        for (index, head) in self.heads.iter().enumerate() {
            let Some(head) = head else { continue };
            match best {
                None => best = Some((index, head.contribution)),
                Some((best_index, best_contribution)) => {
                    if head.contribution > best_contribution
                        || (head.contribution == best_contribution && index < best_index)
                    {
                        best = Some((index, head.contribution));
                    }
                }
            }
        }
        best.map(|(index, _)| index)
    }

    /// The threshold `T`: the summed contribution of every current head.
    fn compute_threshold(&self) -> Result<Fixed, FusionError> {
        let mut threshold = Fixed::ZERO;
        for head in self.heads.iter().flatten() {
            threshold = threshold
                .checked_add(head.contribution)
                .map_err(|_| FusionError::Overflow)?;
        }
        Ok(threshold)
    }

    /// `U(x)`: `L(x)` plus the current head of every stream that has not yet
    /// contributed to `x`.
    fn upper_bound(&self, state: &CandidateState) -> Result<Fixed, FusionError> {
        let mut bound = state.lower_bound;
        for (index, head) in self.heads.iter().enumerate() {
            if state.seen_streams.contains(&index) {
                continue;
            }
            if let Some(head) = head {
                bound = bound
                    .checked_add(head.contribution)
                    .map_err(|_| FusionError::Overflow)?;
            }
        }
        Ok(bound)
    }

    /// Whether candidate `left` should be emitted before candidate `right`.
    ///
    /// The declared total tie-break: score descending, then best rank ascending,
    /// then canonical term byte order ascending.
    fn is_better(
        left_id: &CandidateId,
        left: &CandidateState,
        right_id: &CandidateId,
        right: &CandidateState,
    ) -> bool {
        match left.lower_bound.cmp(&right.lower_bound) {
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Equal => match left.best_rank().cmp(&right.best_rank()) {
                core::cmp::Ordering::Less => true,
                core::cmp::Ordering::Greater => false,
                core::cmp::Ordering::Equal => left_id.as_str() < right_id.as_str(),
            },
        }
    }

    /// The best candidate that is safe to emit, if any.
    ///
    /// A candidate is emittable when its score is final (`U(x) == L(x)`), it is
    /// above the threshold, and it strictly beats the upper bound of every other
    /// candidate. Once every stream is exhausted the threshold is zero and all
    /// remaining candidates are ordered by the declared tie-break instead.
    fn select_emittable(&self) -> Result<Option<CandidateId>, FusionError> {
        if self.frontier.is_empty() {
            return Ok(None);
        }
        let active = self.heads.iter().any(Option::is_some);
        let mut best: Option<CandidateId> = None;
        for (id, state) in &self.frontier {
            if self.upper_bound(state)? != state.lower_bound {
                continue;
            }
            if active {
                if state.lower_bound <= self.threshold {
                    continue;
                }
                let mut dominated = false;
                for (other_id, other) in &self.frontier {
                    if other_id == id {
                        continue;
                    }
                    if state.lower_bound <= self.upper_bound(other)? {
                        dominated = true;
                        break;
                    }
                }
                if dominated {
                    continue;
                }
            }
            match &best {
                None => best = Some(id.clone()),
                Some(best_id) => {
                    let best_state = &self.frontier[best_id];
                    if Self::is_better(id, state, best_id, best_state) {
                        best = Some(id.clone());
                    }
                }
            }
        }
        Ok(best)
    }

    /// Process stream `index`'s head into the frontier, then advance it.
    ///
    /// # Errors
    ///
    /// [`FusionError::Overflow`] when the checked sum leaves the fixed-point
    /// range, [`FusionError::MaxContributionsExceeded`] when the candidate's
    /// contribution count leaves the profile's declared bound, and
    /// [`FusionError::CeilingExceeded`] when its accumulated score leaves the
    /// profile's declared ceiling.
    async fn pull(&mut self, index: usize) -> Result<(), FusionError> {
        let Some(head) = self.heads[index].take() else {
            return Ok(());
        };
        let stratum = self.streams[index].0.clone();
        let entry = self
            .frontier
            .entry(head.item.clone())
            .or_insert_with(CandidateState::new);
        entry.lower_bound = entry
            .lower_bound
            .checked_add(head.contribution)
            .map_err(|_| FusionError::Overflow)?;
        entry
            .contributions
            .push((stratum, head.rank, head.contribution));
        entry.seen_streams.insert(index);

        // Both bounds are checked against the profile's own accessors, never
        // recomputed, so a future change to either derivation cannot drift
        // enforcement away from what the profile declares.
        let count = u32::try_from(entry.contributions.len()).unwrap_or(u32::MAX);
        if count > self.profile.max_contributions() {
            return Err(FusionError::MaxContributionsExceeded {
                item: head.item.as_str().to_owned(),
                count,
                max: self.profile.max_contributions(),
            });
        }
        if entry.lower_bound > self.profile.ceiling() {
            return Err(FusionError::CeilingExceeded {
                item: head.item.as_str().to_owned(),
                score: entry.lower_bound,
                ceiling: self.profile.ceiling(),
            });
        }

        self.heads[index] = self.fetch(index).await?;
        Ok(())
    }
}
