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
//! candidates are removed, so the frontier holds only the un-emitted candidates
//! and never grows with how long the streams are.
//!
//! The frontier is not the whole of what a fusion holds, and saying otherwise
//! would overstate it. Per-stream duplicate detection
//! ([`ProtocolError::DuplicateItem`]) needs the set of items each stream has
//! already emitted, and that set is proportional to the rows **pulled**. The
//! frontier argument bounds exactly that: nothing here ever pulls a row it was
//! not asked for. Certifying the top `k` through [`FusionStream::next`] pulls
//! what the threshold argument requires and no more, and
//! [`FusionStream::trailer`] pulls nothing at all — it reports the state each
//! stream is already in. A caller that wants every row still pays for every
//! row, because it asked for them one at a time.

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
/// Three of the four are the producer's own declaration, converted from the
/// receipt it returned through [`RankedStream::receipt`] — plus, for the strata
/// that never became a stream, the executor's report carried in by
/// [`FusionTrailer::completed_with`].
///
/// [`Self::CeilingReached`] has a second author, and only one meaning. It says
/// that reading stopped at a contribution bound rather than at the end of the
/// rows: everything at or above `bound` was read, and what lies below it was
/// not. A producer says it when it stops at a bound of its own. The fusion says
/// it when the caller's row bound stopped it reading a stream that still held
/// rows — see [`FusionStream::trailer`], which is where a bounded stop is
/// written down. Either way the claim is the same and it is falsifiable in the
/// same way: this stratum is not complete below `bound`. Draining the stream to
/// make it say [`Self::Exhausted`] instead would be both a lie about the answer
/// and the end of the memory bound that makes top-k fusion worth having.
///
/// [`Self::TermsRejected`] has exactly one author, the producer, and it is
/// deliberately not minted anywhere else. A producer that accepts none of the
/// request's terms is rejected at planning
/// ([`RejectionReason::NoAcceptedTerm`](crate::RejectionReason)) and so compiles
/// no unit to run: it has no stratum in the answer to report under, and the
/// per-term fact that nothing served a term is reported per term instead, in
/// [`Plan::unserved_terms`](crate::Plan::unserved_terms). Minting a stratum
/// status from that would report one fact twice, in two shapes that could
/// disagree. What the variant is for is the producer that *was* handed terms and
/// declined them: "I did not look" and "I looked and found nothing"
/// ([`Self::Exhausted`] with no rows) are different answers, and a caller
/// composing [`RankedStream`] producers by hand through [`fuse`](crate::fuse)
/// can say either. The fusion engine validates the claim against the rows the
/// stream actually emitted, so it cannot be used to hide them.
///
/// [`RankedStreamImpl`](crate::RankedStreamImpl), this crate's own executor
/// stream, materializes its rows and declares [`Self::Exhausted`]; a unit that
/// cannot run at all becomes [`Self::ExecutionFailed`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerStatus {
    /// The producer emitted every row it had.
    Exhausted {
        /// How many rows it emitted.
        rows_emitted: u64,
    },
    /// Reading stopped at a contribution bound rather than at the end of the
    /// rows. Declared by the producer, or written down by a fusion the caller's
    /// row bound stopped; see this type's header.
    CeilingReached {
        /// The inclusive bound at which it stopped: every row at or above this
        /// contribution was read, and the rows below it were not.
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
    /// Carry a producer's own terminal declaration into the fused trailer,
    /// variant for variant.
    ///
    /// The two enums are deliberately separate types for the same four facts:
    /// a [`ProducerReceipt`] is what a producer *claims* on the input protocol,
    /// and a [`ProducerStatus`] is what the trailer *reports* after fusion has
    /// checked that claim against the rows it actually pulled. Keeping them
    /// apart is what stops an unverified claim from being mistaken for a
    /// verified one at the type level. The conversion is total and lossless, so
    /// nothing a producer said is reworded on the way through.
    ///
    /// Not every status arrives this way, and that is the other half of keeping
    /// the types apart: a stream the caller's row bound stopped never returned
    /// a receipt at all, so the fusion records its
    /// [`ProducerStatus::CeilingReached`] directly rather than mint a
    /// [`ProducerReceipt`] the producer never uttered.
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
    /// A candidate that has been named by some stream but has accumulated
    /// nothing yet.
    ///
    /// A zero lower bound is the honest starting point rather than a placeholder:
    /// before any contribution is summed, zero is exactly what this candidate is
    /// known to score.
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
    /// name the pinned plan the streams came from. [`fuse`](crate::fuse) does
    /// that for its caller, reading the identity off the streams themselves
    /// through [`RankedStream::plan_id`]. No stream is pulled until the first
    /// [`next`](Self::next) call.
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

    /// Close every stream that is still open at the bound reading stopped at,
    /// and return the terminal trailer.
    ///
    /// Every stream this fusion was handed gets a status, and no stream is read
    /// to produce one. A stream that reached its own end already returned a
    /// receipt, which was checked against the rows it emitted
    /// ([`ProtocolError::ForgedReceipt`]) and is reported unchanged. A stream
    /// still holding rows — because the caller stopped certifying, or because
    /// the threshold argument never needed to look further — is closed with
    /// [`ProducerStatus::CeilingReached`] at the contribution of its current
    /// head: the last row read from it, and therefore the inclusive bound above
    /// which it was read completely.
    ///
    /// # A bounded stop rather than a drain
    ///
    /// Draining here would force each producer to utter `Exhausted`, and the
    /// trailer would then say every stratum was complete — which is false
    /// whenever a bound stopped the reading, and expensive exactly when the
    /// bound mattered most: it pulls every row of every stream, so a top-ten
    /// answer over three million-row strata would read three million rows.
    /// §7's memory bound is the whole reason fused enumeration is top-k, and a
    /// terminal report is not allowed to spend it.
    ///
    /// The bounded status is not a weaker claim, it is an honest one. §6 asks
    /// that every producer keep its own status, not that every producer be
    /// exhausted, and "read down to this contribution and no further" is a
    /// status a consumer can act on: it names precisely where this stratum's
    /// evidence ends. Completeness is still asserted only here, never
    /// mid-stream, and still never by the rows a caller happens to hold.
    ///
    /// A producer that never became a stream is not here and cannot be; the
    /// stage that ran it adds it through [`FusionTrailer::completed_with`].
    ///
    /// Nothing is consumed: the streams are left exactly as they were, so a
    /// caller that reads the trailer and then decides to certify further rows
    /// through [`next`](Self::next) still can, and each stream's own receipt
    /// replaces its bounded status if it later reaches the end.
    ///
    /// # Errors
    ///
    /// [`FusionError::Protocol`] when a stream violates the protocol while its
    /// first row is pulled, and [`FusionError::Overflow`] on a checked sum
    /// overflow — both only on a fusion whose streams have not been read at all
    /// yet, because a bound of zero rows must still report a status per
    /// producer.
    pub async fn trailer(&mut self) -> Result<FusionTrailer, FusionError> {
        self.ensure_initialized().await?;
        for index in 0..self.streams.len() {
            // A head is a row already pulled and not yet merged, so its
            // contribution is the lowest this fusion read from the stream;
            // contributions are monotonically non-increasing with rank, so
            // everything above it was read too. A stream with no head has
            // already recorded its own receipt.
            let Some(bound) = self.heads[index].as_ref().map(|head| head.contribution) else {
                continue;
            };
            let stratum = self.streams[index].0.clone();
            self.statuses
                .insert(stratum, ProducerStatus::CeilingReached { bound });
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

        // The stratum is read by reference, not cloned: an `Iri` owns its text,
        // so cloning one here would be a heap allocation on every row every
        // stream emits, to serve a lookup that only borrows and an error arm
        // that is taken once at most.
        let weight = {
            let stratum = &self.streams[index].0;
            self.profile
                .weight(stratum)
                .ok_or_else(|| FusionError::UnknownStratum {
                    stratum: stratum.as_str().to_owned(),
                })?
        };
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

    /// Whether `other` could still be ordered ahead of the finalized `state`.
    ///
    /// `state` is known final (`U == L`), so the question is only what `other`
    /// can still become. Three cases, and only the third is subtle:
    ///
    /// * `U(other) > L(state)` — `other` may still outscore it. Blocked.
    /// * `U(other) < L(state)` — `other` can never catch it. Free.
    /// * `U(other) == L(state)` — `other` can at most tie. If `other` is itself
    ///   final its tie-break keys are settled, so the declared total order
    ///   decides and exactly one of the pair goes first. If `other` is not yet
    ///   final its best rank can still improve — a later stream may report it at
    ///   a better rank — so it could win a tie it cannot yet be compared on, and
    ///   the conservative answer is to wait.
    ///
    /// The third case is what keeps the frontier bounded: without it a pair of
    /// finalized, exactly-tied candidates blocks on itself forever.
    fn outranks(
        &self,
        other_id: &CandidateId,
        other: &CandidateState,
        id: &CandidateId,
        state: &CandidateState,
    ) -> Result<bool, FusionError> {
        let other_upper = self.upper_bound(other)?;
        Ok(match other_upper.cmp(&state.lower_bound) {
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Equal => {
                if other_upper == other.lower_bound {
                    Self::is_better(other_id, other, id, state)
                } else {
                    true
                }
            }
        })
    }

    /// The best candidate that is safe to emit, if any.
    ///
    /// A candidate is emittable when its score is final (`U(x) == L(x)`), it is
    /// above the threshold, and no other candidate could still be ordered ahead
    /// of it. Once every stream is exhausted the threshold is zero and all
    /// remaining candidates are ordered by the declared tie-break instead.
    ///
    /// # Ties are broken here, not only among the already-emittable
    ///
    /// "Could be ordered ahead of it" is the declared total order — score, then
    /// best rank, then canonical term bytes — and not score alone. A rival whose
    /// upper bound merely *equals* this candidate's final score cannot outscore
    /// it; it can at most tie, and a tie is what the tie-break exists to settle.
    ///
    /// Comparing scores alone would make two candidates with exactly equal final
    /// scores each dominate the other, so neither would ever be emittable and
    /// fusion would pull every stream to exhaustion before the exhausted branch
    /// below could order them. That is not a corner case: reciprocal-rank fusion
    /// over strata that disagree symmetrically produces exact ties routinely —
    /// one candidate at ranks 1 and 2, another at 2 and 1, sum to the same value
    /// — and the cost is the bounded frontier this type exists to provide.
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
                    if self.outranks(other_id, other, id, state)? {
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

        // The candidate's next state is derived before the frontier is touched,
        // for two reasons. It lets the bounds be refused while `head.item` is
        // still owned here, so the frontier key is *moved* into the map rather
        // than cloned — an allocation that would otherwise be paid on every row
        // every stream emits, including the ones already in the frontier. And a
        // refused row then leaves the frontier exactly as it found it, instead
        // of a half-updated candidate no later call may read.
        //
        // Both bounds are checked against the profile's own accessors, never
        // recomputed, so a future change to either derivation cannot drift
        // enforcement away from what the profile declares.
        let existing = self.frontier.get(&head.item);
        let lower_bound = existing
            .map_or(Fixed::ZERO, |state| state.lower_bound)
            .checked_add(head.contribution)
            .map_err(|_| FusionError::Overflow)?;
        let contribution_count = existing.map_or(0, |state| state.contributions.len()) + 1;
        let count = u32::try_from(contribution_count).unwrap_or(u32::MAX);
        if count > self.profile.max_contributions() {
            return Err(FusionError::MaxContributionsExceeded {
                item: head.item.as_str().to_owned(),
                count,
                max: self.profile.max_contributions(),
            });
        }
        if lower_bound > self.profile.ceiling() {
            return Err(FusionError::CeilingExceeded {
                item: head.item.as_str().to_owned(),
                score: lower_bound,
                ceiling: self.profile.ceiling(),
            });
        }

        let entry = self
            .frontier
            .entry(head.item)
            .or_insert_with(CandidateState::new);
        entry.lower_bound = lower_bound;
        entry
            .contributions
            .push((stratum, head.rank, head.contribution));
        entry.seen_streams.insert(index);

        self.heads[index] = self.fetch(index).await?;
        Ok(())
    }
}
