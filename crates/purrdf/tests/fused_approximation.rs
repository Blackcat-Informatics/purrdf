// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One exact producer and one approximate producer, fused, through the whole
//! shipped ladder.
//!
//! Every other test of this channel drives a mock. This one cannot: the claim
//! is that the string `purrdf-hnsw` publishes is the string a consumer reads
//! out of a fused answer, and a mock that carries a string it was handed proves
//! the plumbing rather than the claim. The umbrella crate is the only one that
//! depends on both `purrdf-hnsw` and `purrdf-retrieval`, so this is the only
//! place the two ends can be held up against each other.
//!
//! The two producers are deliberately given the **same** entities. A text index
//! over one set of subjects and a vector space over another would fuse into an
//! answer in which nothing co-occurs, every row would carry one contribution,
//! and the test would pass while proving nothing about composition.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf::hnsw::relation::{HnswSpace, register_ranked_hnsw_relation};
use purrdf::hnsw::{HnswIndex, Params, VectorMatrix, profile};
use purrdf::sparql::{
    CandidateDomains, Completeness, IndexGeneration, KnnGuard, OrderFidelity,
    PropertyFunctionRegistry, RankFidelity, TermKind,
};
use purrdf::{DistanceMetric, RdfDatasetBuilder, RdfLiteral, TermValue, retrieval, text};

const NOTE: &str = "https://example.org/note";
const TEXT_PRODUCER: &str = "https://example.org/pf/search";
const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
const HNSW_PRODUCER: &str = "https://example.org/pf/nearest";
const HNSW_STRATUM: &str = "https://example.org/stratum/vector";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The entities BOTH producers name. One corpus, ranked twice under two laws,
/// which is the shape a fused answer exists for.
const ENTITIES: [(&str, &str); 4] = [
    ("a", "the quick brown fox"),
    ("b", "a quick red fox"),
    ("c", "a slow brown badger"),
    ("d", "the quick silver hare"),
];

fn entity_iri(local: &str) -> String {
    format!("https://example.org/{local}")
}

/// The ladder's futures are awaited in one task and never cross a thread
/// boundary, so a parking waker is the whole runtime they need.
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

/// A provider that reports nothing: both producers declare finite row bounds
/// from their own frozen indexes.
struct NoStatistics;
impl retrieval::Statistics for NoStatistics {
    fn source(&self) -> &'static str {
        "fused-approximation-fixture"
    }
    fn revision(&self) -> &'static str {
        "r1"
    }
    fn cardinality(&self, _predicate: &retrieval::Iri) -> Option<u64> {
        None
    }
    fn selectivity_ppm(
        &self,
        _subject: &retrieval::Iri,
        _term: &retrieval::RequestTerm,
    ) -> Option<u64> {
        None
    }
}

/// A registry holding the exhaustive text producer and the approximate vector
/// producer, over the same four entities.
fn registry() -> PropertyFunctionRegistry {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, body) in ENTITIES {
        let subject = builder.intern_iri(&entity_iri(local));
        let literal = builder.intern_literal(RdfLiteral::simple(body));
        builder.push_quad(subject, note, literal, None);
    }
    let dataset = builder.freeze().expect("the fixture validates");

    let config = text::TextIndexConfig::new(vec![TermValue::iri(NOTE)], text::GraphSelector::Any)
        .expect("one IRI predicate is a well-formed configuration");
    let index = text::TextIndex::from_dataset(&*dataset, &config).expect("the index builds");

    let mut registry = PropertyFunctionRegistry::new();
    let relation = text::TextSearchRelation::new(Arc::new(index));
    let declaration = relation
        .ranked_declaration(
            purrdf::iri::parse(TEXT_STRATUM).expect("the fixture stratum IRI is valid"),
            Some(NOTE.to_owned()),
            // This index holds every document the fixture dataset has, so the
            // exhaustive declaration is the true one -- and it is the control
            // the approximate producer beside it is read against.
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PRODUCER, Arc::new(relation), declaration);

    register_ranked_hnsw_relation(
        &mut registry,
        HNSW_PRODUCER,
        vector_space(),
        purrdf::iri::parse(HNSW_STRATUM).expect("the fixture stratum IRI is valid"),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        CandidateDomains::Unrestricted,
    );
    registry
}

/// A deterministic vector space over the SAME entities the text index holds.
fn vector_space() -> Arc<HnswSpace> {
    let dims = 4;
    let rows = ENTITIES.len();
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let value = ((z >> 11) as f64 / (1u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    let matrix = VectorMatrix::new(rows, dims, data).expect("a valid matrix");
    let index = HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params())
        .expect("the graph builds");
    let terms: Vec<TermValue> = ENTITIES
        .iter()
        .map(|(local, _)| TermValue::iri(entity_iri(local)))
        .collect();
    let guard = KnnGuard::new(rows as u64, rows as u64).expect("a valid guard");
    Arc::new(HnswSpace::from_index(index, terms, guard).expect("a valid space"))
}

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid parameters")
}

/// A request that reaches both producers: lexical text for the inverted index,
/// an entity seed for the graph.
fn request() -> retrieval::RetrievalRequest {
    retrieval::RetrievalRequest::bounded(
        vec![
            retrieval::RequestTerm::Lexical {
                text: "quick fox".to_owned(),
                language: None,
                predicate: Some(
                    retrieval::Iri::parse(NOTE).expect("the fixture predicate is valid"),
                ),
            },
            retrieval::RequestTerm::EntitySeed {
                entity: retrieval::Term::new(format!("<{}>", entity_iri("a"))),
            },
        ],
        retrieval::TopK::new(8),
    )
}

fn profile() -> retrieval::FusionProfile {
    let mut weights = BTreeMap::new();
    for stratum in [TEXT_STRATUM, HNSW_STRATUM] {
        weights.insert(
            retrieval::Iri::parse(stratum).expect("the fixture stratum IRI is valid"),
            retrieval::Fixed::ONE,
        );
    }
    retrieval::FusionProfile::with_decay(weights, retrieval::DecayRule::ReciprocalRank { k: 60 })
        .expect("the fixture fusion profile is valid")
}

fn search() -> retrieval::SearchResult {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, body) in ENTITIES {
        let subject = builder.intern_iri(&entity_iri(local));
        let literal = builder.intern_literal(RdfLiteral::simple(body));
        builder.push_quad(subject, note, literal, None);
    }
    let dataset = builder.freeze().expect("the fixture validates");

    let registry = registry();
    let statistics = NoStatistics;
    let profile = profile();
    let environment = retrieval::AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    block_on(retrieval::search(
        &request(),
        &registry,
        &statistics,
        &*dataset,
        &environment,
        &profile,
    ))
    .expect("the ladder composes both producers")
}

fn stratum(iri: &str) -> retrieval::Iri {
    retrieval::Iri::parse(iri).expect("the fixture stratum IRI is valid")
}

#[test]
fn a_fused_answer_names_which_stratum_was_served_approximately() {
    let result = search();
    let text = stratum(TEXT_STRATUM);
    let hnsw = stratum(HNSW_STRATUM);

    // The two strata are distinguishable from the returned answer alone. No
    // registry is consulted here.
    assert_eq!(
        result.trailer.fidelities[&text],
        RankFidelity::EXACT,
        "the exhaustive producer reports the top of the lattice"
    );
    let approximate = &result.trailer.fidelities[&hnsw];
    assert!(
        approximate.may_omit(),
        "and the approximate one does not: {approximate:?}"
    );

    // The producer's own loss evidence, verbatim through the whole ladder,
    // against the shipped constant.
    let Completeness::Lossy { evidence } = &approximate.completeness else {
        panic!("an HNSW search offers candidates and never certifies absence");
    };
    assert_eq!(
        &**evidence,
        profile::LOSS_EVIDENCE,
        "the string `purrdf-hnsw` publishes is the string a consumer reads out \
         of a fused answer -- byte for byte, across four crates, with nothing \
         re-wording it on the way"
    );
    assert_eq!(
        approximate.order,
        OrderFidelity::Faithful,
        "a graph over untransformed vectors compares exact distances for the \
         candidates it visits, so it is lossy but order-faithful, and the \
         answer's error stays bounded"
    );
}

#[test]
fn the_answer_is_estimated_and_names_only_the_responsible_stratum() {
    let result = search();
    let hnsw = stratum(HNSW_STRATUM);

    let retrieval::ScoreExactness::Estimated {
        deficit,
        inflation,
        unbounded,
    } = &result.trailer.exactness
    else {
        panic!("an approximate stratum makes the answer an estimate");
    };
    // Precise, not blanket: the exhaustive stratum is NOT swept up with it.
    assert_eq!(deficit.len(), 1, "{deficit:?}");
    assert!(deficit.contains(&hnsw));
    assert_eq!(
        inflation, deficit,
        "a stratum that misses a row also promotes every row behind it, so it \
         is named on both sides"
    );
    assert!(
        unbounded.is_empty(),
        "the order is faithful, so the error is bounded: {unbounded:?}"
    );
}

#[test]
fn a_status_and_a_fidelity_are_read_together_end_to_end() {
    // A status is not, on its own, a completeness claim -- on the shipped
    // producers. The status the approximate stratum reports
    // is an ordinary read ending -- the same one an exhaustive producer
    // reports -- which is exactly why it cannot be the channel that carries
    // the approximation.
    let result = search();
    let hnsw = stratum(HNSW_STRATUM);
    assert!(
        result.trailer.statuses.contains_key(&hnsw),
        "the approximate stratum reports a terminal status like any other"
    );
    assert!(
        result.trailer.fidelities[&hnsw].may_omit(),
        "and the declaration beside it qualifies what that status may be read \
         to claim"
    );
}

#[test]
fn the_answer_bounds_each_row_and_names_its_certain_prefix() {
    let result = search();
    let hnsw = stratum(HNSW_STRATUM);

    let mut named_by_hnsw = 0;
    for row in &result.rows {
        let retrieval::ScoreInterval::Bounded {
            deficit: _,
            inflation,
        } = &row.interval
        else {
            panic!("a faithful order leaves every row bounded");
        };
        if row.contributions.iter().any(|(s, _, _)| *s == hnsw) {
            named_by_hnsw += 1;
            assert!(
                *inflation > retrieval::Fixed::ZERO,
                "a row the approximate stratum named may have been promoted by \
                 a row it missed, so its contribution from that stratum is \
                 suspect: {:?}",
                row.entity
            );
        } else {
            assert_eq!(
                *inflation,
                retrieval::Fixed::ZERO,
                "and a row it never named was promoted by nothing: {:?}",
                row.entity
            );
        }
    }
    assert!(
        named_by_hnsw > 0,
        "the fixture must actually fuse the approximate stratum, or this test \
         proves nothing"
    );

    // Non-vacuous, and in the direction that matters: a real approximate
    // producer must be able to COST certainty, or the number is decorative.
    let certain = result.trailer.certain_prefix(&result.rows);
    assert!(
        certain < result.rows.len(),
        "the approximate stratum contributed to rows whose intervals now \
         overlap, so not every place is settled; got {certain} of {}",
        result.rows.len()
    );

    // And the bound is CONSERVATIVE, which is worth stating where a reader will
    // meet it. For a row a lossy stratum named, the whole of that stratum's
    // contribution is charged as possibly-spurious, because the true rank is
    // only known to be at least the emitted one. A producer that could bound
    // how far its beam displaces a rank would tighten this sharply -- and none
    // can, which is why no such term is declared: a field only ever filled with
    // "unbounded" would be a hollow surface. So an answer in which an
    // approximate stratum touches every row may legitimately certify nothing,
    // and that is the honest floor rather than a defect.
    let control = text_only_search();
    assert_eq!(
        control.trailer.certain_prefix(&control.rows),
        control.rows.len(),
        "the same corpus through the exhaustive producer alone certifies every \
         row it returns, so the shortfall above is the approximation's and not \
         an artefact of the fixture"
    );
}

/// The same corpus and request through the exhaustive producer alone.
///
/// The control the assertion above is read against: without it, a certain
/// prefix shorter than the answer proves only that this fixture is hard, not
/// that approximation is what made it so.
fn text_only_search() -> retrieval::SearchResult {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, body) in ENTITIES {
        let subject = builder.intern_iri(&entity_iri(local));
        let literal = builder.intern_literal(RdfLiteral::simple(body));
        builder.push_quad(subject, note, literal, None);
    }
    let dataset = builder.freeze().expect("the fixture validates");

    let config = text::TextIndexConfig::new(vec![TermValue::iri(NOTE)], text::GraphSelector::Any)
        .expect("one IRI predicate is a well-formed configuration");
    let index = text::TextIndex::from_dataset(&*dataset, &config).expect("the index builds");
    let mut registry = PropertyFunctionRegistry::new();
    let relation = text::TextSearchRelation::new(Arc::new(index));
    let declaration = relation
        .ranked_declaration(
            purrdf::iri::parse(TEXT_STRATUM).expect("the fixture stratum IRI is valid"),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PRODUCER, Arc::new(relation), declaration);

    let mut weights = BTreeMap::new();
    weights.insert(stratum(TEXT_STRATUM), retrieval::Fixed::ONE);
    let profile = retrieval::FusionProfile::with_decay(
        weights,
        retrieval::DecayRule::ReciprocalRank { k: 60 },
    )
    .expect("the fixture fusion profile is valid");
    let statistics = NoStatistics;
    let environment = retrieval::AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    block_on(retrieval::search(
        &retrieval::RetrievalRequest::bounded(
            vec![retrieval::RequestTerm::Lexical {
                text: "quick fox".to_owned(),
                language: None,
                predicate: Some(
                    retrieval::Iri::parse(NOTE).expect("the fixture predicate is valid"),
                ),
            }],
            retrieval::TopK::new(8),
        ),
        &registry,
        &statistics,
        &*dataset,
        &environment,
        &profile,
    ))
    .expect("the exhaustive producer composes alone")
}

#[test]
fn the_approximate_stratum_declares_the_generation_that_answered() {
    let result = search();
    let hnsw = stratum(HNSW_STRATUM);
    let attestation = &result.trailer.attestations[&hnsw];
    let IndexGeneration::Declared(generation) = &attestation.generation else {
        panic!("the space names the graph that answered");
    };
    assert_eq!(generation.len(), 64, "a lowercase hex content digest");
    assert!(
        generation
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    );
}

#[test]
fn the_whole_answer_is_byte_identical_across_two_runs() {
    // Determinism through the ladder, not just inside the graph: the same
    // question over the same corpus gives the same rows, the same plan and the
    // same trailer, including everything this work added to it.
    let first = search();
    let second = search();
    assert_eq!(first.rows, second.rows);
    assert_eq!(first.trailer.fidelities, second.trailer.fidelities);
    assert_eq!(first.trailer.exactness, second.trailer.exactness);
    assert_eq!(first.trailer.statuses, second.trailer.statuses);
    assert_eq!(first.evidence_id, second.evidence_id);
    assert_eq!(first.profile_id, second.profile_id);
}
