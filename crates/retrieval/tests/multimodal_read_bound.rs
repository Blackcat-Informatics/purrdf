// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How much of a ranked stream a bounded multimodal read actually consumes.
//!
//! Three configurations of the *same* two producers run through the whole
//! ladder — `plan` → `compile` → `execute` → `fuse`. They differ in exactly two
//! bits: whether the two producers declare the same block or different ones,
//! and whether their candidate sets intersect. Everything else — four hundred
//! rows each, unit weights, a bound of five, `DuplicatePolicy::Unique` — is
//! held fixed, so every number that moves between them moved because of those
//! two bits and nothing else.
//!
//! Five numbers are measured per configuration: the depth the planner recorded,
//! the `LIMIT` the compiler emitted, the rows the executor really pulled out of
//! the relation, the ranks fusion really pulled off the stream, and the
//! terminal status the trailer reports. They are asserted as exact literals,
//! because a range would hide the very drift this file exists to detect.
//!
//! The third of those is the rows of the read the **answer came from**, and a run
//! can take more than one: a plan the planner could not narrow is read
//! speculatively first, at the fused frontier, and that read is discarded whole
//! if some stratum's ceiling cut it. So every read is counted separately, the
//! number of them is measured too, and the rows a discarded attempt cost are
//! asserted where that cost is the subject rather than folded into the depth of
//! the read that answered.
//!
//! Beside them sits a derivation with no executor in it: the rank at which a
//! candidate's lower bound overtakes the fusion threshold, computed from the
//! profile's own decay law through the crate's own `contribution_under`. It
//! predicts the two configurations that reach that gate, and that prediction is
//! asserted. The third never reaches it — a candidate one stream named is not
//! *final* while another stream sharing its block is still open — which is why
//! that configuration drains, and the drain is pinned as a permanent control.
//!
//! Past those three sits the same ladder over configurations of two and three
//! strata, asking a different question: the licence to read a prefix is granted
//! **per stratum**, so one stratum of a request can narrow while another does not.
//! Those tables assert a `(depth, cause)` pair per stratum rather than one number
//! for the run, and each licensed configuration is then checked differentially —
//! the answer over the narrowed prefixes against the answer the same configuration
//! gives when every stratum reads its whole stream, row for row, score for score
//! and order for order.
//!
//! Fixtures use `example.org` throughout; every block tag and stratum IRI below
//! is fixture configuration, never a minted vocabulary.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, CompiledRetrieval, CrossingRank, DecayRule, DepthCause,
    DomainTag, DuplicatePolicy, ExclusionVerdict, Fixed, FusedRow, FusionError, FusionProfile,
    FusionResult, FusionTrailer, Iri, PfAttestation, Plan, PlanId, PlannedResolution,
    ProducerReceipt, ProducerStatus, ProtocolError, RECIP_K, RankFidelity, RankedRow, RankedStream,
    RankedStreamAdapter, ReadAttempts, ReadBound, RequestTerm, RetrievalRequest, RowBlock,
    Statistics, StratumUnit, StreamContract, Term, TopK, compile, contribution_under, execute,
    fuse, observed_resolution, plan, search, speculative_depth, threshold_at,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EvalError, ExclusionBasis, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet, TermKind,
    TermPattern, TermPlacement, Volatility,
};

mod common;

/// The smoothing constant every profile in this file fuses under, read from the
/// crate's own constant rather than written twice.
const K: u32 = RECIP_K as u32;

/// The bound every configuration searches under.
///
/// Small on purpose: the whole question is how much of a four-hundred-row
/// stream a five-row answer has to read.
const TOP_K: TopK = TopK::new(5);

/// How many rows each producer holds, and the row bound each declares.
const ROWS: u64 = 400;

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

fn domain_tag(suffix: &str) -> DomainTag {
    DomainTag::parse(&ex(suffix)).expect("fixture domain tags are valid IRIs")
}

/// A minimal single-threaded executor. The mocks never actually pend.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ParkWaker(std::thread::Thread);
    impl Wake for ParkWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ParkWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

// ---------------------------------------------------------------------------
// 1a. The threshold-crossing derivation: one pure function, no executor.
// ---------------------------------------------------------------------------
//
// The derivation itself is the LIBRARY's — `purrdf_retrieval::threshold_at` and
// `purrdf_retrieval::crossing_rank_at` — and the three wrappers below only give
// it this file's fixture spellings and unwrap its refusals. It was written here
// first, and keeping a second copy of it here would mean every prediction below
// was checked against the arithmetic of the test rather than against the
// arithmetic a host can ask for at plan time.

/// The sum of `weights`' contributions at one rank, under `decay`.
///
/// **One arithmetic path.** Every term comes out of the crate's own
/// [`contribution_under`], through the crate's own [`threshold_at`]. The
/// engine's contributions are truncated fixed point, so `w / (k + r)`
/// recomputed in floating point disagrees with it at exactly the boundary this
/// file is about — the rank where a doubled threshold ties a single lower bound
/// instead of falling below it.
fn sum_at(decay: DecayRule, weights: &[Fixed], rank: u64) -> Fixed {
    threshold_at(decay, weights, rank).expect("fixture thresholds are in range")
}

/// The smallest head rank `r` at which a candidate sitting at `at_rank` in every
/// stratum that names it is emittable.
///
/// `select_emittable` runs two gates in order. The first is `is_final`, which
/// this function says nothing about. The second is the threshold, and this is
/// it: while any head is live a candidate must have
/// `lower_bound > threshold`, **strictly**.
///
/// * `naming` are the weights of the strata that named the candidate, so
///   `L = Σ contribution(w, at_rank)` is what it has already collected;
/// * `sharing` are the weights of the open strata whose declaration admits the
///   candidate's block, so `T(r) = Σ contribution(w, r)` is what
///   `compute_threshold` maximises to for that block once every head sits at
///   rank `r`.
///
/// A stratum that names the candidate is also a stratum that shares its block,
/// so `naming ⊆ sharing` for every fixture here; the two are separate arguments
/// because the gap between them is the whole phenomenon.
fn crossing_rank_at(decay: DecayRule, naming: &[Fixed], at_rank: u64, sharing: &[Fixed]) -> u64 {
    crossed(purrdf_retrieval::crossing_rank_at(
        decay, naming, at_rank, sharing,
    ))
}

/// [`crossing_rank_at`] for the head of a stream: the candidate at rank one.
fn crossing_rank(decay: DecayRule, naming: &[Fixed], sharing: &[Fixed]) -> u64 {
    crossing_rank_at(decay, naming, 1, sharing)
}

/// The rank a crossing this file expects to exist reports.
///
/// Every fixture here fuses at unit weights under a decay that reaches zero, so
/// a positive lower bound always crosses and the saturating case is a defect in
/// the derivation rather than a fixture the file admits.
fn crossed(found: Result<CrossingRank, FusionError>) -> u64 {
    found
        .expect("fixture crossings are computable")
        .rank()
        .expect("a fixture lower bound always crosses inside the expressible range")
}

/// The decay rule every configuration in this file fuses under.
const fn decay() -> DecayRule {
    DecayRule::ReciprocalRank { k: K }
}

/// **The disjoint-block case.** Two strata declaring different blocks. Only the
/// stratum that named the candidate admits its block, so the threshold for that
/// block is that one stream's head alone: `c(1) > c(r)` first holds at `r = 2`.
#[test]
fn a_candidate_whose_block_only_its_namer_admits_crosses_at_the_second_rank() {
    assert_eq!(
        crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE]),
        2,
        "one namer bounded by its own head alone is beaten by the next rank down"
    );
}

/// **The overlap case.** Two strata declaring the *same* block, both naming the
/// candidate. Both sides of the comparison double, so the doubling cancels and
/// the crossing is where it was: `2c(1) > 2c(r)` first holds at `r = 2`.
#[test]
fn a_candidate_both_sharers_named_crosses_at_the_second_rank_too() {
    assert_eq!(
        crossing_rank(
            decay(),
            &[Fixed::ONE, Fixed::ONE],
            &[Fixed::ONE, Fixed::ONE]
        ),
        2,
        "a candidate that collected from every stream admitting its block is \
         bounded by exactly the streams it already read"
    );
}

/// **The case the bound does not survive.** One namer, two open streams sharing
/// the block. The candidate collected one contribution and is measured against
/// two, so the threshold has to fall to *half* its lower bound — which for
/// reciprocal rank means the head must reach past `k` itself.
///
/// The value is exact, not approximate, and it is the value the truncated
/// fixed-point arithmetic gives: at `r = 62` the doubled threshold is
/// `2 · ⌊10¹²/122⌋ = ⌊10¹²/61⌋` **exactly**, a tie rather than a crossing, and
/// a strict comparison does not pass a tie. Re-deriving this in floating point
/// would have said 62.
#[test]
fn a_candidate_one_of_two_sharers_named_crosses_only_past_the_smoothing_constant() {
    assert_eq!(
        crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE, Fixed::ONE]),
        63,
        "a lower bound measured against twice its own weight has to outlast the \
         smoothing constant"
    );

    // The tie at the rank below, stated as the arithmetic rather than as a
    // claim about it: this is why the answer is 63 and not 62.
    assert_eq!(
        sum_at(decay(), &[Fixed::ONE], 1),
        sum_at(decay(), &[Fixed::ONE, Fixed::ONE], 62),
        "at the rank below the crossing the two sides are equal, and a strict \
         comparison does not emit on equality"
    );
}

// ---------------------------------------------------------------------------
// 1b. The harness: three configurations of two producers through the ladder.
// ---------------------------------------------------------------------------

/// What the two producers declare about where their candidates lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blocks {
    /// Each producer declares its own block, and the two do not meet.
    Distinct,
    /// Both producers declare one and the same block.
    Shared,
    /// Neither producer declares a block at all:
    /// [`CandidateDomains::Unrestricted`], so each may name anything and every
    /// candidate is one the other could still name.
    Undeclared,
}

/// Whether the two producers name the same candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Results {
    /// Every candidate is named by exactly one of them.
    Disjoint,
    /// Both name the identical four hundred candidates in the identical order.
    Intersecting,
    /// Both name the identical four hundred candidates, the second producer in
    /// the reverse of the first's order — so every candidate one of them names
    /// early, the other holds and names late.
    Reversed,
}

/// One of the three configurations, and its name.
#[derive(Clone, Copy, Debug)]
struct Configuration {
    name: &'static str,
    blocks: Blocks,
    results: Results,
    /// How many candidates each producer really holds, against the `ROWS` both
    /// of them declare.
    holds: u64,
    /// What both producers declare their exclusion answers are a fact about.
    ///
    /// The fourth bit this file varies, and the only one that changes what a
    /// producer can be *asked* rather than what it promised. Under
    /// [`ExclusionBasis::Unavailable`] a configuration is the one this file
    /// pinned before lookups existed, down to the byte.
    exclusion: ExclusionBasis,
}

/// The control: different blocks, different candidates. The planner's merge
/// argument holds, so the bound is a bound on the read.
const DISTINCT_BLOCKS_DISJOINT_RESULTS: Configuration = Configuration {
    name: "distinct_blocks_disjoint_results",
    blocks: Blocks::Distinct,
    results: Results::Disjoint,
    holds: ROWS,
    exclusion: ExclusionBasis::Unavailable,
};

/// One block, and both producers naming the same candidates.
const SHARED_BLOCK_INTERSECTING_RESULTS: Configuration = Configuration {
    name: "shared_block_intersecting_results",
    blocks: Blocks::Shared,
    results: Results::Intersecting,
    holds: ROWS,
    exclusion: ExclusionBasis::Unavailable,
};

/// One block, and candidates no two producers share — structurally the control,
/// differing from it only in what was declared.
const SHARED_BLOCK_DISJOINT_RESULTS: Configuration = Configuration {
    name: "shared_block_disjoint_results",
    blocks: Blocks::Shared,
    results: Results::Disjoint,
    holds: ROWS,
    exclusion: ExclusionBasis::Unavailable,
};

/// **The same configuration, with the producers answering exclusion lookups.**
///
/// Byte for byte [`SHARED_BLOCK_DISJOINT_RESULTS`] except that both producers
/// declare [`ExclusionBasis::Membership`]: one block, four hundred rows each, no
/// candidate named twice, the same weights and the same bound. The two are the
/// differential pair this whole mechanism is measured by, and the only thing
/// that differs between them is whether a consumer is allowed to *ask*.
const SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS: Configuration = Configuration {
    name: "shared_block_disjoint_results_with_lookups",
    blocks: Blocks::Shared,
    results: Results::Disjoint,
    holds: ROWS,
    exclusion: ExclusionBasis::Membership,
};

/// **Every verdict is `Possible`.** One block, both producers naming the
/// identical four hundred candidates, and both answering exclusion lookups.
///
/// Byte for byte [`SHARED_BLOCK_INTERSECTING_RESULTS`] except for the declared
/// basis. Every lookup finds its candidate, so no verdict ever narrows anything,
/// and what is left to measure is the price of asking — which is what makes this
/// the configuration the lookup budget is pinned in.
const SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS: Configuration = Configuration {
    name: "shared_block_intersecting_results_with_lookups",
    blocks: Blocks::Shared,
    results: Results::Intersecting,
    holds: ROWS,
    exclusion: ExclusionBasis::Membership,
};

/// **The same answer with no block declared at all.** Both producers declare
/// [`CandidateDomains::Unrestricted`] and answer exclusion lookups; everything
/// else is [`SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS`].
///
/// Two unrestricted streams admit every candidate between them exactly as two
/// streams declaring one shared block do, so the threshold sums the same two
/// heads and the plan proves the same stopping rank. It is the neighbour that
/// shows the deepened read is keyed on what the threshold can sum and on who
/// answers lookups, not on a block tag being present.
const UNDECLARED_BLOCKS_DISJOINT_RESULTS_WITH_LOOKUPS: Configuration = Configuration {
    name: "undeclared_blocks_disjoint_results_with_lookups",
    blocks: Blocks::Undeclared,
    results: Results::Disjoint,
    holds: ROWS,
    exclusion: ExclusionBasis::Membership,
};

/// **The declarations license the deepened read and the rows break it.** One
/// block, both producers answering lookups — every declaration identical to
/// [`SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS`] — but the two hold the same
/// four hundred candidates in opposite orders.
///
/// Every lookup therefore answers `Possible`: the candidate a stream names
/// first, the other holds and names last. No candidate becomes final until the
/// other stream reaches it, which is the premise the plan's stopping rank rests
/// on, broken by the rows and invisible to the plan. It is the configuration the
/// fallback exists for, constructed on purpose so that the fallback is shown to
/// fire rather than assumed to.
const SHARED_BLOCK_REVERSED_RESULTS_WITH_LOOKUPS: Configuration = Configuration {
    name: "shared_block_reversed_results_with_lookups",
    blocks: Blocks::Shared,
    results: Results::Reversed,
    holds: ROWS,
    exclusion: ExclusionBasis::Membership,
};

/// **The short streams.** Structurally [`SHARED_BLOCK_DISJOINT_RESULTS`] — one
/// block, no candidate named twice, the same declared four hundred rows and so
/// the same planned depth — except that each producer really holds fewer
/// candidates than the speculative read asks for.
///
/// It exists to separate two things a narrowed read can look like from the
/// outside. Both configurations drain every row their relations hold; only one
/// of them was *cut* doing it.
const SHARED_BLOCK_SHORT_STREAMS: Configuration = Configuration {
    name: "shared_block_short_streams",
    blocks: Blocks::Shared,
    results: Results::Disjoint,
    holds: SHORT_ROWS,
    exclusion: ExclusionBasis::Unavailable,
};

/// How many candidates [`SHARED_BLOCK_SHORT_STREAMS`]'s producers really hold.
///
/// Below the speculative depth on purpose, and above nothing else: four rows per
/// stream is eight candidates, which is more than the bound of five, so the
/// answer is still a bounded answer rather than everything there was.
const SHORT_ROWS: u64 = 4;

/// The two strata, in the order the two producers are registered.
fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

/// The candidate domains each producer declares, under `blocks`.
///
/// One block per producer where one is declared, so the declaration entails the
/// block of every row and nothing is owed per row.
fn declared_domains(blocks: Blocks) -> [CandidateDomains; 2] {
    let within = |tag: &str| CandidateDomains::within([domain_tag(tag)]);
    match blocks {
        Blocks::Distinct => [within("domain/left"), within("domain/right")],
        Blocks::Shared => [within("domain/shared"), within("domain/shared")],
        Blocks::Undeclared => [
            CandidateDomains::Unrestricted,
            CandidateDomains::Unrestricted,
        ],
    }
}

/// The entity-IRI prefix each producer mints its candidates under, under
/// `results`. One prefix for both is what makes the two candidate sets equal.
const fn candidate_prefixes(results: Results) -> [&'static str; 2] {
    match results {
        Results::Disjoint => ["left/", "right/"],
        Results::Intersecting | Results::Reversed => ["shared/", "shared/"],
    }
}

/// Every read one producer served, in the order the executor took them, each
/// counting the rows that read really pulled out of the relation.
///
/// A list rather than one running total, because a run can take more than one
/// read of the same relation: [`search`] attempts a narrowed read first and
/// discards it whole if some stratum's ceiling cut it. Summed into a single
/// counter those two reads would be indistinguishable from one deep read, and
/// the whole question this file measures is which rows the *answer* rests on —
/// so each read is counted separately and named by its position.
#[derive(Debug, Default)]
struct Reads {
    ranked: std::sync::Mutex<Vec<Arc<AtomicU64>>>,
    /// How many **candidate-bound** invocations the relation served, and how
    /// many of those found the candidate.
    ///
    /// Kept apart from the ranked reads above rather than folded into them,
    /// because they are different reads of different queries and this file's
    /// whole subject is which rows an answer rests on. An exclusion lookup is a
    /// point query that contributes no rank and no row to the answer; counted
    /// into `ranked` it would make every `rows_materialised` assertion in this
    /// file read the last lookup instead of the read that answered.
    ///
    /// The second number is what makes the fixture's own answers auditable: a
    /// mock that said `Excluded` to everything would narrow every read and be
    /// indistinguishable, from the outside, from one that answered honestly.
    lookups: AtomicU64,
    lookups_found: AtomicU64,
}

impl Reads {
    /// Begin counting a new ranked read, and hand back the counter it fills.
    fn begin(&self) -> Arc<AtomicU64> {
        let counter = Arc::new(AtomicU64::new(0));
        self.ranked
            .lock()
            .expect("the fixture counters are never poisoned")
            .push(Arc::clone(&counter));
        counter
    }

    /// The rows each ranked read pulled, oldest first.
    fn rows(&self) -> Vec<u64> {
        self.ranked
            .lock()
            .expect("the fixture counters are never poisoned")
            .iter()
            .map(|counter| counter.load(Ordering::SeqCst))
            .collect()
    }

    /// How many candidate-bound invocations this relation served.
    fn lookups(&self) -> u64 {
        self.lookups.load(Ordering::SeqCst)
    }

    /// How many of them found the candidate.
    fn lookups_found(&self) -> u64 {
        self.lookups_found.load(Ordering::SeqCst)
    }
}

/// A producer that mints candidates lazily under `prefix`, counting every row
/// the executor actually takes from it, read by read.
///
/// Lazy on purpose. Nothing is materialised in `open`, so the counter measures
/// the read the emitted `LIMIT` licensed rather than the mock's own
/// construction.
struct CountingProducer {
    arity: PfArity,
    /// The access patterns this relation declares.
    ///
    /// Always the all-free mode, which subsumes every invocation of this arity.
    /// A configuration whose producers declare an exclusion basis adds a second:
    /// the candidate position bound, with a row bound of one. That pair is what
    /// the registry checks a declared basis against, and it is also what makes
    /// the two kinds of call distinguishable from inside the relation — a
    /// candidate-bound call is the lookup, and there is nothing else it could be.
    modes: Vec<BindingPattern>,
    prefix: &'static str,
    /// How many candidates the relation really holds.
    ///
    /// Separate from the `ROWS` it *declares*, because a declaration is a
    /// ceiling and a relation is allowed to hold fewer: a stream that runs out
    /// inside a narrowed read is exhausted rather than cut, and telling those
    /// two apart is the whole of what the fallback keys on.
    holds: u64,
    /// Whether this relation names its candidates from the last it holds down
    /// to the first, rather than from the first up.
    reversed: bool,
    reads: Arc<Reads>,
}

impl PropertyFunction for CountingProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// `ROWS` for the ranking, one for a call that already names its candidate.
    ///
    /// The two numbers are two different promises about two different calls, and
    /// the second is what a declared exclusion basis is admitted against: asking
    /// *do you hold this one* can return this producer's row for that candidate
    /// or nothing, and never a list.
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(CANDIDATE_POSITION) {
            1
        } else {
            ROWS
        }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // A bound position is an input the call site supplied, and the engine
        // drops any row that disagrees with it there; echoing it back is the
        // cheapest correct behaviour, exactly as the other mock producers in
        // this crate's tests do.
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        // A bound candidate is the exclusion lookup and cannot be anything else:
        // the ranking call projects the candidate, so it leaves that position
        // free. The two are therefore counted apart, and the lookup answers from
        // this relation's own term universe rather than by scanning.
        if let Some(candidate) = bound[CANDIDATE_POSITION].clone() {
            self.reads.lookups.fetch_add(1, Ordering::SeqCst);
            let held = self.holds_candidate(&candidate);
            if held {
                self.reads.lookups_found.fetch_add(1, Ordering::SeqCst);
            }
            return Ok(Box::new(LookupCursor {
                row: held.then(|| {
                    bound
                        .iter()
                        .enumerate()
                        .map(|(position, value)| {
                            value.clone().unwrap_or_else(|| {
                                TermValue::iri(format!("{}score{position:06}", ex(self.prefix)))
                            })
                        })
                        .collect()
                }),
            }));
        }
        Ok(Box::new(CountingCursor {
            prefix: self.prefix,
            emitted: 0,
            holds: self.holds,
            reversed: self.reversed,
            bound,
            pulled: self.reads.begin(),
        }))
    }
}

/// The flattened argument position every producer in this file projects its
/// candidate from, and therefore the position a lookup binds.
const CANDIDATE_POSITION: usize = 0;

impl CountingProducer {
    /// Whether `candidate` is one of the candidates this relation holds.
    ///
    /// Read off the IRI rather than by scanning, which is what makes this a
    /// membership answer: the relation mints `{prefix}entity{index:06}` for
    /// every index below `holds`, so a term of that shape with an index in range
    /// is one it holds and anything else is one it does not. A mock that scanned
    /// instead would give the same answers and would not be a point lookup, and
    /// the row bound it declares would be a promise it does not keep.
    fn holds_candidate(&self, candidate: &TermValue) -> bool {
        let TermValue::Iri(iri) = candidate else {
            return false;
        };
        let prefix = format!("{}entity", ex(self.prefix));
        let Some(index) = iri.as_str().strip_prefix(&prefix) else {
            return false;
        };
        // The exact minted spelling, not merely a parseable one: the relation
        // emits six zero-padded digits, so a term with any other shape is not a
        // term it ever minted.
        index.len() == INDEX_WIDTH
            && index.bytes().all(|byte| byte.is_ascii_digit())
            && index.parse::<u64>().is_ok_and(|index| index < self.holds)
    }
}

/// The zero-padded width the fixture mints its candidate indexes at.
const INDEX_WIDTH: usize = 6;

/// The cursor behind a candidate-bound invocation: at most one row, decided
/// before it is pulled.
struct LookupCursor {
    row: Option<PfRow>,
}

impl PfCursor for LookupCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }
}

/// The lazy cursor behind [`CountingProducer`].
struct CountingCursor {
    prefix: &'static str,
    emitted: u64,
    holds: u64,
    reversed: bool,
    bound: Vec<Option<TermValue>>,
    pulled: Arc<AtomicU64>,
}

impl PfCursor for CountingCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted >= self.holds {
            return Ok(None);
        }
        let index = if self.reversed {
            self.holds - 1 - self.emitted
        } else {
            self.emitted
        };
        self.emitted += 1;
        self.pulled.fetch_add(1, Ordering::SeqCst);
        // Zero-padded, so the lexical order of the candidate IRIs is the order
        // the rows arrive in and the declared tie-break's last key agrees with
        // the rank order instead of cutting across it — for a relation that
        // names from the first index up, which is every one but a reversed one.
        let row = [
            TermValue::iri(format!("{}entity{index:06}", ex(self.prefix))),
            TermValue::iri(format!("{}score{index:06}", ex(self.prefix))),
        ];
        let echoed = row
            .iter()
            .enumerate()
            .map(|(position, value)| {
                self.bound
                    .get(position)
                    .and_then(Clone::clone)
                    .unwrap_or_else(|| value.clone())
            })
            .collect();
        Ok(Some(echoed))
    }
}

/// The two request terms, one per producer. Each names its own predicate, so
/// each reaches exactly one producer and each producer is bound to one term.
fn request_terms() -> Vec<RequestTerm> {
    ["title", "body"]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect()
}

/// The shape one producer accepts: an English literal under its own predicate,
/// rendered into the object-side argument position.
fn accepted_terms(predicate: &str) -> Vec<AcceptedTerm> {
    vec![AcceptedTerm {
        pattern: TermPattern {
            kind: TermKind::Literal,
            datatype: None,
            language: Some("en".to_owned()),
            predicate: Some(ex(predicate)),
        },
        placements: vec![TermPlacement {
            facet: RequestFacet::Value,
            position: 1,
            datatype: None,
        }],
    }]
}

/// The registry for one configuration, with both producers' read counters.
fn configured_registry(config: Configuration) -> (PropertyFunctionRegistry, [Arc<Reads>; 2]) {
    let domains = declared_domains(config.blocks);
    let prefixes = candidate_prefixes(config.results);
    let counters = [Arc::new(Reads::default()), Arc::new(Reads::default())];
    let mut registry = PropertyFunctionRegistry::new();
    for (index, (predicate, stratum)) in ["title", "body"].into_iter().zip(strata()).enumerate() {
        let arity = PfArity::new(1, 1);
        // The all-free mode always, and the candidate-bound point mode only
        // where this configuration declares a basis. Declaring the second
        // unconditionally would be a declaration three of this file's
        // configurations do not need and would change the registry fingerprint
        // every one of them is pinned through.
        let mut modes = vec![arity.all_free_mode()];
        if config.exclusion.is_declared() {
            modes.push(BindingPattern::from_bound_positions(
                arity.total(),
                [CANDIDATE_POSITION],
            ));
        }
        let relation = CountingProducer {
            arity,
            modes,
            prefix: prefixes[index],
            holds: config.holds,
            // The second producer only, and only where the configuration asks:
            // the pair then holds one candidate set in two opposite orders.
            reversed: config.results == Results::Reversed && index == 1,
            reads: Arc::clone(&counters[index]),
        };
        registry.register_ranked(
            ex(&format!("pf/{predicate}")),
            Arc::new(relation),
            RankedDeclaration {
                stratum: kernel_iri(stratum.as_str()),
                accepted_terms: accepted_terms(predicate),
                depth_placement: None,
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: domains[index].clone(),
                block_position: None,
                exclusion: config.exclusion,
                mandatory: false,
            },
        );
    }
    (registry, counters)
}

/// Statistics that narrow nothing: each stratum really does hold the four
/// hundred rows its producer declares, and no selectivity is measured.
struct FixtureStatistics {
    cardinalities: BTreeMap<Iri, u64>,
}

impl Statistics for FixtureStatistics {
    fn source(&self) -> &str {
        "example-statistics"
    }

    fn revision(&self) -> &str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.cardinalities.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn fixture_statistics() -> FixtureStatistics {
    let mut cardinalities = BTreeMap::new();
    for stratum in strata() {
        cardinalities.insert(stratum, ROWS);
    }
    for predicate in ["title", "body"] {
        cardinalities.insert(iri(&ex(predicate)), ROWS);
    }
    FixtureStatistics { cardinalities }
}

/// Unit weight for each stratum, under the one decay rule this file uses.
fn fixture_profile() -> FusionProfile {
    let weights = strata()
        .into_iter()
        .map(|stratum| (stratum, Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, decay()).expect("the fixture profile is valid")
}

/// The emitted row ceiling of one compiled unit, parsed out of the text the
/// unit itself renders rather than recomputed from the depth beside it.
fn emitted_limit(unit: &StratumUnit) -> u64 {
    let text = unit.sparql();
    let (_, tail) = text
        .rsplit_once("LIMIT ")
        .expect("a rendered unit carries its own outer bound");
    tail.trim()
        .parse()
        .expect("the emitted bound is a decimal integer")
}

/// Everything one configuration's run is measured on.
struct Measured {
    /// The per-stratum depth the planner recorded.
    planned_depth: BTreeMap<Iri, u32>,
    /// The `LIMIT` the compiler emitted for each unit, from the depth above.
    emitted_limit: BTreeMap<Iri, u64>,
    /// Rows the executor really took out of each relation, one entry per read,
    /// oldest first.
    reads: BTreeMap<Iri, Vec<u64>>,
    /// Ranks fusion really pulled off each stream.
    ranks_pulled: BTreeMap<Iri, u64>,
    /// What the trailer says stopped each stream.
    status: BTreeMap<Iri, ProducerStatus>,
    /// The answer itself, with each row's per-stratum rank.
    rows: Vec<FusedRow>,
    /// How many complete reads of the bundle the answer cost.
    read_attempts: ReadAttempts,
    /// Exclusion lookups the fusion engine says it made against each stratum.
    fused_lookups: BTreeMap<Iri, u64>,
    /// Candidate-bound invocations each relation says it served.
    ///
    /// The producer's own count, beside the engine's above. They are read from
    /// opposite ends of the same seam, so a disagreement is a lookup that went
    /// somewhere other than the producer it was aimed at — and the pair is what
    /// keeps either number from being a self-report nothing checks.
    served_lookups: BTreeMap<Iri, u64>,
    /// How many of those found the candidate.
    ///
    /// The fixture's own answer, so a run in which every verdict was `Excluded`
    /// is distinguishable from one in which the producers really answered.
    found_lookups: BTreeMap<Iri, u64>,
    /// The read-work figure the **library** reports per stratum, cumulative over
    /// every read this call took.
    ///
    /// The counterpart of [`Self::reads`], which is the fixture's own count taken
    /// from inside the producer. The pair is what keeps either number from being
    /// a self-report nothing checks: the producer counts the rows it handed out
    /// and the trailer counts the rows the read returned, from opposite ends of
    /// the same seam, and they are asserted equal.
    reported_materialised: BTreeMap<Iri, Option<u64>>,
    /// Adjacent ranks the fused score could not tell apart, per stratum, as the
    /// trailer reports them.
    collisions: BTreeMap<Iri, u64>,
    /// What the plan surfaced about this configuration's resolution, before any
    /// row was read — carried onto the measurement so a prediction and the
    /// outcome it predicts can be compared in one place.
    planned_resolution: BTreeMap<Iri, PlannedResolution>,
    /// The observed resolution as a caller SEES it: the crate's own rendering of
    /// the trailer's map, byte for byte.
    ///
    /// Kept beside the struct fields above rather than derived from them at the
    /// assertion site, because a test that reads a field proves nothing about
    /// what reaches a caller. A counter present in the struct and absent from
    /// the text is exactly the failure the rendering exists to prevent.
    rendered_resolution: String,
}

impl Measured {
    /// Rows the executor took out of each relation **in the read this answer came
    /// from**, which is the last read each producer served.
    ///
    /// The answering read and not the sum, because the sum answers a different
    /// question. A run that narrowed and was cut discarded its narrowed read
    /// entirely and read again; the rows of the discarded attempt are a cost,
    /// counted in [`Self::reads`] and asserted where that cost is the subject,
    /// but they are not rows the answer rests on and adding them into this number
    /// would say the deep read got deeper.
    fn rows_materialised(&self) -> BTreeMap<Iri, u64> {
        self.reads
            .iter()
            .map(|(stratum, reads)| {
                (
                    stratum.clone(),
                    *reads.last().expect("every producer served a read"),
                )
            })
            .collect()
    }

    /// The answer, as the thing two reads of one configuration have to agree on:
    /// the rows, their scores, and their order.
    fn answer(&self) -> Vec<(String, Fixed)> {
        self.rows
            .iter()
            .map(|row| (row.entity.as_str().to_owned(), row.score))
            .collect()
    }

    /// The value `field` carries for both strata, and a panic naming both when
    /// they disagree — every assertion in this file is about a number the two
    /// symmetric producers reach together.
    fn both<T: Clone + PartialEq + std::fmt::Debug>(
        &self,
        what: &str,
        field: &BTreeMap<Iri, T>,
    ) -> T {
        let [left, right] = strata();
        let left_value = field.get(&left).expect("the left stratum was measured");
        let right_value = field.get(&right).expect("the right stratum was measured");
        assert_eq!(
            left_value, right_value,
            "the two symmetric producers disagree on {what}"
        );
        left_value.clone()
    }

    /// The deepest per-stratum rank any emitted row carries.
    ///
    /// Read off the answer, never off the value under test: this is a property
    /// of which rows the top-k contains, and it is the rank the threshold has to
    /// fall below for the last of them to certify.
    fn deepest_emitted_rank(&self) -> u64 {
        self.rows
            .iter()
            .flat_map(|row| row.contributions.iter().map(|(_, rank, _)| *rank))
            .max()
            .expect("the fixture answer is not empty")
    }

    /// How many strata named the deepest-ranked emitted row.
    fn namers_of_deepest_row(&self) -> usize {
        let deepest = self.deepest_emitted_rank();
        self.rows
            .iter()
            .find(|row| {
                row.contributions
                    .iter()
                    .any(|(_, rank, _)| *rank == deepest)
            })
            .expect("the deepest rank belongs to some emitted row")
            .contributions
            .len()
    }
}

/// Run one configuration through the whole ladder as a caller runs it, with
/// `search`, and measure what that run cost.
///
/// Two of the seven measurements are not on the answer `search` returns — the
/// depth is the plan's and the `LIMIT` is the compiled unit's — so a plan and a
/// bundle are built here as well, and read for those two numbers alone. Both
/// stages are pure functions of the request, the registry and the statistics, so
/// the plan read here is the plan `search` runs; and neither stage opens a
/// relation, so nothing they do is counted by the producers' own read counters.
///
/// The rest is `search`'s: the rows fusion pulled, the statuses the trailer
/// reports, and how many complete reads of the bundle the answer cost. Driving
/// the stages by hand instead would measure a read this crate no longer takes —
/// the whole subject of this file is which read `search` chooses.
fn measure(config: Configuration, dataset: &RdfDataset) -> Measured {
    let (registry, counters) = configured_registry(config);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };

    let answer = block_on(search(
        &request,
        &registry,
        &statistics,
        dataset,
        &env,
        &profile,
    ))
    .expect("the fixture request searches");

    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");

    measured(
        &planned,
        &compiled,
        &counters,
        &answer.trailer,
        answer.rows,
        answer.read_attempts,
    )
}

/// Run one configuration at the depths its plan recorded, reading every stratum
/// in full.
///
/// The reference the speculative path is checked against, and it is the hand-
/// composed pipeline rather than a flag on `search`: `execute` reads each unit at
/// its planned depth, which is the read this file pinned before any narrowing
/// existed. A narrowing that changed an answer would show up as a disagreement
/// between this and [`measure`], and nothing else in this file could catch it.
fn measure_at_planned_depth(config: Configuration, dataset: &RdfDataset) -> Measured {
    let (registry, counters) = configured_registry(config);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);

    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");

    let execution =
        block_on(execute(&compiled, &registry, dataset)).expect("the fixture registry executes");

    let mut streams = Vec::new();
    for stream in execution.streams {
        let plan_id = stream.plan_id;
        let fused_bound = stream.fused_bound;
        let attestation = stream.attestation.clone();
        let adapter =
            RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                .expect("the fixture profile weights every executed stratum");
        streams.push((
            stream.stratum,
            adapter
                .with_plan_id(plan_id)
                .with_fused_bound(fused_bound)
                .with_attestation(attestation),
        ));
    }
    let fused = block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the executed streams fuse");
    let trailer = fused.trailer.completed_with(execution.statuses);

    measured(
        &planned,
        &compiled,
        &counters,
        &trailer,
        fused.rows,
        // One read, by construction: `execute` takes the planned depth and this
        // function calls it once.
        ReadAttempts::Once,
    )
}

/// Assemble one run's measurements out of the stages that produced it.
///
/// One assembly for both paths, so the numbers the two are compared on are read
/// off the same places by the same code — a second reading would be a second
/// chance for the comparison to be about the harness.
fn measured(
    planned: &Plan,
    compiled: &CompiledRetrieval,
    counters: &[Arc<Reads>; 2],
    trailer: &FusionTrailer,
    rows: Vec<FusedRow>,
    read_attempts: ReadAttempts,
) -> Measured {
    Measured {
        planned_depth: planned
            .stratum_depths
            .iter()
            .map(|(stratum, depth)| (stratum.clone(), *depth))
            .collect(),
        emitted_limit: compiled
            .units
            .iter()
            .map(|unit| (unit.stratum.clone(), emitted_limit(unit)))
            .collect(),
        reads: strata()
            .into_iter()
            .zip(counters.iter())
            .map(|(stratum, counter)| (stratum, counter.rows()))
            .collect(),
        ranks_pulled: trailer
            .resolution
            .iter()
            .map(|(stratum, resolution)| (stratum.clone(), resolution.ranks_pulled))
            .collect(),
        status: trailer.statuses.clone(),
        rows,
        read_attempts,
        fused_lookups: trailer
            .resolution
            .iter()
            .map(|(stratum, resolution)| (stratum.clone(), resolution.exclusion_lookups))
            .collect(),
        served_lookups: strata()
            .into_iter()
            .zip(counters.iter())
            .map(|(stratum, counter)| (stratum, counter.lookups()))
            .collect(),
        found_lookups: strata()
            .into_iter()
            .zip(counters.iter())
            .map(|(stratum, counter)| (stratum, counter.lookups_found()))
            .collect(),
        reported_materialised: trailer
            .resolution
            .iter()
            .map(|(stratum, resolution)| (stratum.clone(), resolution.rows_materialised))
            .collect(),
        collisions: trailer
            .resolution
            .iter()
            .map(|(stratum, resolution)| (stratum.clone(), resolution.collisions_observed))
            .collect(),
        planned_resolution: compiled.resolution.clone(),
        rendered_resolution: observed_resolution(&trailer.resolution),
    }
}

/// The five measured numbers of one configuration, rendered for a failure to
/// read as a row of the table this file pins.
fn render(config: Configuration, measured: &Measured) -> String {
    format!(
        "{name}: depth={depth} limit={limit} materialised={materialised} \
         ranks_pulled={pulled} status={status:?}",
        name = config.name,
        depth = measured.both("the planned depth", &measured.planned_depth),
        limit = measured.both("the emitted LIMIT", &measured.emitted_limit),
        materialised = measured.both("the rows materialised", &measured.rows_materialised()),
        pulled = measured.both("the ranks pulled", &measured.ranks_pulled),
        status = measured.both("the terminal status", &measured.status),
    )
}

/// The depth a speculative read asks one stratum of this file's two-stratum
/// configurations for.
///
/// Taken from the crate's own derivation rather than restated as a literal
/// beside it: the number is *asserted* once, in
/// [`the_speculative_depth_is_the_bound_plus_one_head_per_open_stream`], and
/// every other use reads it from the same function the engine reads it from. A
/// second literal here would go on passing if the derivation changed under it.
fn speculative_read_depth() -> u64 {
    u64::from(speculative_depth(
        TOP_K,
        strata().len(),
        u32::try_from(ROWS).expect("the fixture row bound fits a recordable depth"),
    ))
}

/// The rows a speculative read of one such stratum materialises: that depth, and
/// the probe row every emitted bound carries one past it.
fn speculative_limit() -> u64 {
    speculative_read_depth() + 1
}

/// The status a fusion that stopped a live stream writes, at the contribution
/// its head was sitting on. Computed from the profile's own law over the rank
/// the run really reached, never read back out of the trailer.
fn ceiling_at(rank: u64) -> ProducerStatus {
    ProducerStatus::CeilingReached {
        bound: contribution_under(decay(), Fixed::ONE, rank)
            .expect("the fixture contribution is in range"),
    }
}

/// **The control, and the solved case.** Different blocks, different candidates.
///
/// Every candidate lies in exactly one declared block, so the planner's merge
/// argument holds and the request's bound of five becomes the recorded depth.
/// The compiler emits one probe row past it, the executor reads no further, and
/// fusion certifies the fifth row with both heads one rank past the deepest row
/// it emitted.
#[test]
fn distinct_blocks_disjoint_results_read_only_what_the_bound_needs() {
    let dataset = common::empty_dataset();
    let measured = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let report = render(DISTINCT_BLOCKS_DISJOINT_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the bound is what stopped this run");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        5,
        "the disjoint declarations license the request's own bound as the \
         depth — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        6,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        4,
        "and fusion certified five rows without reading past the fourth — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ceiling_at(4),
        "a stream fusion stopped reports the contribution it stopped at — {report}"
    );

    // The derivation, with no executor in it, predicts the measured number.
    // The deepest row this answer contains sits at rank three in its own
    // stratum, and only its own stratum admits its block, so the threshold has
    // to fall to the fourth rank for it to certify.
    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 3,
        "the fifth row of an alternating answer — {report}"
    );
    assert_eq!(
        measured.namers_of_deepest_row(),
        1,
        "disjoint blocks mean one namer per candidate — {report}"
    );
    assert_eq!(
        crossing_rank_at(decay(), &[Fixed::ONE], deepest, &[Fixed::ONE]),
        measured.both("the ranks pulled", &measured.ranks_pulled),
        "the threshold derivation predicts the read — {report}"
    );
}

/// **One block, and both producers naming the same candidates.**
///
/// Two strata sharing a block are not pairwise disjoint, so the planner's merge
/// argument does not hold and the depth falls back to the declared four
/// hundred. Fusion stops at the sixth rank all the same — every candidate
/// collected from both streams that admit its block, so its lower bound is
/// measured against exactly the streams it already read.
///
/// The depth the *plan* records is therefore four hundred and the rows the
/// *read* takes are eight: the speculative read asks each stratum for the fused
/// frontier and one probe row, the fusion certifies inside it, and the answer is
/// that answer. Nothing was thrown away here, which is why the eight is the whole
/// of what this configuration costs.
#[test]
fn shared_block_intersecting_results_answer_at_the_sixth_rank_inside_the_speculative_read() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_INTERSECTING_RESULTS, &dataset);
    let report = render(SHARED_BLOCK_INTERSECTING_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the bound is what stopped this run");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        400,
        "a shared block is not a disjoint pair, so the declaration stands as \
         the depth — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        401,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised()),
        speculative_limit(),
        "so the executor read the frontier and its probe row, not the \
         declaration — {report}"
    );
    assert_eq!(
        measured.read_attempts,
        ReadAttempts::Once,
        "and it read once: the narrowed read certified, so there was nothing to \
         fall back from — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        6,
        "while fusion needed six ranks of it — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ceiling_at(6),
        "a stream fusion stopped reports the contribution it stopped at — {report}"
    );

    // The gap between the two numbers above is the measurement this
    // configuration exists to make, and it is stated as the subtraction rather
    // than as a word: the executor's read is still not bounded by what fusion
    // consumed, and what bounds it now is the frontier rather than the
    // declaration. Two rows — the seventh rank the frontier reaches for, which
    // is the head that made the threshold fall, and the eighth that makes the
    // ending observable.
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised())
            - measured.both("the ranks pulled", &measured.ranks_pulled),
        2,
        "rows the executor materialised that fusion never asked for — {report}"
    );

    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 5,
        "both streams rank the same candidates — {report}"
    );
    assert_eq!(
        measured.namers_of_deepest_row(),
        2,
        "and both of them name every candidate — {report}"
    );
    assert_eq!(
        crossing_rank_at(
            decay(),
            &[Fixed::ONE, Fixed::ONE],
            deepest,
            &[Fixed::ONE, Fixed::ONE]
        ),
        measured.both("the ranks pulled", &measured.ranks_pulled),
        "the threshold derivation predicts the read — {report}"
    );
}

/// **The permanent control: one block, and no candidate named twice.**
///
/// This configuration never reaches the threshold gate at all. A candidate the
/// left stream named is not *final* while the right stream — which declares the
/// same block, and so could still name it — is open, so nothing certifies until
/// both streams run out. Both drain, and both report exhaustion.
///
/// The four hundreds below are the evidence of what an improvement would be
/// improving. They stay asserted exactly, so work that lowers them has to
/// change this literal in the same commit that earns it.
///
/// They are also not the whole bill, and the last assertion says so: with no
/// exclusion basis declared, nothing can settle finality before a stream runs
/// out, so the speculative read is cut, thrown away, and followed by the four
/// hundred anyway. What makes this configuration cheaper is the producers
/// answering lookups — the same rows under a declared basis are answered by a
/// single read at the rank the plan proves the fusion stops by; see
/// [`the_lookups_answer_the_drained_configuration_in_one_read_at_the_proven_stopping_rank`].
#[test]
fn shared_block_disjoint_results_drain_both_streams_unbounded_control() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = render(SHARED_BLOCK_DISJOINT_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the answer is still five rows");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        400,
        "a shared block is not a disjoint pair — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        401,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised()),
        400,
        "the executor read every row the relation held — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        400,
        "and fusion read every one of them, because no candidate is final \
         while a stream sharing its block is still open — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ProducerStatus::Exhausted { rows_emitted: ROWS },
        "a stream read to its end reports exhaustion, not a ceiling — {report}"
    );
    // And the whole bill, which the four hundred above is not: the speculative
    // read came first, was cut, and was thrown away. Stated here so the cost of
    // this configuration is not readable as one number smaller than it is.
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![speculative_limit(), ROWS],
        "the narrowed read this configuration cannot keep, and then the read it \
         always took — {report}"
    );

    // Why the threshold derivation does not apply here: the gate this run never
    // reached would have let it stop at the sixty-sixth rank. The candidates
    // are single-named and two streams share their block, so the crossing is
    // the expensive one — and even that is a sixth of what was read.
    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 3,
        "the fifth row of an alternating answer — {report}"
    );
    assert_eq!(
        crossing_rank_at(decay(), &[Fixed::ONE], deepest, &[Fixed::ONE, Fixed::ONE]),
        66,
        "the threshold alone would have licensed a stop at the sixty-sixth \
         rank; finality is what forced the drain — {report}"
    );
}

/// **Anti-vacuity.** The control and the permanent control have the identical
/// result structure — four hundred rows each, no candidate named twice, the
/// same bound, the same weights. They differ in one thing only: what the two
/// producers *declared* about where their candidates lie.
///
/// If every measured number agreed, this harness would be measuring the
/// fixtures rather than the declaration, and every other assertion in this file
/// would be worth nothing.
#[test]
fn the_declaration_alone_moves_every_measured_number() {
    let dataset = common::empty_dataset();
    let distinct = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let shared = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);

    // First the sameness, so the difference below is known to come from the
    // declaration: the two runs return the same answer.
    let answer = |measured: &Measured| -> Vec<(String, Fixed)> {
        measured
            .rows
            .iter()
            .map(|row| (row.entity.as_str().to_owned(), row.score))
            .collect()
    };
    assert_eq!(
        answer(&distinct),
        answer(&shared),
        "the two configurations hold the identical candidates and must answer \
         identically; only the cost of the answer is in question"
    );

    // Then the difference, number by number.
    assert_ne!(
        distinct.both("the planned depth", &distinct.planned_depth),
        shared.both("the planned depth", &shared.planned_depth),
        "the declaration decides the depth"
    );
    assert_ne!(
        distinct.both("the emitted LIMIT", &distinct.emitted_limit),
        shared.both("the emitted LIMIT", &shared.emitted_limit),
        "the declaration decides the emitted bound"
    );
    assert_ne!(
        distinct.both("the rows materialised", &distinct.rows_materialised()),
        shared.both("the rows materialised", &shared.rows_materialised()),
        "the declaration decides how much the executor reads"
    );
    assert_ne!(
        distinct.both("the ranks pulled", &distinct.ranks_pulled),
        shared.both("the ranks pulled", &shared.ranks_pulled),
        "the declaration decides how much fusion reads"
    );
    assert_ne!(
        distinct.both("the terminal status", &distinct.status),
        shared.both("the terminal status", &shared.status),
        "and the declaration decides which ending the trailer reports"
    );
}

// ---------------------------------------------------------------------------
// 1b'. The speculative read: what it asks for, when it is thrown away, and what
//      it may never change.
// ---------------------------------------------------------------------------
//
// A depth the planner could not narrow is a sound bound and a deep read. The
// engine therefore reads the same bundle speculatively first — every stratum at
// the fused frontier rather than at its planned depth — and keeps that read only
// when the fusion certified inside it. The three assertions below are the three
// things that has to be true of: the depth is derived and not chosen, the
// fallback fires exactly where the narrowing failed, and no answer moves either
// way.

/// **The derivation, with no executor in it.** The speculative depth is `k` plus
/// one unmerged head per stratum, capped by the depth the plan recorded.
///
/// Both halves are the same argument the fusion stage already makes. Certifying
/// `k` rows means certifying the last of them, and the only thing that makes the
/// threshold fall far enough to do so is one unread head on each stream still
/// open — so `k + strata` is where a fusion the threshold can stop has stopped,
/// and `crates/retrieval/tests/fusion.rs` asserts exactly that ceiling over
/// pairwise-disjoint strata. The cap is the other half: the plan's depth is the
/// deepest read that stratum was admitted for, and a frontier past it would be a
/// read nothing had judged against the producer's declared row bound.
#[test]
fn the_speculative_depth_is_the_bound_plus_one_head_per_open_stream() {
    // The frontier itself, below the cap, at three bounds and three widths.
    assert_eq!(
        speculative_depth(TopK::new(5), 2, 400),
        7,
        "five rows, and one unmerged head for each of two open streams"
    );
    assert_eq!(
        speculative_depth(TopK::new(5), 3, 400),
        8,
        "a third stream is a third head to pay for, and nothing else"
    );
    assert_eq!(
        speculative_depth(TopK::new(1), 1, 400),
        2,
        "one row over one stream still needs the head that certifies it"
    );
    assert_eq!(
        speculative_depth(TopK::new(50), 2, 400),
        52,
        "the bound is the term that grows, and it grows one for one"
    );

    // And the cap: at it, past it, and one inside it.
    assert_eq!(
        speculative_depth(TopK::new(5), 2, 7),
        7,
        "a plan that recorded exactly the frontier is read to exactly the \
         frontier, and nothing is narrowed"
    );
    assert_eq!(
        speculative_depth(TopK::new(5), 2, 5),
        5,
        "a plan shallower than the frontier is the binding constraint: the \
         speculative depth IS the planned depth, so no fallback can fire"
    );
    assert_eq!(
        speculative_depth(TopK::new(5), 2, 6),
        6,
        "one rank inside the frontier, and the cap still wins"
    );
    assert_eq!(
        speculative_depth(TopK::new(400), 2, 5),
        5,
        "a bound far past the plan's own depth narrows nothing and asks for \
         nothing extra"
    );

    // Neither term can overflow into a deeper read than the plan admitted.
    assert_eq!(
        speculative_depth(TopK::new(usize::MAX), 2, 400),
        400,
        "a bound past what a depth can express saturates into the cap"
    );
    assert_eq!(
        speculative_depth(TopK::new(5), usize::MAX, 400),
        400,
        "and so does a stratum count past it"
    );

    // The value this file's own two-stratum configurations are read at, tied to
    // the fixtures rather than to a literal.
    assert_eq!(
        speculative_read_depth(),
        7,
        "the bound of five and the two producers of this file"
    );
    assert_eq!(
        speculative_limit(),
        8,
        "and the read that depth licenses carries its probe row"
    );
}

/// **The fallback fires exactly where the narrowing failed**, observed on the
/// answer rather than inferred from a timing.
///
/// The three configurations differ in what their producers *declared*, and that
/// is all — so which of them pays for a second read is a fact about the
/// declaration, exactly as every other number in this file is.
///
/// * distinct blocks: the planner already narrowed this one to the bound itself,
///   so the speculative depth is the planned depth and there is nothing to fall
///   back from;
/// * one block, both naming the same candidates: the plan is deep and the
///   speculative read certifies inside it, which is the case the whole mechanism
///   exists for;
/// * one block, no candidate named twice: nothing is final while a stream sharing
///   its block is open, so the fusion drains whatever it is given, the
///   speculative read is cut, and it is discarded for the planned read.
#[test]
fn the_fallback_fires_for_the_configuration_the_narrowing_does_not_hold_in() {
    let dataset = common::empty_dataset();

    let distinct = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    assert_eq!(
        distinct.read_attempts,
        ReadAttempts::Once,
        "a stratum the planner already narrowed has nothing left to speculate \
         about — {}",
        render(DISTINCT_BLOCKS_DISJOINT_RESULTS, &distinct)
    );
    assert_eq!(
        distinct.both("the reads", &distinct.reads),
        vec![6],
        "and it took one read, at the depth the plan recorded plus its probe row"
    );

    let intersecting = measure(SHARED_BLOCK_INTERSECTING_RESULTS, &dataset);
    assert_eq!(
        intersecting.read_attempts,
        ReadAttempts::Once,
        "the speculative read certified, so it IS the answer — {}",
        render(SHARED_BLOCK_INTERSECTING_RESULTS, &intersecting)
    );
    assert_eq!(
        intersecting.both("the reads", &intersecting.reads),
        vec![speculative_limit()],
        "and the one read it took was the narrowed one"
    );

    let disjoint = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    assert_eq!(
        disjoint.read_attempts,
        ReadAttempts::Twice,
        "the speculative read was cut, so it was thrown away — {}",
        render(SHARED_BLOCK_DISJOINT_RESULTS, &disjoint)
    );
    assert_eq!(
        disjoint.both("the reads", &disjoint.reads),
        vec![speculative_limit(), ROWS],
        "two reads, in order: the narrowed one that was discarded, and the \
         planned one the answer came from"
    );
    assert_eq!(
        disjoint.both("the terminal status", &disjoint.status),
        ProducerStatus::Exhausted { rows_emitted: ROWS },
        "and the kept read is the read this configuration always took"
    );
}

/// **A stream that ran out is not a stream that was cut.**
///
/// Structurally the drained control — one block, no candidate named twice, the
/// same declared four hundred rows and so the same planned depth of four hundred
/// — except that each relation really holds four candidates. The speculative read
/// asks for eight and gets four, and four is every row there was.
///
/// Without this, "fall back whenever a narrowed read ended early" would pass
/// every other test in this file while paying for a second read of a stratum that
/// has already been read to its end — a re-read that cannot find one more row,
/// over the configuration where re-reading is most expensive.
#[test]
fn a_stream_shorter_than_the_speculative_read_is_exhausted_and_is_not_re_read() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_SHORT_STREAMS, &dataset);
    let report = render(SHARED_BLOCK_SHORT_STREAMS, &measured);

    // The premise: this configuration really is one the speculation narrows, so
    // the exhaustion below is an exhaustion *inside* a narrowed read.
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        u32::try_from(ROWS).expect("the fixture row bound fits a recordable depth"),
        "a shared block is not a disjoint pair, so the declaration stands as \
         the depth — {report}"
    );
    assert!(
        SHORT_ROWS < speculative_read_depth(),
        "the relation must hold fewer rows than the speculative read asks for, \
         or this test is about nothing"
    );

    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ProducerStatus::Exhausted {
            rows_emitted: SHORT_ROWS
        },
        "a stream that ran out reports exhaustion, not a depth — {report}"
    );
    assert_eq!(
        measured.read_attempts,
        ReadAttempts::Once,
        "and an exhausted stream has nothing a deeper read could add — {report}"
    );
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![SHORT_ROWS],
        "so exactly one read happened, and it stopped where the rows did — \
         {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        SHORT_ROWS,
        "the fusion drained it, because no candidate is final while a stream \
         sharing its block is open — {report}"
    );
}

/// **The answer never moves.** Every configuration, read speculatively and read
/// at the depths its plan recorded, row for row, score for score and order for
/// order.
///
/// This is the whole soundness claim, and it is the one assertion in this file
/// that would fail for a narrowing that saved a great deal and answered a
/// slightly different question. The reference is the hand-composed pipeline over
/// `execute`, which takes each unit's planned depth and is byte for byte the read
/// this file measured before any speculation existed.
#[test]
fn the_speculative_read_returns_the_answer_the_planned_read_returns() {
    let dataset = common::empty_dataset();
    for config in [
        DISTINCT_BLOCKS_DISJOINT_RESULTS,
        SHARED_BLOCK_INTERSECTING_RESULTS,
        SHARED_BLOCK_DISJOINT_RESULTS,
        SHARED_BLOCK_SHORT_STREAMS,
        SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS,
        SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS,
        UNDECLARED_BLOCKS_DISJOINT_RESULTS_WITH_LOOKUPS,
        SHARED_BLOCK_REVERSED_RESULTS_WITH_LOOKUPS,
    ] {
        let searched = measure(config, &dataset);
        let full = measure_at_planned_depth(config, &dataset);

        // The reference really did read in full, or the two agree for the
        // uninteresting reason.
        assert_eq!(
            full.both("the rows materialised", &full.rows_materialised()),
            full.both("the emitted LIMIT", &full.emitted_limit)
                .min(config.holds),
            "{name}: the reference must read to the depth the plan recorded",
            name = config.name
        );

        assert_eq!(
            searched.answer(),
            full.answer(),
            "{name}: the speculative read returned a different answer than the \
             planned read\n  speculative: {a}\n  planned:     {b}",
            name = config.name,
            a = render(config, &searched),
            b = render(config, &full)
        );
        assert_eq!(
            searched.both("the terminal status", &searched.status),
            full.both("the terminal status", &full.status),
            "{name}: and it must report the same ending, because a narrowed read \
             that certified certified under the same law",
            name = config.name
        );
    }
}

// ---------------------------------------------------------------------------
// 1c. The licence is per stratum: what it costs a stratum, and what it does not.
// ---------------------------------------------------------------------------
//
// Everything in this section runs the same ladder the three configurations above
// do, over the same four hundred rows, the same unit weights and the same bound of
// five. What varies is only how many strata there are and what each of them
// declares — so a depth that moved between two of these tables moved because one
// declaration changed and nothing else did.

/// One stratum of a declared configuration.
///
/// The name is the whole identity: it names the stratum IRI, the request
/// predicate this stratum's producer answers, and the prefix its candidates are
/// minted under. So distinct names mean distinct candidate sets, and every
/// configuration below differs from every other only in `block` and `duplicates`.
#[derive(Clone, Copy, Debug)]
struct Declared {
    name: &'static str,
    /// The single block it declares, or `None` for
    /// [`CandidateDomains::Unrestricted`]. One block, so the declaration entails
    /// the block of every row and no producer owes a block column.
    block: Option<&'static str>,
    /// What it promises about naming one candidate twice.
    duplicates: DuplicatePolicy,
}

impl Declared {
    /// The domain declaration this stratum registers.
    fn domains(self) -> CandidateDomains {
        self.block.map_or(CandidateDomains::Unrestricted, |block| {
            CandidateDomains::within([domain_tag(&format!("domain/{block}"))])
        })
    }

    fn stratum(self) -> Iri {
        iri(&ex(&format!("stratum/{name}", name = self.name)))
    }
}

/// The registry for one declared configuration.
fn declared_registry(strata: &[Declared]) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    for entry in strata {
        let arity = PfArity::new(1, 1);
        let relation = CountingProducer {
            arity,
            modes: vec![arity.all_free_mode()],
            prefix: entry.name,
            holds: ROWS,
            reversed: false,
            // Nothing in this section measures the read, only the depth the plan
            // recorded and the answer that came back, so these counts are
            // collected and never read.
            reads: Arc::new(Reads::default()),
        };
        registry.register_ranked(
            ex(&format!("pf/{name}", name = entry.name)),
            Arc::new(relation),
            RankedDeclaration {
                stratum: kernel_iri(entry.stratum().as_str()),
                accepted_terms: accepted_terms(entry.name),
                depth_placement: None,
                candidate_position: 0,
                duplicates: entry.duplicates,
                fidelity: RankFidelity::EXACT,
                domains: entry.domains(),
                block_position: None,
                exclusion: ExclusionBasis::Unavailable,
                mandatory: false,
            },
        );
    }
    registry
}

/// One request term per stratum, each naming that stratum's own predicate, so
/// each producer receives exactly one term and every stratum survives placement.
fn declared_terms(strata: &[Declared]) -> Vec<RequestTerm> {
    strata
        .iter()
        .map(|entry| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(entry.name))),
        })
        .collect()
}

/// Statistics that narrow nothing: every stratum really holds the four hundred
/// rows its producer declares, and no selectivity is measured.
fn declared_statistics(strata: &[Declared]) -> FixtureStatistics {
    let mut cardinalities = BTreeMap::new();
    for entry in strata {
        cardinalities.insert(entry.stratum(), ROWS);
        cardinalities.insert(iri(&ex(entry.name)), ROWS);
    }
    FixtureStatistics { cardinalities }
}

fn declared_profile(strata: &[Declared]) -> FusionProfile {
    let weights = strata
        .iter()
        .map(|entry| (entry.stratum(), Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, decay()).expect("the fixture profile is valid")
}

/// What one run of a declared configuration is measured on.
struct DeclaredRun {
    /// The depth the planner recorded for each stratum.
    depths: BTreeMap<Iri, u32>,
    /// The cause the plan itself reports for each of those depths.
    causes: BTreeMap<Iri, DepthCause>,
    /// The answer, as the thing two runs have to agree on: rows, scores, order.
    answer: Vec<(String, Fixed)>,
}

/// Run one declared configuration through `plan` → `compile` → `execute` → `fuse`
/// under `bound`.
fn run_declared(strata: &[Declared], bound: ReadBound, dataset: &RdfDataset) -> DeclaredRun {
    let registry = declared_registry(strata);
    let statistics = declared_statistics(strata);
    let profile = declared_profile(strata);
    let request = RetrievalRequest::from_terms(declared_terms(strata), bound);

    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");
    let execution =
        block_on(execute(&compiled, &registry, dataset)).expect("the fixture registry executes");

    let mut streams = Vec::new();
    for stream in execution.streams {
        let plan_id = stream.plan_id;
        let fused_bound = stream.fused_bound;
        let attestation = stream.attestation.clone();
        let adapter =
            RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                .expect("the fixture profile weights every executed stratum");
        streams.push((
            stream.stratum,
            adapter
                .with_plan_id(plan_id)
                .with_fused_bound(fused_bound)
                .with_attestation(attestation),
        ));
    }
    let fused = block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the executed streams fuse");

    DeclaredRun {
        depths: planned
            .stratum_depths
            .iter()
            .map(|(stratum, depth)| (stratum.clone(), *depth))
            .collect(),
        causes: strata
            .iter()
            .map(|entry| {
                let stratum = entry.stratum();
                let cause = planned
                    .explain_depth(&stratum)
                    .expect("a planned stratum explains its own depth");
                (stratum, cause)
            })
            .collect(),
        answer: fused
            .rows
            .iter()
            .map(|row| (row.entity.as_str().to_owned(), row.score))
            .collect(),
    }
}

/// The depth a licensed stratum records: the request's own bound, read off the
/// bound itself rather than restated as a literal beside it.
fn licensed_depth() -> u32 {
    u32::try_from(TOP_K.get()).expect("the fixture bound fits a recordable depth")
}

/// What a stratum the licence does not reach records: the registry's declaration,
/// undisturbed.
fn unlicensed_depth() -> u32 {
    u32::try_from(ROWS).expect("the fixture row bound fits a recordable depth")
}

/// Assert one configuration's per-stratum `(depth, cause)`, named by stratum.
///
/// Per stratum and never "both", because the whole claim of this section is that
/// two strata of one request can record two different depths.
fn assert_depths(what: &str, run: &DeclaredRun, expected: &[(&Declared, u32, DepthCause)]) {
    let measured: Vec<(String, u32, DepthCause)> = expected
        .iter()
        .map(|(entry, _, _)| {
            let stratum = entry.stratum();
            (
                entry.name.to_owned(),
                *run.depths
                    .get(&stratum)
                    .expect("a surviving stratum records a depth"),
                *run.causes
                    .get(&stratum)
                    .expect("a surviving stratum explains its depth"),
            )
        })
        .collect();
    let wanted: Vec<(String, u32, DepthCause)> = expected
        .iter()
        .map(|(entry, depth, cause)| (entry.name.to_owned(), *depth, *cause))
        .collect();
    assert_eq!(measured, wanted, "{what}");
}

/// The top five rows of `run`'s answer.
fn top_five(run: &DeclaredRun) -> Vec<(String, Fixed)> {
    run.answer.iter().take(5).cloned().collect()
}

/// **The differential.** The answer the narrowed read produced, against the answer
/// the same configuration produces when every stratum reads its whole stream.
///
/// The reference run states [`ReadBound::Complete`], which licenses no prefix at
/// all, so every stratum records its declared four hundred and the fusion sees the
/// entire stream. Its first five rows are the top five over the full streams; the
/// bounded run's five rows are the top five over the narrowed prefixes. They are
/// compared row for row, score for score and order for order — a narrowing that
/// changed any of the three would be unsound, and nothing else in this file could
/// catch it.
fn assert_narrowing_changed_no_answer(what: &str, strata: &[Declared], dataset: &RdfDataset) {
    let bounded = run_declared(strata, ReadBound::Bounded(TOP_K), dataset);
    let complete = run_declared(strata, ReadBound::Complete, dataset);

    // The reference really did read everything: if it had narrowed too, the two
    // runs would agree for the uninteresting reason.
    for entry in strata {
        assert_eq!(
            complete.depths.get(&entry.stratum()).copied(),
            Some(unlicensed_depth()),
            "{what}: the reference run must read the whole stream of {name}",
            name = entry.name
        );
    }
    // And the bounded run really did narrow something, or this compares two
    // identical reads and proves nothing.
    assert!(
        strata
            .iter()
            .any(|entry| bounded.depths.get(&entry.stratum()).copied() == Some(licensed_depth())),
        "{what}: no stratum narrowed, so this differential has nothing to test"
    );

    assert_eq!(
        bounded.answer.len(),
        5,
        "{what}: the bound is what stopped the narrowed run"
    );
    assert_eq!(
        bounded.answer,
        top_five(&complete),
        "{what}: the narrowed read returned a different answer than the full read"
    );
}

/// **N1.** Three strata, pairwise-disjoint blocks, every one of them `Unique`.
/// Every stratum is licensed, exactly as two disjoint strata always were.
const N1: [Declared; 3] = [
    Declared {
        name: "alpha",
        block: Some("alpha"),
        duplicates: DuplicatePolicy::Unique,
    },
    Declared {
        name: "beta",
        block: Some("beta"),
        duplicates: DuplicatePolicy::Unique,
    },
    Declared {
        name: "gamma",
        block: Some("gamma"),
        duplicates: DuplicatePolicy::Unique,
    },
];

/// **N2.** One stratum, restricting no block at all, `Unique`. There is no other
/// stratum for it to share a candidate with, so it is licensed whatever it says
/// about blocks.
const N2: [Declared; 1] = [Declared {
    name: "solo",
    block: None,
    duplicates: DuplicatePolicy::Unique,
}];

/// **N3.** Three strata: one whose block no other names, and a pair that share
/// one. The disjoint stratum narrows; the pair do not.
const N3: [Declared; 3] = [
    Declared {
        name: "apart",
        block: Some("apart"),
        duplicates: DuplicatePolicy::Unique,
    },
    Declared {
        name: "together-one",
        block: Some("together"),
        duplicates: DuplicatePolicy::Unique,
    },
    Declared {
        name: "together-two",
        block: Some("together"),
        duplicates: DuplicatePolicy::Unique,
    },
];

/// **N4.** Two strata with disjoint blocks, one `Unique` and one `Allowed`. The
/// `Unique` one narrows; the `Allowed` one does not.
const N4: [Declared; 2] = [
    Declared {
        name: "unique-side",
        block: Some("unique-side"),
        duplicates: DuplicatePolicy::Unique,
    },
    Declared {
        name: "allowed-side",
        block: Some("allowed-side"),
        duplicates: DuplicatePolicy::Allowed,
    },
];

/// **The forbidden repair.** A stratum that shares a block with another stratum of
/// the same request is never licensed a prefix, however the rest of the request is
/// declared.
///
/// This names that one shape and no wider one. "Any configuration where some pair
/// intersects licenses nothing" would be the wider claim, and it is exactly the
/// over-refusal `N3` and `N4` exist to forbid: they *do* intersect somewhere, and
/// a stratum outside the intersection is still licensed. What is pinned here is
/// only that a stratum inside one reads its declaration.
#[test]
fn a_stratum_sharing_a_block_with_another_is_never_licensed() {
    let dataset = common::empty_dataset();

    // Two strata, one block: both are inside the intersection, so neither
    // narrows. This is the two-producer control at the top of the file, restated
    // over the general harness so the general harness is known to reproduce it.
    let pair = [
        Declared {
            name: "shared-one",
            block: Some("shared"),
            duplicates: DuplicatePolicy::Unique,
        },
        Declared {
            name: "shared-two",
            block: Some("shared"),
            duplicates: DuplicatePolicy::Unique,
        },
    ];
    let run = run_declared(&pair, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "two strata sharing one block",
        &run,
        &[
            (&pair[0], unlicensed_depth(), DepthCause::Declaration),
            (&pair[1], unlicensed_depth(), DepthCause::Declaration),
        ],
    );

    // The same, inside a request that also holds a stratum nobody shares with:
    // the sharers are still unlicensed, and sharing is still the reason.
    let run = run_declared(&N3, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "the sharing pair inside a three-stratum request",
        &run,
        &[
            (&N3[1], unlicensed_depth(), DepthCause::Declaration),
            (&N3[2], unlicensed_depth(), DepthCause::Declaration),
        ],
    );

    // An `Unrestricted` declaration shares every block there is, so with a second
    // stratum present it is inside every intersection — and so is the other
    // stratum, whose own blocks it meets. Both read their declarations.
    let unrestricted = [
        Declared {
            name: "restricted-side",
            block: Some("restricted-side"),
            duplicates: DuplicatePolicy::Unique,
        },
        Declared {
            name: "anything",
            block: None,
            duplicates: DuplicatePolicy::Unique,
        },
    ];
    let run = run_declared(&unrestricted, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "an unrestricted stratum shares every block, including its own neighbour's",
        &run,
        &[
            (
                &unrestricted[0],
                unlicensed_depth(),
                DepthCause::Declaration,
            ),
            (
                &unrestricted[1],
                unlicensed_depth(),
                DepthCause::Declaration,
            ),
        ],
    );
}

/// **N1.** Pairwise-disjoint blocks, every stratum `Unique`: every stratum is
/// licensed the request's own bound, and the narrowing changes no answer.
#[test]
fn pairwise_disjoint_unique_strata_are_each_licensed_the_requests_bound() {
    let dataset = common::empty_dataset();
    let run = run_declared(&N1, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "three pairwise-disjoint unique strata",
        &run,
        &[
            (&N1[0], licensed_depth(), DepthCause::LicensedPrefix),
            (&N1[1], licensed_depth(), DepthCause::LicensedPrefix),
            (&N1[2], licensed_depth(), DepthCause::LicensedPrefix),
        ],
    );
    assert_narrowing_changed_no_answer("N1", &N1, &dataset);
}

/// **N2.** A single stratum restricting no block is licensed anyway: there is no
/// second stratum for it to share a candidate with, so the premise disjointness
/// supplies is what "one stratum" already means.
#[test]
fn a_single_unrestricted_unique_stratum_is_licensed_the_requests_bound() {
    let dataset = common::empty_dataset();
    let run = run_declared(&N2, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "one unrestricted unique stratum",
        &run,
        &[(&N2[0], licensed_depth(), DepthCause::LicensedPrefix)],
    );
    assert_narrowing_changed_no_answer("N2", &N2, &dataset);
}

/// **N3.** A stratum whose block no other stratum names narrows, even though two
/// *other* strata of the same request share a block with each other.
///
/// The intersection those two make is an intersection about *their* candidates. It
/// cannot let either of them name one of `apart`'s, because a candidate lies in
/// exactly one block and `apart`'s is neither of theirs — so `apart`'s fused
/// scores are still single terms and its own `Unique` prefix still holds every row
/// of the answer it was going to supply.
#[test]
fn a_stratum_disjoint_from_a_sharing_pair_narrows_while_the_pair_does_not() {
    let dataset = common::empty_dataset();
    let run = run_declared(&N3, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "one disjoint stratum beside a sharing pair",
        &run,
        &[
            (&N3[0], licensed_depth(), DepthCause::LicensedPrefix),
            (&N3[1], unlicensed_depth(), DepthCause::Declaration),
            (&N3[2], unlicensed_depth(), DepthCause::Declaration),
        ],
    );
    assert_narrowing_changed_no_answer("N3", &N3, &dataset);
}

/// **N4.** A `Unique` stratum narrows beside an `Allowed` one whose blocks it does
/// not meet.
///
/// The `Allowed` declaration says that stream's rows are not its candidates, which
/// is a fact about the candidates *it* names. It names none of `unique-side`'s —
/// their blocks do not meet — so it cannot cost `unique-side` a candidate out of
/// its own five-row prefix. It costs itself its own prefix, and nothing else.
#[test]
fn a_unique_stratum_narrows_beside_a_disjoint_allowed_one() {
    let dataset = common::empty_dataset();
    let run = run_declared(&N4, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "a unique stratum beside a disjoint allowed one",
        &run,
        &[
            (&N4[0], licensed_depth(), DepthCause::LicensedPrefix),
            (&N4[1], unlicensed_depth(), DepthCause::Declaration),
        ],
    );
    assert_narrowing_changed_no_answer("N4", &N4, &dataset);
}

/// **Anti-vacuity for the whole section.** A single `Allowed` stratum, alone in its
/// request, is still not licensed — so `Allowed` is read as a property of the
/// stratum that declares it and not merely ignored.
///
/// Without this, every assertion above would pass for a build that had dropped the
/// duplicate-policy premise altogether.
#[test]
fn a_lone_allowed_stratum_is_not_licensed_by_being_alone() {
    let dataset = common::empty_dataset();
    let lone = [Declared {
        name: "lone-allowed",
        block: Some("lone-allowed"),
        duplicates: DuplicatePolicy::Allowed,
    }];
    let run = run_declared(&lone, ReadBound::Bounded(TOP_K), &dataset);
    assert_depths(
        "one allowed stratum, alone",
        &run,
        &[(&lone[0], unlicensed_depth(), DepthCause::Declaration)],
    );
}

// ---------------------------------------------------------------------------
// 1d. Instance optimality, against a brute-forced certificate.
// ---------------------------------------------------------------------------

/// How far past the certificate the engine's total read may sit.
///
/// **What it bounds:** the ratio of the *total* ranks the engine pulled across
/// every stream of a generated instance to the total depth of that instance's
/// certificate — the shallowest per-stream prefix from which the answer is
/// entailed. It is a per-instance bound, asserted on each instance below, not an
/// average over them.
///
/// The certificate is computed by an oracle that already knows where every
/// stream ends. The engine knows neither, so it pays for two things the
/// certificate does not: one unmerged head per open stream, which is the only
/// thing that makes the threshold fall at all, and — where a candidate is named
/// by one stream but shares its block with others — a threshold summed over
/// every sharer, which its single contribution has to outlast.
///
/// Six is the pinned value because the worst instance below reaches 16 ranks
/// against a certificate of 3, which is 5⅓. The gap is concentrated in
/// `one_block_disjoint_items`, where finality — not the threshold — forbids the
/// stop: exactly the configuration the four-hundred-row control pins at full
/// scale. Every other instance sits at or below 2.
const CERTIFICATE_FACTOR: u64 = 6;

/// One stratum of a generated instance.
struct CertStratum {
    name: &'static str,
    /// The blocks this stream declares. Every item it holds lies in one of them.
    tags: Vec<&'static str>,
    weight: Fixed,
    /// Its items, in rank order.
    items: Vec<String>,
}

/// A generated instance: a handful of short streams and a bound.
struct Instance {
    name: &'static str,
    strata: Vec<CertStratum>,
    top_k: TopK,
}

const BLOCK_DOCS: &str = "http://example.org/domain/documents";
const BLOCK_PEOPLE: &str = "http://example.org/domain/people";

/// The block an item really lies in, read off the item itself — which is what
/// the partition axiom says a block is.
fn block_of(item: &str) -> &'static str {
    if item.contains("/doc/") {
        BLOCK_DOCS
    } else {
        BLOCK_PEOPLE
    }
}

/// `count` items under `prefix`, zero padded so lexical order is rank order.
fn items(prefix: &str, first: u64, count: u64) -> Vec<String> {
    (first..first + count)
        .map(|index| format!("http://example.org/{prefix}/{index:06}"))
        .collect()
}

/// A scripted stream for the certificate oracle: a prefix of one stratum's
/// items, ending in whatever receipt the caller states.
struct ScriptedStream {
    rows: VecDeque<RankedRow<Term>>,
    receipt: ProducerReceipt,
    contract: StreamContract,
}

// The trait's methods are `async`; these bodies are synchronous because the rows
// are pre-scripted. The keyword is required to implement the trait.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        Ok(self.rows.pop_front())
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    /// The certificate oracle's streams declare no exclusion basis, so being
    /// asked for a verdict is the disagreement
    /// [`ProtocolError::ExclusionUnavailable`] names rather than a question
    /// they could answer. The oracle's whole question is what a fusion could
    /// conclude from exactly these rows, and a verdict is not one of them.
    async fn exclusion(&mut self, _candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        Err(ProtocolError::ExclusionUnavailable)
    }

    fn plan_id(&self) -> Option<PlanId> {
        None
    }

    fn attestation(&self) -> PfAttestation {
        PfAttestation::UNDECLARED
    }
}

fn cert_stratum_iri(name: &str) -> Iri {
    iri(&ex(&format!("stratum/{name}")))
}

fn cert_profile(instance: &Instance) -> FusionProfile {
    let weights = instance
        .strata
        .iter()
        .map(|entry| (cert_stratum_iri(entry.name), entry.weight))
        .collect();
    FusionProfile::with_decay(weights, decay()).expect("the generated profile is valid")
}

/// The instance's streams, each cut to `depths[index]` rows and declaring that
/// it ended there.
///
/// An oracle's prefix ends because the oracle chose to cut it, and a cut prefix
/// that claimed a deeper stream would be a different question. `Exhausted` is
/// therefore the honest receipt: the certificate asks what a fusion could
/// conclude from exactly these rows.
fn cert_streams(instance: &Instance, depths: &[usize]) -> Vec<(Iri, ScriptedStream)> {
    instance
        .strata
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let rows: VecDeque<RankedRow<Term>> = entry
                .items
                .iter()
                .take(depths[index])
                .enumerate()
                .map(|(position, item)| {
                    let rank = u64::try_from(position + 1).expect("generated ranks fit");
                    RankedRow::new(
                        rank,
                        contribution_under(decay(), entry.weight, rank)
                            .expect("generated contributions fit"),
                        Term::new(item),
                        RowBlock::Declared(
                            DomainTag::parse(block_of(item))
                                .expect("generated block tags are valid IRIs"),
                        ),
                    )
                })
                .collect();
            let emitted = u64::try_from(rows.len()).expect("generated row counts fit");
            (
                cert_stratum_iri(entry.name),
                ScriptedStream {
                    rows,
                    receipt: ProducerReceipt::Exhausted {
                        rows_emitted: emitted,
                    },
                    contract: StreamContract::new(
                        DuplicatePolicy::Unique,
                        RankFidelity::EXACT,
                        CandidateDomains::within(entry.tags.iter().map(|tag| {
                            DomainTag::parse(tag).expect("generated block tags are valid IRIs")
                        })),
                        ExclusionBasis::Unavailable,
                    ),
                },
            )
        })
        .collect()
}

fn cert_fuse(instance: &Instance, depths: &[usize]) -> FusionResult<Term> {
    block_on(fuse::<ScriptedStream, Term>(
        cert_streams(instance, depths),
        &cert_profile(instance),
        instance.top_k,
    ))
    .expect("the generated streams obey the protocol")
}

/// The answer a fusion returned, as the thing a certificate has to reproduce:
/// the rows, their scores, and their order.
fn answer_of(result: &FusionResult<Term>) -> Vec<(String, Fixed)> {
    result
        .rows
        .iter()
        .map(|row| (row.entity.as_str().to_owned(), row.score))
        .collect()
}

/// Every depth vector over the instance's streams, from all-zero upward.
fn depth_vectors(lengths: &[usize]) -> Vec<Vec<usize>> {
    let mut vectors = vec![Vec::new()];
    for length in lengths {
        let mut next = Vec::new();
        for prefix in &vectors {
            for depth in 0..=*length {
                let mut extended = prefix.clone();
                extended.push(depth);
                next.push(extended);
            }
        }
        vectors = next;
    }
    vectors
}

/// The certificate: the cheapest per-stream prefix from which this instance's
/// answer is entailed.
///
/// Brute force over every prefix vector, keeping those whose fusion reproduces
/// the full answer exactly — same rows, same scores, same order — and among
/// them the one with the smallest total depth. Ties are settled by the shallower
/// deepest stream, then lexically, so the answer is one vector and not an
/// arbitrary member of a set.
fn certificate(instance: &Instance) -> Vec<usize> {
    let lengths: Vec<usize> = instance
        .strata
        .iter()
        .map(|entry| entry.items.len())
        .collect();
    let full = cert_fuse(instance, &lengths);
    let target = answer_of(&full);
    let mut best: Option<Vec<usize>> = None;
    for candidate in depth_vectors(&lengths) {
        if answer_of(&cert_fuse(instance, &candidate)) != target {
            continue;
        }
        let key = |vector: &[usize]| {
            (
                vector.iter().sum::<usize>(),
                vector.iter().copied().max().unwrap_or(0),
                vector.to_vec(),
            )
        };
        match &best {
            Some(current) if key(current) <= key(&candidate) => {}
            _ => best = Some(candidate),
        }
    }
    best.expect("the full prefix always entails the answer it produced")
}

/// The instances the certificate is brute-forced over: two and three strata, at
/// most a dozen items each, across the overlap patterns a declaration can take.
fn instances() -> Vec<Instance> {
    vec![
        // Disjoint blocks, disjoint items: the shape the planner's merge
        // argument covers.
        Instance {
            name: "two_disjoint_blocks",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 8),
                },
            ],
            top_k: TopK::new(3),
        },
        // One block, and the two streams rank the same items in opposite
        // orders: total overlap, maximal disagreement.
        Instance {
            name: "one_block_reversed_agreement",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8).into_iter().rev().collect(),
                },
            ],
            top_k: TopK::new(3),
        },
        // One block, partial overlap: four items in common, four in neither.
        Instance {
            name: "one_block_partial_overlap",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 5, 8),
                },
            ],
            top_k: TopK::new(4),
        },
        // Two disjoint blocks and a third stream that cross-cuts both: the
        // quality-prior shape, where no stream may be skipped by anyone.
        Instance {
            name: "cross_cutting_prior",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 6),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 6),
                },
                CertStratum {
                    name: "prior",
                    tags: vec![BLOCK_DOCS, BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 3)
                        .into_iter()
                        .zip(items("person", 1, 3))
                        .flat_map(|(doc, person)| [doc, person])
                        .collect(),
                },
            ],
            top_k: TopK::new(3),
        },
        // Disjoint blocks of very different lengths, under a bound that reaches
        // past the shorter one.
        Instance {
            name: "uneven_disjoint_blocks",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 4),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 12),
                },
            ],
            top_k: TopK::new(5),
        },
        // One block, disjoint items: the configuration finality forbids stopping
        // early in, carried down to a size a brute force can certify.
        Instance {
            name: "one_block_disjoint_items",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 100, 8),
                },
            ],
            top_k: TopK::new(3),
        },
    ]
}

/// The engine reads within a pinned constant factor of the certificate on every
/// generated instance.
#[test]
fn the_engine_reads_within_a_pinned_factor_of_the_certificate() {
    for instance in instances() {
        let lengths: Vec<usize> = instance
            .strata
            .iter()
            .map(|entry| entry.items.len())
            .collect();
        let run = cert_fuse(&instance, &lengths);
        let engine: u64 = instance
            .strata
            .iter()
            .map(|entry| {
                run.trailer
                    .resolution
                    .get(&cert_stratum_iri(entry.name))
                    .expect("the generated profile weights every generated stratum")
                    .ranks_pulled
            })
            .sum();

        let cert = certificate(&instance);
        let cert_total: u64 = cert
            .iter()
            .map(|depth| u64::try_from(*depth).expect("generated depths fit"))
            .sum();

        // A certificate of zero would make the ratio meaningless, and an answer
        // of `k` rows cannot be entailed by no rows at all.
        assert!(
            cert_total > 0,
            "{}: the certificate reads nothing, which cannot entail an answer",
            instance.name
        );
        assert!(
            engine <= CERTIFICATE_FACTOR * cert_total,
            "{}: the engine pulled {engine} ranks against a certificate of \
             {cert_total} ({cert:?}), which is past the pinned factor of \
             {CERTIFICATE_FACTOR}",
            instance.name
        );

        // And the certificate really is a certificate: reading it reproduces the
        // answer, and every strictly shallower prefix does not.
        assert_eq!(
            answer_of(&cert_fuse(&instance, &cert)),
            answer_of(&run),
            "{}: the certificate must reproduce the answer it certifies",
            instance.name
        );
        for (index, depth) in cert.iter().enumerate() {
            if *depth == 0 {
                continue;
            }
            let mut shallower = cert.clone();
            shallower[index] -= 1;
            assert_ne!(
                answer_of(&cert_fuse(&instance, &shallower)),
                answer_of(&run),
                "{}: {shallower:?} already entails the answer, so {cert:?} is \
                 not the certificate",
                instance.name
            );
        }
    }
}

/// The three configurations' measured table, in one place, so a failure reads
/// as the row that moved rather than as five separate assertions.
///
/// Every number here is also asserted individually above; this exists because
/// the table is the evidence, and evidence spread across three tests is evidence
/// nobody reads together.
#[test]
fn the_measured_table_is_what_it_was() {
    let dataset = common::empty_dataset();
    let rendered: Vec<String> = [
        DISTINCT_BLOCKS_DISJOINT_RESULTS,
        SHARED_BLOCK_INTERSECTING_RESULTS,
        SHARED_BLOCK_DISJOINT_RESULTS,
    ]
    .into_iter()
    .map(|config| render(config, &measure(config, &dataset)))
    .collect();

    assert_eq!(
        rendered,
        vec![
            format!(
                "distinct_blocks_disjoint_results: depth=5 limit=6 materialised=6 \
                 ranks_pulled=4 status={:?}",
                ceiling_at(4)
            ),
            format!(
                "shared_block_intersecting_results: depth=400 limit=401 \
                 materialised=8 ranks_pulled=6 status={:?}",
                ceiling_at(6)
            ),
            format!(
                "shared_block_disjoint_results: depth=400 limit=401 \
                 materialised=400 ranks_pulled=400 status={:?}",
                ProducerStatus::Exhausted { rows_emitted: ROWS }
            ),
        ],
        "the measured table moved"
    );

    // Ordered set equality is not what is being claimed — the three rows are
    // three distinct measurements, and a harness that collapsed two of them
    // would be measuring less than it says.
    let distinct: BTreeSet<&String> = rendered.iter().collect();
    assert_eq!(
        distinct.len(),
        3,
        "three configurations must measure three different things"
    );
}

/// The finality licence has one spelling, and the promise it is *not* keeps its
/// own.
///
/// Three questions — whether a candidate is final, what its upper bound is, and
/// what its score interval is — are one question about whether a stream may
/// still name it. The engine's own comments say the licence must be taken in
/// all of them or none, because a candidate certified under one reading and
/// bounded under another is a score whose interval describes a different read
/// than the certification did. A licence spelled once per site is three chances
/// to disagree, and the disagreement is silent: each site still compiles, and
/// each still passes every test that exercises only the other two.
///
/// `pull` reads the same declaration for the opposite purpose — refusing a
/// stream that names a candidate its own declaration put out of reach — and
/// that enforcement must not inherit the licence. Merging them would report a
/// stream contradicting an *observation* as having broken its *declaration*,
/// blaming the wrong promise and naming a witness that never made it. So the
/// split is pinned here, in both directions, rather than left to a comment.
#[test]
fn the_finality_licence_is_spelled_once_and_enforcement_keeps_its_own() {
    const SOURCE: &str = include_str!("../src/fusion_stream.rs");

    assert_eq!(
        SOURCE.matches("fn may_still_name").count(),
        1,
        "the licence is one function, so there is one place for it to grow"
    );
    assert_eq!(
        SOURCE.matches("self.may_still_name(").count(),
        5,
        "finality, the upper bound and the score interval take the licence, and \
         the exclusion-lookup phase takes it twice — once to choose which \
         streams are blocking a candidate and once at the moment of asking, \
         because an answer recorded earlier in the same phase can have settled \
         the candidate in between. A site that stopped taking it would be \
         reading a different question"
    );
    assert_eq!(
        SOURCE.matches("self.could_name(").count(),
        4,
        "the raw declaration predicate is read exactly four times: once by the \
         licence, twice by the two enforcement sites in `pull` that refuse a \
         broken domain promise, and once by the verdict check that refuses a \
         producer calling a candidate possible when its own declared domains \
         cannot reach it. All three of the latter are ENFORCEMENT — they blame a \
         declaration — which is exactly why none of them may take the licence: a \
         licence narrowed by an observation would report a broken observation as \
         a broken declaration. A fifth reading is either a licence that bypassed \
         `may_still_name` or an enforcement that should have"
    );
    assert_eq!(
        SOURCE.matches("state.excluded.contains(").count(),
        2,
        "the observational half of the licence is read once by `may_still_name` \
         and once by the frontier arm of `pull` that refuses a stream naming a \
         candidate it excluded. The certified-candidate arm reads the emitted \
         record's own copy instead, because a certified candidate no longer has \
         a `CandidateState` to read"
    );
}

// ---------------------------------------------------------------------------
// 4. The exclusion lookup: what asking buys, what it costs, and what it may not
//    change.
// ---------------------------------------------------------------------------
//
// `shared_block_disjoint_results` above is the permanent control, and the reason
// it drains is stated there: a candidate the left stream named is not *final*
// while the right stream — which declares the same block, and so could still
// name it — is open. Nothing declarative can settle that. Both declarations are
// true, and the thing that is false is a fact about the corpus that no promise
// about blocks can express: the right stream will never name that candidate.
//
// An exclusion lookup converts "has not named it yet" into "will never name it"
// by observation. The three tests below measure what that is worth, what it
// costs, and what it does not touch.

/// The value of the whole helper table for one configuration, rendered for a
/// failure to read beside the numbers the rest of this file pins.
fn render_with_lookups(config: Configuration, measured: &Measured) -> String {
    format!(
        "{base} fused_lookups={fused:?} served={served:?} found={found:?} attempts={attempts:?}",
        base = render(config, measured),
        fused = measured.fused_lookups,
        served = measured.served_lookups,
        found = measured.found_lookups,
        attempts = measured.read_attempts,
    )
}

/// The total over both strata of one per-stratum table.
fn total(field: &BTreeMap<Iri, u64>) -> u64 {
    field.values().sum()
}

/// **The control's finality problem, solved by observation.**
///
/// The configuration is [`SHARED_BLOCK_DISJOINT_RESULTS`] with one bit changed:
/// both producers declare [`ExclusionBasis::Membership`]. Same block, same four
/// hundred rows, same disjoint candidates, same weights, same bound.
///
/// The control reads four hundred ranks because nothing certifies until both
/// streams run out. Here a candidate the left stream named is looked up against
/// the right stream once, the right stream answers that its universe does not
/// contain it, and the candidate becomes final — so the read stops where the
/// **threshold** licenses it to, which is the gate the control never reached.
///
/// That rank is asserted against this file's own derivation rather than a
/// literal: `crossing_rank_at` is the pure function with no executor in it, and
/// the deepest rank the answer actually contains is read off the answer. Two
/// streams share the block, so the threshold is summed over both and a
/// single-named candidate has to outlast twice its own weight — the expensive
/// crossing, and still a sixth of the drain.
#[test]
fn membership_lookups_stop_the_drain_where_the_threshold_licenses() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset);
    let control = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &measured);

    assert_eq!(measured.rows.len(), 5, "the answer is still five rows");

    // The derivation, with no executor in it, and the rank it predicts. The
    // deepest rank is read off the answer rather than written down, exactly as
    // the three configurations above read theirs.
    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 3,
        "the fifth row of an alternating answer — {report}"
    );
    let licensed = crossing_rank_at(decay(), &[Fixed::ONE], deepest, &[Fixed::ONE, Fixed::ONE]);
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        licensed,
        "with finality settled by observation, the threshold is what stops the \
         read — and it stops it exactly where the derivation says — {report}"
    );

    // And it is strictly less than the control's drain, which is the whole
    // point. Stated against the control's own measured number rather than
    // against the literal four hundred, so a change that moved both would still
    // have to move this comparison.
    let drained = control.both("the ranks pulled", &control.ranks_pulled);
    assert_eq!(drained, ROWS, "the control still drains — {report}");
    assert!(
        measured.both("the ranks pulled", &measured.ranks_pulled) < drained,
        "the licensed read must be strictly shallower than the drain it \
         replaces — {report}"
    );

    // The lookups really happened, at both ends of the seam. The engine's count
    // is the answering run's; the producer's is cumulative over every run this
    // search took, so it can only be larger.
    for stratum in strata() {
        let fused = measured.fused_lookups[&stratum];
        let served = measured.served_lookups[&stratum];
        assert!(
            fused > 0,
            "stratum {stratum} was never asked, so nothing here was measured — {report}"
        );
        assert!(
            served >= fused,
            "the producer served fewer lookups than the engine says it made, so \
             one of the two numbers is not about this seam — {report}"
        );
    }

    // And the answers were real. These producers hold disjoint candidate sets,
    // so an honest membership answer never finds a foreign candidate — which is
    // exactly what a mock that said `Excluded` to everything would also show. The
    // all-`Possible` twin below is the other half of that pair: there every
    // lookup finds, so the two together show the fixture is answering from its
    // own universe rather than from a constant.
    assert_eq!(
        total(&measured.found_lookups),
        0,
        "these producers share no candidate, so no lookup may find one — {report}"
    );

    // What the lookups did NOT cost: a single extra ranked row. The only
    // ranked read is the one the plan asked for — its proven stopping rank and
    // the probe row — so the point queries added nothing to the sorted access
    // this file measures, which is what makes a lookup a lookup.
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![stopping_read_limit()],
        "an exclusion lookup is a point query and must add no ranked row — {report}"
    );
}

/// The rank [`PlannedResolution::stopping_rank`] proves this file's two-stratum
/// configurations cannot pull past, where the two share whatever they admit and
/// both answer lookups — computed by the library's own derivation, never
/// restated.
///
/// The value is pinned once, as a literal with its arithmetic, in
/// [`the_lookups_answer_the_drained_configuration_in_one_read_at_the_proven_stopping_rank`];
/// every other use reads it from here.
fn stopping_read_depth() -> u64 {
    let k = u64::try_from(TOP_K.get()).expect("the fixture bound fits a u64");
    crossing_rank_at(decay(), &[Fixed::ONE], k, &[Fixed::ONE, Fixed::ONE])
}

/// The rows a read taken to [`stopping_read_depth`] materialises: that depth,
/// and the probe row every emitted bound carries one past it.
fn stopping_read_limit() -> u64 {
    stopping_read_depth() + 1
}

/// Plan and compile one configuration exactly as [`search`] does, for the
/// depths the bundle derives before anything is read.
fn compiled_for(config: Configuration) -> CompiledRetrieval {
    let (registry, _) = configured_registry(config);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    compile(&planned, &env).expect("a fresh plan is admitted")
}

/// The speculative depth the bundle derives for each of its units, which the
/// two symmetric strata of this file must agree on.
fn speculative_read_depths(config: Configuration) -> u64 {
    let compiled = compiled_for(config);
    let depths: BTreeSet<u32> = compiled
        .units
        .iter()
        .map(|unit| compiled.speculative_read_depth(unit))
        .collect();
    assert_eq!(
        depths.len(),
        1,
        "{name}: the two symmetric strata disagree on their speculative depth: {depths:?}",
        name = config.name
    );
    u64::from(*depths.first().expect("the fixture compiles two units"))
}

/// **The drained configuration, answered by one read.**
///
/// [`SHARED_BLOCK_DISJOINT_RESULTS`] reads every row there is because nothing
/// is final while a stream sharing its block is open. With the producers
/// answering lookups, finality settles by observation — and the plan knows,
/// before a row is read, that it will: every stratum sharing a block answers
/// lookups. So the speculative read is taken to the rank
/// [`PlannedResolution::stopping_rank`] proves the fusion cannot pull past, the
/// fusion certifies inside it, and that read is the answer.
///
/// # The closed form, per stratum
///
/// `k = 5`, unit weights, reciprocal rank with smoothing constant `60` at the
/// fixed-point scale `10¹²`, and two streams admitting the shared block:
///
/// * the fifth row is worth at least one stream's fifth contribution,
///   `c(5) = ⌊10¹²/65⌋ = 15 384 615 384`, because each stream's first five rows
///   are five distinct candidates;
/// * a pull from head rank `r` happens only while the threshold — at most both
///   heads, each at most `c(r)` — is at least that, and `2·c(r) < c(5)` first
///   holds at `r = 71`. At `r = 70` it is a tie, `2·⌊10¹²/130⌋ = 15 384 615 384`,
///   and the engine's strict comparison does not pass a tie;
/// * so each stratum is read to depth **71**, and the read carries its probe row:
///   **72** rows materialised per stratum, in **one** read.
///
/// Against the control's `8 + 400 = 408` per stratum. Fusion itself stops at
/// sixty-six — the fifth row really sits at rank three, and the proof can only
/// assume rank five — so the read is five ranks and a probe row past what fusion
/// used, which is the price of a depth decided with no row read.
#[test]
fn the_lookups_answer_the_drained_configuration_in_one_read_at_the_proven_stopping_rank() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset);
    let control = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = format!(
        "declared={} control={}",
        render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &measured),
        render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS, &control),
    );

    // The closed form, as the arithmetic: the tie one rank shallower, and the
    // crossing at the rank the depth is.
    assert_eq!(
        sum_at(decay(), &[Fixed::ONE], 5),
        sum_at(decay(), &[Fixed::ONE, Fixed::ONE], 70),
        "at rank seventy both heads together tie one stream's fifth contribution"
    );
    assert!(
        sum_at(decay(), &[Fixed::ONE], 5) > sum_at(decay(), &[Fixed::ONE, Fixed::ONE], 71),
        "and at seventy-one they fall strictly below it"
    );
    assert_eq!(
        stopping_read_depth(),
        71,
        "the library's derivation lands on the closed form"
    );
    assert_eq!(
        speculative_read_depths(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS),
        71,
        "and the bundle asks each stratum for exactly that depth — {report}"
    );

    // One read, of exactly that depth and its probe row, at both ends of the
    // seam: the producer's own count and the trailer's figure.
    assert_eq!(
        measured.read_attempts,
        ReadAttempts::Once,
        "the fusion certified inside the deepened read, so it IS the answer — {report}"
    );
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![72],
        "one read per stratum: the proven stopping rank and its probe row — {report}"
    );
    assert_eq!(
        measured.both("the reported rows", &measured.reported_materialised),
        Some(72),
        "and the trailer bills that read and nothing else — {report}"
    );
    assert_eq!(
        stopping_read_limit(),
        72,
        "the literal above is the closed form, not a measurement copied back"
    );

    // The read the fusion used, inside the read it was given. The proof is an
    // upper bound on this, and the gap is the rank-five assumption.
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        66,
        "fusion stops where the fifth row, at rank three, crosses — {report}"
    );
    assert!(
        measured.both("the ranks pulled", &measured.ranks_pulled) <= stopping_read_depth(),
        "a fusion that pulled past the proven stopping rank would falsify the \
         derivation the read depth rests on — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ceiling_at(66),
        "the fusion, not the read, stopped both streams — {report}"
    );

    // The permanent control stays on record beside it: the same rows with no
    // basis declared still pay the discarded frontier and the whole drain.
    assert_eq!(
        speculative_read_depths(SHARED_BLOCK_DISJOINT_RESULTS),
        speculative_read_depth(),
        "with no basis declared, nothing licenses a deeper read than the \
         frontier — {report}"
    );
    assert_eq!(control.read_attempts, ReadAttempts::Twice, "{report}");
    assert_eq!(
        control.both("the reads", &control.reads),
        vec![speculative_limit(), ROWS],
        "the control's bill is unchanged: the frontier, discarded, then all four \
         hundred — {report}"
    );
    assert_eq!(
        control.both("the reported rows", &control.reported_materialised),
        Some(speculative_limit() + ROWS),
        "{report}"
    );

    // And the same answer. Against the control, the rows, scores and order: its
    // rows were certified after both streams ran out, so their threshold
    // witnesses are zero, and a witness is a fact about when a row certified.
    assert_eq!(
        measured.answer(),
        control.answer(),
        "the declared basis changed what the answer cost and nothing about the \
         answer — {report}"
    );
    // Against the planned read of the same declarations, byte for byte —
    // contributions, intervals and threshold witnesses included — because the
    // fusion over the deepened read is the fusion over the full one, up to the
    // rank it stopped at.
    let full = measure_at_planned_depth(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset);
    assert_eq!(
        full.both("the reads", &full.reads),
        vec![ROWS],
        "the reference must read in full — {report}"
    );
    assert_eq!(
        measured.rows, full.rows,
        "the deepened read answers exactly as the planned read does — {report}"
    );
}

/// **Undeclared domains: the same deepened read, and the identical answer.**
///
/// Both producers declare no block at all and answer lookups. Two unrestricted
/// streams admit every candidate between them, so the threshold sums the same
/// two heads a shared block does and the plan proves the same stopping rank —
/// and the answer, read once at that depth, is byte for byte the answer the
/// planned read gives.
#[test]
fn undeclared_domains_answering_lookups_read_once_and_answer_identically() {
    let dataset = common::empty_dataset();
    let config = UNDECLARED_BLOCKS_DISJOINT_RESULTS_WITH_LOOKUPS;
    let measured = measure(config, &dataset);
    let full = measure_at_planned_depth(config, &dataset);
    let report = format!(
        "searched={} planned={}",
        render_with_lookups(config, &measured),
        render_with_lookups(config, &full),
    );

    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        400,
        "two unrestricted strata license no prefix — {report}"
    );
    assert_eq!(
        speculative_read_depths(config),
        stopping_read_depth(),
        "two unrestricted heads are the same two heads a shared block sums — {report}"
    );
    assert_eq!(measured.read_attempts, ReadAttempts::Once, "{report}");
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![stopping_read_limit()],
        "{report}"
    );
    assert!(
        total(&measured.fused_lookups) > 0,
        "finality settled by lookups, not by a stream running out — {report}"
    );

    // The observing oracle: the reference read every row the plan allowed, so
    // an answer that agrees with it agrees because the deepened read held.
    assert_eq!(
        full.both("the reads", &full.reads),
        vec![ROWS],
        "the reference must read in full — {report}"
    );
    assert_eq!(
        measured.rows, full.rows,
        "the answer read once at the proven stopping rank is the answer the \
         planned read gives, byte for byte — {report}"
    );
    assert_eq!(
        measured.rows,
        measure(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset).rows,
        "and it is the answer the declared-block twin gives — {report}"
    );
}

/// **The prediction undershoots, and the fallback answers.**
///
/// Every declaration is the one that licenses the deepened read — a shared
/// block, both producers answering lookups — so the plan asks for the same
/// seventy-one ranks. The rows break the premise underneath it: the two
/// producers hold the same candidates in opposite orders, every lookup answers
/// `Possible`, and no candidate is final until the other stream reaches it. So
/// the fusion reads past the deepened depth, the stream reports the cut, and
/// the attempt is discarded for the planned read.
///
/// Asserted as the fallback *firing*, not merely as the answer being right: two
/// reads, the deepened one and the full one, in that order. A fallback that
/// never fired in this file would be indistinguishable from dead code.
#[test]
fn a_stopping_rank_the_rows_overrun_falls_back_to_the_planned_read() {
    let dataset = common::empty_dataset();
    let config = SHARED_BLOCK_REVERSED_RESULTS_WITH_LOOKUPS;
    let measured = measure(config, &dataset);
    let full = measure_at_planned_depth(config, &dataset);
    let report = format!(
        "searched={} planned={}",
        render_with_lookups(config, &measured),
        render_with_lookups(config, &full),
    );

    // The premise the plan can see is the licensed one.
    assert_eq!(
        speculative_read_depths(config),
        stopping_read_depth(),
        "the declarations are the deepened read's own — {report}"
    );
    // And the rows break the one it cannot: every lookup found its candidate.
    assert!(
        total(&measured.served_lookups) > 0,
        "the lookups were asked — {report}"
    );
    assert_eq!(
        total(&measured.found_lookups),
        total(&measured.served_lookups),
        "every candidate is held by both producers, so every lookup answers \
         Possible — {report}"
    );

    assert_eq!(
        measured.read_attempts,
        ReadAttempts::Twice,
        "the deepened read was cut, so the fallback fired — {report}"
    );
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![stopping_read_limit(), ROWS],
        "the deepened read, discarded, then the planned one — {report}"
    );
    assert_eq!(
        measured.both("the reported rows", &measured.reported_materialised),
        Some(stopping_read_limit() + ROWS),
        "and the bill counts both — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        ROWS,
        "no candidate is final until the other stream names it, so both drain — \
         {report}"
    );

    // The answer is the planned read's, row for row.
    assert_eq!(
        measured.rows, full.rows,
        "the fallback returns exactly the answer the planned read gives — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        full.both("the terminal status", &full.status),
        "{report}"
    );
}

/// **The differential, in one test so it cannot pass vacuously.**
///
/// Two registries differing in exactly one thing: whether the producers declare
/// an exclusion basis. Three claims, and all three have to hold of the same pair
/// of runs — the answer is identical, the declaring run really asked, and the
/// declaring run really read less. Split across three tests, the first could
/// pass over a mechanism that never ran and the third could pass over an answer
/// that changed.
#[test]
fn the_declared_basis_alone_buys_a_shallower_read_of_the_same_answer() {
    let dataset = common::empty_dataset();
    let silent = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let declaring = measure(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset);
    let report = format!(
        "silent={} declaring={}",
        render(SHARED_BLOCK_DISJOINT_RESULTS, &silent),
        render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &declaring),
    );

    // (1) The answer is the same one — same entities, same scores, same order.
    assert_eq!(
        declaring.answer(),
        silent.answer(),
        "the two registries hold the identical candidates and must answer \
         identically; only the cost of the answer is in question — {report}"
    );

    // (2) The declaring run really used the mechanism.
    assert!(
        total(&declaring.fused_lookups) > 0,
        "the declaring run made no lookup, so nothing distinguishes it — {report}"
    );
    assert_eq!(
        total(&silent.fused_lookups),
        0,
        "and the silent run made none, which is what makes the pair a \
         difference of one bit — {report}"
    );

    // (3) And it read strictly less, in at least one stratum.
    let narrowed = strata()
        .into_iter()
        .filter(|stratum| declaring.ranks_pulled[stratum] < silent.ranks_pulled[stratum])
        .count();
    assert!(
        narrowed > 0,
        "the declaring run read at least as deep as the silent one in every \
         stratum, so the lookups bought nothing — {report}"
    );
}

/// **The budget bites: a candidate that cannot win is never looked up.**
///
/// One block, both producers naming the identical candidates, and both
/// answering lookups — so **every** verdict is `Possible` and no verdict ever
/// narrows anything. What is left is the price of asking, and this is where it
/// is pinned.
///
/// The bound is derived, not configured. A candidate enters the frontier only
/// when some row names it, so at most `Σ ranks_pulled` candidates ever exist; a
/// candidate is asked only of the streams that have not named it, so at most
/// `strata - 1` per candidate; and each pair is asked **once** for the whole
/// read, because an answer is a standing fact about a producer's universe rather
/// than something that changes between passes. That last clause is the one worth
/// a test: without it the count would be the pairs times the number of passes of
/// the row loop, which is quadratic in exactly the configuration this mechanism
/// exists to make cheap.
#[test]
fn a_candidate_that_cannot_win_is_never_looked_up() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &dataset);
    let control = measure(SHARED_BLOCK_INTERSECTING_RESULTS, &dataset);
    let report = render_with_lookups(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &measured);

    // Every lookup found its candidate, so every verdict was `Possible`. This is
    // the anti-vacuity half of the pair: the disjoint configuration above finds
    // nothing, this one finds everything, and a fixture answering from a
    // constant could not do both.
    assert_eq!(
        total(&measured.found_lookups),
        total(&measured.served_lookups),
        "both producers hold every candidate, so every lookup must find one — {report}"
    );
    assert!(
        total(&measured.served_lookups) > 0,
        "a configuration that asked nothing would satisfy the equality above \
         vacuously — {report}"
    );

    // Nothing narrowed, which is the control this measurement needs: the read is
    // the one the same configuration takes without lookups, rank for rank.
    assert_eq!(
        measured.ranks_pulled, control.ranks_pulled,
        "no verdict narrowed anything here, so the read must be unchanged — {report}"
    );
    assert_eq!(
        measured.answer(),
        control.answer(),
        "and so must the answer — {report}"
    );

    // The derived ceiling: one lookup per (frontier candidate, non-naming
    // stream) pair, for the whole read.
    let other_streams =
        u64::try_from(strata().len() - 1).expect("the fixture stratum count fits a u64");
    let pairs_ceiling = total(&measured.ranks_pulled) * other_streams;
    assert!(
        total(&measured.fused_lookups) <= pairs_ceiling,
        "the lookup count must be bounded by the pairs, not by the passes: \
         {} lookups against a ceiling of {pairs_ceiling} — {report}",
        total(&measured.fused_lookups)
    );

    // And the exact count, pinned so a change that loosened the threshold filter
    // has to move this literal in the same commit that earns it. Five, against a
    // pair ceiling of twelve: the candidates a lookup would not have changed the
    // fate of are the ones never asked about.
    assert_eq!(
        total(&measured.fused_lookups),
        5,
        "the measured price of asking, in a configuration where asking buys \
         nothing — {report}"
    );
    assert_eq!(
        pairs_ceiling, 12,
        "the ceiling this is measured against, stated so the comparison above \
         is not a comparison with an unbounded number — {report}"
    );

    // And what declaring a basis costs this configuration in ranked rows. Before
    // a row is read, it is indistinguishable from the drained configuration —
    // the same block, the same basis on both sides, the same premise — so the
    // plan deepens its read to the same proven stopping rank: one read of 72
    // rows per stratum where the undeclared twin's frontier read of 8 certifies.
    // The fusion needs six ranks either way; the difference is the price of a
    // depth decided with no row read, and it is pinned so it cannot move
    // unnoticed in either direction.
    assert_eq!(measured.read_attempts, ReadAttempts::Once, "{report}");
    assert_eq!(
        measured.both("the reads", &measured.reads),
        vec![stopping_read_limit()],
        "the deepened read, taken once — {report}"
    );
    assert_eq!(
        control.both("the reads", &control.reads),
        vec![speculative_limit()],
        "against the undeclared twin's frontier read — {report}"
    );
}

// ---------------------------------------------------------------------------
// 5. Keeping the cost visible: the counters, the prediction, and the text.
// ---------------------------------------------------------------------------
//
// Every number this file measures is a cost, and a cost nobody can read is a
// cost nobody pays attention to. The tests below are about the *reporting*
// surface rather than about the engine: that each counter really moves with the
// corpus rather than sitting at a constant, that the read-work figure counts the
// read a fallback throws away, that a host can learn the crossing its own
// declarations condemn it to before it runs anything, and that all of it reaches
// the text a caller reads.
//
// Every one of them is a **two-run** assertion. A single run's counter is
// satisfied by a counter that is always zero, always four hundred, or always
// whatever the fixture happens to produce; only a pair of runs over the same
// surface, differing in what the corpus costs, can tell a measurement from a
// constant.

/// The rows the fixture's own counters say a configuration's producer served,
/// across every read of the run — the producer's side of the seam, against the
/// trailer's.
fn served_rows(measured: &Measured, stratum: &Iri) -> u64 {
    measured.reads[stratum].iter().sum()
}

/// The read-work figure the trailer reports for `stratum`, with the absence
/// refused: every configuration in this file runs through `execute`, and a
/// stratum of such a run always has a read to count.
fn reported_rows(measured: &Measured, stratum: &Iri) -> u64 {
    measured.reported_materialised[stratum]
        .expect("a stratum the executor read reports the rows its read returned")
}

/// The weights the two producers of a configuration declare, as the threshold
/// derivation takes them.
///
/// Derived from the configuration's own two bits rather than read off the plan:
/// a stratum's threshold is summed over the streams whose declared blocks meet
/// its own, so two producers declaring one block share a threshold and two
/// declaring different ones do not. This is the test's independent derivation of
/// the set the planner surfaces, and the two are asserted equal.
fn declared_sharing(config: Configuration) -> Vec<Fixed> {
    match config.blocks {
        Blocks::Distinct => vec![Fixed::ONE],
        Blocks::Shared | Blocks::Undeclared => vec![Fixed::ONE, Fixed::ONE],
    }
}

/// The weights of the strata that named the deepest row of an answer, as the
/// threshold derivation takes them. Every stratum in this file carries unit
/// weight, so this is the count read off the answer.
fn naming_weights(measured: &Measured) -> Vec<Fixed> {
    vec![Fixed::ONE; measured.namers_of_deepest_row()]
}

/// **The two-run oracle for `ranks_pulled` and the read-work figure.**
///
/// Two configurations of the identical producers over the identical candidates,
/// differing only in what the two of them *declared* about where their
/// candidates lie. One answers inside the fourth rank; the other is condemned to
/// drain. If either counter reported the same number for both, it would not be
/// measuring the read — and every assertion in this file that rests on it would
/// be resting on a constant.
///
/// The read-work figure is checked from **both ends of the seam**: the trailer's
/// number against the fixture's own counter inside the producer, which counts
/// rows as the cursor hands them out and knows nothing about trailers. A figure
/// the engine reported from a value it had fabricated would agree with itself
/// and disagree with this.
#[test]
fn the_ranks_and_the_read_work_both_move_between_a_cheap_run_and_a_corpus_cost_run() {
    let dataset = common::empty_dataset();
    let cheap = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let costly = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = format!(
        "cheap={} costly={}",
        render(DISTINCT_BLOCKS_DISJOINT_RESULTS, &cheap),
        render(SHARED_BLOCK_DISJOINT_RESULTS, &costly),
    );

    // The premise: the two runs return the identical answer, so nothing below
    // is a difference in what was asked for.
    assert_eq!(
        cheap.answer(),
        costly.answer(),
        "the two configurations hold the identical candidates and must answer \
         identically; only the cost of the answer is in question — {report}"
    );

    for stratum in strata() {
        // (1) `ranks_pulled` moves, and neither end is zero — a counter stuck at
        //     zero would satisfy an inequality against a non-zero one only in
        //     one direction, and this pins both.
        let cheap_ranks = cheap.ranks_pulled[&stratum];
        let costly_ranks = costly.ranks_pulled[&stratum];
        assert!(
            cheap_ranks > 0 && costly_ranks > 0,
            "stratum {stratum}: a run that pulled no rank measured nothing — {report}"
        );
        assert!(
            cheap_ranks < costly_ranks,
            "stratum {stratum}: `ranks_pulled` did not move with the corpus \
             cost, so it is not measuring the read — {report}"
        );

        // (2) The read-work figure moves too, and by more than the ranks did:
        //     the costly run paid for a discarded speculative read on top of the
        //     drain, which is a cost `ranks_pulled` cannot show.
        let cheap_rows = reported_rows(&cheap, &stratum);
        let costly_rows = reported_rows(&costly, &stratum);
        assert!(
            cheap_rows > 0 && costly_rows > 0,
            "stratum {stratum}: a read that returned no row is not a read this \
             fixture takes — {report}"
        );
        assert!(
            cheap_rows < costly_rows,
            "stratum {stratum}: the read-work figure did not move with the \
             corpus cost — {report}"
        );

        // (3) And it is the producer's own count, read from the other end of the
        //     seam.
        assert_eq!(
            cheap_rows,
            served_rows(&cheap, &stratum),
            "stratum {stratum}: the trailer and the producer disagree about what \
             the cheap run's read returned — {report}"
        );
        assert_eq!(
            costly_rows,
            served_rows(&costly, &stratum),
            "stratum {stratum}: the trailer and the producer disagree about what \
             the costly run's reads returned — {report}"
        );
    }

    // And the number of reads, which is the other thing that moved.
    assert_eq!(cheap.read_attempts, ReadAttempts::Once, "{report}");
    assert_eq!(costly.read_attempts, ReadAttempts::Twice, "{report}");
}

/// **The read a fallback throws away is still on the bill.**
///
/// The drained control is the one configuration here that pays for two complete
/// reads: a speculative one its ceiling cut and threw away, and the planned one
/// the answer came from. `ranks_pulled` says four hundred either way, and so
/// would a read-work figure that counted only the answering read — which is
/// exactly the reporting failure this number exists to prevent, because the
/// headline this branch claims is a reduction in materialised rows.
///
/// The neighbour is the control that keeps the sum honest: a configuration that
/// read **once** must report that one read and not a sum of something. A figure
/// that added blindly would pass the first half of this test and fail the
/// second.
#[test]
fn the_read_work_figure_counts_the_read_the_fallback_discarded() {
    let dataset = common::empty_dataset();
    let twice = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let once = measure(SHARED_BLOCK_INTERSECTING_RESULTS, &dataset);
    let report = format!(
        "twice={} once={}",
        render(SHARED_BLOCK_DISJOINT_RESULTS, &twice),
        render(SHARED_BLOCK_INTERSECTING_RESULTS, &once),
    );

    assert_eq!(twice.read_attempts, ReadAttempts::Twice, "{report}");
    assert_eq!(once.read_attempts, ReadAttempts::Once, "{report}");

    for stratum in strata() {
        // The two-read case: the figure is the sum of both reads, which is
        // strictly more than the answering read alone.
        let answering = twice.reads[&stratum]
            .last()
            .copied()
            .expect("every producer served a read");
        assert_eq!(
            reported_rows(&twice, &stratum),
            speculative_limit() + ROWS,
            "stratum {stratum}: the discarded read is missing from the bill — {report}"
        );
        assert!(
            reported_rows(&twice, &stratum) > answering,
            "stratum {stratum}: a figure equal to the answering read would make \
             the second read invisible — {report}"
        );

        // The one-read case: the figure is that read, and nothing was added to
        // it.
        assert_eq!(
            reported_rows(&once, &stratum),
            speculative_limit(),
            "stratum {stratum}: a run that read once must report that read — {report}"
        );
        assert_eq!(
            once.reads[&stratum].len(),
            1,
            "stratum {stratum}: and it really did read once — {report}"
        );
    }

    // The two runs disagree, which is what makes the pair a measurement rather
    // than a restatement of one number.
    assert_ne!(
        reported_rows(&twice, &strata()[0]),
        reported_rows(&once, &strata()[0]),
        "{report}"
    );
}

/// **The two-run oracle for the exclusion-lookup counter.**
///
/// Both runs declare a membership basis, so neither is the vacuous zero case,
/// and they still report different counts: one configuration's candidates are
/// disjoint, so a verdict settles finality and the read stops early with lookups
/// spread over the ranks it reached; the other's are shared, so every verdict is
/// `Possible` and the only lookups made are the handful the threshold filter let
/// through. A counter that reported the same number for both, or zero for both,
/// would be measuring nothing.
///
/// The third run is the control that says the counter is not simply always
/// non-zero: the identical configuration with the basis undeclared asks nothing,
/// and reports it.
#[test]
fn the_exclusion_lookup_counter_moves_between_two_runs_that_both_ask() {
    let dataset = common::empty_dataset();
    let disjoint = measure(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &dataset);
    let shared = measure(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &dataset);
    let silent = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = format!(
        "disjoint={} shared={} silent={}",
        render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS_WITH_LOOKUPS, &disjoint),
        render_with_lookups(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &shared),
        render_with_lookups(SHARED_BLOCK_DISJOINT_RESULTS, &silent),
    );

    assert!(
        total(&disjoint.fused_lookups) > 0,
        "the disjoint run asked nothing — {report}"
    );
    assert!(
        total(&shared.fused_lookups) > 0,
        "the shared run asked nothing — {report}"
    );
    assert_ne!(
        total(&disjoint.fused_lookups),
        total(&shared.fused_lookups),
        "two runs that ask under different corpora report the same count, so \
         the counter is not measuring the asking — {report}"
    );
    assert_eq!(
        total(&silent.fused_lookups),
        0,
        "and a run whose producers declared no basis must report none — {report}"
    );
}

/// **The prediction a host can read before it runs anything.**
///
/// The threshold-crossing derivation is the library's, and the plan surfaces it:
/// [`PlannedResolution::crossing_rank_at`] answers it from the decay law and the
/// weights of the strata whose declared blocks meet this one's, which is
/// everything the derivation needs that is not a fact about rows. Nothing here
/// is hardcoded — each expected value is recomputed in this test, from the
/// configuration's own two bits, through the same library function — and the
/// three configurations must yield three *different* answers, so a constant or a
/// stub fails.
///
/// The second half is the one that makes the surface worth having: for every
/// configuration whose read really reached the threshold gate, the surfaced
/// prediction **equals** the rank fusion stopped at. For the one that never
/// reaches it, it does not, and that is the honest reading of it: this bounds the
/// threshold gate and says nothing about finality, so a configuration finality
/// forbids from stopping drains past its own crossing.
#[test]
fn the_plan_surfaces_the_crossing_and_it_predicts_the_read_that_reaches_it() {
    let dataset = common::empty_dataset();
    let mut surfaced_values = Vec::new();

    for config in [
        DISTINCT_BLOCKS_DISJOINT_RESULTS,
        SHARED_BLOCK_INTERSECTING_RESULTS,
        SHARED_BLOCK_DISJOINT_RESULTS,
    ] {
        let measured = measure(config, &dataset);
        let report = render(config, &measured);
        let deepest = measured.deepest_emitted_rank();
        let naming = naming_weights(&measured);
        let sharing = declared_sharing(config);

        for stratum in strata() {
            let planned = measured
                .planned_resolution
                .get(&stratum)
                .expect("the fixture profile weights every planned stratum");

            // The set the planner derived from the declarations, against the set
            // this test derives from the configuration's own two bits.
            assert_eq!(
                planned.sharing_weights,
                sharing,
                "{name}: the plan's sharing set is not the one the declarations \
                 describe — {report}",
                name = config.name
            );

            // The derivation, run here, against the derivation the plan
            // surfaces. Both are the library's; what is under test is that the
            // planner feeds it the right terms.
            let predicted = crossing_rank_at(decay(), &naming, deepest, &sharing);
            let surfaced = crossed(planned.crossing_rank_at(&naming, deepest));
            assert_eq!(
                predicted,
                surfaced,
                "{name}: the plan surfaced a crossing the derivation does not \
                 give — {report}",
                name = config.name
            );

            // And where the read reached the gate, it is the read.
            let pulled = measured.ranks_pulled[&stratum];
            let reached_the_gate = matches!(
                measured.status[&stratum],
                ProducerStatus::CeilingReached { .. }
            );
            if reached_the_gate {
                assert_eq!(
                    surfaced,
                    pulled,
                    "{name}: the run stopped at the threshold gate, so the \
                     surfaced prediction must be the rank it stopped at — {report}",
                    name = config.name
                );
            } else {
                assert!(
                    surfaced < pulled,
                    "{name}: this configuration drains past its own crossing, \
                     which is the whole reason finality is a separate gate — \
                     {report}",
                    name = config.name
                );
            }

            surfaced_values.push(surfaced);
        }
    }

    // Three configurations, three different surfaced values. A stub, a constant
    // or a prediction that read only the profile would collapse these.
    let distinct: BTreeSet<u64> = surfaced_values.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        3,
        "the three configurations must be told apart by what the plan surfaces, \
         and these are {surfaced_values:?}"
    );
}

/// **The plan-time half, with nothing executed at all.**
///
/// The point of putting the crossing on the compiled plan is that a host learns
/// what its declarations have committed it to *before* it pays for a read. So
/// this test compiles and stops: no dataset is opened, no producer is called, no
/// row exists. What it asks is the worst case — one namer, measured against every
/// stratum that shares its blocks — and the two configurations answer with the
/// two numbers the pure derivation at the top of this file already pins, which is
/// where they are asserted as arithmetic rather than as declarations.
#[test]
fn a_host_learns_the_crossing_its_declarations_condemn_it_to_without_reading_a_row() {
    let mut answers = Vec::new();
    for config in [
        DISTINCT_BLOCKS_DISJOINT_RESULTS,
        SHARED_BLOCK_DISJOINT_RESULTS,
    ] {
        let (registry, _counters) = configured_registry(config);
        let statistics = fixture_statistics();
        let profile = fixture_profile();
        let request = RetrievalRequest::bounded(request_terms(), TOP_K);
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &statistics,
            fusion_profile: Some(&profile),
        };
        let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
        let compiled = compile(&planned, &env).expect("a fresh plan is admitted");

        for stratum in strata() {
            let resolution = compiled
                .resolution
                .get(&stratum)
                .expect("the fixture profile weights every planned stratum");
            answers.push(crossed(resolution.crossing_rank(&[Fixed::ONE])));
        }
    }

    // One namer against its own head alone, and one namer against two sharers.
    // Both are read from the derivation rather than written as literals; the
    // literals live in the two pure tests at the top of this file, which is the
    // one place they are claimed as arithmetic.
    assert_eq!(
        answers,
        vec![
            crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE]),
            crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE]),
            crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE, Fixed::ONE]),
            crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE, Fixed::ONE]),
        ],
        "a declaration that shares a block condemns the read to outlast the \
         smoothing constant, and the plan says so before anything runs"
    );
    assert_ne!(
        answers[0], answers[2],
        "if the two declarations predicted the same read there would be nothing \
         for a host to learn here"
    );
}

/// **Case C: a truthful `Exhausted` says nothing about what the answer cost.**
///
/// Three runs, five rows each. Two of them report the identical terminal status
/// — `Exhausted`, and truthfully: both streams really did run out — and one of
/// them cost a hundred times what the other did. The third reports a ceiling over
/// a cheap read. From the statuses alone the first two are indistinguishable, and
/// that is not a defect in the status: exhaustion is exactly what happened.
///
/// The counter is the only thing that tells them apart, which is the whole
/// argument for keeping it in the observed resolution rather than treating it as
/// diagnostics. The status and the counter are therefore asserted **as a pair**,
/// in one test: a test that pinned the statuses in one place and the counts in
/// another would let a change make an expensive answer look like a cheap one
/// without failing anything.
#[test]
fn a_truthful_exhaustion_over_a_costly_answer_is_told_from_a_cheap_one_only_by_the_counter() {
    let dataset = common::empty_dataset();
    let drained = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let short = measure(SHARED_BLOCK_SHORT_STREAMS, &dataset);
    let ceilinged = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let report = format!(
        "drained={} short={} ceilinged={}",
        render(SHARED_BLOCK_DISJOINT_RESULTS, &drained),
        render(SHARED_BLOCK_SHORT_STREAMS, &short),
        render(DISTINCT_BLOCKS_DISJOINT_RESULTS, &ceilinged),
    );

    // Every one of them is a five-row answer, so the answer's size tells nobody
    // anything either.
    for measured in [&drained, &short, &ceilinged] {
        assert_eq!(measured.rows.len(), 5, "{report}");
    }

    // The pair whose statuses agree. Asserted together — status and counter in
    // one tuple — because that pairing IS the claim.
    assert_eq!(
        (
            drained.both("the terminal status", &drained.status),
            drained.both("the ranks pulled", &drained.ranks_pulled),
        ),
        (ProducerStatus::Exhausted { rows_emitted: ROWS }, ROWS),
        "the drained run: a truthful exhaustion over a corpus-cost read — {report}"
    );
    assert_eq!(
        (
            short.both("the terminal status", &short.status),
            short.both("the ranks pulled", &short.ranks_pulled),
        ),
        (
            ProducerStatus::Exhausted {
                rows_emitted: SHORT_ROWS
            },
            SHORT_ROWS
        ),
        "the short run: the same truthful exhaustion over a cheap read — {report}"
    );

    // The two statuses are the same KIND of ending, and the counters are two
    // orders of magnitude apart. That is the case for keeping the counter
    // prominent, stated as arithmetic.
    assert!(
        matches!(
            drained.both("the terminal status", &drained.status),
            ProducerStatus::Exhausted { .. }
        ) && matches!(
            short.both("the terminal status", &short.status),
            ProducerStatus::Exhausted { .. }
        ),
        "both endings must be exhaustion, or this pair is told apart by \
         something else — {report}"
    );
    assert!(
        drained.both("the ranks pulled", &drained.ranks_pulled)
            > 10 * short.both("the ranks pulled", &short.ranks_pulled),
        "and the costly run must cost enough that a reader who ignored the \
         counter would be badly wrong — {report}"
    );

    // And the ceiling over a cheap read, which is the other half of the pairing:
    // a different status, a cheap counter, and the same five rows.
    assert_eq!(
        (
            ceilinged.both("the terminal status", &ceilinged.status),
            ceilinged.both("the ranks pulled", &ceilinged.ranks_pulled),
        ),
        (ceiling_at(4), 4),
        "the ceilinged run: a bound the fusion itself imposed, over a cheap \
         read — {report}"
    );
    assert_ne!(
        drained.both("the ranks pulled", &drained.ranks_pulled),
        ceilinged.both("the ranks pulled", &ceilinged.ranks_pulled),
        "{report}"
    );
}

/// **What a caller actually sees.**
///
/// The observed resolution is rendered by the crate, not by this test, and every
/// counter has to be in the bytes: a counter that lives in a struct field and
/// never reaches the text is a cost nobody reads. So the expected text is
/// assembled here out of sources that are **not** the trailer — the profile's own
/// separating depth, this file's threshold derivation, the fixture's counters
/// inside the producers — and compared with the rendering byte for byte.
///
/// The configuration is the one whose producers answer lookups over a shared
/// block, so the exclusion count in the text is non-zero; it reads exactly once,
/// so the producer's own lookup counter and the engine's are counts of the same
/// asking and either can stand in for the other.
#[test]
fn the_rendered_observed_resolution_carries_every_counter_a_caller_pays_for() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &dataset);
    let report = render_with_lookups(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS, &measured);

    // The premise this rendering is worth asserting over: the run really asked.
    assert_eq!(measured.read_attempts, ReadAttempts::Once, "{report}");
    assert!(
        total(&measured.served_lookups) > 0,
        "a run that asked nothing would render a zero and prove nothing — {report}"
    );

    // Every term of the expected text, from a source that is not the trailer.
    // The collision count is the one term that is a zero, and it is DERIVED
    // rather than copied: this profile still separates adjacent ranks far past
    // anything this run pulls, so no adjacent pair it read could have collided.
    // (The oracle that shows the counter moves off zero is in `fusion.rs`, over
    // a profile that collides inside the stream it fuses; there is no such
    // profile here, and writing one would make this file about something else.)
    let profile = fixture_profile();
    for stratum in strata() {
        assert!(
            profile
                .monotone_depth(&stratum)
                .expect("the fixture profile weights every stratum")
                .covers(measured.ranks_pulled[&stratum]),
            "stratum {stratum}: this profile stops separating inside the read,              so the zero below would not be derivable — {report}"
        );
        assert_eq!(
            measured.collisions[&stratum], 0,
            "stratum {stratum}: the separation covers the read, so an observed              collision would contradict the law — {report}"
        );
    }
    let deepest = measured.deepest_emitted_rank();
    let predicted_ranks = crossing_rank_at(
        decay(),
        &naming_weights(&measured),
        deepest,
        &declared_sharing(SHARED_BLOCK_INTERSECTING_RESULTS_WITH_LOOKUPS),
    );
    let mut expected = String::new();
    for stratum in strata() {
        let separates_to = profile
            .monotone_depth(&stratum)
            .expect("the fixture profile weights every stratum")
            .rank()
            .map_or_else(|| "beyond-any-plan".to_owned(), |rank| rank.to_string());
        expected.push_str(&format!(
            "{stratum} separates_to={separates_to} ranks_pulled={predicted_ranks} \
             collisions_observed=0 exclusion_lookups={lookups} \
             rows_materialised={rows}\n",
            lookups = measured.served_lookups[&stratum],
            rows = served_rows(&measured, &stratum),
        ));
    }

    assert_eq!(
        measured.rendered_resolution, expected,
        "the text a caller reads is not the cost this run paid — {report}"
    );

    // And the text really does carry a non-zero lookup count, rather than
    // satisfying the comparison above with nothing but zeroes. One stratum here
    // is asked and the other is not — the frontier asks the stream that has NOT
    // named a candidate — so the claim is about the asked one, and the unasked
    // one's honest zero is in the text beside it.
    let asked = *measured
        .served_lookups
        .values()
        .max()
        .expect("both strata are measured");
    assert!(
        asked > 0,
        "a rendering in which every lookup count is zero pins nothing about the \
         counter this configuration exists to exercise — {report}"
    );
    assert!(
        measured
            .rendered_resolution
            .contains(&format!("exclusion_lookups={asked} ")),
        "the count the producers served is not in the text a caller reads — {report}"
    );
}
