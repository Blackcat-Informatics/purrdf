// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a declared exclusion basis buys a read that spans two *different*
//! modalities, measured end to end through the umbrella alone.
//!
//! `purrdf-retrieval`'s own `text_exclusion_lookup.rs` measures the same mechanism over
//! two text producers. The interesting configuration is the one this file builds: a real
//! [`TextSearchRelation`] over a real inverted index, and a real [`HnswRelation`] over a
//! real approximate-nearest-neighbour graph, ranked in one fused answer. They share a
//! block and hold disjoint candidates, which is the shape where a bound is not on its own
//! a bound on the read: no candidate either stream names is ever final while the other
//! stream is open, because the other stream declares the same block and might still name
//! it. Nothing but learning that a stream will NEVER name a candidate resolves it.
//!
//! # Why the vector side is the hard one
//!
//! A vector producer bounds *itself*: its declaration carries a depth placement, so the
//! consumer hands it the number of neighbours to rank as an argument, and a call carrying
//! a depth answers the ranked question — `is this candidate among your best n`. Those
//! absences are not exclusions, because a candidate at rank n+1 is one the stream will
//! still name. The lookup this file measures is a different question, and it reaches the
//! producer as a different question: the consumer renders the exclusion unit with the
//! depth left FREE and declares the candidate a prepare parameter, so the call is
//! admitted in the producer's membership mode and answered by a term-order lookup that
//! traverses no graph.
//!
//! # The fixture is built so the lookups can be wrong
//!
//! The vector space holds **no row for any text subject**. That is the candidate that
//! otherwise forces the vector stratum to be drained, and it is also the answer a broken
//! lookup gets wrong in the expensive direction. The text index, symmetrically, holds a
//! document for every vector subject carrying text that shares no term with the needle —
//! so the text side's lookups are real dictionary searches rather than an absent-subject
//! shortcut, and both sides answer `Excluded` having actually looked.
//!
//! Every IRI below is this test's own, in the host's role. PurRDF mints no vocabulary.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf::hnsw::relation::{HnswObservations, HnswRelation, HnswSpace};
use purrdf::hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf::retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, ExclusionBasis, Fixed, FusedRow,
    FusionProfile, Iri, OrderFidelity, RECIP_K, RankFidelity, RequestTerm, RetrievalRequest,
    Statistics, Term, TopK, compile, execute, plan, search,
};
use purrdf::sparql::{KnnGuard, PropertyFunctionRegistry, RankedDeclaration, TermKind};
use purrdf::text::{
    GraphSelector, SearchObservations, TextIndex, TextIndexConfig, TextSearchRelation,
};
use purrdf::{DistanceMetric, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

// ---------------------------------------------------------------------------
// The host's vocabulary and the fixture's dimensions
// ---------------------------------------------------------------------------

/// The one predicate the text corpus is indexed over.
const NOTE: &str = "https://example.org/note";
/// The IRI this host registers the text relation under.
const TEXT_PF: &str = "https://example.org/pf/search";
/// The IRI this host registers the vector relation under.
const VECTOR_PF: &str = "https://example.org/pf/neighbours";
/// The stratum this host ranks lexical rows within.
const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
/// The stratum this host ranks vector rows within.
const VECTOR_STRATUM: &str = "https://example.org/stratum/vector";
/// The one block both producers declare. Sharing it is what removes the planner's merge
/// argument and leaves finality as the only thing that can end the read.
const SHARED_BLOCK: &str = "https://example.org/domain/shared";
/// The datatype the depth argument is written under — the host's, never invented here.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The needle the text producer is asked for.
const NEEDLE: &str = "alpha beta";

/// How many documents the needle reaches, and how many rows the vector space holds.
///
/// Deep enough that a drained read and a licensed one are genuinely different numbers —
/// a candidate one of two block-sharing streams named has to outlast twice its own
/// weight, which under reciprocal-rank decay is deep — and small enough that the graph
/// still builds inside a test suite.
const CORPUS: usize = 80;

/// The vector space's dimensionality.
const DIMS: usize = 8;

/// The bound the request searches under.
const TOP_K: TopK = TopK::new(5);

/// The smoothing constant the fusion profile decays by, read from the crate's own
/// constant rather than written twice.
const K: u32 = RECIP_K as u32;

/// The subject of the `at`-th text document.
fn text_subject(at: usize) -> String {
    format!("https://example.org/doc/text/{at}")
}

/// The term the `at`-th vector row stands for.
fn vector_term(at: usize) -> String {
    format!("https://example.org/doc/vec/{at}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf::iri::Iri {
    purrdf::iri::parse(text).expect("fixture IRIs are valid")
}

fn shared_block() -> DomainTag {
    DomainTag::parse(SHARED_BLOCK).expect("the fixture domain tag is a valid IRI")
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
// The corpus
// ---------------------------------------------------------------------------

/// The `(subject, text)` rows the inverted index is built from.
///
/// [`CORPUS`] documents the needle reaches, under the text subject prefix — and one
/// document per *vector* term whose text shares no term with the needle. The second group
/// is what makes a text-side lookup a real dictionary search: the index really holds a
/// document of that subject, so the relation must binary-search its term dictionary for
/// each needle term and find no posting, which is exactly the case a naive implementation
/// would answer by ranking the partition.
///
/// Those documents are never candidates. A document carrying no needle term is in no
/// candidate set at any depth, so the two strata's candidates stay disjoint.
fn text_rows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = (0..CORPUS)
        .map(|at| {
            (
                text_subject(at),
                // The repeated term varies the term frequency, so the documents do not
                // all score the same and the ranking is a ranking rather than a
                // tie-break order.
                format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim()),
            )
        })
        .collect();
    out.extend((0..CORPUS).map(|at| (vector_term(at), "zulu yankee xray whiskey".to_owned())));
    out
}

/// The dataset every stage runs against.
///
/// The property-function calls this file compiles read nothing out of it — a ranked
/// producer answers from its own index — but it is the dataset the text index was built
/// from, which is the wiring a host actually has.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(NOTE);
    for (subject, text) in text_rows() {
        let subject = builder.intern_iri(&subject);
        let object = builder.intern_literal(RdfLiteral::simple(&text));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// The inverted index: one predicate, every graph, untagged literals in the default
/// graph — so exactly one partition, which is what
/// `TextSearchRelation::ranked_declaration` requires.
fn text_index(dataset: &RdfDataset) -> Arc<TextIndex> {
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration is well formed");
    Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture index builds"))
}

/// The vector space: [`CORPUS`] rows, named by the vector terms and by **no text
/// subject**.
///
/// That disjointness is the fixture's load-bearing property twice over. It is what makes
/// a text candidate one this producer will never name — the fact an exclusion lookup
/// reports — and it is what makes such a lookup read no vector at all, because a term the
/// space holds no row for is settled by canonical term order alone.
fn vector_space() -> Arc<HnswSpace> {
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    let mut data = Vec::with_capacity(CORPUS * DIMS);
    for _ in 0..CORPUS * DIMS {
        // splitmix64, spelled here so the fixture depends on no private helper.
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let value = ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    let matrix = VectorMatrix::new(CORPUS, DIMS, data).expect("a valid matrix");
    // The beam is as wide as the corpus, so the approximate read can actually reach the
    // depth the plan asks for and the two runs below differ in the declared basis rather
    // than in how far the beam happened to get.
    let params = Params::new(8, 16, 64, CORPUS).expect("valid parameters");
    let index =
        HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params).expect("it builds");
    let terms: Vec<TermValue> = (0..CORPUS)
        .map(|at| TermValue::iri(vector_term(at)))
        .collect();
    let guard = KnnGuard::new(CORPUS as u64, CORPUS as u64).expect("a valid guard");
    Arc::new(HnswSpace::from_index(index, terms, guard).expect("a valid space"))
}

// ---------------------------------------------------------------------------
// The wiring
// ---------------------------------------------------------------------------

/// The request: one lexical term for the text producer, one entity seed for the vector
/// producer.
///
/// The seed is a term the space holds a row for — this producer searches *from* a term
/// whose vector it already has — and it is a vector term, so it is on the vector side of
/// the disjoint split like every other row of that space.
fn request_terms() -> Vec<RequestTerm> {
    vec![
        RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(NOTE)),
        },
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", vector_term(0))),
        },
    ]
}

/// Both producers registered, with `basis` written into the declaration each relation
/// hands out for itself.
///
/// The declarations are the relations' own — positions, accepted terms, duplicate policy,
/// fidelity and basis all come from `ranked_declaration` — and the only field this
/// function touches is `exclusion`, which is what makes the two runs below a differential
/// pair rather than two different registrations.
fn registry(
    dataset: &RdfDataset,
    basis: ExclusionBasis,
) -> (
    PropertyFunctionRegistry,
    Arc<SearchObservations>,
    Arc<HnswObservations>,
) {
    let mut registry = PropertyFunctionRegistry::new();

    let text = TextSearchRelation::new(text_index(dataset));
    let text_observations = text.observations();
    let mut text_declaration: RankedDeclaration = text
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::within([shared_block()]),
        )
        .expect("a single-partition index declares a ranked order");
    assert_eq!(
        text_declaration.exclusion,
        ExclusionBasis::Membership,
        "the text relation's own declaration is the membership basis; this fixture varies \
         it rather than inventing it"
    );
    text_declaration.exclusion = basis;
    registry.register_ranked(TEXT_PF, Arc::new(text), text_declaration);

    let vector = HnswRelation::new(vector_space());
    let vector_observations = vector.observations();
    let mut vector_declaration = vector.ranked_declaration(
        kernel_iri(VECTOR_STRATUM),
        // The space's rows are IRIs, and an IRI seed is what this host's requests name.
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        // The vectors handed to the builder are the values this host meant; no upstream
        // stage approximated them. The beam's own loss is the relation's to declare, and
        // it declares it unconditionally.
        OrderFidelity::Faithful,
        CandidateDomains::within([shared_block()]),
    );
    assert_eq!(
        vector_declaration.exclusion,
        ExclusionBasis::Membership,
        "the vector relation's own declaration is the membership basis, lossy beam and all"
    );
    vector_declaration.exclusion = basis;
    registry.register_ranked(VECTOR_PF, Arc::new(vector), vector_declaration);

    (registry, text_observations, vector_observations)
}

/// Statistics that narrow nothing: each stratum really does hold the rows its producer
/// declares, and no selectivity is measured.
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
    cardinalities.insert(iri(TEXT_STRATUM), text_rows().len() as u64);
    cardinalities.insert(iri(NOTE), text_rows().len() as u64);
    cardinalities.insert(iri(VECTOR_STRATUM), CORPUS as u64);
    FixtureStatistics { cardinalities }
}

/// Unit weight for each stratum under reciprocal-rank decay.
fn fixture_profile() -> FusionProfile {
    let weights = [iri(TEXT_STRATUM), iri(VECTOR_STRATUM)]
        .into_iter()
        .map(|stratum| (stratum, Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

// ---------------------------------------------------------------------------
// The measurement
// ---------------------------------------------------------------------------

/// What one run cost and what it answered.
struct Measured {
    /// The fused answer, reduced to the part a reader compares.
    answer: Vec<(Term, Fixed, Vec<(Iri, u64, Fixed)>)>,
    /// Ranks fusion pulled off the text stream.
    text_ranks: u64,
    /// Ranks fusion pulled off the vector stream.
    vector_ranks: u64,
    /// Exclusion lookups fusion performed against either stream.
    fused_lookups: u64,
    /// Membership lookups the text relation really performed.
    text_lookups: u64,
    /// Membership lookups the vector relation really performed.
    vector_lookups: u64,
    /// Distances the vector relation's membership path computed.
    vector_membership_distances: u64,
    /// How many invocations entered the text relation's partition ranker — its own
    /// work counter, one entry per ranking of the whole candidate set.
    text_rankings: u64,
    /// How many beam searches the vector relation ran, and how many graph candidates
    /// they visited — its own work counters.
    vector_searches: u64,
    vector_graph_candidates: u64,
    /// The read-work figure the trailer reports for the text stratum: the rows its
    /// one read produced.
    text_rows_materialised: Option<u64>,
    /// The same figure for the vector stratum.
    vector_rows_materialised: Option<u64>,
}

impl Measured {
    /// One run's numbers, rendered so a failure shows the whole row of the table rather
    /// than the single number that tripped.
    fn report(&self, name: &str) -> String {
        format!(
            "{name}: rows={rows} text_ranks={text_ranks} vector_ranks={vector_ranks} \
             fused_lookups={fused} text_lookups={text_lookups} \
             vector_lookups={vector_lookups} vector_membership_distances={distances} \
             text_rankings={rankings} vector_searches={searches} \
             vector_graph_candidates={candidates} text_rows_materialised={text_rows:?} \
             vector_rows_materialised={vector_rows:?}",
            rows = self.answer.len(),
            text_ranks = self.text_ranks,
            vector_ranks = self.vector_ranks,
            fused = self.fused_lookups,
            text_lookups = self.text_lookups,
            vector_lookups = self.vector_lookups,
            distances = self.vector_membership_distances,
            rankings = self.text_rankings,
            searches = self.vector_searches,
            candidates = self.vector_graph_candidates,
            text_rows = self.text_rows_materialised,
            vector_rows = self.vector_rows_materialised,
        )
    }

    /// Ranks pulled off both streams together.
    fn ranks_pulled(&self) -> u64 {
        self.text_ranks + self.vector_ranks
    }
}

/// One fused row reduced to what two runs must agree on.
///
/// The threshold witness is deliberately left out: it records the global threshold in
/// force when the row was certified, and certifying a row earlier — which is the whole
/// point of an exclusion lookup — legitimately moves it. The candidate, the score and the
/// provenance are the answer.
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

/// Run the request once under `basis` and measure it.
fn measure(dataset: &RdfDataset, basis: ExclusionBasis) -> Measured {
    let (registry, text_observations, vector_observations) = registry(dataset, basis);
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

    let ranks_of = |stratum: &str| {
        result
            .trailer
            .resolution
            .get(&iri(stratum))
            .map_or(0, |resolution| resolution.ranks_pulled)
    };
    let materialised_of = |stratum: &str| {
        result
            .trailer
            .resolution
            .get(&iri(stratum))
            .and_then(|resolution| resolution.rows_materialised)
    };
    Measured {
        answer: result.rows.iter().map(reduce).collect(),
        text_ranks: ranks_of(TEXT_STRATUM),
        vector_ranks: ranks_of(VECTOR_STRATUM),
        fused_lookups: result
            .trailer
            .resolution
            .values()
            .map(|resolution| resolution.exclusion_lookups)
            .sum(),
        text_lookups: text_observations.membership_lookups(),
        vector_lookups: vector_observations.membership_lookups(),
        vector_membership_distances: vector_observations.membership_distances(),
        text_rankings: text_observations.rankings(),
        vector_searches: vector_observations.searches(),
        vector_graph_candidates: vector_observations.graph_candidates(),
        text_rows_materialised: materialised_of(TEXT_STRATUM),
        vector_rows_materialised: materialised_of(VECTOR_STRATUM),
    }
}

/// **One text stratum and one vector stratum, sharing a block and holding disjoint
/// candidates: the same answer, out of a strictly shorter read, paid for with membership
/// lookups on both sides.**
///
/// All of it is asserted in one test, because each half alone passes over a defect the
/// other catches. "Fewer ranks pulled" alone is satisfied by a run that stopped early and
/// answered wrongly. "The same answer" alone is satisfied by a basis nothing ever asked
/// about. "Lookups happened" alone is satisfied by lookups that each ranked a partition
/// or traversed a graph, which is the cost this mechanism exists to avoid.
///
/// The run under `ExclusionBasis::Unavailable` is the control, and it is the read this
/// shape had before a vector producer could declare a basis at all: one shared block and
/// disjoint candidates, so no candidate is ever final while either stream is open, so
/// both streams drain.
#[test]
fn a_declared_basis_shortens_a_text_and_vector_read_without_moving_the_answer() {
    let dataset = dataset();
    let control = measure(&dataset, ExclusionBasis::Unavailable);
    let asked = measure(&dataset, ExclusionBasis::Membership);
    let report = format!("{}; {}", control.report("control"), asked.report("asked"));

    // 1. The answer is the same one, row for row, score for score, in order.
    assert_eq!(
        asked.answer, control.answer,
        "a basis is evidence about what a stream will never name; it may shorten the read \
         and must not change what the read answers — {report}"
    );
    assert_eq!(
        asked.answer.len(),
        usize::try_from(TOP_K.get()).expect("the bound is small"),
        "the fixture must actually fill the bound, or the comparison above compares two \
         empty answers — {report}"
    );

    // 2. The read is strictly shorter, and both strata really were read in the control.
    assert!(
        control.text_ranks > 0 && control.vector_ranks > 0,
        "the control must have read both modalities, or the comparison below is about one \
         stratum — {report}"
    );
    assert!(
        asked.ranks_pulled() < control.ranks_pulled(),
        "a shared block with disjoint candidates drains both streams unless a stream can \
         be asked — {report}"
    );

    // 2b. And it is paid for once, and only as far as the fusion pulled. Each
    //     stratum is one invocation, opened at its planned depth and read a row per
    //     pull. Both strata declare the shared block and answer lookups — the lossy
    //     beam included, whose membership answer is exact — so the fusion stops
    //     where the fifth row crosses both heads' threshold, at rank 66, and each
    //     stratum's read produced exactly those 66 rows.
    assert_eq!(
        (asked.text_ranks, asked.vector_ranks),
        (66, 66),
        "the fusion stopped where the fifth row crosses — {report}"
    );
    assert_eq!(
        (asked.text_rows_materialised, asked.vector_rows_materialised),
        (Some(66), Some(66)),
        "and each read produced the ranks the fusion pulled, not a row more — {report}"
    );
    // The control, on record beside it: no basis, so nothing settles finality
    // before a stream runs out, and each stratum's one read goes on through all 80
    // of its rows — once. The vector producer takes the planned depth as its own
    // `k`, which sits on its declared bound, so its read ends at that bound rather
    // than on a probe row.
    assert_eq!(
        (
            control.text_rows_materialised,
            control.vector_rows_materialised
        ),
        (Some(80), Some(80)),
        "every row of each stratum, read once — {report}"
    );
    // 2c. The producers' own work: one text ranking and one beam search per
    //     search, in the declaring run and the control alike, and the beam visited
    //     the same graph in both — the read opened at the planned depth is the
    //     control's own invocation, so asking lookups beside it cost the vector
    //     index no traversal.
    assert_eq!(
        (asked.text_rankings, asked.vector_searches),
        (1, 1),
        "one ranking and one beam search — {report}"
    );
    assert_eq!(
        (control.text_rankings, control.vector_searches),
        (1, 1),
        "and the control paid exactly the same — {report}"
    );
    assert_eq!(
        asked.vector_graph_candidates, control.vector_graph_candidates,
        "the declaring run's beam is the control's beam — {report}"
    );

    // 3. The lookups really happened, and the control really did not ask.
    assert!(
        asked.fused_lookups > 0,
        "fusion must have asked, or the shorter read above came from something else — \
         {report}"
    );
    assert_eq!(
        control.fused_lookups, 0,
        "and the control must not have asked, which is what `Unavailable` means — {report}"
    );

    // 4. BOTH strata served lookups, counted by the relations themselves rather than by
    //    the consumer that asked. This is the fact the whole file exists for: the vector
    //    producer bounds itself, so its lookup had to arrive in a different mode from its
    //    ranked read, and a counter on the relation is the only place that shows it did.
    assert!(
        asked.text_lookups > 0,
        "the text index holds a document for every vector candidate, so its lookups are \
         real binary searches over its own dictionary — {report}"
    );
    assert!(
        asked.vector_lookups > 0,
        "the vector relation must have answered lookups of its own, or only one modality \
         was ever asked — {report}"
    );
    assert_eq!(
        control.text_lookups, 0,
        "and the control's text relation answered none — {report}"
    );
    assert_eq!(
        control.vector_lookups, 0,
        "nor did the control's vector relation — {report}"
    );

    // 5. Not one of the vector lookups read a vector. The space holds no row for a text
    //    subject, so the answer is settled by canonical term order alone, and a
    //    membership path that computed a distance would have read one.
    assert_eq!(
        asked.vector_membership_distances, 0,
        "the vector space holds no row for any text subject, so every lookup is decided \
         without reading a vector — {report}"
    );
}

/// **A lookup against a vector relation computes zero distances and visits zero graph
/// nodes.**
///
/// The claim the vector side of the mechanism rests on, isolated from the ranked read
/// that necessarily precedes it. The stratum is planned, compiled and executed exactly as
/// [`search`] runs it — so the lookup goes through the rendered exclusion unit, its
/// prepared plan and the substitution channel, not through a hand-built call — and the
/// relation's counters are read *before and after* the lookups. The deltas are the
/// lookups' own cost and nothing else's.
///
/// Zero is the load-bearing value in both dimensions. A relation that answered the lookup
/// by ranking would show graph candidates; one that answered it by reading the candidate's
/// vector would show a distance. Neither is an inference from a clock: this relation has
/// exactly one call site into the traversal and one into the pairwise evaluation, and each
/// increments its own counter.
#[test]
fn a_vector_lookup_computes_no_distance_and_visits_no_graph_node() {
    let dataset = dataset();
    let (registry, _text_observations, vector_observations) =
        registry(&dataset, ExclusionBasis::Membership);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };

    let planned = plan(&request, &registry, &statistics).expect("the request plans");
    let bundle = compile(&planned, &env).expect("the plan compiles");
    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset)).expect("the units execute");
    let stream = execution
        .streams
        .iter_mut()
        .find(|stream| stream.stratum == iri(VECTOR_STRATUM))
        .expect("the vector stratum streamed");

    // The readings the deltas are taken against. Executing the unit ran the stratum's
    // ranked read, which is the traversal these counters already hold; what follows must
    // add nothing to it.
    let searches_before = vector_observations.searches();
    let graph_before = vector_observations.graph_candidates();
    let distances_before = vector_observations.membership_distances();
    let lookups_before = vector_observations.membership_lookups();

    // Three text subjects: candidates of the other modality, which this space holds no
    // row for. Each is a term the vector stream will never name, which is exactly what
    // the verdict has to report.
    let asked: Vec<Term> = (0..3)
        .map(|at| Term::new(format!("<{}>", text_subject(at))))
        .collect();
    for candidate in &asked {
        let verdict = block_on(stream.stream.exclusion(candidate))
            .unwrap_or_else(|error| panic!("the vector stream answers a lookup: {error:?}"));
        assert_eq!(
            verdict,
            purrdf::retrieval::ExclusionVerdict::Excluded,
            "the space holds no row for {candidate:?}, so it will never name it"
        );
    }

    let report = format!(
        "searches {}->{} graph_candidates {}->{} membership_distances {}->{} \
         membership_lookups {}->{}",
        searches_before,
        vector_observations.searches(),
        graph_before,
        vector_observations.graph_candidates(),
        distances_before,
        vector_observations.membership_distances(),
        lookups_before,
        vector_observations.membership_lookups(),
    );

    assert_eq!(
        vector_observations.membership_lookups() - lookups_before,
        asked.len() as u64,
        "one lookup per candidate asked, so the deltas below are about these lookups — \
         {report}"
    );
    assert_eq!(
        vector_observations.searches(),
        searches_before,
        "a lookup enters no beam — {report}"
    );
    assert_eq!(
        vector_observations.graph_candidates(),
        graph_before,
        "a lookup visits no graph node — {report}"
    );
    assert_eq!(
        vector_observations.membership_distances(),
        distances_before,
        "and a lookup about a term the space does not hold reads no vector at all — \
         {report}"
    );
}
