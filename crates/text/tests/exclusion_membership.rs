// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a candidate-bound invocation of ranked retrieval costs, and what it
//! answers.
//!
//! `TextSearchRelation` declares `ExclusionBasis::Membership`, which is a promise
//! with two halves: that *do you hold this document* is answered cheaply, and
//! that a "no" is exact. Both halves are executed here, and neither is argued
//! from timing.
//!
//! **The cost** is counted rather than measured. `SearchObservations` reports how
//! many membership lookups the relation performed and how many invocations
//! entered the partition ranker at all, and the relation has exactly one call
//! site into that ranker, so a zero is a statement that nothing was ranked rather
//! than an inference about how much was. A fixture small enough to run in a test
//! is small enough that ranking it and not ranking it take the same measurable
//! time, so a wall-clock assertion here would pass over an implementation that
//! ranks the whole index.
//!
//! **The exactness** is walked, not sampled. Every document of the fixture is
//! asked in both directions against the streamed answer, for every needle: a
//! document the stream names must be answered with a row, and a document it does
//! not name must be answered with none. The scores are compared with `==` on
//! `Fixed` — the arithmetic is exact integer fixed point, so a tolerance here
//! would be admitting a divergence the type cannot have.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_eval::{
    CandidateDomains, Completeness, DuplicatePolicy, EvalError, ExclusionBasis, OrderFidelity,
    PfArgs, PfRow, PropertyFunction, PropertyFunctionRegistry, RankFidelity, RankedDeclaration,
};
use purrdf_text::{
    Analyzer, Fixed, GraphSelector, PartitionFilter, Scored, TextIndex, TextIndexConfig,
    TextSearchRelation, explain, select,
};

/// The one predicate whose literals the fixture indexes.
const NOTE: &str = "http://example.org/note";

/// The caller-supplied IRI the registration tests call ranked retrieval by.
const SEARCH: &str = "http://example.org/pf#search";

/// The stratum the registration tests declare a producer for.
const STRATUM: &str = "http://example.org/stratum/text";

/// Six documents in one partition — one graph, no language tags — chosen so that
/// every interesting membership case is present:
///
/// ```text
/// d1  "quick quick quick brown"   holds both needle terms, most of "quick"
/// d2  "quick quick brown fox"     holds both
/// d3  "quick brown fox jumps"     holds both
/// d4  "quick fox jumps high"      holds "quick" only
/// d5  "lazy dog sleeps late"      holds NEITHER — the excluded document
/// d6  "brown bear sleeps late"    holds "brown" only
/// ```
///
/// `d5` is what makes the point-lookup assertion non-vacuous, and `d4` and `d6`
/// are what stop "held a term" from collapsing into "held every term": a
/// document holding one of two needle terms is a candidate and must be answered
/// with a row.
const ROWS: [(&str, &str); 6] = [
    ("d1", "quick quick quick brown"),
    ("d2", "quick quick brown fox"),
    ("d3", "quick brown fox jumps"),
    ("d4", "quick fox jumps high"),
    ("d5", "lazy dog sleeps late"),
    ("d6", "brown bear sleeps late"),
];

/// The needles every exhaustive walk below runs, chosen to cover the whole range
/// of membership outcomes over [`ROWS`]: a needle most documents hold, one a
/// single document holds, one no document holds, one whose terms are spread
/// across every document, and one that analyzes to no terms at all.
const NEEDLES: [&str; 5] = [
    "quick brown",
    "lazy dog",
    "zebra",
    "quick brown fox jumps high lazy dog sleeps late bear",
    "--- ...",
];

// ── fixtures ─────────────────────────────────────────────────────────────────

fn corpus() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text) in ROWS {
        let subject = builder.intern_iri(&format!("http://example.org/{local}"));
        let object = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, note, object, None);
    }
    builder.freeze().expect("the fixture must validate")
}

fn index() -> Arc<TextIndex> {
    let dataset = corpus();
    Arc::new(
        TextIndex::from_dataset(
            &dataset,
            &TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
                .expect("the fixture configuration is well formed"),
        )
        .expect("the fixture indexes"),
    )
}

/// The analyzed tokens of `text` — the query side of the one pipeline the index
/// side used, spelled the way the relation spells it.
fn analyze(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    Analyzer::new().analyze(text, &mut tokens);
    tokens
        .into_iter()
        .map(|token| token.text.into_owned())
        .collect()
}

/// How many **distinct** terms a needle analyzes to, which is the number of
/// membership lookups one candidate-bound invocation performs per document the
/// bound subject occupies.
fn distinct_terms(needle: &str) -> u64 {
    let mut terms = analyze(needle);
    terms.sort_unstable();
    terms.dedup();
    terms.len() as u64
}

/// A plain-string needle in the relation's argument vector, with every other
/// position free.
fn search_args(needle: &str) -> Vec<Option<TermValue>> {
    vec![
        None,
        Some(TermValue::simple_literal(needle)),
        None,
        None,
        None,
        None,
    ]
}

fn invoke(
    relation: &TextSearchRelation,
    bound: &[Option<TermValue>],
) -> Result<Vec<PfRow>, EvalError> {
    let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
    let (subject, object) = refs.split_at(relation.arity().subject);
    let args = PfArgs::new(subject, object);
    let mut cursor = relation.open(&args, None)?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        rows.push(row);
    }
    Ok(rows)
}

/// The whole ranked answer for `needle`, as the streaming path produces it: every
/// partition, no ceiling, no bound position.
fn streamed(index: &TextIndex, needle: &str) -> Vec<Scored> {
    select(
        index,
        &analyze(needle),
        &PartitionFilter::unconstrained(),
        None,
        None,
    )
    .expect("the fixture needles rank")
}

/// The subject of every document of the index, in document-id order.
fn subjects(index: &TextIndex) -> Vec<TermValue> {
    index
        .documents()
        .map(|document| document.subject().clone())
        .collect()
}

// ── 1. The point lookup, measured ────────────────────────────────────────────

/// **A candidate-bound invocation over a document the needle does not reach
/// performs exactly one membership lookup per needle term and does not enter the
/// partition ranker — and its present neighbour, which does.**
///
/// Both halves are in one test because either alone passes over a defect. An
/// assertion that the excluded case ranks nothing passes trivially over a
/// relation that has stopped answering at all; an assertion that the present
/// case answers passes over a relation that ranks the whole index every time. The
/// pair pins the difference: same needle, same relation, two documents, and the
/// counters move differently.
#[test]
fn a_candidate_bound_invocation_is_a_point_lookup() {
    let relation = TextSearchRelation::new(index());
    let observed = relation.observations();
    let needle = "quick brown";
    let terms = distinct_terms(needle);
    assert_eq!(
        terms, 2,
        "the fixture needle analyzes to two distinct terms"
    );
    assert_eq!(
        relation.index().partition_count(),
        1,
        "one partition, so a subject occupies exactly one document"
    );

    // `d5` holds neither needle term.
    let mut bound = search_args(needle);
    bound[0] = Some(TermValue::iri("http://example.org/d5"));
    assert_eq!(
        invoke(&relation, &bound).expect("a bound needle"),
        Vec::<PfRow>::new(),
        "a document holding no needle term is in no candidate set, so it is named \
         at no rank"
    );
    assert_eq!(
        observed.membership_lookups(),
        terms,
        "one binary search per needle term over the one document that subject occupies"
    );
    assert_eq!(
        observed.rankings(),
        0,
        "and nothing was ranked: no posting list walked, no candidate set built, no \
         corpus statistic computed"
    );

    // `d3` holds both of them. The answer needs a rank, which is a fact about
    // every other candidate of the partition, so this one does rank — and the
    // membership lookups are the same handful either way.
    let mut bound = search_args(needle);
    bound[0] = Some(TermValue::iri("http://example.org/d3"));
    let rows = invoke(&relation, &bound).expect("a bound needle");
    assert_eq!(rows.len(), 1, "a present document answers with its one row");
    assert_eq!(
        observed.membership_lookups(),
        2 * terms,
        "the present document costs exactly the same lookups as the absent one"
    );
    assert_eq!(
        observed.rankings(),
        1,
        "exactly one invocation — the present one — entered the ranker"
    );

    // A needle that analyzes to no terms costs nothing at all: no term to look
    // up, and nothing to rank.
    let before = observed.membership_lookups();
    let mut bound = search_args("--- ...");
    bound[0] = Some(TermValue::iri("http://example.org/d3"));
    assert_eq!(
        invoke(&relation, &bound).expect("a bound needle"),
        Vec::<PfRow>::new(),
        "a needle naming no term matches nothing, which is an answer rather than a \
         refusal"
    );
    assert_eq!(observed.membership_lookups(), before, "no term to look up");
    assert_eq!(observed.rankings(), 1, "and still nothing further ranked");
}

/// **A free `?doc` still enters the ranker, and performs no membership lookup at
/// all.**
///
/// The control for the test above: the counters are not merely stuck. This is the
/// ordinary retrieval shape, and it is the one that must still rank.
#[test]
fn an_unbound_document_is_not_a_lookup() {
    let relation = TextSearchRelation::new(index());
    let observed = relation.observations();
    let rows = invoke(&relation, &search_args("quick brown")).expect("a bound needle");
    assert_eq!(
        rows.len(),
        5,
        "five of the six documents hold at least one needle term"
    );
    assert_eq!(
        observed.membership_lookups(),
        0,
        "there is no bound document to look up"
    );
    assert_eq!(observed.rankings(), 1, "and the ranker answered the call");
}

// ── 2. Both directions, over every document of the fixture ───────────────────

/// **For every document of the fixture and every needle, the candidate-bound
/// answer agrees with the streamed answer in both directions.**
///
/// Every document the stream names is answered with a row; every document it does
/// not name is answered with none. The whole fixture is walked — the loop is over
/// `index.documents()` and asserts a count at the end, so a fixture that grew
/// cannot quietly stop being covered — and nothing is sampled.
///
/// This is what `ExclusionBasis::Membership` promises, executed against the only
/// oracle that can falsify it: the stream itself.
#[test]
fn every_document_agrees_with_the_stream_in_both_directions() {
    let index = index();
    let relation = TextSearchRelation::new(Arc::clone(&index));
    let subjects = subjects(&index);
    assert_eq!(
        subjects.len(),
        ROWS.len(),
        "the walk must cover every document the fixture holds"
    );

    let mut named = 0_usize;
    let mut excluded = 0_usize;
    for needle in NEEDLES {
        let stream = streamed(&index, needle);
        for (document, subject) in subjects.iter().enumerate() {
            let document = u32::try_from(document).expect("the fixture is small");
            let in_stream = stream.iter().any(|row| row.document == document);

            let mut bound = search_args(needle);
            bound[0] = Some(subject.clone());
            let rows = invoke(&relation, &bound).expect("a bound needle");

            if in_stream {
                named += 1;
                assert_eq!(
                    rows.len(),
                    1,
                    "the stream names document {document} for needle {needle:?}, so the \
                     candidate-bound call must answer with its row"
                );
            } else {
                excluded += 1;
                assert_eq!(
                    rows.len(),
                    0,
                    "the stream does not name document {document} for needle {needle:?}, so \
                     the candidate-bound call must answer with nothing"
                );
            }
        }
    }

    // Both directions were actually exercised. Without this the test would pass
    // over a fixture whose every document is named, or whose every document is
    // excluded, and would then be asserting only one of the two claims.
    assert!(named > 0, "some document must be named by some needle");
    assert!(
        excluded > 0,
        "some document must be excluded by some needle"
    );
    assert_eq!(
        named + excluded,
        NEEDLES.len() * ROWS.len(),
        "every (needle, document) pair is decided exactly once"
    );
}

// ── 3. Exactness ─────────────────────────────────────────────────────────────

/// **`explain`'s point score is the streamed score exactly, for every document of
/// the fixture and every needle — `==` on `Fixed`, with no tolerance.**
///
/// `explain` and the ranker share `PreparedQuery::contribution`, so this is not
/// two implementations agreeing to some number of digits; it is one arithmetic
/// path read from two entry points, and an equality is the only assertion that
/// would catch it becoming two.
///
/// A document the stream does not name has a point score of exactly zero, and
/// that is asserted too: it is the other half of what makes an exclusion sound,
/// because "named at no rank" and "contributes exactly zero" are the same claim.
#[test]
fn the_point_score_is_the_streamed_score_exactly() {
    let index = index();
    let subjects = subjects(&index);
    let mut compared = 0_usize;
    let mut nonzero = 0_usize;

    for needle in NEEDLES {
        let terms = analyze(needle);
        let stream = streamed(&index, needle);
        for document in 0..u32::try_from(subjects.len()).expect("the fixture is small") {
            let mut point = Fixed::ZERO;
            for share in explain(&index, document, &terms).expect("the fixture documents explain") {
                point = point
                    .checked_add(share.contribution)
                    .expect("the fixture scores do not overflow");
            }
            match stream.iter().find(|row| row.document == document) {
                Some(row) => {
                    assert_eq!(
                        point, row.score,
                        "the point score of document {document} for needle {needle:?} must be \
                         the streamed score, exactly"
                    );
                    if point != Fixed::ZERO {
                        nonzero += 1;
                    }
                }
                None => assert_eq!(
                    point,
                    Fixed::ZERO,
                    "a document the stream does not name contributes exactly zero, which is \
                     what makes excluding it sound"
                ),
            }
            compared += 1;
        }
    }

    assert_eq!(
        compared,
        NEEDLES.len() * ROWS.len(),
        "every (needle, document) pair is compared"
    );
    assert!(
        nonzero > 0,
        "the equality must have been asserted against scores that are not all zero, or it \
         would hold over a scorer that returns nothing"
    );
}

// ── 4. What the registry admits this relation's basis against ────────────────

/// The declaration this relation makes for `stratum`, under `fidelity`.
fn declaration(relation: &TextSearchRelation, fidelity: RankFidelity) -> RankedDeclaration {
    relation
        .ranked_declaration(
            purrdf_core::parse_iri(STRATUM).expect("the fixture stratum is a valid IRI"),
            Some(NOTE.to_owned()),
            fidelity,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index has one ranked order")
}

/// Register `declaration` for real, returning the panic message where the
/// registry refused it.
fn refusal(relation: &TextSearchRelation, declaration: RankedDeclaration) -> Option<String> {
    let basis = declaration.exclusion;
    let relation = relation.clone();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(move || {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(SEARCH.to_owned(), Arc::new(relation), declaration);
        // Read back, so an admitted result is a registration that really
        // happened rather than merely one that did not panic.
        assert_eq!(
            registry
                .ranked_declaration(SEARCH)
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

fn lossy() -> RankFidelity {
    RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from("approximate: this index holds a sample of the corpus"),
        },
        order: OrderFidelity::Faithful,
    }
}

/// **The declaration this relation makes carries `Membership`, and the registry
/// admits it because the relation declares the candidate-bound point mode behind
/// it.**
///
/// The two halves of the basis are two different declarations and are asserted
/// separately: the field says what an exclusion *means*, and the mode table says
/// it can be answered cheaply. A basis with no such mode is the registration that
/// would turn a lookup into a table scan, once per frontier candidate per
/// stratum.
#[test]
fn the_declared_basis_is_membership_and_it_registers() {
    let relation = TextSearchRelation::new(index());
    let declared = declaration(&relation, RankFidelity::EXACT);
    assert_eq!(
        declared.exclusion,
        ExclusionBasis::Membership,
        "an exclusion from this producer is a fact about its own term universe"
    );
    assert_eq!(
        declared.duplicates,
        DuplicatePolicy::Unique,
        "the rest of the declaration is unchanged by the basis"
    );
    assert_eq!(
        refusal(&relation, declared),
        None,
        "the candidate-bound point mode this relation declares is exactly the condition \
         the registry admits a basis under"
    );
}

/// **`Search` is admissible from this relation under an exhaustive fidelity, and
/// refused under a lossy one — and `Membership` survives the lossy one.**
///
/// There is no text-specific refusal to assert here, and inventing one would be
/// the over-refusal this workspace treats as the mirror of a silent drop.
/// `TextSearchRelation::ranked_declaration` takes `fidelity` as a **parameter**:
/// BM25 over the index is exhaustive, but whether the index covers what the host
/// means by its corpus is a fact only the host holds, so this relation declares
/// no completeness of its own and cannot be the party that makes `Search` wrong.
///
/// What is asserted instead is the honest shape of it. A host that declares its
/// text producer exhaustive may declare either basis, because for such a producer
/// "my search did not find it" and "I do not hold it" are the same fact. A host
/// that declares it lossy may not declare `Search` — a sampled index that did not
/// find a document has not said the corpus lacks it — and may still declare
/// `Membership`, which is exact about the sample it does hold however lossy the
/// search over it is. That last one is the case a completeness check applied to
/// the wrong axis would wrongly refuse, so it is executed rather than reasoned
/// about.
#[test]
fn search_is_admissible_from_an_exhaustive_host_and_not_from_a_lossy_one() {
    let relation = TextSearchRelation::new(index());

    let mut exhaustive_search = declaration(&relation, RankFidelity::EXACT);
    exhaustive_search.exclusion = ExclusionBasis::Search;
    assert_eq!(
        refusal(&relation, exhaustive_search),
        None,
        "this relation declares no completeness of its own, so a host that declares its \
         search exhaustive may declare the basis that rests on exactly that"
    );

    let mut lossy_search = declaration(&relation, lossy());
    lossy_search.exclusion = ExclusionBasis::Search;
    let refused = refusal(&relation, lossy_search)
        .expect("a lossy search that did not find a document has not said it is absent");
    assert!(
        refused.contains("exclusion basis of search")
            && refused.contains("completeness lossy")
            && refused.contains("ExclusionBasis::Membership"),
        "the refusal names the basis, the axis it conflicts with, and the registration the \
         host probably meant: {refused}"
    );

    assert_eq!(
        refusal(&relation, declaration(&relation, lossy())),
        None,
        "and the neighbour differing in exactly that one term is admitted: a membership \
         answer is about this index's own term universe, which a host's lossy coverage of \
         its corpus does not make less exact"
    );
}
