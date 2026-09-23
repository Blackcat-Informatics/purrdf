// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! The fused read taken on demand against the same read materialised, over **real**
//! producers — and one governed per-candidate exclusion lookup, timed alone.
//!
//! `purrdf-retrieval`'s own `benches/on_demand_read.rs` puts the same two reads side
//! by side over mock producers that mint their rows as they are pulled. This file asks
//! the same question over a real [`TextSearchRelation`] (both for a text+text pair and
//! for the text side of a text+HNSW pair) and a real [`HnswRelation`] over a real
//! approximate-nearest-neighbour graph. It lives in the umbrella rather than in
//! `purrdf-retrieval` because `purrdf-retrieval` depends on `purrdf-hnsw` in neither
//! direction — `purrdf` is the one crate that already depends on both producers and on
//! the retrieval composition layer that reads them.
//!
//! # The three configurations
//!
//! Each producer pair runs in three shapes, named for what they vary:
//!
//! * **A, `distinct_blocks`** — the two producers declare two different domain tags, so
//!   the planner's own merge argument (not an exclusion lookup) tells each stream it can
//!   stop at its own bound. This is the cheap baseline: on-demand and materialised
//!   should sit close together, because neither stream is ever held open on the other's
//!   account.
//! * **B, `shared_block_intersecting`** — both producers declare **one** shared domain
//!   tag and name the **same** candidates (the text+HNSW pair's vector space holds a row
//!   for every one of the text index's subjects), so the fusion certifies rows early:
//!   the on-demand read stops as soon as the fifth row crosses, and the materialised
//!   read has already paid for the whole planned depth of each stratum.
//! * **C, `shared_block_disjoint`** — one shared block, disjoint candidates, with a real
//!   exclusion lookup on both sides (the text side's right-hand or vector-side index
//!   holds documents/rows the other candidate set never touches, so its lookups are
//!   genuine dictionary searches or term-order decisions rather than an absent-subject
//!   shortcut). This is the shape neither bound alone resolves — the shape
//!   `text_exclusion_lookup.rs` and `multimodal_exclusion_lookup.rs` build to prove the
//!   mechanism correct — and it is the shape where an on-demand read that stops at the
//!   threshold crossing is worth the most against a materialised read that has to open
//!   every stratum at its planned depth up front.
//!
//! Each shape runs at two corpus sizes, because A and C's on-demand cost is close to
//! flat in the corpus and the materialised cost is not.
//!
//! # The single lookup
//!
//! `single_lookup` isolates the cost the umbrella's own exclusion-lookup tests could
//! only ever measure in aggregate: one governed per-candidate lookup, prepared once
//! (the stratum is planned, compiled and executed exactly as `search` runs it, and the
//! resulting stream is reused across every timed iteration) and then timed alone,
//! against a real text index and against a real HNSW graph, for a candidate the
//! relation answers `Excluded` and one it answers `Possible`. This is the call
//! `plan → compile → execute → fuse` makes once per frontier candidate against every
//! stratum still open, and no bench in this repository had priced it in isolation
//! before this file.
//!
//! Report-only, per this repository's rule: benches exist so a later change has a
//! number to move, never so a speedup can be asserted. The deterministic counterparts —
//! the shortened read, the answer unmoved, and the lookup counted as one bound
//! invocation serving at most one row — are asserted in `purrdf-retrieval`'s
//! `tests/text_exclusion_lookup.rs` and in this crate's own
//! `tests/multimodal_exclusion_lookup.rs`.

use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf::hnsw::relation::{HnswRelation, HnswSpace};
use purrdf::hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf::retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, Fixed, FusionProfile, Iri,
    OrderFidelity, RECIP_K, RankFidelity, RankedStreamAdapter, RequestTerm, RetrievalRequest,
    Statistics, Term, TopK, compile, execute, fuse, plan, search,
};
use purrdf::sparql::{KnnGuard, PropertyFunctionRegistry, RankedDeclaration, TermKind};
use purrdf::text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};
use purrdf::{DistanceMetric, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

/// The fixture namespace. A bench mints no vocabulary of its own, and a
/// reserved-for-documentation authority is the only one it may put in a term.
fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf::iri::Iri {
    purrdf::iri::parse(text).expect("fixture IRIs are valid")
}

/// A single-threaded executor; nothing here ever pends.
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

/// The needle every text producer here is searched for.
const NEEDLE: &str = "alpha beta";

/// A matching document's text: it always reaches [`NEEDLE`], and its term frequency
/// varies with `at` so a corpus does not score every row identically.
fn matching_text(at: usize) -> String {
    format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim())
}

/// A document that reaches no needle term at all — filler that makes a text lookup a
/// real dictionary search rather than an absent-subject shortcut.
const NON_MATCHING: &str = "zulu yankee xray whiskey";

/// The bound every request in this file searches under.
const TOP_K: TopK = TopK::new(5);

/// The vector space's dimensionality.
const DIMS: usize = 8;

/// The corpus sizes each comparison runs at.
const SIZES: [usize; 2] = [200, 2_000];

/// The corpus size the single-lookup group builds its fixtures at.
const LOOKUP_ROWS: usize = 2_000;

/// The smoothing constant every fusion profile here decays by.
fn recip_k() -> u32 {
    u32::try_from(RECIP_K).expect("the smoothing constant fits")
}

/// The one shared domain tag configurations B and C declare on both producers.
fn shared_block() -> DomainTag {
    DomainTag::parse(&ex("domain/shared")).expect("the fixture domain tag is a valid IRI")
}

/// Statistics that narrow nothing: each stratum holds exactly what its fixture built.
struct Cardinalities(BTreeMap<Iri, u64>);

impl Statistics for Cardinalities {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.0.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// Unit weight for every named stratum under reciprocal-rank decay.
fn profile_over(strata: &[String]) -> FusionProfile {
    let weights = strata
        .iter()
        .map(|stratum| (iri(stratum), Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: recip_k() })
        .expect("the fixture profile is valid")
}

/// The dataset built from `(subject, text)` rows, all under one predicate.
fn build_dataset(rows: &[(String, String)], predicate: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate_id = builder.intern_iri(predicate);
    for (subject, text) in rows {
        let subject_id = builder.intern_iri(subject);
        let object_id = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject_id, predicate_id, object_id, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// One predicate's index: every graph, untagged literals in the default graph — so
/// exactly one partition, which is what `TextSearchRelation::ranked_declaration`
/// requires.
fn text_index(dataset: &RdfDataset, predicate: &str) -> Arc<TextIndex> {
    let config = TextIndexConfig::new(
        vec![TermValue::iri(predicate.to_owned())],
        GraphSelector::Any,
    )
    .expect("the fixture configuration is well formed");
    Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture index builds"))
}

/// Deterministic vectors, splitmix64, spelled here so the fixture depends on no
/// private helper.
fn splitmix_vectors(seed: u64, rows: usize, dims: usize) -> Vec<f64> {
    let mut state = seed;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let value = ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    data
}

/// An HNSW space of `terms.len()` rows, named by `terms`.
fn vector_space_of(terms: &[String]) -> Arc<HnswSpace> {
    let rows = terms.len();
    let data = splitmix_vectors(0x51DE_0000_1234_ABCD_u64, rows, DIMS);
    let matrix = VectorMatrix::new(rows, DIMS, data).expect("a valid matrix");
    // The beam is as wide as the corpus, so the approximate read can actually reach
    // the depth the plan asks for.
    let params = Params::new(8, 16, 64, rows).expect("valid parameters");
    let index =
        HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params).expect("it builds");
    let term_values: Vec<TermValue> = terms
        .iter()
        .map(|term| TermValue::iri(term.clone()))
        .collect();
    let rows_u64 = u64::try_from(rows).expect("the bench corpus fits");
    let guard = KnnGuard::new(rows_u64, rows_u64).expect("a valid guard");
    Arc::new(HnswSpace::from_index(index, term_values, guard).expect("a valid space"))
}

/// The read `search` takes: every stratum on demand.
fn on_demand(
    registry: &PropertyFunctionRegistry,
    statistics: &Cardinalities,
    dataset: &RdfDataset,
    request: &RetrievalRequest,
    profile: &FusionProfile,
) -> usize {
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(profile),
    };
    block_on(search(
        request, registry, statistics, dataset, &env, profile,
    ))
    .expect("the fixture searches")
    .rows
    .len()
}

/// The same answer, read materialised: `execute` at the planned depths, then `fuse`.
fn materialised(
    registry: &PropertyFunctionRegistry,
    statistics: &Cardinalities,
    dataset: &RdfDataset,
    request: &RetrievalRequest,
    profile: &FusionProfile,
) -> usize {
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(profile),
    };
    let planned = plan(request, registry, statistics).expect("the fixture plans");
    let compiled = compile(&planned, &env).expect("the fixture compiles");
    let execution = block_on(execute(&compiled, registry, dataset)).expect("it executes");
    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, profile, &stream.stratum)
                    .expect("both strata are weighted")
                    .with_plan_id(stream.plan_id)
                    .with_fused_bound(stream.fused_bound)
                    .with_attestation(stream.attestation);
            (stream.stratum, adapter)
        })
        .collect();
    block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        profile,
        compiled.fused_bound,
    ))
    .expect("the materialised streams fuse")
    .rows
    .len()
}

/// A shape both producer pairs run in.
#[derive(Clone, Copy)]
enum Shape {
    DistinctBlocks,
    SharedIntersecting,
    SharedDisjoint,
}

const SHAPES: [(Shape, &str); 3] = [
    (Shape::DistinctBlocks, "distinct_blocks"),
    (Shape::SharedIntersecting, "shared_block_intersecting"),
    (Shape::SharedDisjoint, "shared_block_disjoint"),
];

// ---------------------------------------------------------------------------
// The text+text pair
// ---------------------------------------------------------------------------

/// The predicate the left side of a text+text pair is indexed over.
fn left_predicate() -> String {
    ex("predicate/left")
}

/// The predicate the right side of a text+text pair is indexed over.
fn right_predicate() -> String {
    ex("predicate/right")
}

/// One text+text pair's fixture: both sides' `(subject, text)` rows and the domain
/// tag each side declares.
struct TextPair {
    left_rows: Vec<(String, String)>,
    right_rows: Vec<(String, String)>,
    left_block: DomainTag,
    right_block: DomainTag,
}

fn text_pair(shape: Shape, rows: usize) -> TextPair {
    match shape {
        Shape::DistinctBlocks => TextPair {
            left_rows: (0..rows)
                .map(|at| (ex(&format!("distinct-left/{at}")), matching_text(at)))
                .collect(),
            right_rows: (0..rows)
                .map(|at| (ex(&format!("distinct-right/{at}")), matching_text(at)))
                .collect(),
            left_block: DomainTag::parse(&ex("domain/left")).expect("a valid tag"),
            right_block: DomainTag::parse(&ex("domain/right")).expect("a valid tag"),
        },
        Shape::SharedIntersecting => {
            let shared_rows: Vec<(String, String)> = (0..rows)
                .map(|at| (ex(&format!("shared/{at}")), matching_text(at)))
                .collect();
            TextPair {
                left_rows: shared_rows.clone(),
                right_rows: shared_rows,
                left_block: shared_block(),
                right_block: shared_block(),
            }
        }
        Shape::SharedDisjoint => {
            let mut right_rows: Vec<(String, String)> = (0..rows)
                .map(|at| (ex(&format!("b/{at}")), matching_text(at)))
                .collect();
            // A non-matching document under every left subject, so the right side's
            // lookups are real dictionary searches rather than an absent-subject
            // shortcut.
            right_rows
                .extend((0..rows).map(|at| (ex(&format!("a/{at}")), NON_MATCHING.to_owned())));
            TextPair {
                left_rows: (0..rows)
                    .map(|at| (ex(&format!("a/{at}")), matching_text(at)))
                    .collect(),
                right_rows,
                left_block: shared_block(),
                right_block: shared_block(),
            }
        }
    }
}

fn text_pair_dataset(pair: &TextPair) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let left_id = builder.intern_iri(&left_predicate());
    for (subject, text) in &pair.left_rows {
        let subject_id = builder.intern_iri(subject);
        let object_id = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject_id, left_id, object_id, None);
    }
    let right_id = builder.intern_iri(&right_predicate());
    for (subject, text) in &pair.right_rows {
        let subject_id = builder.intern_iri(subject);
        let object_id = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject_id, right_id, object_id, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

fn text_pair_registry(dataset: &RdfDataset, pair: &TextPair) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let sides = [
        (
            left_predicate(),
            ex("pf/left"),
            ex("stratum/left"),
            pair.left_block.clone(),
        ),
        (
            right_predicate(),
            ex("pf/right"),
            ex("stratum/right"),
            pair.right_block.clone(),
        ),
    ];
    for (predicate, producer, stratum, block) in sides {
        let relation = TextSearchRelation::new(text_index(dataset, &predicate));
        let declaration: RankedDeclaration = relation
            .ranked_declaration(
                kernel_iri(&stratum),
                Some(predicate),
                RankFidelity::EXACT,
                CandidateDomains::within([block]),
            )
            .expect("a single-partition index declares a ranked order");
        registry.register_ranked(producer, Arc::new(relation), declaration);
    }
    registry
}

fn text_pair_statistics(pair: &TextPair) -> Cardinalities {
    let mut cardinalities = BTreeMap::new();
    let left_count = pair.left_rows.len() as u64;
    let right_count = pair.right_rows.len() as u64;
    cardinalities.insert(iri(&ex("stratum/left")), left_count);
    cardinalities.insert(iri(&left_predicate()), left_count);
    cardinalities.insert(iri(&ex("stratum/right")), right_count);
    cardinalities.insert(iri(&right_predicate()), right_count);
    Cardinalities(cardinalities)
}

fn text_pair_request() -> RetrievalRequest {
    let terms = [left_predicate(), right_predicate()]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(&predicate)),
        })
        .collect();
    RetrievalRequest::bounded(terms, TOP_K)
}

fn text_pair_profile() -> FusionProfile {
    profile_over(&[ex("stratum/left"), ex("stratum/right")])
}

fn text_text_reads(c: &mut Criterion) {
    for (shape, name) in SHAPES {
        let mut group = c.benchmark_group(format!("multimodal_on_demand_read/text_text/{name}"));
        for &rows in &SIZES {
            let pair = text_pair(shape, rows);
            let dataset = text_pair_dataset(&pair);
            let registry = text_pair_registry(&dataset, &pair);
            let statistics = text_pair_statistics(&pair);
            let request = text_pair_request();
            let profile = text_pair_profile();
            group.bench_with_input(BenchmarkId::new("on_demand", rows), &rows, |b, _| {
                b.iter(|| {
                    black_box(on_demand(
                        &registry,
                        &statistics,
                        &dataset,
                        &request,
                        &profile,
                    ))
                });
            });
            group.bench_with_input(BenchmarkId::new("materialised", rows), &rows, |b, _| {
                b.iter(|| {
                    black_box(materialised(
                        &registry,
                        &statistics,
                        &dataset,
                        &request,
                        &profile,
                    ))
                });
            });
        }
        group.finish();
    }
}

// ---------------------------------------------------------------------------
// The text+HNSW pair
// ---------------------------------------------------------------------------

/// The predicate the text side of a text+HNSW pair is indexed over.
fn note_predicate() -> String {
    ex("predicate/note")
}

/// The datatype the vector producer's neighbour-count argument is written under.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// One text+HNSW pair's fixture: the text side's `(subject, text)` rows, the vector
/// side's row terms, the seed the request's entity term names, and the domain tag
/// each side declares.
struct HnswPair {
    text_rows: Vec<(String, String)>,
    vector_terms: Vec<String>,
    seed: String,
    text_block: DomainTag,
    vector_block: DomainTag,
}

fn hnsw_pair(shape: Shape, rows: usize) -> HnswPair {
    match shape {
        Shape::DistinctBlocks => {
            let text_rows: Vec<(String, String)> = (0..rows)
                .map(|at| (ex(&format!("text-distinct/{at}")), matching_text(at)))
                .collect();
            let vector_terms: Vec<String> = (0..rows)
                .map(|at| ex(&format!("vec-distinct/{at}")))
                .collect();
            HnswPair {
                text_rows,
                seed: vector_terms[0].clone(),
                vector_terms,
                text_block: DomainTag::parse(&ex("domain/text")).expect("a valid tag"),
                vector_block: DomainTag::parse(&ex("domain/vector")).expect("a valid tag"),
            }
        }
        Shape::SharedIntersecting => {
            // The vector space holds a row for every one of the text index's own
            // subjects, so the two strata's candidates fully overlap.
            let subjects: Vec<String> = (0..rows).map(|at| ex(&format!("shared/{at}"))).collect();
            let text_rows = subjects
                .iter()
                .enumerate()
                .map(|(at, subject)| (subject.clone(), matching_text(at)))
                .collect();
            HnswPair {
                text_rows,
                seed: subjects[0].clone(),
                vector_terms: subjects,
                text_block: shared_block(),
                vector_block: shared_block(),
            }
        }
        Shape::SharedDisjoint => {
            // The vector space holds no row for any text subject — the candidate
            // that would otherwise force the vector stratum to be drained — and the
            // text index holds a non-matching document for every vector subject, so
            // its lookups are real dictionary searches.
            let text_subjects: Vec<String> =
                (0..rows).map(|at| ex(&format!("text/{at}"))).collect();
            let vector_terms: Vec<String> = (0..rows).map(|at| ex(&format!("vec/{at}"))).collect();
            let mut text_rows: Vec<(String, String)> = text_subjects
                .iter()
                .enumerate()
                .map(|(at, subject)| (subject.clone(), matching_text(at)))
                .collect();
            text_rows.extend(
                vector_terms
                    .iter()
                    .map(|subject| (subject.clone(), NON_MATCHING.to_owned())),
            );
            HnswPair {
                text_rows,
                seed: vector_terms[0].clone(),
                vector_terms,
                text_block: shared_block(),
                vector_block: shared_block(),
            }
        }
    }
}

fn hnsw_pair_registry(dataset: &RdfDataset, pair: &HnswPair) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();

    let text_relation = TextSearchRelation::new(text_index(dataset, &note_predicate()));
    let text_declaration: RankedDeclaration = text_relation
        .ranked_declaration(
            kernel_iri(&ex("stratum/text")),
            Some(note_predicate()),
            RankFidelity::EXACT,
            CandidateDomains::within([pair.text_block.clone()]),
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(ex("pf/text"), Arc::new(text_relation), text_declaration);

    let vector_relation = HnswRelation::new(vector_space_of(&pair.vector_terms));
    let vector_declaration = vector_relation.ranked_declaration(
        kernel_iri(&ex("stratum/vector")),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        OrderFidelity::Faithful,
        CandidateDomains::within([pair.vector_block.clone()]),
    );
    registry.register_ranked(
        ex("pf/vector"),
        Arc::new(vector_relation),
        vector_declaration,
    );

    registry
}

fn hnsw_pair_statistics(pair: &HnswPair) -> Cardinalities {
    let mut cardinalities = BTreeMap::new();
    let text_count = pair.text_rows.len() as u64;
    cardinalities.insert(iri(&ex("stratum/text")), text_count);
    cardinalities.insert(iri(&note_predicate()), text_count);
    cardinalities.insert(
        iri(&ex("stratum/vector")),
        u64::try_from(pair.vector_terms.len()).expect("the bench corpus fits"),
    );
    Cardinalities(cardinalities)
}

fn hnsw_pair_request(seed: &str) -> RetrievalRequest {
    let terms = vec![
        RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(&note_predicate())),
        },
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<{seed}>")),
        },
    ];
    RetrievalRequest::bounded(terms, TOP_K)
}

fn hnsw_pair_profile() -> FusionProfile {
    profile_over(&[ex("stratum/text"), ex("stratum/vector")])
}

fn text_hnsw_reads(c: &mut Criterion) {
    for (shape, name) in SHAPES {
        let mut group = c.benchmark_group(format!("multimodal_on_demand_read/text_hnsw/{name}"));
        for &rows in &SIZES {
            let pair = hnsw_pair(shape, rows);
            let dataset = build_dataset(&pair.text_rows, &note_predicate());
            let registry = hnsw_pair_registry(&dataset, &pair);
            let statistics = hnsw_pair_statistics(&pair);
            let request = hnsw_pair_request(&pair.seed);
            let profile = hnsw_pair_profile();
            group.bench_with_input(BenchmarkId::new("on_demand", rows), &rows, |b, _| {
                b.iter(|| {
                    black_box(on_demand(
                        &registry,
                        &statistics,
                        &dataset,
                        &request,
                        &profile,
                    ))
                });
            });
            group.bench_with_input(BenchmarkId::new("materialised", rows), &rows, |b, _| {
                b.iter(|| {
                    black_box(materialised(
                        &registry,
                        &statistics,
                        &dataset,
                        &request,
                        &profile,
                    ))
                });
            });
        }
        group.finish();
    }
}

// ---------------------------------------------------------------------------
// One governed per-candidate exclusion lookup, prepared once
// ---------------------------------------------------------------------------

/// Bench one already-open stratum's `exclusion` call against `candidate`, under the
/// given `id`. The stratum is prepared once, before this is called; every criterion
/// iteration re-times the same lookup.
fn bench_single_lookup(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    id: &str,
    stream: &mut purrdf::retrieval::StratumStream<'_>,
    candidate: &Term,
) {
    group.bench_function(id, |b| {
        b.iter(|| {
            black_box(block_on(stream.stream.exclusion(candidate)).expect("the lookup answers"))
        });
    });
}

fn single_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("multimodal_on_demand_read/single_lookup");

    // A real text index: the shared-block, disjoint text+text pair, so the right
    // side's lookups are real dictionary searches over its own term table.
    {
        let pair = text_pair(Shape::SharedDisjoint, LOOKUP_ROWS);
        let dataset = text_pair_dataset(&pair);
        let registry = text_pair_registry(&dataset, &pair);
        let statistics = text_pair_statistics(&pair);
        let request = text_pair_request();
        let profile = text_pair_profile();
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &statistics,
            fusion_profile: Some(&profile),
        };
        let planned = plan(&request, &registry, &statistics).expect("the fixture plans");
        let compiled = compile(&planned, &env).expect("the fixture compiles");
        let mut execution =
            block_on(execute(&compiled, &registry, &dataset)).expect("the fixture executes");
        let left_index = execution
            .streams
            .iter()
            .position(|stream| stream.stratum == iri(&ex("stratum/left")))
            .expect("the left stratum ran");
        let right_index = execution
            .streams
            .iter()
            .position(|stream| stream.stratum == iri(&ex("stratum/right")))
            .expect("the right stratum ran");
        // One row off each stream: a candidate the left side ranked (which the right
        // index holds only a non-matching document for — `Excluded`, having really
        // searched) and a candidate the right side ranked itself (`Possible`).
        let left_first = block_on(execution.streams[left_index].stream.next())
            .expect("the stream reads")
            .expect("the left stratum ranked at least one row")
            .1;
        let right_first = block_on(execution.streams[right_index].stream.next())
            .expect("the stream reads")
            .expect("the right stratum ranked at least one row")
            .1;

        bench_single_lookup(
            &mut group,
            "text_excluded",
            &mut execution.streams[right_index],
            &left_first,
        );
        bench_single_lookup(
            &mut group,
            "text_possible",
            &mut execution.streams[right_index],
            &right_first,
        );
    }

    // A real HNSW graph: the shared-block, disjoint text+HNSW pair, so the vector
    // side's `Excluded` verdict about a text subject is real term-order work and its
    // `Possible` verdict is about a row it really holds.
    {
        let pair = hnsw_pair(Shape::SharedDisjoint, LOOKUP_ROWS);
        let dataset = build_dataset(&pair.text_rows, &note_predicate());
        let registry = hnsw_pair_registry(&dataset, &pair);
        let statistics = hnsw_pair_statistics(&pair);
        let request = hnsw_pair_request(&pair.seed);
        let profile = hnsw_pair_profile();
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &statistics,
            fusion_profile: Some(&profile),
        };
        let planned = plan(&request, &registry, &statistics).expect("the fixture plans");
        let compiled = compile(&planned, &env).expect("the fixture compiles");
        let mut execution =
            block_on(execute(&compiled, &registry, &dataset)).expect("the fixture executes");
        let vector_index = execution
            .streams
            .iter()
            .position(|stream| stream.stratum == iri(&ex("stratum/vector")))
            .expect("the vector stratum ran");
        let vector_first = block_on(execution.streams[vector_index].stream.next())
            .expect("the stream reads")
            .expect("the vector stratum ranked at least one row")
            .1;
        // A text subject: the vector space holds no row for it at all, so the
        // lookup is settled by term order alone — `Excluded`, having read no vector.
        let text_candidate = Term::new(format!("<{}>", ex("text/0")));

        bench_single_lookup(
            &mut group,
            "hnsw_excluded",
            &mut execution.streams[vector_index],
            &text_candidate,
        );
        bench_single_lookup(
            &mut group,
            "hnsw_possible",
            &mut execution.streams[vector_index],
            &vector_first,
        );
    }

    group.finish();
}

criterion_group!(benches, text_text_reads, text_hnsw_reads, single_lookup);
criterion_main!(benches);
