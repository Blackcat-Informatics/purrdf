// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! The fused read taken on demand, against the same read materialised.
//!
//! [`search`] reads every stratum it rendered as one invocation held open and read a
//! row per pull; the hand-composed pipeline over [`execute`] materialises each
//! stratum at its planned depth before its first row is readable. Both give the
//! same answer. These benches put the two side by side over the three shapes that
//! decide what the difference is worth:
//!
//! * `intersecting` — two strata sharing a block and naming the same candidates, so
//!   the fusion certifies at its sixth rank of a deep plan: the on-demand read
//!   produces six rows per stratum where the materialised one produces the plan;
//! * `disjoint` — the same block and no candidate named twice, with nothing
//!   answerable, so the fusion drains both streams: the two reads produce the same
//!   rows, and the difference is the price of reading them one pull at a time;
//! * `disjoint_asking` — the same rows with both producers answering exclusion
//!   lookups, so the fusion stops at the threshold crossing and the on-demand read
//!   stops there with it.
//!
//! Each group runs at two corpus sizes, because the first and third shapes' on-demand
//! cost is flat in the corpus and the materialised cost is not.
//!
//! A fourth group, `depth_taking`, is about a producer that takes its depth as an
//! argument — the exact nearest-neighbour relation over a sealed embedding artifact —
//! and prices the alternative to opening it once at the planned depth. Each case
//! reads the 66 ranks the drained-with-lookups shape pulls: `at_planned` from one
//! invocation opened at the planned depth (the space's every row), `at_stop` from one
//! opened at `k = 66`, and `continued` from a `k = 66` invocation read out and then a
//! second one at the planned depth read to rank 80, the continuation an overrun would
//! need. The scan measures every row at any `k`, so the first two are expected to sit
//! together and the third to pay a second whole scan; the exact counters behind that
//! are asserted in `purrdf-sparql-eval`'s kNN tests and in the umbrella's
//! `multimodal_exact_knn.rs`.
//!
//! Report-only, per this repository's rule: benches exist so a later change has a
//! number to move, never so a speedup can be asserted. The deterministic
//! counterparts — rows produced and producer-reported work, exactly, against the
//! materialised control — are asserted in `tests/multimodal_read_bound.rs`.

use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDataset, RdfDatasetBuilder, RdfTermTarget, StageImplementation, TargetSet, TermValue,
    VectorDtype,
};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed,
    FusionProfile, Iri, RECIP_K, RankFidelity, RankedStreamAdapter, RequestTerm, RetrievalRequest,
    Statistics, Term, TopK, compile, execute, fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EmbeddingKnnRelation, EmbeddingSpace, EvalError, ExclusionBasis,
    KnnGuard, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    RankArithmetic, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
    Volatility,
};

/// The fixture namespace. A bench mints no vocabulary of its own, and a
/// reserved-for-documentation authority is the only one it may put in a term.
fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
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

/// The flattened argument position every producer here projects its candidate from.
const CANDIDATE_POSITION: usize = 0;

/// One shape of the two producers.
#[derive(Clone, Copy)]
struct Shape {
    name: &'static str,
    /// Whether the two producers name the same candidates.
    intersecting: bool,
    /// Whether they answer exclusion lookups.
    asking: bool,
}

const SHAPES: [Shape; 3] = [
    Shape {
        name: "intersecting",
        intersecting: true,
        asking: false,
    },
    Shape {
        name: "disjoint",
        intersecting: false,
        asking: false,
    },
    Shape {
        name: "disjoint_asking",
        intersecting: false,
        asking: true,
    },
];

/// A ranked producer holding `rows` candidates under `prefix`, minted as pulled.
struct Producer {
    prefix: &'static str,
    rows: u64,
    modes: Vec<BindingPattern>,
}

impl PropertyFunction for Producer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(CANDIDATE_POSITION) {
            1
        } else {
            self.rows
        }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        // A bound candidate is the exclusion lookup: answered from the minted
        // spelling, never by scanning.
        if let Some(candidate) = bound[CANDIDATE_POSITION].clone() {
            let held = matches!(&candidate, TermValue::Iri(text)
                if text.strip_prefix(&ex(self.prefix)).and_then(|rest| rest.strip_prefix("entity"))
                    .and_then(|index| index.parse::<u64>().ok())
                    .is_some_and(|index| index < self.rows));
            return Ok(Box::new(Rows {
                prefix: self.prefix,
                next: 0,
                end: u64::from(held),
                bound,
            }));
        }
        Ok(Box::new(Rows {
            prefix: self.prefix,
            next: 0,
            end: self.rows,
            bound,
        }))
    }
}

/// The cursor behind [`Producer`]: candidates `next..end`, minted as pulled.
struct Rows {
    prefix: &'static str,
    next: u64,
    end: u64,
    bound: Vec<Option<TermValue>>,
}

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.next >= self.end {
            return Ok(None);
        }
        let minted = [
            TermValue::iri(format!("{}entity{:09}", ex(self.prefix), self.next)),
            TermValue::iri(format!("{}score{:09}", ex(self.prefix), self.next)),
        ];
        self.next += 1;
        Ok(Some(
            minted
                .iter()
                .enumerate()
                .map(|(position, value)| {
                    self.bound
                        .get(position)
                        .cloned()
                        .flatten()
                        .unwrap_or_else(|| value.clone())
                })
                .collect(),
        ))
    }
}

fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

fn registry(shape: Shape, rows: u64) -> PropertyFunctionRegistry {
    let arity = PfArity::new(1, 1);
    let block = DomainTag::parse(&ex("domain/shared")).expect("a valid tag");
    let prefixes = if shape.intersecting {
        ["shared/", "shared/"]
    } else {
        ["left/", "right/"]
    };
    let mut registry = PropertyFunctionRegistry::new();
    for ((predicate, stratum), prefix) in ["title", "body"].into_iter().zip(strata()).zip(prefixes)
    {
        let mut modes = vec![arity.all_free_mode()];
        if shape.asking {
            modes.push(BindingPattern::from_bound_positions(
                arity.total(),
                [CANDIDATE_POSITION],
            ));
        }
        registry.register_ranked(
            ex(&format!("pf/{predicate}")),
            Arc::new(Producer {
                prefix,
                rows,
                modes,
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("a valid IRI"),
                accepted_terms: vec![AcceptedTerm {
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
                }],
                depth_placement: None,
                candidate_position: CANDIDATE_POSITION,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                arithmetic: RankArithmetic::FloatFree,
                domains: CandidateDomains::within([block.clone()]),
                block_position: None,
                exclusion: if shape.asking {
                    ExclusionBasis::Membership
                } else {
                    ExclusionBasis::Unavailable
                },
                mandatory: false,
            },
        );
    }
    registry
}

fn request() -> RetrievalRequest {
    let terms = ["title", "body"]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect();
    RetrievalRequest::bounded(terms, TopK::new(5))
}

/// Statistics that narrow nothing: each stratum holds what its producer declares.
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

fn statistics(rows: u64) -> Cardinalities {
    let mut cardinalities = BTreeMap::new();
    for stratum in strata() {
        cardinalities.insert(stratum, rows);
    }
    for predicate in ["title", "body"] {
        cardinalities.insert(iri(&ex(predicate)), rows);
    }
    Cardinalities(cardinalities)
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        DecayRule::ReciprocalRank {
            k: u32::try_from(RECIP_K).expect("the smoothing constant fits"),
        },
    )
    .expect("the fixture profile is valid")
}

/// The read `search` takes: every stratum on demand.
fn on_demand(
    registry: &PropertyFunctionRegistry,
    statistics: &Cardinalities,
    dataset: &RdfDataset,
) -> usize {
    let profile = profile();
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(&profile),
    };
    block_on(search(
        &request(),
        registry,
        statistics,
        dataset,
        &env,
        &profile,
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
) -> usize {
    let profile = profile();
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), registry, statistics).expect("the fixture plans");
    let compiled = compile(&planned, &env).expect("the fixture compiles");
    let execution = block_on(execute(&compiled, registry, dataset)).expect("it executes");
    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                    .expect("both strata are weighted")
                    .with_plan_id(stream.plan_id)
                    .with_fused_bound(stream.fused_bound)
                    .with_attestation(stream.attestation);
            (stream.stratum, adapter)
        })
        .collect();
    block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the materialised streams fuse")
    .rows
    .len()
}

fn reads(c: &mut Criterion) {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid");
    for shape in SHAPES {
        let mut group = c.benchmark_group(format!("on_demand_read/{}", shape.name));
        for rows in [400_u64, 4_000] {
            let registry = registry(shape, rows);
            let statistics = statistics(rows);
            group.bench_with_input(BenchmarkId::new("on_demand", rows), &rows, |b, _| {
                b.iter(|| black_box(on_demand(&registry, &statistics, &dataset)));
            });
            group.bench_with_input(BenchmarkId::new("materialised", rows), &rows, |b, _| {
                b.iter(|| black_box(materialised(&registry, &statistics, &dataset)));
            });
        }
        group.finish();
    }
}

// ---------------------------------------------------------------------------
// A producer that takes its depth
// ---------------------------------------------------------------------------

/// The ranks the drained-with-lookups shape pulls from each stratum.
const PULLED: usize = 66;

/// The dimensionality of the bench's embedding space.
const DIMS: usize = 16;

fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        ex(name),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("the fixture artifact identity is well formed")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            ex(name),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("the fixture stage is well formed"),
    )
}

/// A sealed embedding artifact of `rows` deterministic vectors, opened as a space.
fn embedding_space(rows: usize) -> EmbeddingSpace {
    let dimension = u32::try_from(DIMS).expect("small");
    let empty = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&empty).expect("source pack");
    let mut targets = Vec::with_capacity(rows);
    let mut bindings = Vec::with_capacity(rows);
    for at in 0..rows {
        let term = ex(&format!("vec/{at:09}"));
        let target = RdfTermTarget::Iri(term.clone())
            .into_target(true, None)
            .expect("the fixture term target is well formed");
        bindings.push((target.id, TermValue::iri(term)));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|target| target.id).collect())
        .expect("the fixture target set is well formed");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    declared.sort_unstable_by_key(|target| target.id);
    let contract = EmbeddingFamilyContract {
        model: identity("model"),
        engine: identity("engine"),
        tokenizer: identity("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(dimension, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("the family derives");
    let projection = ProjectionSpec::derive(family.id, dimension, PrefixPostprocessing::None);
    let vector_space = projection.vector_space_id;
    let mut state = 0x0DE9_7B7A_4E00_0001_u64;
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: dimension,
        rows: bindings
            .iter()
            .map(|(target, _)| {
                let values = (0..DIMS)
                    .map(|_| {
                        // splitmix64, so the bench depends on no private helper.
                        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                        let mut z = state;
                        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                        z ^= z >> 31;
                        f64::from(u32::try_from(z >> 40).expect("24 bits fit")) + 1.0
                    })
                    .collect();
                MatrixRow::new(*target, values)
            })
            .collect(),
        projections: vec![projection],
    };
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: declared,
        target_sets: vec![set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("the fixture artifact encodes");
    let rows = u64::try_from(rows).expect("the bench corpus fits");
    let guard = KnnGuard::new(rows, rows).expect("a valid guard");
    EmbeddingSpace::from_artifact(&encoded.bytes, set.id, vector_space, bindings, guard)
        .expect("the fixture space opens")
}

/// Open one ranked invocation of `relation` from the seed at `k`, and pull `pulled`
/// rows off it, skipping the first `skip`.
fn pull_at(
    relation: &EmbeddingKnnRelation,
    seed: &TermValue,
    k: usize,
    skip: usize,
    pulled: usize,
) -> usize {
    let count = TermValue::typed_literal(k.to_string(), "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [None];
    let object = [Some(seed), Some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None).expect("the invocation opens");
    let mut read = 0;
    for _ in 0..skip + pulled {
        if cursor.next().expect("the invocation reads").is_none() {
            break;
        }
        read += 1;
    }
    read
}

fn depth_taking(c: &mut Criterion) {
    let mut group = c.benchmark_group("on_demand_read/depth_taking");
    for rows in [400_usize, 4_000] {
        let relation = EmbeddingKnnRelation::new(Arc::new(embedding_space(rows)));
        let seed = TermValue::iri(ex("vec/000000000"));
        group.bench_with_input(BenchmarkId::new("at_planned", rows), &rows, |b, _| {
            b.iter(|| black_box(pull_at(&relation, &seed, rows, 0, PULLED)));
        });
        group.bench_with_input(BenchmarkId::new("at_stop", rows), &rows, |b, _| {
            b.iter(|| black_box(pull_at(&relation, &seed, PULLED, 0, PULLED)));
        });
        group.bench_with_input(BenchmarkId::new("continued", rows), &rows, |b, _| {
            b.iter(|| {
                black_box(pull_at(&relation, &seed, PULLED, 0, PULLED));
                black_box(pull_at(&relation, &seed, rows, PULLED, 80 - PULLED))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, reads, depth_taking);
criterion_main!(benches);
