// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What an exclusion lookup may declare, and what it may answer.
//!
//! An exclusion lookup is the one channel in the ranked-stream protocol that
//! carries a question *in*: the consumer names a candidate and the producer says
//! whether it will ever name it. Everything that channel buys rests on the
//! answer being true, so this file is about the two places a false one is
//! caught.
//!
//! **At registration**, against the declarations the registry already holds. A
//! basis is a promise that a lookup can be answered cheaply and that "excluded"
//! means something exact, and both halves are checkable before any query runs.
//!
//! **At fusion**, against the rows. A producer that excludes a candidate and
//! then names it has contradicted itself, and a producer that calls a candidate
//! possible while its own declared domains put that candidate out of reach has
//! broken the promise it made about blocks. Neither is repaired.
//!
//! # Every refusal here is executed beside its valid neighbour
//!
//! A refusal is a claim, and over-refusal is the mirror of the silent drop: it
//! looks like correct strictness and nothing appears broken until a host writes
//! the registration that should work and does not. So every test below runs both
//! cases — the one that must be refused, and the neighbouring one that differs
//! in exactly the term under test and must be admitted. The
//! `Membership`-from-a-lossy-producer case is the one that matters most: it is
//! the registration a naive completeness check would wrongly refuse, and the
//! answer it gives is provably exact however lossy the search is.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    CandidateDomains, Completeness, DecayRule, DomainTag, DuplicatePolicy, ExclusionBasis,
    ExclusionVerdict, Fixed, FusionError, FusionProfile, Iri, OrderFidelity, ProducerReceipt,
    ProtocolError, RECIP_K, RankFidelity, RankedRow, RankedStream, RowBlock, StreamContract, Term,
    TopK, contribution_under, fuse,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, RankedDeclaration,
};

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn tag(suffix: &str) -> DomainTag {
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
// 1. Registration: what the registry admits a declared basis against.
// ---------------------------------------------------------------------------

/// A relation whose declared modes and per-mode row bounds are the fixture's to
/// choose.
///
/// Nothing else about it matters here: registration reads the declarations and
/// never opens the relation, so `open` exists to satisfy the trait and is never
/// called by any test in this section.
struct DeclaredRelation {
    arity: PfArity,
    modes: Vec<BindingPattern>,
    /// The row bound reported for a mode that binds position zero — the
    /// candidate position every declaration in this section projects from.
    bound_rows: u64,
    /// The row bound reported for every other mode.
    free_rows: u64,
}

impl PropertyFunction for DeclaredRelation {
    fn volatility(&self) -> purrdf_sparql_eval::Volatility {
        purrdf_sparql_eval::Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(0) {
            self.bound_rows
        } else {
            self.free_rows
        }
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(EmptyCursor))
    }
}

struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

/// Whether the relation declares a mode that binds the candidate position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateMode {
    /// Only the all-free mode, which serves a candidate-bound call by scanning.
    FreeOnly,
    /// The all-free mode and a candidate-bound one whose row bound is a point
    /// bound.
    PointLookup,
    /// A candidate-bound mode that declares more than one row — a mode that
    /// exists and is not a point lookup.
    BoundButUnbounded,
}

/// Register one producer under `basis`, with `completeness` and `mode`.
///
/// Returns the registry, so a caller can read the declaration back, and panics
/// exactly where [`PropertyFunctionRegistry::register_ranked`] does.
fn register(
    basis: ExclusionBasis,
    completeness: Completeness,
    mode: CandidateMode,
) -> PropertyFunctionRegistry {
    let arity = PfArity::new(1, 1);
    let candidate_bound = BindingPattern::from_bound_positions(arity.total(), [0]);
    let modes = match mode {
        CandidateMode::FreeOnly => vec![arity.all_free_mode()],
        CandidateMode::PointLookup | CandidateMode::BoundButUnbounded => {
            vec![arity.all_free_mode(), candidate_bound]
        }
    };
    let bound_rows = match mode {
        CandidateMode::BoundButUnbounded => 100,
        CandidateMode::FreeOnly | CandidateMode::PointLookup => 1,
    };
    let relation = DeclaredRelation {
        arity,
        modes,
        bound_rows,
        free_rows: 100,
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/producer"),
        Arc::new(relation),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(&ex("stratum/producer")).expect("fixture IRI"),
            accepted_terms: Vec::new(),
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            fidelity: RankFidelity {
                completeness,
                order: OrderFidelity::Faithful,
            },
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: basis,
            mandatory: false,
        },
    );
    registry
}

/// The panic message [`register`] raised, or `None` where it did not panic.
///
/// The registration is run for real either way, so an "admitted" result below is
/// a registration that really happened rather than one nobody tried.
fn refusal(
    basis: ExclusionBasis,
    completeness: Completeness,
    mode: CandidateMode,
) -> Option<String> {
    // The registry is built inside the closure and dropped with it, so nothing
    // observed after a panic was mutated by one.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let registry = register(basis, completeness, mode);
        // Read back, so the admitted arm proves the declaration is really in the
        // registry rather than merely that nothing panicked.
        assert_eq!(
            registry
                .ranked_declaration(&ex("pf/producer"))
                .expect("the producer registered")
                .exclusion,
            basis,
            "an admitted registration stores the basis it was handed"
        );
    }));
    std::panic::set_hook(previous);
    outcome.err().map(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "a panic with no message".to_owned())
    })
}

fn lossy() -> Completeness {
    Completeness::Lossy {
        evidence: Arc::from("approximate: beam search; recall unmeasured above 10^6"),
    }
}

/// **`Search` is refused from a lossy producer, and admitted from an exhaustive
/// one — and `Membership` is admitted from the lossy one.**
///
/// Three registrations differing in one term each, all executed. The first is
/// the refusal; the second shows the refusal is keyed on the completeness axis
/// and not on the basis; the third is the case that matters most, because it is
/// the one a completeness check applied to the wrong variant would wrongly
/// refuse.
///
/// `Membership` is exact independently of completeness: a term the producer's
/// universe does not contain is a term it names at no rank, whatever its search
/// dropped. Refusing it would throw away the only evidence that can settle
/// finality for a lossy producer, and the refusal would look like strictness.
#[test]
fn search_needs_a_complete_search_and_membership_does_not() {
    let refused = refusal(ExclusionBasis::Search, lossy(), CandidateMode::PointLookup)
        .expect("a lossy search cannot say a candidate is absent");
    assert!(
        refused.contains("exclusion basis of search")
            && refused.contains("completeness lossy")
            && refused.contains("ExclusionBasis::Membership"),
        "the refusal names the basis, the axis it conflicts with, and the \
         registration the host probably meant: {refused}"
    );

    assert_eq!(
        refusal(
            ExclusionBasis::Search,
            Completeness::Complete,
            CandidateMode::PointLookup,
        ),
        None,
        "the same basis from a producer whose search is complete is exactly what \
         the variant is for"
    );

    assert_eq!(
        refusal(
            ExclusionBasis::Membership,
            lossy(),
            CandidateMode::PointLookup
        ),
        None,
        "a membership answer is about the producer's term universe, which a \
         lossy search does not make less exact; refusing this would be refusing \
         a provably exact answer"
    );
}

/// **A basis needs a candidate-bound point mode, and is admitted with one.**
///
/// Three registrations again. Without any candidate-bound mode the lookup would
/// be served by the all-free mode — a table scan, once per frontier candidate
/// per stratum — so the promise of a cheap answer is one the relation cannot
/// keep. With a candidate-bound mode that still declares a hundred rows, the
/// mode exists and the promise still fails, which is the case that shows the
/// refusal is about the *bound* and not merely about the mode's existence. With
/// a point bound, it is admitted.
#[test]
fn a_basis_needs_a_point_bound_candidate_mode() {
    let no_mode = refusal(
        ExclusionBasis::Membership,
        Completeness::Complete,
        CandidateMode::FreeOnly,
    )
    .expect("a producer that can only scan must not be looked up");
    assert!(
        no_mode.contains("declares no access mode that binds its candidate position")
            && no_mode.contains("ExclusionBasis::Unavailable"),
        "the refusal names the missing declaration and the honest alternative: {no_mode}"
    );

    let wide_mode = refusal(
        ExclusionBasis::Membership,
        Completeness::Complete,
        CandidateMode::BoundButUnbounded,
    )
    .expect("a candidate-bound mode that is not a point lookup is still a scan");
    assert_eq!(
        no_mode, wide_mode,
        "the mode existing is not the question; the row bound is, so both \
         shapes earn the identical refusal"
    );

    assert_eq!(
        refusal(
            ExclusionBasis::Membership,
            Completeness::Complete,
            CandidateMode::PointLookup,
        ),
        None,
        "and a relation that declares the candidate-bound point mode is admitted"
    );
}

/// **`Unavailable` is admitted from everything, which is what makes it the
/// honest silence.**
///
/// Every combination of the two axes above, with no basis declared. None of them
/// is refused, because a producer that answers no lookup makes no promise for
/// either check to hold it to — and a registry that refused one of these would
/// be refusing every producer written before the channel existed.
#[test]
fn declaring_no_basis_is_admitted_from_every_producer() {
    for completeness in [Completeness::Complete, lossy()] {
        for mode in [
            CandidateMode::FreeOnly,
            CandidateMode::PointLookup,
            CandidateMode::BoundButUnbounded,
        ] {
            assert_eq!(
                refusal(ExclusionBasis::Unavailable, completeness.clone(), mode),
                None,
                "a producer that answers no exclusion lookup promises nothing \
                 for either check to refuse ({mode:?})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Fusion: what a verdict may say, and what the rows may then do.
// ---------------------------------------------------------------------------

/// The smoothing constant every profile in this file fuses under.
const K: u32 = RECIP_K as u32;

const fn decay() -> DecayRule {
    DecayRule::ReciprocalRank { k: K }
}

/// A hand-built ranked stream with a scripted answer to every exclusion lookup.
///
/// The verdicts are a table rather than a computation, because what these tests
/// exercise is a producer that answers *wrongly* — and a producer that derived
/// its answers from its own rows could not.
struct ScriptedStream {
    rows: VecDeque<RankedRow<Term>>,
    emitted: u64,
    contract: StreamContract,
    /// The verdict for each candidate, by canonical lexical. A candidate not in
    /// the table is `Possible`, which is the honest answer for a producer that
    /// cannot rule one out.
    verdicts: BTreeMap<String, ExclusionVerdict>,
    /// A candidate whose lookup fails outright rather than answering.
    failing: Option<String>,
    /// How many lookups this stream was asked, so a test can show the mechanism
    /// really ran.
    asked: u64,
}

impl ScriptedStream {
    fn new(contract: StreamContract, rows: Vec<RankedRow<Term>>) -> Self {
        Self {
            rows: rows.into(),
            emitted: 0,
            contract,
            verdicts: BTreeMap::new(),
            failing: None,
            asked: 0,
        }
    }

    fn excluding(mut self, candidates: &[&str]) -> Self {
        for candidate in candidates {
            self.verdicts.insert(
                term(candidate).as_str().to_owned(),
                ExclusionVerdict::Excluded,
            );
        }
        self
    }

    fn failing_on(mut self, candidate: &str) -> Self {
        self.failing = Some(term(candidate).as_str().to_owned());
        self
    }
}

#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        Ok(self.rows.pop_front().inspect(|_| self.emitted += 1))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.emitted,
        })
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    async fn exclusion(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        self.asked += 1;
        // The mock honours its own contract, because a mock that answered where
        // it declared nothing would hide exactly the disagreement this variant
        // exists to name.
        if !self.contract.exclusion.is_declared() {
            return Err(ProtocolError::ExclusionUnavailable);
        }
        if self.failing.as_deref() == Some(candidate.as_str()) {
            return Err(ProtocolError::ExclusionLookupFailed {
                stratum: ex("stratum/scripted"),
                reason: "the fixture dataset read was refused".to_owned(),
            });
        }
        Ok(self
            .verdicts
            .get(candidate.as_str())
            .copied()
            .unwrap_or(ExclusionVerdict::Possible))
    }
}

/// The canonical lexical of one fixture candidate.
fn term(suffix: &str) -> Term {
    Term::new(format!("<{}>", ex(suffix)))
}

/// One row of a fixture stream, drawn from `block`.
fn row(rank: u64, item: &str, block: Option<&str>) -> RankedRow<Term> {
    RankedRow::new(
        rank,
        contribution_under(decay(), Fixed::ONE, rank).expect("fixture contributions are in range"),
        term(item),
        block.map_or(RowBlock::Undeclared, |block| RowBlock::Declared(tag(block))),
    )
}

/// A contract over `domains`, declaring `basis`.
fn contract(domains: CandidateDomains, basis: ExclusionBasis) -> StreamContract {
    StreamContract::new(DuplicatePolicy::Unique, RankFidelity::EXACT, domains, basis)
}

fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        decay(),
    )
    .expect("the fixture profile is valid")
}

/// Fuse the two scripted streams at `top_k`.
fn fused(
    left: ScriptedStream,
    right: ScriptedStream,
    top_k: TopK,
) -> Result<Vec<(String, Fixed)>, FusionError> {
    let [left_iri, right_iri] = strata();
    let profile = profile();
    block_on(fuse::<ScriptedStream, Term>(
        vec![(left_iri, left), (right_iri, right)],
        &profile,
        top_k,
    ))
    .map(|result| {
        result
            .rows
            .into_iter()
            .map(|fused_row| (fused_row.entity.as_str().to_owned(), fused_row.score))
            .collect()
    })
}

/// The shared block both producers in the sections below declare.
const SHARED: &str = "domain/shared";

/// A stream over `SHARED` declaring membership lookups.
fn sharer(rows: Vec<RankedRow<Term>>) -> ScriptedStream {
    ScriptedStream::new(
        contract(
            CandidateDomains::within([tag(SHARED)]),
            ExclusionBasis::Membership,
        ),
        rows,
    )
}

/// The left stream of the contradiction fixture: four candidates it alone holds.
fn left_rows() -> Vec<RankedRow<Term>> {
    vec![
        row(1, "x1", Some(SHARED)),
        row(2, "x2", Some(SHARED)),
        row(3, "x3", Some(SHARED)),
        row(4, "x4", Some(SHARED)),
    ]
}

/// **A producer that excludes a candidate and then names it is refused by
/// name.**
///
/// The right stream answers `Excluded` for `x1`, which is what lets the fusion
/// stop waiting for it — and then emits a row naming `x1` at rank two. Both
/// statements cannot be true, and the refusal names the one that was
/// contradicted rather than blaming the domain declaration, which was never
/// broken.
///
/// The neighbour differs in exactly that row: the right stream keeps the same
/// verdict and names something else, and the fusion succeeds.
#[test]
fn excluding_a_candidate_and_then_naming_it_is_refused() {
    let contradicting = fused(
        sharer(left_rows()),
        sharer(vec![row(1, "y1", Some(SHARED)), row(2, "x1", Some(SHARED))]).excluding(&["x1"]),
        TopK::new(4),
    );
    match contradicting {
        Err(FusionError::Protocol(error)) => match *error {
            ProtocolError::ExclusionContradicted { item, stratum } => {
                assert_eq!(item, term("x1").as_str(), "the refusal names the candidate");
                assert_eq!(
                    stratum,
                    ex("stratum/right"),
                    "and the producer that contradicted itself"
                );
            }
            other => panic!("expected ExclusionContradicted, got {other:?}"),
        },
        other => panic!("expected a protocol refusal, got {other:?}"),
    }

    let truthful = fused(
        sharer(left_rows()),
        sharer(vec![row(1, "y1", Some(SHARED)), row(2, "y2", Some(SHARED))]).excluding(&["x1"]),
        TopK::new(4),
    )
    .expect("a truthful exclusion fuses");
    assert_eq!(
        truthful.len(),
        4,
        "the neighbour differs in one row and answers normally"
    );
    assert!(
        truthful.iter().any(|(item, _)| item == term("x1").as_str()),
        "including the very candidate the right stream excluded, which the left \
         stream really holds: {truthful:?}"
    );
}

/// **A producer that calls a candidate possible while its own declared domains
/// put that candidate out of reach is refused as the domain violation it is.**
///
/// The left stream is unrestricted and names `x1` from the `documents` block —
/// evidence about the *candidate*, which is why an unrestricted stream's block
/// is honoured. The right stream declares `CandidateDomains::Within` over
/// `people` alone, so it has promised never to name anything in `documents`; a
/// `Possible` verdict for `x1` is that promise broken, and it is broken by the
/// answer rather than by a row.
///
/// The intersection of the *declarations* does not catch this: the only namer is
/// unrestricted, so nothing about the declarations alone rules the right stream
/// out. It is the placement the row established that does, which is why the
/// check reads it.
///
/// The neighbour differs in exactly the block the candidate was placed in.
#[test]
fn calling_a_candidate_possible_outside_the_declared_domain_is_refused() {
    let people_only = || {
        ScriptedStream::new(
            contract(
                CandidateDomains::within([tag("domain/people")]),
                ExclusionBasis::Membership,
            ),
            vec![
                row(1, "p1", Some("domain/people")),
                row(2, "p2", Some("domain/people")),
            ],
        )
    };
    let unrestricted = |block: &str| {
        ScriptedStream::new(
            contract(CandidateDomains::Unrestricted, ExclusionBasis::Unavailable),
            vec![row(1, "x1", Some(block)), row(2, "x2", Some(block))],
        )
    };

    match fused(
        unrestricted("domain/documents"),
        people_only(),
        TopK::new(3),
    ) {
        Err(FusionError::Protocol(error)) => match *error {
            ProtocolError::OutsideDeclaredDomain { item, stratum, .. } => {
                assert_eq!(item, term("x1").as_str(), "the refusal names the candidate");
                assert_eq!(
                    stratum,
                    ex("stratum/right"),
                    "and the producer whose declaration it contradicts"
                );
            }
            other => panic!("expected OutsideDeclaredDomain, got {other:?}"),
        },
        other => panic!("expected a protocol refusal, got {other:?}"),
    }

    let in_domain = fused(unrestricted("domain/people"), people_only(), TopK::new(3))
        .expect("a candidate inside the declared block fuses");
    assert_eq!(
        in_domain.len(),
        3,
        "the neighbour differs only in the block the row named: {in_domain:?}"
    );
}

/// **A lookup that fails fails the fused request, and does not degrade.**
///
/// The right stream's lookup for `x1` returns a typed failure rather than a
/// verdict. The whole request fails with that error, unchanged, because the only
/// other thing it could become is `Possible` — which reads as the safe,
/// conservative answer and is indistinguishable from a working lookup that
/// answered honestly. Nothing downstream could ever tell the two apart: the
/// answer stays correct and the read merely stops narrowing.
///
/// The neighbour is the identical fixture with the failure removed, which fuses
/// — so what fails the request is the failure and not the fixture.
#[test]
fn a_failed_lookup_fails_the_request_rather_than_answering_possible() {
    match fused(
        sharer(left_rows()),
        sharer(vec![row(1, "y1", Some(SHARED)), row(2, "y2", Some(SHARED))]).failing_on("x1"),
        TopK::new(4),
    ) {
        Err(FusionError::Protocol(error)) => match *error {
            ProtocolError::ExclusionLookupFailed { stratum, reason } => {
                assert_eq!(
                    stratum,
                    ex("stratum/scripted"),
                    "the refusal carries the producer's own naming of itself"
                );
                assert_eq!(
                    reason, "the fixture dataset read was refused",
                    "and its own reason, verbatim"
                );
            }
            other => panic!("expected ExclusionLookupFailed, got {other:?}"),
        },
        other => panic!("expected a protocol refusal, got {other:?}"),
    }

    let working = fused(
        sharer(left_rows()),
        sharer(vec![row(1, "y1", Some(SHARED)), row(2, "y2", Some(SHARED))]).excluding(&["x1"]),
        TopK::new(4),
    )
    .expect("the same fixture without the failure fuses");
    assert_eq!(
        working.len(),
        4,
        "so the failure is what failed the request, not the shape of the \
         fixture: {working:?}"
    );
}

/// **A stream that declares no basis is never asked, and says so if it is.**
///
/// The fusion engine reads the declared basis once, before any row is pulled,
/// and asks only the streams that declared one. A stream that declares
/// `Unavailable` and is asked anyway answers
/// [`ProtocolError::ExclusionUnavailable`] — not a fabricated `Possible`, which
/// would be this layer's opinion about a host's corpus.
///
/// Both halves are executed: the fusion runs to completion over a pair in which
/// one stream declares nothing, proving the engine did not ask it; and the same
/// stream is then asked directly, proving what it would have said.
#[test]
fn a_stream_that_declares_no_basis_is_not_asked_and_refuses_if_it_is() {
    let silent = || {
        ScriptedStream::new(
            contract(
                CandidateDomains::within([tag(SHARED)]),
                ExclusionBasis::Unavailable,
            ),
            vec![row(1, "y1", Some(SHARED)), row(2, "y2", Some(SHARED))],
        )
    };
    let answer = fused(sharer(left_rows()), silent(), TopK::new(4))
        .expect("a fusion over a stream that answers no lookup is the fusion it always was");
    assert_eq!(answer.len(), 4, "and the answer is unchanged: {answer:?}");

    let mut asked = silent();
    assert!(
        matches!(
            block_on(asked.exclusion(&term("x1"))),
            Err(ProtocolError::ExclusionUnavailable)
        ),
        "a stream asked for a verdict it declared no basis for names the \
         disagreement rather than inventing an answer"
    );
    assert_eq!(
        asked.asked, 1,
        "and the ask really reached it, so the refusal above is the stream's \
         own and not a filter in front of it"
    );
}
