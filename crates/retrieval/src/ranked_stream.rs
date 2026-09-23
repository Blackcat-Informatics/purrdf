// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ranked-stream input protocol.
//!
//! A stratum's producer emits its rows in rank order, one row at a time, and
//! declares when it is done. Fusion is only sound if those declarations are
//! true, so the boundary is validated: the fusion engine checks rank contiguity
//! and re-derives every contribution as rows arrive, and compares each
//! producer's terminal receipt against what it actually emitted. A producer that
//! cannot describe its own completeness cleanly reports it through this channel;
//! it never gets to return a plausible-looking answer that quietly omitted a
//! row.
//!
//! # One fixed rank law, and three declared promises
//!
//! The rank check is absolute. Ranks are 1-based, contiguous and ascending for
//! every ranked stream there is, with no registration able to soften it, and the
//! fusion engine measures every row against the next rank it expects from that
//! stream ([`ProtocolError::OutOfOrderRanks`],
//! [`ProtocolError::NonContiguousRanks`]).
//!
//! The other checks are not absolute, because the registry does not state them
//! absolutely. A ranked producer declares its duplicate handling, the blocks of
//! the candidate universe it may name, and the fidelity of the rows it can name
//! where it is registered ([`RankedDeclaration`]), and the consumer of its rows
//! is this layer — so the consumer reads those declarations and holds the stream
//! to the promises it actually made. [`StreamContract`] carries all three with
//! the stream through [`RankedStream::contract`].
//!
//! # Two of the three are refusable; the third is not, and that is the point
//!
//! A false duplicate policy and a false domain declaration are both contradicted
//! by a row that *arrives* — the repeat, or the candidate outside the declared
//! block — so each has a refusal below naming the declaration the row was
//! measured against ([`ProtocolError::DuplicateItem`],
//! [`ProtocolError::OutsideDeclaredDomain`]).
//!
//! [`StreamContract::fidelity`] has no refusal and can have none, because what
//! would falsify it is precisely what never arrives. A stream that ran out of
//! rows and a stream whose search merely stopped finding them are
//! indistinguishable from here: both stop yielding, both leave contiguous ranks
//! behind. So an undeclared approximation is invisible to this layer by
//! construction, and silence about it reads as completeness — which is the
//! reading the term exists to stop being automatic. It is carried into the
//! answer rather than checked at the door.
//!
//! All three are promises about rows nobody has read yet, so all three are
//! verified exactly as far as the rows actually pulled reach, and no further.
//! That is stated on each term rather than implied: a declaration no pulled row
//! contradicts is believed, because there is nothing else a consumer could do
//! with it short of reading the whole stream, which is the cost the declaration
//! exists to avoid.
//!
//! # The domain promise is backed row by row
//!
//! A restricted declaration is not merely a set a consumer compares other
//! declarations against. Every row says which block of the candidate universe it
//! was drawn from ([`RankedRow::block`]), because the arithmetic the restriction
//! licenses rests on an axiom about the host's corpus — the tags **partition**
//! the candidate universe, so a candidate lies in exactly one block — and a row
//! that names its block is what makes that axiom checkable at all.
//!
//! So a [`CandidateDomains::Within`] stream owes a block on every row, and it
//! owes one its own declaration admits
//! ([`ProtocolError::UnbackedDomainDeclaration`],
//! [`ProtocolError::BlockOutsideDeclaredDomain`]). A
//! [`CandidateDomains::Unrestricted`] stream owes none: it restricts no
//! arithmetic — its head counts in every block's bound — so there is no promise
//! for a row to back. It may still name one, and a named block is honoured
//! whoever named it, because a block is evidence about the *candidate* rather
//! than about the stream.
//!
//! Two streams that name one candidate from two different blocks have proven the
//! axiom false for that candidate, and that is
//! [`ProtocolError::CandidateInTwoBlocks`] — the refusal that keeps a false
//! tagging from becoming a wrong order rather than an error. It is the same
//! verified-as-far-as-the-rows-reach standard as the two promises above: a
//! violation among rows nobody pulled is not detected, and this layer does not
//! claim otherwise.

use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, DuplicatePolicy, ExclusionBasis, PfAttestation, RankFidelity,
    RankedDeclaration,
};
use purrdf_text::Fixed;

use crate::iri::Term;

/// The promise a ranked producer makes about one invocation's rows: whether an
/// item may repeat.
///
/// It is the producer's own — [`RankedDeclaration::duplicates`], supplied by the
/// host where the producer is registered. It is not inferred and has no default:
/// a stream either repeats items or it does not, and the consumer's behaviour
/// differs, so there is no honest "unstated" answer the way there is for a
/// stream that descends from no plan ([`RankedStream::plan_id`]).
///
/// # Why rank order is not a term of the contract
///
/// Rank order is a law, not a promise a producer gets to phrase. Every ranked
/// stream owes its consumer the same one: ranks are 1-based, contiguous and
/// ascending, so rows arrive numbered 1, 2, 3 with no gap, no repeat and no step
/// backwards, and rows a producer scores equally still take distinct
/// consecutive ranks under whatever total tie-break it applies. Because the law
/// is identical for every stream, there is nothing for a contract to carry and
/// no variant a consumer could branch on.
///
/// It is enforced rather than believed, and enforced on the rank itself:
/// [`FusionStream`](crate::FusionStream) holds the next rank it expects from
/// each stream and checks every row against it, raising
/// [`ProtocolError::OutOfOrderRanks`] below that rank and
/// [`ProtocolError::NonContiguousRanks`] above it. The only other quantity
/// fusion can observe is the *contribution*, which it computes itself from
/// `(decay rule, K, weight, rank)` and refuses if the producer's copy disagrees
/// ([`ProtocolError::ContributionMismatch`]); the producer supplies no term of
/// that either. Reading a rank claim in contribution space instead refused
/// conforming producers whenever fixed-point decay quantized two adjacent ranks
/// to one value — a property of the profile's arithmetic and of the depth read,
/// never of the stream.
///
/// # Why the contract travels with the stream
///
/// A stratum carries exactly one producer, refused at registration by
/// [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
/// and again at the admission waist, so a stratum's contract *is* its producer's
/// contract — nothing is combined, averaged or weakened across producers to
/// obtain it. The admission waist reads it off the registry once, into
/// [`StratumUnit::contract`](crate::StratumUnit), [`execute`](crate::execute)
/// tags every stream it returns with it
/// ([`StratumStream::contract`](crate::StratumStream)), and
/// [`FusionStream`](crate::FusionStream) asks each stream for it before pulling
/// a row. That is the same route a plan identity takes, and for the same reason:
/// a fact re-fetched at the end would be true of the registry rather than of the
/// stream that is actually being read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreamContract {
    /// The duplicate handling the producer declared. A
    /// [`DuplicatePolicy::Unique`] stream is believed and a repeat is a protocol
    /// violation; a [`DuplicatePolicy::Allowed`] stream is de-duplicated by the
    /// consumer, which is what that policy says a consumer must do.
    pub duplicates: DuplicatePolicy,
    /// What the producer declared about the rows it can name: whether its search
    /// finds every row that was due, and whether a row it does name arrives at a
    /// rank no better than the one it earned.
    ///
    /// [`RankFidelity::EXACT`] is what every stream promised before this term
    /// existed, and a fusion of such streams behaves exactly as it always did.
    /// A degraded declaration is what lets the answer stop claiming more than it
    /// can support — see [`ScoreExactness`](crate::ScoreExactness), which reads
    /// this term, and [`RankFidelity`] for why the two axes fail independently.
    ///
    /// # Why this is declared rather than observed
    ///
    /// Because a consumer cannot tell the difference. A stream that ran out of
    /// rows and a stream whose search merely stopped finding them both simply
    /// stop yielding, and the ranks are contiguous either way. Nothing in the
    /// input protocol distinguishes them, so an undeclared approximation is
    /// invisible here by construction and is read as completeness — which is
    /// the reading this term exists to stop being automatic.
    ///
    /// # Why there is no default, when one looks obviously safe
    ///
    /// [`RankFidelity::EXACT`] is the top of the lattice, so defaulting to it
    /// would put the *strongest* claim in the mouth of a producer that said
    /// nothing. That is the one direction a default must never go. This is the
    /// identical refusal [`Self::domains`] already makes, for the identical
    /// reason: the value is a promise about the producer's own search, and only
    /// the host that wired it up knows it.
    ///
    /// # It is believed, on the same terms as its neighbours
    ///
    /// Fusion cannot verify a fidelity declaration any more than it can verify
    /// that a [`DuplicatePolicy::Unique`] stream will not repeat — the rows that
    /// would falsify it are exactly the rows that never arrived. A producer
    /// declaring [`RankFidelity::EXACT`] while quietly missing rows yields an
    /// answer that declaration made wrong, and this layer does not claim
    /// otherwise.
    pub fidelity: RankFidelity,
    /// Which blocks of the candidate universe the producer declared it may
    /// name.
    ///
    /// [`CandidateDomains::Unrestricted`] says "anything", which is the wider
    /// promise and the one every stream made before this term existed:
    /// [`FusionStream`](crate::FusionStream) then behaves exactly as it always
    /// did on this term's account. A [`CandidateDomains::Within`] declaration is
    /// one of the two ways fusion can certify a candidate without first reading
    /// a stream that was never going to name it — the drain that makes a top-ten
    /// answer over two disjoint million-row strata read two million rows. The
    /// other is [`Self::exclusion`], so `Unrestricted` here does not on its own
    /// mean the read drains: a producer that answers a lookup bounds it by
    /// observation instead.
    ///
    /// # Why the consumer cannot derive this for itself
    ///
    /// Because a stream that says nothing about its own candidates offers no
    /// random access. Against such a producer a [`RankedStream`] is `next` and
    /// `receipt`, so the only way to learn that it does *not* hold a candidate
    /// is to read it to its end. A consumer that certified earlier knowing
    /// nothing would be emitting a score that a still-open stream might have
    /// raised — a lower bound presented as an exact value — so exact scores and
    /// a k-bounded read over strata that do not overlap are jointly unachievable
    /// unless the producers say something. This is one of the two things they
    /// can say, and [`Self::exclusion`] is the other: this promises about whole
    /// blocks, once, and is read without asking; that is answered per candidate,
    /// and reaches the case this cannot — two producers over one block whose
    /// results never overlap, where both declarations are true and neither
    /// settles anything.
    ///
    /// # It is verified over the rows pulled, and nowhere else
    ///
    /// Fusion refuses a stream that names a candidate its declaration cannot
    /// reach ([`ProtocolError::OutsideDeclaredDomain`]), which catches every
    /// contradiction that arrives in a row it actually read. A false
    /// declaration that no pulled row contradicts yields a score that
    /// declaration made wrong, and this layer does not claim otherwise. It is
    /// the identical trust the [`Self::duplicates`] term already carries: a
    /// false [`DuplicatePolicy::Unique`] is detected when the repeat is pulled,
    /// and not before.
    pub domains: CandidateDomains,
    /// What the producer's answer to an **exclusion lookup** means, or
    /// [`ExclusionBasis::Unavailable`] where it answers none.
    ///
    /// The one term of this contract that is not a promise about rows nobody has
    /// read: it is permission to *ask*. [`Self::domains`] says which blocks of
    /// the universe this producer can reach, which settles finality only where
    /// the blocks separate the producers; this is how a consumer settles it
    /// where they do not, by asking about one candidate and being answered from
    /// the producer's own index.
    ///
    /// # Why the basis travels here and the verdict does not
    ///
    /// The basis is a standing declaration — true from registration, unchanged
    /// by how deep anyone reads — so it belongs beside the three terms that are
    /// already read once, before the first row. The verdict is a *measurement*,
    /// taken per candidate against the dataset, and it arrives through
    /// [`RankedStream::exclusion`] where its failures can fail the request that
    /// asked for it.
    ///
    /// # It is believed as a basis and checked as an answer
    ///
    /// Nothing here can verify that a producer's `Membership` really is
    /// membership. What is verified is the *agreement* between this channel and
    /// the rows: a producer that excludes a candidate and then names it is
    /// refused by name ([`ProtocolError::ExclusionContradicted`]), and a
    /// producer that calls a candidate possible while its own declared domains
    /// put that candidate out of reach is refused as the domain violation it is
    /// ([`ProtocolError::OutsideDeclaredDomain`]).
    ///
    /// # Why there is no default here either
    ///
    /// [`ExclusionBasis::Unavailable`] looks like the safe default and is not
    /// one to fabricate: it is the *narrow* answer, so defaulting to it would
    /// silently discard a capability a producer really has, and defaulting to
    /// either of the others would put a claim about a host's corpus in the mouth
    /// of a producer that made none. Both directions are wrong, which is why the
    /// term is positional in [`Self::new`] like its neighbours.
    pub exclusion: ExclusionBasis,
}

impl StreamContract {
    /// The contract `declaration` states, verbatim.
    #[must_use]
    pub fn declared(declaration: &RankedDeclaration) -> Self {
        Self {
            duplicates: declaration.duplicates,
            exclusion: declaration.exclusion,
            // Cloned, never rebuilt. The evidence inside is an `Arc<str>` the
            // producer authored, and a consumer reads those bytes rather than a
            // summary of them, so this hop must move the string itself — one
            // refcount bump, and the same characters out as in.
            fidelity: declaration.fidelity.clone(),
            domains: declaration.domains.clone(),
        }
    }

    /// A contract stated directly, for a stream a caller built itself.
    ///
    /// Every term is positional and none may be omitted, which is the other half
    /// of what lets a consumer read an absent declaration as
    /// [`RankFidelity::EXACT`]: a caller assembling a stream by hand states its
    /// fidelity or does not compile.
    ///
    /// ```compile_fail
    /// # use purrdf_retrieval::{StreamContract, DuplicatePolicy, CandidateDomains, RankFidelity};
    /// // The arity before the exclusion term existed. There is no overload and
    /// // no default to fall back to.
    /// let _ = StreamContract::new(
    ///     DuplicatePolicy::Unique,
    ///     RankFidelity::EXACT,
    ///     CandidateDomains::Unrestricted,
    /// );
    /// ```
    ///
    /// ```
    /// # use purrdf_retrieval::{
    /// #     StreamContract, DuplicatePolicy, CandidateDomains, RankFidelity, ExclusionBasis,
    /// # };
    /// let contract = StreamContract::new(
    ///     DuplicatePolicy::Unique,
    ///     RankFidelity::EXACT,
    ///     CandidateDomains::Unrestricted,
    ///     ExclusionBasis::Unavailable,
    /// );
    /// assert_eq!(contract.fidelity, RankFidelity::EXACT);
    /// assert_eq!(contract.exclusion, ExclusionBasis::Unavailable);
    /// ```
    ///
    /// The pair is the proof: the `compile_fail` block alone would pass for any
    /// error at all, so the twin differing only in the supplied term is what
    /// shows the term is the reason.
    ///
    /// Not a `const fn`: the fidelity term carries the producer's own evidence,
    /// which is a string, and a string cannot cross a `const fn`. Nothing in
    /// this workspace declared a `StreamContract` in a `const` context, so the
    /// loss costs a caller nothing — and the alternative, keeping `const` by
    /// making the evidence a `&'static str`, would have confined the term to
    /// producers whose disclosure is compiled in and shut out every producer
    /// that reads its own from a loaded artifact.
    #[must_use]
    pub fn new(
        duplicates: DuplicatePolicy,
        fidelity: RankFidelity,
        domains: CandidateDomains,
        exclusion: ExclusionBasis,
    ) -> Self {
        Self {
            duplicates,
            fidelity,
            domains,
            exclusion,
        }
    }
}

/// What a producer says about one candidate when it is asked whether that
/// candidate is out of its reach.
///
/// Two variants and a third outcome. The `Err` arm of
/// [`RankedStream::exclusion`] is the third, and it is not a variant here on
/// purpose: a lookup that *failed* has said nothing, and the one value a failure
/// must never collapse into is [`Self::Possible`] — which reads as the safe,
/// conservative answer while being indistinguishable from a working lookup that
/// answered honestly. Such a collapse costs nothing that any test would notice:
/// the answer stays correct, the read merely stops narrowing, and a dataset read
/// that has been failing for months looks exactly like a corpus whose producers
/// overlap. So the failure is a typed error that fails the fused request, and
/// this enum has no arm for it to hide in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExclusionVerdict {
    /// This producer will never name this candidate.
    ///
    /// What that *means* is [`StreamContract::exclusion`]'s basis: under
    /// [`ExclusionBasis::Membership`] the candidate is not in the producer's
    /// term universe at all, and under [`ExclusionBasis::Search`] the
    /// producer's (complete) search did not find it. Either way a consumer may
    /// stop waiting for this stream to name it — and if the stream names it
    /// anyway, that is [`ProtocolError::ExclusionContradicted`] rather than a
    /// quietly merged extra contribution.
    Excluded,
    /// This producer may still name this candidate.
    ///
    /// The honest answer whenever the producer cannot rule the candidate out,
    /// and the answer a consumer already assumed before it asked — so a fusion
    /// over producers that answer `Possible` to everything reads exactly what it
    /// read before exclusion lookups existed, minus nothing.
    Possible,
}

/// Which block of the candidate universe one row was drawn from, or an honest
/// absence.
///
/// # Why `Undeclared` is a first-class value and not an `Option`
///
/// This follows [`IndexGeneration::Undeclared`](purrdf_sparql_eval::IndexGeneration)
/// and [`PfAttestation::UNDECLARED`], for the reason those exist: a producer may
/// have nothing to say, and asking it to fabricate a block would be asking it to
/// fabricate evidence. A stream computed over its arguments, a walk of the
/// dataset, an unrestricted index that holds no notion of a host's partition —
/// none of those has a block, and `None` would invite a reader to treat the
/// absence as a value it could unwrap or default. The variant says what happened:
/// this row named no block.
///
/// It is silence, never a claim. [`Self::Undeclared`] does **not** say the
/// candidate is outside every block, does not say it is in all of them, and
/// carries no permission to pick one — it says nobody stated anything, which is
/// exactly why a [`CandidateDomains::Within`] stream may not utter it (see
/// [`ProtocolError::UnbackedDomainDeclaration`]): that stream has already made a
/// promise, and a row that declares nothing is a row that backs nothing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowBlock {
    /// The row names no block. See this type's docs: an absence, not a claim.
    Undeclared,
    /// The row was drawn from this block of the candidate universe.
    ///
    /// A [`DomainTag`] is a caller-chosen IRI naming a block of a partition, so
    /// this is the producer's statement that *this candidate* lies in *that*
    /// block — a fact about the candidate, which is why a second stream naming
    /// the same candidate in a different block is a contradiction
    /// ([`ProtocolError::CandidateInTwoBlocks`]) rather than a difference of
    /// opinion.
    Declared(DomainTag),
}

impl RowBlock {
    /// The block this row names, or `None` where it named none.
    ///
    /// A reader that needs the tag itself — to compare two rows, or to measure
    /// one against a declaration — asks for it here. The absence stays an
    /// absence: this is a projection of the variant, never a defaulting of it.
    #[must_use]
    pub const fn tag(&self) -> Option<&DomainTag> {
        match self {
            Self::Undeclared => None,
            Self::Declared(tag) => Some(tag),
        }
    }
}

/// One row of a ranked stream: its rank, its contribution, the item it names,
/// and the block that item was drawn from.
///
/// A named value rather than a tuple because the row now carries four facts, two
/// of which are the same shape to a reader skimming a call site — and because
/// three of them are claims a consumer checks against a different thing: the rank
/// against the rank law, the contribution against the profile, and the block
/// against the stream's own declaration and against every other row that named
/// the same item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedRow<I> {
    /// The row's 1-based rank within this stream. Contiguous and ascending; see
    /// [`RankedDeclaration`]'s note on the single rank law.
    pub rank: u64,
    /// The profile's reciprocal-rank contribution for this rank, which fusion
    /// re-derives and refuses on a mismatch
    /// ([`ProtocolError::ContributionMismatch`]).
    pub contribution: Fixed,
    /// The candidate this row names.
    pub item: I,
    /// The block of the candidate universe this row was drawn from.
    ///
    /// Owed by a [`CandidateDomains::Within`] stream on every row, and owed by
    /// no other — see this module's header, and [`RowBlock`] for why the absence
    /// is a value rather than an `Option`.
    pub block: RowBlock,
}

impl<I> RankedRow<I> {
    /// A row that names `block`.
    #[must_use]
    pub const fn new(rank: u64, contribution: Fixed, item: I, block: RowBlock) -> Self {
        Self {
            rank,
            contribution,
            item,
            block,
        }
    }
}

/// A producer's declaration of how its stream ended.
///
/// This is the *receipt* a producer returns from [`RankedStream::receipt`]. It
/// becomes a [`ProducerStatus`](crate::ProducerStatus) in the fused trailer, and
/// the trailer preserves every producer's own status rather than reducing them
/// to one aggregate flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerReceipt {
    /// The producer emitted every row **its search produced**, and stopped
    /// because there were no more rather than because something stopped it.
    ///
    /// The count is checked against the rows fusion actually pulled, so a
    /// producer may not miscount what it emitted. Nothing here checks — or
    /// could check — that what it emitted was everything that was due: that is
    /// what [`StreamContract::fidelity`] declares, and the two are read
    /// together in the answer. A producer whose search is exhaustive says the
    /// stronger thing with this receipt; one that declared
    /// [`Completeness::Lossy`](purrdf_sparql_eval::Completeness::Lossy) says
    /// only that its search ran out.
    Exhausted {
        /// How many rows it emitted. Must equal the number fusion pulled.
        rows_emitted: u64,
    },
    /// The producer stopped at the depth it was given, and more rows existed.
    ///
    /// A read ending authored by the producer, stated in **rank** space: ranks
    /// one through `rank` were read and emitted, and the producer looked at
    /// nothing below `rank`. The rows did not run out — the depth did. A
    /// producer handed a depth of fifty that still had a fifty-first row says
    /// this; a producer handed a depth of fifty whose index held forty rows says
    /// [`Self::Exhausted`], because the depth is not what stopped it -- its own
    /// rows ran out first. Whether those were every row that was DUE is a
    /// question that ending does not answer; the stratum's fidelity does.
    ///
    /// # Why this is not [`Self::CeilingReached`]
    ///
    /// Both say "this stratum is not complete", and that is the whole of what
    /// they share. They differ in who stopped the read and in what space the
    /// stopping point is written:
    ///
    /// * this one is the **producer's** ending, and its stopping point is a
    ///   rank — a number the producer was handed as its depth and can act on
    ///   directly;
    /// * [`Self::CeilingReached`] as fusion writes it
    ///   ([`FusionStream::trailer`](crate::FusionStream::trailer)) is
    ///   **fusion's** ending, and its stopping point is a contribution — a
    ///   value in the profile's fixed-point space that the producer was never
    ///   given and that names no depth the producer understands.
    ///
    /// Collapsing them into one "stopped early" would destroy exactly the
    /// distinction a consumer acts on. A consumer that wants more rows must
    /// decide which knob to turn, and the two variants name different knobs:
    /// this one is answered by re-planning at a greater depth, the other by
    /// certifying further rows against the same streams. Neither number
    /// converts into the other, either — a rank becomes a contribution only
    /// under a profile the producer never saw, and a contribution does not
    /// convert back into a rank at all once fixed-point decay has quantized two
    /// adjacent ranks to one value. A consumer handed the merged fact would
    /// have to guess, and half its guesses would be wrong.
    ///
    /// A producer may stop at its depth; it may not miscount what it emitted.
    /// Fusion checks `rank` against the rows it actually pulled and refuses a
    /// disagreement as [`ProtocolError::ForgedReceipt`], exactly as it checks
    /// [`Self::Exhausted`]'s count — the depth is a licence to stop reading,
    /// never a licence to misreport.
    DepthReached {
        /// The last 1-based rank the producer emitted. Because ranks are
        /// contiguous from one, this is also the number of rows it emitted,
        /// which is what fusion measures it against.
        rank: u64,
    },
    /// The producer stopped at the row bound it had itself declared, so whether
    /// anything lay below that rank could not be observed.
    ///
    /// The ending of a producer that bounds *itself*. A relation that declared a
    /// [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) is handed its depth as
    /// an argument rather than bounded by a `LIMIT`, and that argument is never
    /// raised past the row count the relation registered — asking for more asks the
    /// relation to contradict its own registration, which a conforming relation
    /// refuses. So at a depth that already sits on the declaration the read is asked
    /// for exactly `rank` rows, returns exactly `rank` rows, and no row past them
    /// could have been requested.
    ///
    /// # Why this is neither of the two endings beside it
    ///
    /// * [`Self::Exhausted`] would claim the rows ran out. Nobody looked: the read
    ///   stopped where it was told to, and an index holding a thousand rows and one
    ///   holding exactly `rank` are indistinguishable from here. That is the
    ///   completeness claim minted from a declaration that this whole vocabulary
    ///   exists to prevent.
    /// * [`Self::DepthReached`] would name the planned depth as the stopper and
    ///   assert that a further row existed. Neither half is known: the depth was
    ///   reached, but it is the producer's own bound that made the row past it
    ///   unaskable, and whether such a row exists is exactly what could not be
    ///   observed.
    ///
    /// So it names the stopper it really had. A consumer that wants the question
    /// answered has one honest move, and it is a different move from either
    /// neighbour's: raise the producer's declared row bound — re-planning deeper
    /// cannot help, because the depth is already at the declaration and the argument
    /// will not be raised past it.
    ///
    /// `rank` is measured like [`Self::DepthReached`]'s: a producer may stop at the
    /// bound it declared, and it may not miscount what it emitted
    /// ([`ProtocolError::ForgedReceipt`]).
    RowBoundReached {
        /// The last 1-based rank the producer emitted, which is the declared row
        /// bound it read to. Because ranks are contiguous from one, it is also the
        /// number of rows it emitted, which is what fusion measures it against.
        rank: u64,
    },
    /// The read ran a query text this layer did not write, and that text is the
    /// stopper: whether anything lay below `rank` could not be observed.
    ///
    /// The ending of a unit a caller assembled itself
    /// ([`StratumUnit::new`](crate::StratumUnit::new)). The retrieval layer renders a
    /// bound one row past the planned depth onto such a text, but only on its
    /// *outside*; a `LIMIT` on a sub-`SELECT` inside it, or a pattern that matches
    /// less than the producer holds, cuts the read before that bound is consulted and
    /// is no part of what the layer reads of that text.
    ///
    /// # Why this is none of the three endings beside it
    ///
    /// * [`Self::Exhausted`] would claim the rows ran out, on the strength of a probe
    ///   slot that may never have existed. That is the one ending that names no
    ///   stopper, minted from a text whose bounds the layer never read — and it is
    ///   exactly the defect this vocabulary exists to prevent, reached through the one
    ///   door that stayed open longest.
    /// * [`Self::DepthReached`] would name the planned depth as the stopper and assert
    ///   that a further row existed. Neither half is known: the depth may not have
    ///   been reached at all.
    /// * [`Self::RowBoundReached`] would blame the producer's registration for a cut
    ///   the caller's own text may have made.
    ///
    /// A consumer that wants the question answered has one honest move, and it is a
    /// different move from any neighbour's: run the query
    /// [`compile`](crate::compile) renders, whose bounds this layer wrote and can
    /// therefore reason about.
    ///
    /// `rank` is measured like [`Self::DepthReached`]'s: a caller may run its own
    /// query, and the stream may not miscount what it emitted
    /// ([`ProtocolError::ForgedReceipt`]).
    SuppliedQueryEnded {
        /// The last 1-based rank the stream emitted, which — ranks being contiguous
        /// from one — is also the number of rows it emitted, and is what fusion
        /// measures it against. Zero for a read that emitted nothing, which is not a
        /// claim that there was nothing to emit.
        rank: u64,
    },
    /// The producer stopped at a declared score bound rather than at
    /// exhaustion.
    CeilingReached {
        /// The inclusive bound at which it stopped.
        bound: Fixed,
    },
    /// The producer could not run. It emits no rows.
    ExecutionFailed {
        /// A human-readable reason, carried verbatim.
        reason: String,
    },
    /// The producer declined the request terms it was given. It emits no rows.
    TermsRejected,
}

/// A producer's violation of the ranked-stream protocol.
///
/// Every variant names the exact dimension that failed. The fusion engine
/// returns these unchanged; it never repairs a malformed stream into a
/// plausible order.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A rank arrived below the next expected rank — a producer that went
    /// backwards.
    #[error("stream rank went backwards: expected {expected}, got {got}")]
    OutOfOrderRanks {
        /// The rank fusion expected next.
        expected: u64,
        /// The rank the producer supplied.
        got: u64,
    },

    /// A rank arrived above the next expected rank — a producer that skipped
    /// ranks.
    #[error("stream ranks are not contiguous: skipped {gap} rank(s)")]
    NonContiguousRanks {
        /// How many ranks were skipped.
        gap: u64,
    },

    /// The same item was emitted twice within one stream that declared
    /// [`DuplicatePolicy::Unique`].
    ///
    /// Raised only under that declaration. A stream declaring
    /// [`DuplicatePolicy::Allowed`] says repeats happen and the consumer must
    /// de-duplicate, and this layer is that consumer: it de-duplicates such a
    /// stream instead of refusing it — one contribution per `(stratum, item)`,
    /// at the best rank the stream gave it. Raising this for a policy that
    /// predicted the repeat would be a refusal of valid input.
    ///
    /// # The promise is held for the whole fusion, not merely for the frontier
    ///
    /// Raised wherever the repeat is observed: while the earlier occurrence is
    /// still an un-emitted frontier candidate, and equally after that
    /// occurrence has been certified and left the frontier. A fused answer may
    /// not contain one entity twice, and that is not a property a bound on how
    /// long fusion remembers can be allowed to weaken — so the engine
    /// remembers, and a producer that cannot keep the promise it declared is
    /// told so rather than smoothed over. See
    /// [`FusionStream::pull`](crate::FusionStream) for what the remembering
    /// costs and why it is bounded by the rows *emitted*.
    ///
    /// # Both fields, because "which stream lied" is the actionable half
    ///
    /// A repeated item names *what* went wrong; the stratum names *who*. A
    /// consumer fusing several producers can act on the pair — the named
    /// producer's declared [`DuplicatePolicy`] is wrong, and until it is fixed
    /// that stratum's ranks are not trustworthy — and can act on neither half
    /// alone: the item alone does not say which of five strata to go and fix,
    /// and the stratum alone does not say which of its rows to look at.
    #[error("stream for stratum {stratum} emitted item {item:?} more than once")]
    DuplicateItem {
        /// The repeated item's canonical text.
        item: String,
        /// The stratum whose stream repeated it, exactly as that stream was
        /// tagged when it was handed to fusion.
        stratum: String,
    },

    /// A stream named a candidate that its declared domains cannot reach.
    ///
    /// Raised only against a [`CandidateDomains::Within`] declaration, and only
    /// where another stream has already named the same candidate: a domain tag
    /// names a block of a partition of the candidate universe, so a candidate
    /// lies in exactly one block, and two producers whose declarations put it
    /// in blocks with nothing in common cannot both be telling the truth about
    /// it. One of the two declarations is wrong, and the consumer cannot know
    /// which — so it reports the contradiction rather than picking a side.
    ///
    /// # This is not a refusal of an overlapping index
    ///
    /// Two producers that really do rank the same entities are declared over
    /// the same tag, or over tag sets that share one, or
    /// [`CandidateDomains::Unrestricted`], and every one of those fuses
    /// normally — the same rows, the same score, one row carrying both
    /// contributions. What is refused is the *pair of declarations* that says
    /// those entities are in two disjoint blocks while the rows say otherwise.
    /// A host that finds this refusal firing has a tagging that does not
    /// describe its corpus, and widening the declaration is a one-line fix that
    /// costs only the early certification the narrower claim would have bought.
    ///
    /// # Why the answer is not silently widened instead
    ///
    /// Because the declaration has already been *used*. Fusion certifies
    /// candidates early on the strength of it, so by the time this row arrives
    /// an earlier answer may already have been emitted on the assumption this
    /// stream would never name it — and quietly merging the late contribution
    /// would put a score in the caller's hands that its own provenance
    /// contradicts. The same reasoning
    /// [`Self::DuplicateItem`] gives for refusing rather than dropping applies
    /// here: the evidence has been shown to be unreliable, and a plausible
    /// answer computed from it is the worst of the available outcomes.
    ///
    /// # All three fields, because each answers a different question
    ///
    /// The item says *what*, the stratum says *who broke it*, and `named_by`
    /// says *against what* — the stratum whose own declaration, already
    /// applied, put the candidate out of this one's reach. Without the third a
    /// reader sees a producer refused for naming one of its own documents and
    /// has no way to find the declaration it collided with.
    #[error(
        "stream for stratum {stratum} named item {item:?}, which its declared candidate domains \
         cannot reach; stratum {named_by} already named it"
    )]
    OutsideDeclaredDomain {
        /// The candidate's canonical text.
        item: String,
        /// The stratum whose stream named it outside its declaration, exactly
        /// as that stream was tagged when it was handed to fusion.
        stratum: String,
        /// A stratum that had already named the candidate, and whose own
        /// declared domains are what this one's cannot meet.
        ///
        /// Where several strata had named it, this is the first one whose
        /// declaration is genuinely disjoint from the offender's, in the order
        /// the streams were handed to fusion — the witness that makes the
        /// contradiction visible, rather than an arbitrary member of the set.
        named_by: String,
    },

    /// A stream that restricted its candidate domains emitted a row that names
    /// no block, so nothing backs the restriction.
    ///
    /// Raised only against a [`CandidateDomains::Within`] declaration, and
    /// against every such row — the first one is enough, because the promise is
    /// about the whole stream.
    ///
    /// # Why an unbacked restriction is refused rather than read as `Unrestricted`
    ///
    /// Because the restriction has already been *used*, and quietly widening it
    /// is not available at the point the row arrives: fusion skips streams that
    /// provably cannot name a candidate and bounds unseen items by the best
    /// single block, so rows may already have been certified on the strength of
    /// this declaration. The two remaining options are to refuse, or to keep
    /// certifying against an axiom — a candidate lies in exactly one block —
    /// that nothing in this fusion can check. Refusing is the one that does not
    /// hand a caller an order computed from an unchecked premise.
    ///
    /// It is not a refusal of a producer that cannot say which block its rows
    /// lie in. Such a producer declares [`CandidateDomains::Unrestricted`],
    /// which owes no block, fuses normally, and costs only the early
    /// certification the narrower claim would have bought; or it is registered
    /// once per block, each declaration naming the single block its rows really
    /// lie in, where the block is *entailed* by the declaration and no row has
    /// to repeat it. Both exits are one line, and both are in
    /// `RankedDeclaration::block_position`'s docs.
    ///
    /// # Both fields, because the promise and its author are different questions
    ///
    /// The stratum says who made a promise it does not back; the blocks say what
    /// the promise was, which is what a reader needs in order to choose between
    /// the two exits — one block means delete the row-level expectation, several
    /// means split the producer or name a block column. The rank says which row
    /// was measured, and it is not always the first: a stream may back its
    /// declaration for a hundred rows and then stop, and a refusal that named no
    /// rank would send a reader to row one.
    #[error(
        "stream for stratum {stratum} restricted its candidates to {declared:?} but the row at \
         rank {rank} names no block, so nothing backs that restriction"
    )]
    UnbackedDomainDeclaration {
        /// The stratum whose stream emitted the blockless row, exactly as that
        /// stream was tagged when it was handed to fusion.
        stratum: String,
        /// The blocks the stream declared, in canonical order.
        declared: Vec<String>,
        /// The 1-based rank of the row that named no block.
        rank: u64,
    },

    /// A row names a block its own stream's declaration does not include.
    ///
    /// The stream contradicts itself: it promised its candidates lie in a named
    /// set of blocks and then said this one came from outside that set. No
    /// second stream is involved and none is needed, which is what separates
    /// this from [`Self::OutsideDeclaredDomain`] — that one reports two
    /// declarations that cannot both be true of one candidate, and this one
    /// reports a single stream whose row contradicts its own registration.
    ///
    /// Raised whatever the stream declared *except* under
    /// [`CandidateDomains::Unrestricted`], which admits every block, so an
    /// unrestricted stream cannot reach it: a stream that promised nothing has
    /// nothing to contradict. That is deliberate rather than incidental — a
    /// block volunteered by an unrestricted stream is still honoured as evidence
    /// about the candidate (see [`Self::CandidateInTwoBlocks`]), and refusing it
    /// here would refuse a host that told the truth about a producer it had not
    /// restricted.
    ///
    /// # All four fields
    ///
    /// The item says *which row*, the stratum says *who*, `block` says what the
    /// row claimed and `declared` says what the registration claimed. The last
    /// two are the contradiction itself and neither half states it: a reader
    /// holding only the named block cannot see which set it fell outside, and a
    /// reader holding only the set cannot see what arrived.
    #[error(
        "stream for stratum {stratum} named item {item:?} in block {block}, which its declared \
         domains {declared:?} do not include"
    )]
    BlockOutsideDeclaredDomain {
        /// The candidate's canonical text.
        item: String,
        /// The stratum whose stream named the block, exactly as that stream was
        /// tagged when it was handed to fusion.
        stratum: String,
        /// The block the row named.
        block: String,
        /// The blocks that stream declared, in canonical order.
        declared: Vec<String>,
    },

    /// Two streams named one candidate from two different blocks, so the
    /// candidate lies in two blocks — which the whole declared-domain arithmetic
    /// says it cannot.
    ///
    /// This is the axiom itself, falsified by rows. A [`DomainTag`] names a
    /// block of a *partition* of the candidate universe, so a candidate lies in
    /// exactly one block; fusion bounds every item nobody has named yet by the
    /// largest single block's sum of open heads, and skips a stream whose
    /// declaration cannot reach a candidate's blocks when deciding that
    /// candidate's score is final. Both of those are sound only under the axiom.
    /// Two rows that place one candidate in two blocks are a proof the host's
    /// tagging does not describe its corpus, and everything computed under it —
    /// including rows already emitted — is a bound presented as a value.
    ///
    /// # Why this is not [`Self::OutsideDeclaredDomain`]
    ///
    /// That refusal compares *declarations*: it fires when a stream names a
    /// candidate the already-applied declarations of other streams put out of
    /// its reach. It therefore cannot see the case this one exists for — two
    /// streams whose declarations overlap, each naming the same candidate from a
    /// different block. Nothing about the declarations is contradictory there
    /// (one shared block would make both true), and the threshold is still
    /// wrong: the streams that can reach the first block and the streams that
    /// can reach the second are different sets, so an unseen item's bound is the
    /// larger single set while the candidate collected both. That gap is exactly
    /// what per-row blocks close, and it closes silently or not at all.
    ///
    /// # Five fields, because a reader has five questions
    ///
    /// The item names the candidate whose tagging is wrong. The two strata name
    /// the two producers whose rows disagree — one of the two tagged it wrongly
    /// and this layer cannot know which, so it names both rather than picking a
    /// side. The two blocks are the disagreement itself: without them a reader
    /// sees two strata that both legitimately hold the candidate and no reason
    /// they were refused. None of the five is derivable from the others, and no
    /// subset makes the report actionable.
    #[error(
        "streams for strata {stratum} and {named_by} name item {item:?} from two different \
         blocks, {block} and {named_by_block}: a candidate lies in exactly one block, so this \
         tagging cannot be true"
    )]
    CandidateInTwoBlocks {
        /// The candidate's canonical text.
        item: String,
        /// The stratum whose row arrived last, exactly as that stream was tagged
        /// when it was handed to fusion.
        stratum: String,
        /// The block that row named.
        block: String,
        /// The stratum that had already named the candidate — deterministically,
        /// the stream that recorded the candidate's block first in the order the
        /// streams were handed to fusion.
        named_by: String,
        /// The block that stratum named the candidate from.
        named_by_block: String,
    },

    /// A producer's contribution does not equal the profile's declared
    /// reciprocal-rank value for its stratum and rank. The fusion engine
    /// recomputes the contribution from the profile and refuses a mismatch
    /// rather than fuse a number the profile did not authorize.
    ///
    /// # Non-increase with rank is enforced here, and only here
    ///
    /// Fusion's certification argument depends on contributions being
    /// monotonically non-increasing with rank: the threshold summed over the
    /// stream heads is an upper bound only while they are. That law has no
    /// separate check, because after this one there is nothing left to check.
    /// Ranks reach the comparison already held contiguous and ascending
    /// ([`Self::OutOfOrderRanks`], [`Self::NonContiguousRanks`]), the stratum's
    /// weight is fixed for the whole stream, and
    /// `reciprocal_rank::contribution_under(decay, weight, rank)` is
    /// non-increasing in the rank for every rule and every weight a
    /// [`FusionProfile`](crate::FusionProfile) admits — a profile refuses a
    /// weight at or below zero, and the property is proven over the surviving
    /// domain by
    /// `reciprocal_rank::tests::every_decay_rule_is_non_increasing_in_the_rank`.
    /// So a value that rises with rank is necessarily a value the profile did
    /// not compute, and it is refused by this variant, as the wrong number it
    /// is. A separate "your contribution rose" refusal would blame the stream's
    /// shape for a wrong number, and nothing — conforming or hostile — could
    /// ever reach it.
    ///
    /// Equality between adjacent ranks is **not** a violation. The contribution
    /// is the consumer's own function of the rank, so two adjacent ranks carry
    /// one value exactly when the profile's fixed-point decay has stopped
    /// separating them at that depth. The answer stays correct and
    /// deterministic there — the declared tie-break is total — at a lower rank
    /// resolution, which the fused trailer reports per stratum rather than
    /// refusing.
    #[error("stream contribution {got:?} does not match the profile's {expected:?}")]
    ContributionMismatch {
        /// The value the profile computes for the stratum and rank.
        expected: Fixed,
        /// The value the producer supplied.
        got: Fixed,
    },

    /// A producer supplied a terminal receipt that contradicts what it emitted
    /// — declaring fewer rows than were pulled, or claiming it emitted none
    /// after it emitted some.
    #[error("producer receipt is inconsistent: declared {declared}, actually emitted {actual}")]
    ForgedReceipt {
        /// The row count the receipt declared (zero for the zero-row statuses).
        declared: u64,
        /// The row count fusion actually pulled.
        actual: u64,
    },

    /// A producer failed after it had already emitted rows. A clean
    /// [`ProducerReceipt`] cannot follow rows; this is the only honest way to
    /// report a mid-stream failure.
    #[error("producer failed after emitting {rows_before} row(s)")]
    ErrorAfterRows {
        /// How many rows the producer emitted before failing.
        rows_before: u64,
    },

    /// A producer did not terminate within its declared contribution budget.
    /// The producer reports this itself; fusion never invents an end for it.
    #[error("producer did not terminate")]
    NeverEndingSource,

    /// A row's contribution could not be formed at all, so the producer has no
    /// value it could honestly emit for that rank.
    ///
    /// This is the one variant that is not a producer's *misbehaviour*: it is a
    /// producer reporting that the fusion law it was asked to compute under
    /// cannot produce a number for this row — the fixed-point arithmetic left
    /// its range. Emitting a wrapped or substituted value instead would be a
    /// wrong order presented as a right one, which is what the whole protocol
    /// exists to prevent.
    ///
    /// No profile this crate can build reaches it: a validated
    /// [`FusionProfile`](crate::FusionProfile) fixes `K >= 1` and a weight whose
    /// admitted ceiling already fits, and a contribution is that weight times a
    /// reciprocal at most one, so the product cannot leave the range. That is an
    /// argument, though, and this variant is what keeps it from having to be an
    /// assertion: the conversion from a rank to a contribution is total, it runs
    /// on a library path a caller reaches with its own stream, and a wrong
    /// argument here should fail that caller's request rather than abort its
    /// process.
    #[error("no contribution can be formed at rank {rank}: {reason}")]
    UncomputableContribution {
        /// The 1-based rank whose contribution could not be formed.
        rank: u64,
        /// The arithmetic refusal, rendered.
        reason: String,
    },

    /// A producer answered an exclusion lookup with
    /// [`ExclusionVerdict::Excluded`] for a candidate and then emitted a row
    /// naming it.
    ///
    /// The two statements cannot both be true, and this names the one that was
    /// contradicted. It is deliberately **not**
    /// [`Self::OutsideDeclaredDomain`]: that refusal blames a *declaration*
    /// made once at registration about whole blocks of the universe, and
    /// nothing about this producer's domains was broken — what was broken is an
    /// observation it made about this one candidate, through a channel its
    /// registration merely opened. Reporting it as a domain violation would
    /// send a host to fix a `CandidateDomains` that is perfectly correct.
    ///
    /// It is refused rather than repaired because the repair is a wrong answer
    /// either way. Merging the row would add a contribution to a candidate whose
    /// score a consumer may already have certified as final *on the strength of
    /// this producer's own word*; dropping it would silently discard a row the
    /// producer emitted under the rank law.
    #[error("stratum {stratum} excluded item {item} and then named it")]
    ExclusionContradicted {
        /// The candidate, in its canonical lexical form.
        item: String,
        /// The stratum whose producer contradicted itself.
        stratum: String,
    },

    /// An exclusion lookup could not be performed at all.
    ///
    /// The third outcome [`ExclusionVerdict`] deliberately has no variant for:
    /// the producer was asked and something failed — the dataset read, the
    /// prepared plan, the producer's own row bound — so nothing is known about
    /// this candidate. It fails the fused request, because the alternative is
    /// reading a failed measurement as [`ExclusionVerdict::Possible`], which is
    /// a swallowed error wearing the costume of a conservative answer.
    #[error("exclusion lookup for stratum {stratum} failed: {reason}")]
    ExclusionLookupFailed {
        /// The stratum whose lookup failed.
        stratum: String,
        /// The underlying refusal, rendered.
        reason: String,
    },

    /// A stream was asked for an exclusion verdict while declaring
    /// [`ExclusionBasis::Unavailable`].
    ///
    /// Unreachable from this crate's own fusion engine, which asks only the
    /// streams whose contract declared a basis. It is reachable from a stream a
    /// caller assembled by hand whose contract and whose implementation of
    /// [`RankedStream::exclusion`] disagree, and it is that disagreement — not a
    /// fabricated [`ExclusionVerdict::Possible`] — that is reported.
    #[error("stream was asked for an exclusion verdict but declares no exclusion basis")]
    ExclusionUnavailable,

    /// A stream declared [`ExclusionBasis::Search`] beside a
    /// [`Completeness::Lossy`](crate::Completeness::Lossy) search.
    ///
    /// Refused when the stream is handed to
    /// [`FusionStream::new`](crate::FusionStream::new), before a row is pulled or
    /// a lookup asked. A lossy search that did not find a candidate has not said
    /// the candidate is absent, so honouring its `Excluded` would retire the very
    /// residual the loss earns and certify a score the missing rows could still
    /// raise. The registry refuses the same pairing when a producer is registered
    /// ([`ExclusionBasis::is_exact_under`] is the one predicate both read); this
    /// is the refusal for a stream that never went through a registry.
    ///
    /// The stratum names the stream and the evidence is the producer's own,
    /// verbatim, so a host can see which declaration made the search lossy.
    #[error(
        "stream for stratum {stratum} declares an exclusion basis of search beside a lossy search \
         ({evidence}); a search that may miss rows cannot say a candidate is absent. Declare \
         ExclusionBasis::Membership if the answer is about the producer's own term universe, or \
         ExclusionBasis::Unavailable"
    )]
    SearchExclusionFromLossySearch {
        /// The stratum the stream was tagged with when it was handed to fusion.
        stratum: String,
        /// The producer's own completeness evidence, verbatim.
        evidence: String,
    },

    /// The read behind a stream failed after the consumer had started merging its
    /// rows, or failed in a way that invalidates the whole run.
    ///
    /// A stream read on demand produces each row when it is pulled, so a producer's
    /// failure at its fortieth row surfaces at the fortieth pull — after thirty-nine
    /// rows have already been merged and, possibly, emitted. Those rows cannot be
    /// taken back, and a failed stratum reported as an ordinary status beside an
    /// answer built partly out of it would be the answer claiming a stratum it did
    /// not have. So such a failure fails the fused request, naming the stratum, how
    /// far the read had got, and the producer's own reason. A failure *before* the
    /// first row is not this: it is reported as that stratum's
    /// [`ProducerReceipt::ExecutionFailed`], exactly as a materialised read's
    /// failure is, and every other stratum answers.
    ///
    /// The one failure that is this at any depth is a producer that returned more
    /// rows than its registry declared it could — the whole-run refusal a
    /// materialised read reports as
    /// [`ExecutionError::RowBoundBreached`](crate::ExecutionError::RowBoundBreached),
    /// because the broken number is not confined to the stratum that exposed it.
    ///
    /// It is not [`Self::ErrorAfterRows`], which is a hand-built stream's own report
    /// and carries no reason: a producer read through this layer always has one, and
    /// dropping it would leave a host with a count and nothing to fix.
    #[error("stratum {stratum}: the read failed after {rows_before} row(s): {reason}")]
    ReadFailed {
        /// The stratum whose read failed.
        stratum: String,
        /// How many rows the stream had handed out before the failure.
        rows_before: u64,
        /// The producer's or the executor's own refusal, rendered.
        reason: String,
    },

    /// The read behind a stream ended under a different attestation from the one it
    /// announced before its first row.
    ///
    /// A consumer reads a stream's [`RankedStream::attestation`] before it pulls a
    /// row and certifies every row under it — an attested-short index widens the
    /// intervals it certifies against, and the trailer's exactness and evidence
    /// identity are derived from it. A read held open while its consumer merges is
    /// therefore held to that announcement when it stops
    /// ([`RankedStream::settle`]): the generation it pinned must be the only
    /// generation it served from, and the service level it reports at the stop must
    /// be the one it reported at the open. Either moving means the rows were
    /// certified under evidence the read did not end with — an index rebuilt under
    /// the read, or a shortfall discovered after the rows it affects were merged as
    /// whole. The answer is refused rather than relabelled, because relabelling
    /// would keep rows that were certified under the wrong law.
    ///
    /// The same family as [`Self::ContributionMismatch`] and for the same reason: a
    /// value the consumer holds is checked against the value the producer ends up
    /// standing behind, and a disagreement is named with both sides, repaired never.
    #[error(
        "stratum {stratum}: the read ended under a different attestation from the one it \
         announced before its first row: {reason}"
    )]
    AttestationMoved {
        /// The stratum whose read moved.
        stratum: String,
        /// Both sides of the disagreement, or the witness rule the read's own receipt
        /// broke, rendered.
        reason: String,
    },

    /// An exclusion lookup was answered under a different attestation from the one
    /// its stream's read pinned.
    ///
    /// A verdict and a ranked read are two answers to one question — whether this
    /// producer names a candidate — and they must agree for the verdict to settle
    /// anything: an `Excluded` a fusion acts on stands in for the rows the ranked
    /// read would have named had it been read further. That is only true of rows the
    /// *same* index generation would have named. A lookup answered by another
    /// generation — an index rebuilt between the read's open and the lookup — can
    /// exclude a candidate the pinned generation holds, and a fusion that stops
    /// before the stream would have named it returns a different answer with nothing
    /// in it to say so. So each lookup's own witness is read under the sole-witness
    /// rule and held to the attestation the read pinned — generation and service
    /// level both, because a lookup served from an index that has since found itself
    /// short is no more the pinned index than a rebuilt one — and a disagreement
    /// fails the request, naming both sides.
    ///
    /// The same family as [`Self::AttestationMoved`] and
    /// [`Self::ContributionMismatch`]: a value the consumer holds is checked against
    /// the value the producer stood behind, and a disagreement is named with both
    /// sides, repaired never. It is its own variant because it is a different
    /// promise broken at a different instant: that one is the read disagreeing with
    /// its own announcement when it stops, this is a point answer disagreeing with
    /// the read it was asked beside, at the moment it is asked.
    #[error(
        "stratum {stratum}: an exclusion lookup was answered under a different attestation \
         from the one its stream's read pinned: {reason}"
    )]
    ExclusionAttestationMoved {
        /// The stratum whose lookup was answered elsewhere.
        stratum: String,
        /// Both sides of the disagreement, or the witness rule the lookup's own
        /// receipt broke, rendered.
        reason: String,
    },
}

/// What a stream's read stands behind at the instant its consumer stops reading it.
///
/// Returned by [`RankedStream::settle`]. A read materialised before its first row
/// was readable settles to what it already said; a read produced on demand settles
/// here, because this is the first instant at which how far it was read is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadSettlement {
    /// What the index behind the rows attests now that the read has stopped.
    ///
    /// Held against [`RankedStream::attestation`], which the consumer read before the
    /// first row; see [`ProtocolError::AttestationMoved`].
    pub attestation: PfAttestation,
    /// How many rows the read behind the stream produced, when the stream knows:
    /// [`RankedStream::rows_materialised`] as of this instant.
    pub rows_materialised: Option<u64>,
    /// How many rows the stream handed its consumer, when the stream counts them.
    ///
    /// Held against the rows the consumer pulled
    /// ([`ProtocolError::ForgedReceipt`]), so a settlement cannot describe a read the
    /// consumer did not take. `None` for a stream that keeps no such count.
    pub rows_emitted: Option<u64>,
}

/// A producer's ranked rows, pulled one at a time.
///
/// `next` yields a [`RankedRow`] in rank order and `Ok(None)` once the stream is
/// exhausted; `receipt` then reports how it ended. Ranks are 1-based and
/// contiguous. Contributions are the profile's reciprocal-rank values — a
/// conforming producer computes them with [`contribution`](crate::contribution).
/// Fusion re-derives every one of them from `(decay rule, K, weight, rank)` and
/// refuses a stream that supplies a different number
/// ([`ProtocolError::ContributionMismatch`]), which is also what holds the
/// sequence non-increasing with rank: the profile's own curve never rises, so a
/// rising value is a value the profile did not compute.
///
/// Each row also says which block of the candidate universe it was drawn from. A
/// stream that declared [`CandidateDomains::Within`] owes that on every row and
/// owes one its own declaration admits; a stream that declared
/// [`CandidateDomains::Unrestricted`] owes none and says
/// [`RowBlock::Undeclared`], which is an absence rather than a claim. See this
/// module's header for what the block buys and what it costs a producer.
// The trait is consumed only by `FusionStream` in this crate; its futures are
// awaited in the same task and never cross a thread boundary, so the `Send`
// bound the lint wants to express would add nothing. Making it a required bound
// would instead exclude legitimate single-threaded producers.
#[allow(async_fn_in_trait)]
pub trait RankedStream {
    /// The item a row carries. It must round-trip through the caller's canonical
    /// term lexical so the layer can order and report it without parsing RDF.
    type Item: Clone + Ord + Into<Term>;

    /// Pull the next row, or `Ok(None)` when the stream is exhausted.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] when the producer cannot report its own rows
    /// honestly — for example [`ProtocolError::ErrorAfterRows`] after a failure
    /// that followed emitted rows, or [`ProtocolError::NeverEndingSource`].
    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError>;

    /// Report how the stream ended. Called only after `next` returned
    /// `Ok(None)`.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] when the producer cannot produce a consistent
    /// receipt.
    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError>;

    /// The promises this stream makes about its rows: its duplicate handling,
    /// and which blocks of the candidate universe it may name.
    ///
    /// Read once by [`FusionStream::new`](crate::FusionStream::new), before any
    /// row is pulled, and then applied to every row of this stream. A producer
    /// registered through the seam reports exactly what it declared — see
    /// [`StreamContract::declared`] — and a stream a caller assembled by hand
    /// states its own.
    ///
    /// There is deliberately **no default**, which is where this differs from
    /// [`plan_id`](Self::plan_id). A stream may honestly descend from no plan,
    /// so `None` is an answer; a stream cannot honestly decline to say whether
    /// it repeats items, because it either does or it does not and the consumer
    /// keeps a different amount of state for each. A default would be this
    /// layer fabricating a declaration the producer never made, and then holding
    /// the producer to it.
    ///
    /// The domain term has no default for a subtler reason, because it does
    /// have an obviously safe value —
    /// [`CandidateDomains::Unrestricted`] licenses nothing and is what every
    /// stream effectively said before the term existed. It is still written
    /// rather than assumed: the value is a *promise about the producer's
    /// corpus*, the host is the only party that knows it, and a consumer that
    /// filled it in would be choosing, on the host's behalf, between an answer
    /// that costs a full drain and one that might be missing a contribution.
    /// A caller that means "anything" says so in one word.
    fn contract(&self) -> StreamContract;

    /// Ask this producer whether `candidate` is out of its reach.
    ///
    /// The one channel in this protocol that carries a question *in*. Every
    /// other method reports what the producer has already decided; this asks it
    /// about one candidate the consumer names, and what the answer means is
    /// [`StreamContract::exclusion`]'s basis.
    ///
    /// The candidate is named in the canonical term lexical rather than in
    /// [`Self::Item`], because that is the only spelling the consumer still
    /// holds. A fused frontier keys candidates by [`Term`] — `Item` is converted
    /// on the way in and the original is dropped with the row — so an `Item`
    /// parameter would oblige this layer to keep a second copy of every frontier
    /// candidate for a question most fusions never ask. Round-tripping through
    /// that lexical is exactly what [`Self::Item`]'s `Into<Term>` bound is for.
    ///
    /// # Called only where the contract says it may be
    ///
    /// [`FusionStream`](crate::FusionStream) asks this of a stream whose
    /// contract declared a basis, and never of one that declared
    /// [`ExclusionBasis::Unavailable`]. A producer that declared no basis may
    /// therefore return [`ProtocolError::ExclusionUnavailable`] unconditionally,
    /// and that is the honest body for it.
    ///
    /// # There is no default, and the reason is the one `contract` gives
    ///
    /// A default would have to be either [`ExclusionVerdict::Possible`] — this
    /// layer answering a question about a host's corpus that only the producer
    /// can answer, and answering it in the direction that is never wrong and
    /// never useful — or an error, which would make every conforming
    /// hand-written producer's declared basis a lie its author never wrote. The
    /// declaration and the implementation belong to the same author, so both are
    /// required of that author.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::ExclusionLookupFailed`] when the lookup itself failed,
    /// [`ProtocolError::ExclusionAttestationMoved`] when it was answered under an
    /// attestation other than the one this stream's read pinned, and
    /// [`ProtocolError::ExclusionUnavailable`] when the producer answers no
    /// such lookup. None is degraded to a verdict: a failed measurement
    /// reported as [`ExclusionVerdict::Possible`] is a swallowed error that
    /// leaves the answer correct and the read unbounded, which nothing
    /// downstream could ever notice.
    async fn exclusion(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError>;

    /// The pinned plan these rows descend from, when the stream has one.
    ///
    /// This is how a plan's identity reaches the answer: [`execute`] tags each
    /// stream it returns with the plan its unit was compiled from, and
    /// [`fuse`](crate::fuse) reads the tag back off the streams it is handed and
    /// puts it in the [`FusionTrailer`](crate::FusionTrailer). The identity in a
    /// fused answer is therefore the one that travelled the pipeline, not one
    /// re-fetched from the plan at the end, and a stream whose tag changed on
    /// the way is a stream `fuse` refuses to fuse beside its siblings
    /// ([`FusionError::PlanIdMismatch`](crate::FusionError::PlanIdMismatch)).
    ///
    /// The default is `None`, which is the honest answer for a producer that
    /// descends from no plan at all — a hand-built stream, or one a caller
    /// assembled outside the ladder. Such a stream still fuses; the fused
    /// answer simply names no pinned plan, because no single plan produced it.
    ///
    /// [`execute`]: crate::execute
    fn plan_id(&self) -> Option<crate::id::PlanId> {
        None
    }

    /// The row bound this stream's depth was derived for, when the stream
    /// descends from a plan that recorded one.
    ///
    /// A bounded read is only honest for the bound it was taken under. Under
    /// declarations that let the planner narrow a depth to the caller's `k`, a
    /// stream cut at `k` rows cannot serve a fusion for `k + 1` — the row that
    /// would have been the `k + 1`-th was never read — and nothing about the rows
    /// themselves says so. So the bound travels with them, exactly as
    /// [`plan_id`](Self::plan_id) does, and [`fuse`](crate::fuse) refuses a
    /// mismatch with its own `top_k` argument by name
    /// ([`FusionError::ReadBoundMismatch`](crate::FusionError::ReadBoundMismatch))
    /// rather than answering out of a read taken for a different question.
    ///
    /// The default is `None`, which is the honest answer for a stream whose depth
    /// no plan bounded — a hand-built stream, or one a caller assembled outside the
    /// ladder. Such a stream fuses at whatever bound its caller names, because
    /// there is no other bound for that one to disagree with. This is the same
    /// reasoning [`plan_id`](Self::plan_id) defaults on, and it is why the two are
    /// separate answers: a stream can descend from a plan and still be re-bounded
    /// by hand, and a stream can carry a bound while naming no plan.
    fn fused_bound(&self) -> Option<crate::fuse::TopK> {
        None
    }

    /// What the index behind these rows attests: which generation answered, and
    /// whether that generation was whole.
    ///
    /// Read once by [`FusionStream::new`](crate::FusionStream::new), **before
    /// any row is pulled**, and carried into the fused trailer
    /// ([`FusionTrailer::attestations`](crate::FusionTrailer::attestations)).
    /// Reading it first is reading the truth for the whole read rather than a
    /// convenience: a generation is pinned when the index is opened, so the
    /// version that answers row one is the version that answers row `n`, and
    /// asking at the end would ask a stream that a bounded fusion may have
    /// stopped mid-read — or, worse, would let a top-k stop *overwrite* the
    /// answer. That is the same failure the two identities above already avoid
    /// by travelling with the stream instead of being re-fetched from the
    /// registry at the end.
    ///
    /// A read produced on demand announces here what its invocation attested the
    /// instant it opened, and is then **held to it** when the fusion stops
    /// ([`settle`](Self::settle)): an index that moved under the read, or a
    /// service level that changed between the open and the stop, is refused
    /// ([`ProtocolError::AttestationMoved`]) rather than reported. So reading the
    /// announcement first loses nothing a read discovered later: a later discovery
    /// that would change the announcement fails the answer built on it.
    ///
    /// # Why this is not a [`ProducerReceipt`] variant
    ///
    /// An incomplete index is not a read ending. Every [`ProducerReceipt`]
    /// variant answers "what stopped this read", and "a shard of my index
    /// failed to load" answers a different question: it is true of the whole
    /// invocation from the instant it opened, and it stays true whichever way
    /// the read then ends. Made terminal it would be *destroyed* by a top-k
    /// stop, because a bounded fusion never collects a receipt from a stream it
    /// stopped — the incompleteness would vanish exactly in the runs where the
    /// bound mattered. Held as an attestation, both facts survive: the stratum
    /// reports the bounded stop it got and the short index it had.
    ///
    /// # Why this has a default and [`contract`](Self::contract) does not
    ///
    /// The default is [`PfAttestation::UNDECLARED`], for precisely the reason
    /// [`plan_id`](Self::plan_id) defaults to `None`: a stream may honestly
    /// descend from no index at all — a hand-built stream, a computation over
    /// its arguments, a walk of the dataset already being queried — and
    /// "nothing to name" is that stream's true answer rather than a gap in it.
    /// The absence stays an absence all the way out, too: `Undeclared` is
    /// silence, never a certificate that the index was current or whole (see
    /// [`ServiceLevel`](purrdf_sparql_eval::ServiceLevel), which has no
    /// `Whole` variant on purpose).
    ///
    /// [`contract`](Self::contract) is the opposite case and is defaulted
    /// nowhere. A stream either repeats items or it does not; there is no third
    /// state to be honestly silent about, and the consumer holds different
    /// state for each answer, so a default there would be this layer inventing
    /// a declaration and then enforcing it. A default here invents nothing — it
    /// says the producer said nothing, which is what happened.
    fn attestation(&self) -> PfAttestation {
        PfAttestation::UNDECLARED
    }

    /// How many rows the read behind this stream actually produced, when the
    /// stream knows.
    ///
    /// **The work, beside the consumption.** Every other number a fusion reports
    /// about a stratum counts what the *fusion* did with the stream:
    /// [`StratumResolution::ranks_pulled`](crate::StratumResolution::ranks_pulled)
    /// is how far down the ranking the answer needed to go, and it is the number
    /// a narrowing is judged by. It is also, on its own, a measurement of the
    /// counter the narrowing was built to lower. A plan whose depth the planner
    /// could not narrow materialises its whole declared length and then hands
    /// six ranks of it to a fusion that certifies immediately; `ranks_pulled`
    /// says six, and the four hundred rows that were read to produce them are
    /// invisible. This is that number.
    ///
    /// Counted in rows the producer's read returned, including the probe row an
    /// emitted bound carries one past the depth — the probe is a row the read
    /// paid for, and a figure that excluded it would report a read as cheaper
    /// than it was by exactly the row that makes its ending observable.
    ///
    /// Read by [`FusionStream::trailer`](crate::FusionStream::trailer), through
    /// [`settle`](Self::settle), when the fusion stops — not when it starts. For a
    /// read materialised before its first row was readable the two instants give
    /// the same number; for a read produced on demand only the second is the cost,
    /// because the rows it produced are exactly the rows the fusion asked for, and
    /// the fusion decides that by stopping.
    ///
    /// # Why the default is `None` and not zero
    ///
    /// The default is `None`, for exactly the reason [`plan_id`](Self::plan_id)
    /// defaults to `None`: a stream may honestly have no materialised read
    /// behind it — a generator, a computation over its arguments, a hand-built
    /// list of rows a test wrote — and "there is no read to count" is that
    /// stream's true answer rather than a gap in it. Zero is not that answer.
    /// Zero is a measurement, and a stream that reported it would be claiming
    /// its read was free; the whole use of this number is comparing what a read
    /// cost against what the fusion consumed, and a fabricated zero would make
    /// every such comparison flattering.
    ///
    /// [`execute`](crate::execute) answers it for every stream it returns,
    /// because it is the party that made the read.
    fn rows_materialised(&self) -> Option<u64> {
        None
    }

    /// What the read behind this stream stands behind now that its consumer has
    /// stopped reading it: the attestation it ends under, what it cost, and how many
    /// rows it handed out.
    ///
    /// Called by [`FusionStream::trailer`](crate::FusionStream::trailer) on **every**
    /// stream — one that ran out and one a bounded fusion stopped alike — because the
    /// trailer is the instant each read's extent is final. A stream the fusion stopped
    /// never returns a receipt, so this is the one report such a stream makes about
    /// its own end, and it is where a read produced on demand closes its evidence:
    /// the witness of an invocation held open while the fusion merged is only
    /// complete once the merging stops.
    ///
    /// Non-consuming: a caller that reads the trailer and then pulls further rows may
    /// settle again, and the later settlement describes the longer read.
    ///
    /// # Default
    ///
    /// The attestation the stream already announced, its
    /// [`rows_materialised`](Self::rows_materialised), and no emitted-row count — the
    /// true settlement of a stream whose read finished before its first row was
    /// readable, which has nothing left to learn about itself.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::AttestationMoved`] when the read's own receipt cannot be a
    /// single attestation — the index it served from moved under it.
    async fn settle(&mut self) -> Result<ReadSettlement, ProtocolError> {
        Ok(ReadSettlement {
            attestation: self.attestation(),
            rows_materialised: self.rows_materialised(),
            rows_emitted: None,
        })
    }
}
