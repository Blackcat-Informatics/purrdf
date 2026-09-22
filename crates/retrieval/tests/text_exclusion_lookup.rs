// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a *real* text producer's exclusion basis buys a fused read, measured
//! end to end.
//!
//! `exclusion_lookup.rs` proves the protocol against mocks: what a basis may be
//! declared against, and what a stream may answer. This file asks the question
//! that no mock can answer — whether a shipped producer can actually serve the
//! lookup, and whether serving it is cheap — by running the same request twice
//! through the whole ladder (`plan` → `compile` → `execute` → `fuse`, as
//! `search` runs it) over two real `TextSearchRelation`s.
//!
//! The two runs differ in exactly one term: the `exclusion` field of the
//! declaration the relation itself hands out. Everything else — the indexes, the
//! request, the weights, the bound, the blocks, the dataset — is identical, so
//! every number that moves between them moved because of that one field.
//!
//! # The shape, and why it is this one
//!
//! Both producers declare **one shared block** and hold **disjoint** candidates.
//! That is the configuration where a bound cannot bound the read on its own: no
//! candidate either stream names is ever final while the other stream is open,
//! because the other stream declares the same block and might still name it. It
//! is the permanent drain control of `multimodal_read_bound.rs`, and the only
//! thing that resolves it is learning, rather than being told, that a stream will
//! never name a candidate.
//!
//! The right-hand index holds a document for every left-hand subject, carrying
//! text that matches no needle term. That is deliberate: it makes the lookup do
//! real work rather than fall out of "no such subject". The right producer must
//! binary-search its dictionary for each needle term against a document it really
//! holds, find no posting, and answer `Excluded` — which is exactly the case
//! where a naive implementation would rank the partition and get the same answer
//! far more slowly.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, Fixed, FusedRow, FusionProfile,
    Iri, RECIP_K, RankFidelity, RequestTerm, RetrievalRequest, Statistics, Term, TopK, search,
};
use purrdf_sparql_eval::{
    EvalError, ExclusionBasis, PfArgs, PfArity, PfCursor, PropertyFunction,
    PropertyFunctionRegistry, RankedDeclaration, Volatility,
};
use purrdf_text::{
    GraphSelector, SearchObservations, TextIndex, TextIndexConfig, TextSearchRelation,
};

/// The needle both producers are asked for.
const NEEDLE: &str = "alpha beta";

/// How many matching documents each side holds.
///
/// Above the rank at which the fusion threshold licenses a stop — a candidate
/// one of two block-sharing streams named has to outlast twice its own weight,
/// which under reciprocal-rank decay is deep — so that a drained read and a
/// licensed one are genuinely different numbers. Small enough that the whole
/// file still runs in a test suite.
const MATCHING: usize = 100;

/// The bound the request searches under.
const TOP_K: TopK = TopK::new(5);

/// The smoothing constant the fusion profile decays by, read from the crate's own
/// constant rather than written twice.
const K: u32 = RECIP_K as u32;

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

/// A minimal single-threaded executor. Nothing in this pipeline actually pends.
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
// The two sides
// ---------------------------------------------------------------------------

/// Which of the two producers a helper is speaking about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The left producer: matching documents under `ex:left`, and nothing else.
    Left,
    /// The right producer: matching documents of its own under `ex:right`, plus
    /// a non-matching document for every one of the left producer's subjects.
    Right,
}

impl Side {
    /// The predicate whose literals this side's index is built over.
    fn predicate(self) -> String {
        match self {
            Self::Left => ex("left"),
            Self::Right => ex("right"),
        }
    }

    /// The stratum this side's producer is registered for.
    fn stratum(self) -> String {
        match self {
            Self::Left => ex("stratum/left"),
            Self::Right => ex("stratum/right"),
        }
    }

    /// The IRI this side's relation is called by.
    fn producer(self) -> String {
        match self {
            Self::Left => ex("pf/left"),
            Self::Right => ex("pf/right"),
        }
    }

    /// The subject-name prefix of this side's matching documents. Distinct per
    /// side, which is what makes the two candidate sets disjoint.
    fn prefix(self) -> &'static str {
        match self {
            Self::Left => "a",
            Self::Right => "b",
        }
    }
}

/// The two sides in registration order.
const SIDES: [Side; 2] = [Side::Left, Side::Right];

/// The `(subject, text)` rows one side's index is built from.
///
/// Both sides hold [`MATCHING`] documents the needle reaches, under their own
/// subject prefix. The right side additionally holds one document per *left*
/// subject whose text shares no term with the needle — the documents that make an
/// exclusion lookup a real dictionary search rather than an absent-subject
/// shortcut.
fn rows(side: Side) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = (0..MATCHING)
        .map(|at| {
            (
                ex(&format!("{}{at}", side.prefix())),
                // The repeated term varies the term frequency, so the documents
                // of one side do not all score the same and the ranking is a
                // ranking rather than a tie-break order.
                format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim()),
            )
        })
        .collect();
    if side == Side::Right {
        out.extend((0..MATCHING).map(|at| {
            (
                ex(&format!("{}{at}", Side::Left.prefix())),
                "zulu yankee xray whiskey".to_owned(),
            )
        }));
    }
    out
}

/// The dataset every stage runs against: both sides' rows, each under its own
/// predicate, in the default graph.
///
/// The property-function calls this file compiles read nothing out of it — a
/// ranked producer answers from its own index — but it is the dataset the indexes
/// were built from, which is the wiring a host actually has.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for side in SIDES {
        let predicate = builder.intern_iri(&side.predicate());
        for (subject, text) in rows(side) {
            let subject = builder.intern_iri(&subject);
            let object = builder.intern_literal(RdfLiteral::simple(&text));
            builder.push_quad(subject, predicate, object, None);
        }
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// One side's index: one predicate, every graph, untagged literals in the default
/// graph — so exactly one partition, which is the condition
/// `TextSearchRelation::ranked_declaration` requires.
fn index(dataset: &RdfDataset, side: Side) -> Arc<TextIndex> {
    let config = TextIndexConfig::new(vec![TermValue::iri(side.predicate())], GraphSelector::Any)
        .expect("the fixture configuration is well formed");
    Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture index builds"))
}

// ---------------------------------------------------------------------------
// The recorder
// ---------------------------------------------------------------------------

/// A relation that delegates everything to a real [`TextSearchRelation`] while
/// recording, per invocation, whether the candidate position was bound.
///
/// Delegation rather than a stand-in matters twice over: the declared modes and
/// row bounds are the real ones, so the registry's admission of the basis is the
/// real admission; and the rows are the real rows, so the answer compared below
/// is the answer a host gets.
///
/// What it adds is the one fact the counters inside the relation cannot supply on
/// their own — how many invocations were *lookups* — which is what turns
/// "the ranker ran `n` times" into "the ranker ran exactly once per non-lookup
/// invocation and never for a lookup".
#[derive(Debug)]
struct Recorder {
    /// The relation under observation.
    inner: TextSearchRelation,
    /// Invocations whose candidate position was bound: the exclusion lookups.
    candidate_bound: AtomicU64,
    /// Invocations whose candidate position was free: the ranked reads.
    candidate_free: AtomicU64,
}

impl Recorder {
    fn new(index: Arc<TextIndex>) -> Self {
        Self {
            inner: TextSearchRelation::new(index),
            candidate_bound: AtomicU64::new(0),
            candidate_free: AtomicU64::new(0),
        }
    }

    fn observations(&self) -> Arc<SearchObservations> {
        self.inner.observations()
    }

    fn candidate_bound(&self) -> u64 {
        self.candidate_bound.load(Ordering::Relaxed)
    }

    fn candidate_free(&self) -> u64 {
        self.candidate_free.load(Ordering::Relaxed)
    }
}

impl PropertyFunction for Recorder {
    fn volatility(&self) -> Volatility {
        self.inner.volatility()
    }

    fn arity(&self) -> PfArity {
        self.inner.arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        self.inner.modes()
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.inner.rows_per_invocation(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let counter = if args.get(TextSearchRelation::DOC).is_some() {
            &self.candidate_bound
        } else {
            &self.candidate_free
        };
        counter.fetch_add(1, Ordering::Relaxed);
        self.inner.open(args, ceiling)
    }
}

// ---------------------------------------------------------------------------
// The wiring
// ---------------------------------------------------------------------------

/// The one block both producers declare. Sharing it is what removes the
/// planner's merge argument and makes finality the thing that decides the read.
fn shared_block() -> DomainTag {
    DomainTag::parse(&ex("domain/shared")).expect("the fixture domain tag is a valid IRI")
}

/// One request term per producer, each naming that producer's own predicate, so
/// each producer is bound to exactly one term and both search the same needle.
fn request_terms() -> Vec<RequestTerm> {
    SIDES
        .into_iter()
        .map(|side| RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(&side.predicate())),
        })
        .collect()
}

/// Statistics that narrow nothing: each stratum really does hold the rows its
/// producer declares, and no selectivity is measured.
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
    for side in SIDES {
        let held = rows(side).len() as u64;
        cardinalities.insert(iri(&side.stratum()), held);
        cardinalities.insert(iri(&side.predicate()), held);
    }
    FixtureStatistics { cardinalities }
}

/// Unit weight for each stratum under reciprocal-rank decay.
fn fixture_profile() -> FusionProfile {
    let weights = SIDES
        .into_iter()
        .map(|side| (iri(&side.stratum()), Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

/// Both producers registered, with `basis` written into the declaration each
/// relation hands out for itself.
///
/// The declaration is the relation's own — positions, accepted term, duplicate
/// policy and basis all come from `ranked_declaration` — and the only field this
/// function touches is `exclusion`, which is what makes the two runs below a
/// differential pair rather than two different registrations.
fn registry(
    dataset: &RdfDataset,
    basis: ExclusionBasis,
) -> (PropertyFunctionRegistry, [Arc<Recorder>; 2]) {
    let mut registry = PropertyFunctionRegistry::new();
    let recorders = SIDES.map(|side| Arc::new(Recorder::new(index(dataset, side))));
    for (side, recorder) in SIDES.into_iter().zip(recorders.iter()) {
        let mut declaration: RankedDeclaration = recorder
            .inner
            .ranked_declaration(
                kernel_iri(&side.stratum()),
                Some(side.predicate()),
                RankFidelity::EXACT,
                // Both producers over one block: the shape in which a bound is
                // not on its own a bound on the read.
                CandidateDomains::within([shared_block()]),
            )
            .expect("a single-partition index declares a ranked order");
        assert_eq!(
            declaration.exclusion,
            ExclusionBasis::Membership,
            "the relation's own declaration is the membership basis; this fixture varies it \
             rather than inventing it"
        );
        declaration.exclusion = basis;
        let relation: Arc<dyn PropertyFunction> = Arc::<Recorder>::clone(recorder);
        registry.register_ranked(side.producer(), relation, declaration);
    }
    (registry, recorders)
}

/// What one run cost and what it answered.
struct Measured {
    /// The fused answer, reduced to the part a reader compares: the candidate,
    /// its fused score, and which stratum contributed what at which rank.
    answer: Vec<(Term, Fixed, Vec<(Iri, u64, Fixed)>)>,
    /// Ranks fusion pulled off each stream, summed.
    ranks_pulled: u64,
    /// Exclusion lookups fusion performed against each stream, summed.
    fused_lookups: u64,
    /// Membership lookups each relation really performed, by side.
    membership_lookups: [u64; 2],
    /// Invocations of each relation whose candidate position was bound.
    candidate_bound: [u64; 2],
    /// Invocations of each relation whose candidate position was free.
    candidate_free: [u64; 2],
    /// How many invocations of each relation entered the partition ranker.
    rankings: [u64; 2],
}

impl Measured {
    /// One run's numbers, rendered so that a failure shows the whole row of the
    /// table rather than the single number that tripped.
    fn report(&self, name: &str) -> String {
        format!(
            "{name}: rows={rows} ranks_pulled={ranks} fused_lookups={fused} \
             candidate_bound={bound:?} candidate_free={free:?} rankings={rankings:?} \
             membership_lookups={membership:?}",
            rows = self.answer.len(),
            ranks = self.ranks_pulled,
            fused = self.fused_lookups,
            bound = self.candidate_bound,
            free = self.candidate_free,
            rankings = self.rankings,
            membership = self.membership_lookups,
        )
    }
}

/// Run the request once under `basis` and measure it.
fn measure(dataset: &RdfDataset, basis: ExclusionBasis) -> Measured {
    let (registry, recorders) = registry(dataset, basis);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let result = block_on(search(
        &request,
        &registry,
        &statistics,
        dataset,
        &env,
        &profile,
    ))
    .expect("the fixture request searches");

    let observed: [Arc<SearchObservations>; 2] =
        [recorders[0].observations(), recorders[1].observations()];
    Measured {
        answer: result.rows.iter().map(reduce).collect(),
        ranks_pulled: result
            .trailer
            .resolution
            .values()
            .map(|resolution| resolution.ranks_pulled)
            .sum(),
        fused_lookups: result
            .trailer
            .resolution
            .values()
            .map(|resolution| resolution.exclusion_lookups)
            .sum(),
        membership_lookups: [
            observed[0].membership_lookups(),
            observed[1].membership_lookups(),
        ],
        candidate_bound: [
            recorders[0].candidate_bound(),
            recorders[1].candidate_bound(),
        ],
        candidate_free: [recorders[0].candidate_free(), recorders[1].candidate_free()],
        rankings: [observed[0].rankings(), observed[1].rankings()],
    }
}

/// One fused row reduced to what two runs must agree on.
///
/// The threshold witness is deliberately left out: it records the global
/// threshold in force when the row was certified, and certifying a row earlier —
/// which is the whole point of an exclusion lookup — legitimately moves it. The
/// candidate, the score and the provenance are the answer.
fn reduce(row: &FusedRow) -> (Term, Fixed, Vec<(Iri, u64, Fixed)>) {
    (
        row.entity.clone(),
        row.score,
        row.contributions
            .iter()
            .map(|(stratum, rank, contribution)| (stratum.clone(), *rank, *contribution))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// The measurement
// ---------------------------------------------------------------------------

/// **The declared basis alone: the same answer, out of a strictly shorter read,
/// paid for with membership lookups and no extra ranking.**
///
/// All of it is asserted in one test, because each half alone passes over a
/// defect the other catches. "Fewer ranks pulled" alone is satisfied by a run
/// that stopped early and answered wrongly. "The same answer" alone is satisfied
/// by a basis nothing ever asked about. "Lookups happened" alone is satisfied by
/// lookups that each ranked a partition, which is the cost this mechanism exists
/// to avoid.
///
/// The run under `ExclusionBasis::Unavailable` is the control, and it is the read
/// this shape had before a basis could be declared: one shared block and disjoint
/// candidates, so no candidate is ever final while either stream is open, so both
/// streams drain.
#[test]
fn a_declared_basis_shortens_a_real_text_read_without_moving_the_answer() {
    let dataset = dataset();
    let control = measure(&dataset, ExclusionBasis::Unavailable);
    let asked = measure(&dataset, ExclusionBasis::Membership);
    let report = format!("{}; {}", control.report("control"), asked.report("asked"));

    // 1. The answer is the same one, row for row, score for score, in order.
    assert_eq!(
        asked.answer, control.answer,
        "the basis is evidence about what a stream will never name; it may shorten the read \
         and must not change what the read answers"
    );
    assert_eq!(
        asked.answer.len(),
        usize::try_from(TOP_K.get()).expect("the bound is small"),
        "the fixture must actually fill the bound, or the comparison above compares two \
         empty answers"
    );

    // 2. The read is strictly shorter.
    assert!(
        asked.ranks_pulled < control.ranks_pulled,
        "a shared block with disjoint candidates drains both streams unless a stream can be \
         asked — {report}"
    );

    // 3. The lookups really happened, and really reached the relation.
    assert!(
        asked.fused_lookups > 0,
        "fusion must have asked, or the shorter read above came from something else — {report}"
    );
    assert_eq!(
        control.fused_lookups, 0,
        "and the control must not have asked, which is what `Unavailable` means"
    );
    assert!(
        asked.candidate_bound.iter().sum::<u64>() >= asked.fused_lookups,
        "every lookup fusion counted arrived at a relation as a candidate-bound invocation;          the relation's count is cumulative over every read attempt this search took, so it          can only be the larger of the two — invoked = {:?}, counted = {}",
        asked.candidate_bound,
        asked.fused_lookups
    );
    assert_eq!(
        control.candidate_bound,
        [0, 0],
        "and the control relation was never invoked with a bound candidate"
    );

    // 4. The right producer's lookups did real dictionary work — it holds a
    //    document for every left subject — and the left producer's did not,
    //    because it holds no document for a right subject at all. Both are
    //    `Excluded`; only one of them had anything to search.
    assert!(
        asked.membership_lookups[1] > 0,
        "the right index holds a non-matching document for every left candidate, so its \
         lookups are real binary searches over its own dictionary — {report}"
    );
    assert_eq!(
        asked.membership_lookups[0], 0,
        "the left index holds no document for a right candidate, so there is nothing to \
         look a term up in"
    );

    // 5. And not one of those lookups entered the partition ranker. This is the
    //    claim the whole mechanism rests on, and it is asserted against the
    //    invocations rather than against a clock: the ranker ran exactly once
    //    per candidate-free invocation, so it ran zero times for the
    //    candidate-bound ones.
    for (at, side) in SIDES.into_iter().enumerate() {
        assert_eq!(
            asked.rankings[at], asked.candidate_free[at],
            "{side:?}: the ranker must have been entered exactly once per ranked read and \
             never for a lookup, yet {} invocations ranked against {} ranked reads",
            asked.rankings[at], asked.candidate_free[at]
        );
        assert!(
            asked.candidate_free[at] > 0,
            "{side:?}: the ranked read itself must have happened, or the equality above \
             holds over two zeroes"
        );
    }
}
