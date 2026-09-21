// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One request, two modalities that disagree, one fused answer.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p purrdf-retrieval --example fused_search
//! ```
//!
//! The host below indexes four documents twice over: once lexically, with a real
//! BM25 [`TextIndex`], and once geometrically, with a real
//! [`EmbeddingSpace`](purrdf_sparql_eval::EmbeddingSpace) over a sealed PURREMB
//! artifact. Each is registered as its own ranked producer under its own
//! stratum, and one request reaches both — a needle for the text index and a
//! seed term for the nearest-neighbour search.
//!
//! # The two producers disagree, which is the entire point
//!
//! Fusion exists for the case where two modalities rank the same corpus
//! differently, so this corpus is built so that they do:
//!
//! * BM25 answers `doc-delta`, `doc-beta`, `doc-alpha` — by how often the needle
//!   occurs, with `doc-gamma` holding it not at all.
//! * kNN answers `doc-alpha`, `doc-beta`, `doc-gamma`, `doc-delta` — by distance
//!   from the seed, which every document has a vector for.
//!
//! Neither producer's own order survives intact. The document both rank highly
//! wins; the document one producer loved and the other ranked last does not; and
//! the document only one producer reached at all is in the answer too, carrying
//! that producer's provenance alone. Every row printed below names which
//! producer put it there and at what rank, so the compromise is legible rather
//! than asserted.
//!
//! # The bound is stated in the request, because fused enumeration is top-k
//!
//! A request states a row bound rather than defaulting to one: no candidate can be
//! emitted until it is known not to reappear in another stratum and raise its
//! total, so fusion is bounded work by construction. It is stated on the
//! *request* rather than handed to `search`, because the planner derives each
//! stratum's depth from it — over producers whose declared candidate blocks do not
//! overlap, the bound is the depth. Both producers here rank the same four
//! documents and so promise nothing narrower than "anything", which is exactly the
//! shape that licenses no narrowing; the bound is also above what four documents
//! can yield, so nothing in this answer is decided by it, and the trailer reports
//! what each producer actually did — which for a corpus this small is to run out
//! of rows.
//!
//! # The trailer is the rest of the answer
//!
//! The rows are what was asked for; the trailer is what a consumer needs in order
//! to know what the rows are worth, so this example prints all of it rather than
//! stopping at the ranking. After the statuses and the planned resolution it
//! reports:
//!
//! * **what each index attested** — which generation answered, and whether that
//!   generation declared itself short. Both are pinned when the index is opened,
//!   before a single row is pulled, so they describe the read that produced these
//!   rows and not the state of an index some time afterwards;
//! * **the evidence identity**, a digest of exactly those attestations. Two
//!   answers assembled from differently-aged indexes carry different ids even
//!   when the plan, the profile and every row match;
//! * **what that makes of the scores** — exact, or an estimate naming the strata
//!   whose shortfall made it one, on both sides. The error runs in two
//!   directions because this layer scores by rank alone: a stratum that misses a
//!   row withholds that row's contribution *and* promotes every row behind it.
//!   An estimate is still a real order over real rows; what it withdraws is the
//!   claim that a missing row would have stayed missing;
//! * **the candidate domains in force**, which say what each producer could ever
//!   have named. Both producers here rank the same four documents and so promise
//!   nothing narrower than "anything".
//!
//! Every IRI here is the host's own `example.org` vocabulary. PurRDF mints none,
//! and there is no default producer, stratum or weight to fall back on.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTermTarget, StageImplementation, TargetId,
    TargetSet, TargetSetId, TermValue, VectorDtype, VectorSpaceId, parse_iri,
};
use purrdf_retrieval::{
    AdmissionEnvironment, Completeness, DecayRule, Fixed, FusionProfile, Iri, OrderFidelity,
    ProducerStatus, RankFidelity, RequestTerm, RetrievalRequest, ScoreExactness, ScoreInterval,
    SearchResult, Statistics, Term, TopK, search,
};
use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, EmbeddingKnnRelation, EmbeddingSpace, IndexGeneration, KnnGuard,
    PropertyFunctionRegistry, ServiceLevel, TermKind,
};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The predicate the lexical corpus is indexed over.
const NOTE: &str = "https://example.org/note";
/// The IRI this host registers the text producer under.
const TEXT_PRODUCER: &str = "https://example.org/pf/note-search";
/// The IRI this host registers the nearest-neighbour producer under.
const KNN_PRODUCER: &str = "https://example.org/pf/neighbours";
/// The stratum the text producer ranks within.
const SAMPLE_PRODUCER: &str = "https://example.org/pf/archive-search";
const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
const SAMPLE_STRATUM: &str = "https://example.org/stratum/archive";

/// What the archive producer below does not promise, in its own words.
///
/// A host writes this, not the library: only the host knows how its index
/// relates to the corpus it means. It is carried into the answer verbatim.
const SAMPLE_EVIDENCE: &str = "approximate: this index covers the first half of the archive \
     only; a miss here is not evidence the archive lacks the document";
/// The stratum the nearest-neighbour producer ranks within.
const KNN_STRATUM: &str = "https://example.org/stratum/neighbour";
/// The predicate the host's unserved spatial term names.
const PLACE: &str = "https://example.org/place";
/// The datatype this host renders a neighbour count with.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// The reciprocal-rank smoothing constant this host fuses under.
const K: u32 = 60;
/// How many fused rows this host wants. Fused enumeration is top-k by
/// construction, so the bound is stated rather than defaulted; four documents
/// are all this corpus holds, so nothing here is decided by it.
const TOP_K: TopK = TopK::new(10);
/// The needle the lexical half of the request carries.
const NEEDLE: &str = "cat";
/// The seed the nearest-neighbour half of the request searches from.
const SEED: &str = "doc-alpha";

/// The corpus, as `(subject local name, note, vector)`.
///
/// Both halves are hand-checkable. Every note is four tokens long, so `avgdl` is
/// exactly four and BM25 orders the matches by how often `"cat"` occurs:
/// `doc-delta` (three), `doc-beta` (two), `doc-alpha` (one), and `doc-gamma`
/// never. The vectors are squared-Euclidean from `doc-alpha` at the origin:
/// `doc-beta` at 25, `doc-gamma` at 100, `doc-delta` at 2500.
///
/// So the two producers rank the same four documents in nearly opposite orders,
/// and agree only that `doc-beta` belongs in the middle.
const CORPUS: [(&str, &str, [f64; 2]); 4] = [
    ("doc-alpha", "cat rope kite drum", [0.0, 0.0]),
    ("doc-beta", "cat cat rope kite", [3.0, 4.0]),
    ("doc-gamma", "rope kite drum bell", [6.0, 8.0]),
    ("doc-delta", "cat cat cat rope", [30.0, 40.0]),
];

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("the host's IRIs are valid")
}

fn subject(local: &str) -> String {
    format!("https://example.org/{local}")
}

/// The dataset every stage runs against: one `note` triple per document, in the
/// default graph, and the very rows the text index is built from.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text, _) in CORPUS {
        let document = builder.intern_iri(&subject(local));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(document, note, literal, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// A real BM25 index over the dataset above.
///
/// One predicate and one untagged, default-graph corpus is exactly one
/// partition, which is the condition `TextSearchRelation::ranked_declaration`
/// requires before it will claim a ranked order.
fn text_index(data: &RdfDataset) -> TextIndex {
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("one IRI predicate is a well-formed configuration");
    TextIndex::from_dataset(data, &config).expect("the index builds over the fixture dataset")
}

/// A fixture artifact identity, distinct per `name`.
fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        subject(name),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("the host's artifact identity is well formed")
}

/// A fixture applied stage, distinct per `name`.
fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            subject(name),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("the host's stage is well formed"),
    )
}

/// Encode a sealed PURREMB artifact holding one vector per corpus document under
/// the squared-Euclidean metric.
///
/// A host with an embedding pipeline reads these bytes from wherever that
/// pipeline wrote them; this example mints them so the file runs on its own. The
/// contract is declared in full either way — the model, the engine, the stages
/// applied and not applied, the dtype and the metric — because that declaration
/// is what a vector space's identity is made of.
fn artifact() -> (
    Vec<u8>,
    TargetSetId,
    VectorSpaceId,
    Vec<(TargetId, TermValue)>,
) {
    let dimension = 2u32;
    let empty = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&empty).expect("the source pack seals");

    let mut targets = Vec::with_capacity(CORPUS.len());
    let mut bindings = Vec::with_capacity(CORPUS.len());
    for (local, _, _) in CORPUS {
        let term = TermValue::iri(subject(local));
        let TermValue::Iri(text) = &term else {
            unreachable!("the host's document terms are IRIs")
        };
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("the host's term target is well formed");
        bindings.push((target.id, term));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|target| target.id).collect())
        .expect("the host's target set is well formed");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("the dataset target"));
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
            .expect("two fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("the family derives");
    let projection = ProjectionSpec::derive(family.id, dimension, PrefixPostprocessing::None);
    let vector_space = projection.vector_space_id;

    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: dimension,
        rows: CORPUS
            .iter()
            .zip(&bindings)
            .map(|((_, _, vector), (target, _))| MatrixRow::new(*target, vector.to_vec()))
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
    let encoded = builder.build().expect("the host's artifact encodes");
    (encoded.bytes, set.id, vector_space, bindings)
}

/// A real, fully verified embedding space over the artifact above.
fn embedding_space() -> EmbeddingSpace {
    let (bytes, target_set, vector_space, bindings) = artifact();
    EmbeddingSpace::from_artifact(
        &bytes,
        target_set,
        vector_space,
        bindings,
        KnnGuard::new(16, 8).expect("the host's guard bounds are positive"),
    )
    .expect("the host's space opens")
}

/// Register both relations as ranked producers, each under the host's own
/// producer IRI and stratum, using the declaration each relation hands out.
fn registry(data: &RdfDataset) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();

    let text = TextSearchRelation::new(Arc::new(text_index(data)));
    let text_declaration = text
        .ranked_declaration(
            parse_iri(TEXT_STRATUM).expect("the host's stratum IRI is valid"),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            // This host's notes and its embedded entities are the same
            // entities — the example's whole point is a candidate both
            // producers name — so neither producer restricts its domain.
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PRODUCER, Arc::new(text), text_declaration);

    let knn = EmbeddingKnnRelation::new(Arc::new(embedding_space()));
    let knn_declaration = knn.ranked_declaration(
        parse_iri(KNN_STRATUM).expect("the host's stratum IRI is valid"),
        // The space's rows are IRIs, and an IRI seed is what this host's
        // requests name.
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        // An exhaustive scan over vectors this host knows are its whole corpus.
        // The first half of that is the relation's own property; the second half
        // is this host's statement, and the relation refuses to make it on the
        // host's behalf for the reason the sample producer below exists.
        RankFidelity::EXACT,
        // As above: one entity space, ranked twice by two laws.
        CandidateDomains::Unrestricted,
    );
    registry.register_ranked(KNN_PRODUCER, Arc::new(knn), knn_declaration);

    // A third producer over a genuinely PARTIAL index, declaring so. BM25 over
    // the documents this index holds is exhaustive; whether those documents are
    // the corpus the host means is a fact only the host has, and this is where
    // it says it. Without the declaration the trailer below would report this
    // stratum exactly as it reports the two above, and an answer missing half
    // an archive would read as complete.
    let sample = TextSearchRelation::new(Arc::new(sample_index()));
    let sample_declaration = sample
        .ranked_declaration(
            parse_iri(SAMPLE_STRATUM).expect("the host's stratum IRI is valid"),
            Some(NOTE.to_owned()),
            RankFidelity {
                completeness: Completeness::Lossy {
                    evidence: Arc::from(SAMPLE_EVIDENCE),
                },
                // The rows it DOES hold are scored and ordered exactly, so a
                // row's emitted rank is a lower bound on its true rank and the
                // answer's error stays bounded.
                order: OrderFidelity::Faithful,
            },
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(SAMPLE_PRODUCER, Arc::new(sample), sample_declaration);

    registry
}

/// A BM25 index over the first half of the corpus only.
///
/// Built from a dataset that really is short, so the declaration beside it is a
/// true statement rather than a label: a reader can delete the declaration and
/// watch the answer start claiming more than it has.
fn sample_index() -> TextIndex {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text, _) in &CORPUS[..CORPUS.len() / 2] {
        let document = builder.intern_iri(&subject(local));
        let literal = builder.intern_literal(RdfLiteral::simple(*text));
        builder.push_quad(document, note, literal, None);
    }
    let data = builder.freeze().expect("the sample dataset is valid");
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("one IRI predicate is a well-formed configuration");
    TextIndex::from_dataset(&*data, &config).expect("the sample index builds")
}

/// The request: a needle for the text producer, a seed for the neighbour
/// producer, and a geometry no producer here accepts.
///
/// The seed is what kNN takes — it searches *from* a term whose vector the space
/// already holds. The geometry is deliberate: a request may name a modality no
/// registered producer accepts, and the answer says so per term rather than
/// letting it vanish.
///
/// The bound is part of the request, because it is what the planner derives each
/// stratum's depth from: `search` reads it from here rather than taking it as an
/// argument of its own.
fn request() -> RetrievalRequest {
    RetrievalRequest::bounded(
        vec![
            RequestTerm::Lexical {
                text: NEEDLE.to_owned(),
                language: None,
                predicate: Some(iri(NOTE)),
            },
            RequestTerm::EntitySeed {
                entity: Term::new(format!("<{}>", subject(SEED))),
            },
            RequestTerm::Spatial {
                geometry: "POINT(0 0)".to_owned(),
                predicate: iri(PLACE),
                max_distance: None,
            },
        ],
        TOP_K,
    )
}

/// Unit weights for both strata and `K` smoothing.
///
/// Equal weights are what make the printed provenance readable: every difference
/// in the fused order comes from the ranks the two producers assigned, and none
/// of it from the law preferring a modality.
///
/// [`Fixed::ONE`] is the number one. The neighbouring constructor
/// `Fixed::from_raw(1)` is one raw unit — `10^-12` — and since a fusion reads
/// weights only as ratios, a map that mixed the two would run, refuse nothing,
/// and rank as though this stratum did not exist. Build a weight map with one
/// constructor.
///
/// How many contributions a candidate may receive is not a parameter: it is the
/// number of strata declared here, because a candidate surfaces at most once in
/// each.
fn profile() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(TEXT_STRATUM), Fixed::ONE);
    weights.insert(iri(KNN_STRATUM), Fixed::ONE);
    weights.insert(iri(SAMPLE_STRATUM), Fixed::ONE);
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the host's profile is valid")
}

/// A statistics provider that reports nothing.
///
/// Every producer declares a finite row bound measured from its own frozen
/// data, so there is no unbounded declaration for a cardinality to bound. A
/// provider that invented one would put a number into the plan that no
/// measurement supports.
struct NoStatistics;

impl Statistics for NoStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        None
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// Drive one future to completion on this thread.
///
/// The ladder's futures are awaited in a single task and never cross a thread
/// boundary, so a parking waker is the whole runtime they need; a host that
/// already has an executor awaits `search` on that instead.
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

/// The stratum IRI without the host's shared prefix, so the printed table reads.
fn short(stratum: &Iri) -> &str {
    stratum
        .as_str()
        .rsplit_once('/')
        .map_or_else(|| stratum.as_str(), |(_, tail)| tail)
}

fn report(result: &SearchResult) {
    println!("fused ranking (top {} requested)", TOP_K.get());
    for (position, row) in result.rows.iter().enumerate() {
        let provenance = row
            .contributions
            .iter()
            .map(|(stratum, rank, value)| {
                format!(
                    "{} rank {rank} (+{})",
                    short(stratum),
                    value.to_decimal_lexical()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        // `Term` prints as its canonical lexical, which is what a row is about;
        // `{:?}` would print the newtype around it.
        let error = match &row.interval {
            ScoreInterval::Bounded { deficit, inflation } => format!(
                "-{} / +{}",
                inflation.to_decimal_lexical(),
                deficit.to_decimal_lexical()
            ),
            ScoreInterval::Unbounded { perturbed } => {
                format!("unbounded ({} stratum/strata)", perturbed.len())
            }
        };
        println!(
            "  {}. {}  score {} ({error})  [{provenance}]",
            position + 1,
            row.entity,
            row.score.to_decimal_lexical()
        );
    }
    // How many of those places hold whatever the degraded strata did or did not
    // find. Membership, never absence: a row past the prefix is possible, not
    // excluded.
    println!(
        "  {} of {} rows are certain of their places",
        result.trailer.certain_prefix(&result.rows),
        result.rows.len()
    );

    println!("\nproducer statuses");
    for (stratum, status) in &result.trailer.statuses {
        let rendered = match status {
            ProducerStatus::Exhausted { rows_emitted } => {
                format!("exhausted after {rows_emitted} rows")
            }
            ProducerStatus::DepthReached { rank } => {
                format!("stopped at the depth it was given, after rank {rank}")
            }
            ProducerStatus::RowBoundReached { rank } => {
                format!(
                    "read to the {rank} rows it declared it can serve; whether more exist was \
                     not observable"
                )
            }
            ProducerStatus::SuppliedQueryEnded { rank } => {
                format!(
                    "ran a query text of the caller's own and emitted {rank} row(s); what that \
                     text left unread was not observable"
                )
            }
            ProducerStatus::CeilingReached { bound } => {
                format!(
                    "still held rows; read down to {} and stopped",
                    bound.to_decimal_lexical()
                )
            }
            ProducerStatus::ExecutionFailed { reason } => format!("could not run: {reason}"),
            ProducerStatus::TermsRejected => "declined the terms it was handed".to_owned(),
        };
        // The status and the fidelity are printed on one line because they are
        // read on one line: a status says how the read ENDED, the fidelity
        // says whether the rows that ended it were all the rows that were DUE.
        // `exhausted` beside `lossy` is not a contradiction and not a
        // completeness claim.
        let promise = match result.trailer.fidelities.get(stratum) {
            None => "not asked".to_owned(),
            Some(fidelity) => {
                let mut parts = Vec::new();
                match &fidelity.completeness {
                    Completeness::Complete => parts.push("complete".to_owned()),
                    Completeness::Lossy { evidence } => {
                        parts.push(format!("LOSSY — {evidence}"));
                    }
                }
                match &fidelity.order {
                    OrderFidelity::Faithful => parts.push("order faithful".to_owned()),
                    OrderFidelity::Perturbed { evidence } => {
                        parts.push(format!("order PERTURBED — {evidence}"));
                    }
                }
                parts.join("; ")
            }
        };
        println!("  {}: {rendered}", short(stratum));
        println!("      declared: {promise}");
    }

    // What the depths this plan records were going to cost in rank resolution,
    // as the admission waist measured them before a single row was read. A host
    // that wants this and nothing else never has to run the search at all:
    // `compile` against an environment naming the profile answers it on its own.
    println!("\nplanned rank resolution (known before anything ran)");
    for (stratum, planned) in &result.planned_resolution {
        let separation = planned.separation.rank().map_or_else(
            || "no depth a plan can express".to_owned(),
            |rank| format!("rank {rank}"),
        );
        let verdict = if planned.fully_separated() {
            "every planned rank is ordered by score alone"
        } else {
            "the deepest planned ranks fall to the declared tie-break"
        };
        println!(
            "  {}: planned to read {} ranks; this law separates to {separation} — {verdict}",
            short(stratum),
            planned.requested_depth
        );
    }

    // The trailer is where a consumer looks for "what should I know about this
    // answer", so the exemplar reads all of it rather than the rows alone. Each
    // block below is a fact no row can carry: which index answered, whether that
    // index was whole, what that makes of the scores, and which slice of the
    // candidate universe each producer was ever able to name.
    println!("\nwhat each index attested (pinned before a row was pulled)");
    for (stratum, attestation) in &result.trailer.attestations {
        let generation = match &attestation.generation {
            IndexGeneration::Declared(generation) => format!("generation {generation}"),
            IndexGeneration::Undeclared => "no generation declared".to_owned(),
        };
        let service = match &attestation.service {
            ServiceLevel::Undeclared => "declared no shortfall".to_owned(),
            ServiceLevel::Incomplete { reason } => format!("declared itself SHORT: {reason}"),
        };
        println!("  {}: {generation}; {service}", short(stratum));
    }

    // A pure function of the attestations above, so two answers built from the
    // same indexes carry the same id and two built from differently-aged ones do
    // not — whatever else about the query matched.
    println!("\nevidence identity {}", result.evidence_id);

    // What each producer promised it could ever name. `Unrestricted` is the
    // widest promise and the one both producers here make, because both rank the
    // same four documents; a `Within` declaration is what lets fusion certify a
    // candidate without draining a stratum that was never going to name it.
    println!("\ncandidate domains in force");
    for (stratum, domains) in &result.trailer.domains {
        let rendered = match domains {
            CandidateDomains::Unrestricted => "may name any candidate".to_owned(),
            CandidateDomains::Within(tags) => format!(
                "restricted to {}",
                tags.iter()
                    .map(DomainTag::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        println!("  {}: {rendered}", short(stratum));
    }

    println!("\nrequest terms nothing served");
    if result.unserved_terms.is_empty() {
        println!("  (none)");
    } else {
        for unserved in &result.unserved_terms {
            println!("  term {}: {:?}", unserved.request_term, unserved.reason);
        }
    }

    println!("\nmay these scores be read as values?");
    match &result.trailer.exactness {
        ScoreExactness::Exact => {
            // The narrower true claim, and deliberately not "every index was
            // whole": a producer attests nothing about wholeness unless it is
            // short, so silence is what most of them offer, and reading `Exact`
            // as a certificate would put words in the mouth of every producer
            // that stayed silent.
            println!(
                "  yes: no stratum in this fusion declared itself degraded on either axis, \
                 so every score is the whole sum of the contributions that were due"
            );
        }
        ScoreExactness::Estimated {
            deficit,
            inflation,
            unbounded,
        } => {
            println!("  no — they are estimates, and the error runs both ways:");
            println!(
                "    may have withheld (scores too LOW):  {}",
                deficit.iter().map(short).collect::<Vec<_>>().join(", ")
            );
            println!(
                "    may have over-contributed (too HIGH): {}",
                inflation.iter().map(short).collect::<Vec<_>>().join(", ")
            );
            if unbounded.is_empty() {
                println!("    no finite bound: (none)");
            } else {
                println!(
                    "    NO FINITE BOUND:                      {}",
                    unbounded.iter().map(short).collect::<Vec<_>>().join(", ")
                );
            }
        }
    }

    println!("\nfused under profile {}", result.profile_id);
    println!("from plan {}", result.plan_id);
}

fn main() {
    let data = dataset();
    let registry = registry(&data);
    let statistics = NoStatistics;
    let environment = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let profile = profile();

    let result = block_on(search(
        &request(),
        &registry,
        &statistics,
        &*data,
        &environment,
        &profile,
    ))
    .expect("the host's producers answer");

    verify(&result);

    report(&result);
}

/// What this example claims, checked rather than printed.
///
/// An example that only prints is a smoke test: it proves the code runs, and
/// says nothing about whether the numbers above mean what the prose beside them
/// says. These are the claims a reader is here to see, so they are asserted —
/// `cargo run --example fused_search` fails if the demonstration stops
/// demonstrating.
fn verify(result: &SearchResult) {
    let archive = iri(SAMPLE_STRATUM);

    // The declaration reached the answer, verbatim.
    let Some(Completeness::Lossy { evidence }) = result
        .trailer
        .fidelities
        .get(&archive)
        .map(|fidelity| fidelity.completeness.clone())
    else {
        panic!("the partial index declared a loss and the answer must carry it");
    };
    assert_eq!(
        &*evidence, SAMPLE_EVIDENCE,
        "the host's own words, not a summary of them"
    );

    // The two exhaustive producers are NOT swept up with it.
    for stratum in [iri(TEXT_STRATUM), iri(KNN_STRATUM)] {
        assert_eq!(
            result.trailer.fidelities.get(&stratum),
            Some(&RankFidelity::EXACT),
            "a stratum that promised everything is reported as promising \
             everything, not left out for a reader to interpret"
        );
    }

    // The verdict names it on both sides, and nothing is unbounded.
    let ScoreExactness::Estimated {
        deficit,
        inflation,
        unbounded,
    } = &result.trailer.exactness
    else {
        panic!("one stratum is short, so these scores are estimates");
    };
    assert!(deficit.contains(&archive) && inflation.contains(&archive));
    assert_eq!(deficit.len(), 1, "and only that stratum: {deficit:?}");
    assert!(
        unbounded.is_empty(),
        "its order is faithful, so the error stays bounded"
    );

    // The two directions, on the rows themselves. This is the property a
    // one-sided reading gets wrong, and the printed table above is where a
    // reader can see it: a row the short index NAMED may have been promoted by
    // a row it missed, and a row it never named may be missing that
    // contribution entirely.
    let mut named = 0;
    let mut unnamed = 0;
    for row in &result.rows {
        let ScoreInterval::Bounded { deficit, inflation } = &row.interval else {
            panic!("no stratum here perturbs its order");
        };
        if row.contributions.iter().any(|(s, _, _)| *s == archive) {
            named += 1;
            assert!(
                *inflation > Fixed::ZERO,
                "{}: named by the short index, so its contribution is suspect",
                row.entity
            );
        } else {
            unnamed += 1;
            assert!(
                *deficit > Fixed::ZERO,
                "{}: the short index could have named it and did not",
                row.entity
            );
        }
    }
    assert!(
        named > 0 && unnamed > 0,
        "the fixture must show BOTH directions, or it demonstrates half the \
         point: {named} named, {unnamed} not"
    );
}
