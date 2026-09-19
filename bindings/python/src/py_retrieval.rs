// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 Python bindings for `purrdf-retrieval` — the composition layer over the
//! ranked property-function producers, exposed as the
//! `purrdf_native.retrieval` submodule.
//!
//! # Surface
//!
//! The Rust ladder is four stages — `plan`, `compile`, `execute`, `fuse` — and
//! `search` is the four run as one. Every stage whose value is **plain data**
//! is a function here, and `search` is the composition:
//!
//! * [`plan`] — the pure planner. Returns the plan document: its canonical id,
//!   every producer decision with the dimension that refused it, the per-stratum
//!   depths, the statistics snapshot the planner consulted, and every request
//!   term that reached no producer at all. No weights: those are the fusion
//!   law's, chosen at `search` time and deliberately not a planning input.
//! * [`compile`] — semantic admission plus emission. Returns the per-stratum
//!   SPARQL a host can read, run or log verbatim, and — when the call names the
//!   fusion law it means to fuse under — what the plan's depths will cost in
//!   rank resolution, which is the question a host asks *before* paying to
//!   execute anything.
//! * [`search`] — `fuse ∘ execute ∘ compile ∘ plan`. Returns the fused ranking
//!   with per-row, per-stratum provenance, every applicable producer's own
//!   terminal status, the unserved-term evidence, and both identities that name
//!   which plan and which fusion law produced the answer.
//!
//! `execute` and `fuse` have no function of their own here, and that is a
//! property of their *values*, not a reduction of the surface: `execute` yields
//! live ranked streams and `fuse` consumes them through an async protocol, so a
//! Python-level boundary between those two stages would have to hand out an
//! opaque handle borrowing the registry and dataset this call built. Their whole
//! effect is in [`search`], which runs both.
//!
//! ```python
//! from purrdf_native import retrieval
//!
//! data = '''
//!   <https://example.org/a> <https://example.org/note> "the quick brown fox" .
//!   <https://example.org/b> <https://example.org/note> "a quick red fox" .
//! '''
//! answer = retrieval.search(
//!     data,
//!     [("lexical", "quick fox", None, "https://example.org/note")],
//!     text_producers={
//!         "https://example.org/pf/search": (
//!             "https://example.org/stratum/lexical",
//!             "https://example.org/note",
//!             "any",
//!         )
//!     },
//!     weights={"https://example.org/stratum/lexical": retrieval.SCALE},
//!     statistics={"source": "host-statistics", "revision": "r1"},
//!     k=60,
//!     decay="reciprocal_rank",
//!     top_k=10,
//! )
//! assert answer["rows"][0]["entity"] == "<https://example.org/a>"
//! ```
//!
//! # Exactness crosses the boundary intact
//!
//! Every weight, score and contribution is `purrdf_retrieval::Fixed` — an exact
//! base-10 fixed-point value, never a float. It crosses in the only two
//! spellings that preserve it: a weight arrives as a Python `int` in raw
//! fixed-point units (`SCALE` is one whole unit, [`SCALE_DIGITS`] its exponent),
//! and a score leaves as its exact decimal `str`. A `float` weight would be a
//! binary approximation of the decimal the host wrote, and a `float` score would
//! be an approximation of a number the fusion law computed exactly, so neither
//! is accepted or produced.
//!
//! Raw units are the whole convention on this side of the boundary, so a weight
//! of **one** is `retrieval.SCALE`, not the Python literal `1` — which is one
//! raw unit, `10 ** -SCALE_DIGITS`. A fusion reads weights only as ratios, so a
//! `weights` dict mixing the two spellings is a factor-of-`10 ** SCALE_DIGITS`
//! error that runs, refuses nothing, and returns a plausible ranking in which
//! the smaller stratum has effectively been switched off. Write every weight in
//! the same spelling: `n * retrieval.SCALE` for a weight of `n`.
//!
//! # Nothing is defaulted, because PurRDF mints nothing
//!
//! Producers, strata, weights and the statistics provider's own identity are all
//! caller-supplied. There is no default producer IRI, no default stratum, no
//! default weight, and no invented statistics revision: `k`, `decay` and `top_k`
//! are required keywords for the same reason.
//!
//! `decay` is the one of those that looks most like it could have a default and
//! least can. It names the rank-decay rule the fusion law runs under, and the two
//! rules compute **different numbers** from the same weights:
//!
//! * `"reciprocal_rank"` truncates the reciprocal to the layer's declared scale
//!   *before* the weight is applied. The inner truncation is a ceiling no weight
//!   can lift, so this rule stops separating adjacent ranks at the same depth —
//!   just past a million — for every weight at or above one.
//! * `"weighted_reciprocal_rank"` folds the weight into the numerator as one
//!   exactly-rounded division. Its value is never below the other's, and its
//!   reachable depth grows with the weight, so a heavier stratum is legitimately
//!   readable deeper.
//!
//! A host that needs rank resolution past the first rule's wall names the second,
//! and [`weight_for_depth`] quotes the weight that buys the depth under whichever
//! of the two it was asked about. Choosing one on the caller's behalf would fuse
//! under arithmetic the caller never wrote, so an omitted `decay` is refused and
//! an unknown spelling is refused by name.
//!
//! # The ranked producer a Python host configures
//!
//! A ranked producer is registered through
//! `purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked`, which takes
//! a live relation object. `text_producers` builds the one shipped ranked
//! relation whose entire configuration is plain data — `purrdf-text`'s BM25
//! search over an indexed predicate — so a host declares it by naming a
//! predicate rather than by constructing a Rust value. Registering several is
//! how one request fuses across several strata.
//!
//! Each of them names a stratum of its **own**. A stratum is served by one
//! producer, because a rank means nothing outside the list that assigned it: two
//! producers under one stratum could only have their ranked rows concatenated,
//! which ranks the second's best row below the whole of the first's output. Two
//! producers whose scores are already comparable — shards or segments of one
//! index — belong inside one producer that merges them by score; two that score
//! differently belong in two strata, where the weighted sum across strata is the
//! point of the fusion. A `text_producers` map that names one stratum twice is
//! refused where it is registered.
//!
//! # What a producer may name, and what a host has to tell it
//!
//! A `text_producers` entry may be written as a fourth element beside the three
//! it already carries — `(stratum, predicate, graph, domains)` — where `domains`
//! is `None` or a list of domain-tag IRIs. It is the second promise a producer
//! makes about its own rows, beside its duplicate policy, and like that one it
//! is host-supplied configuration read at registration rather than anything the
//! engine infers.
//!
//! It buys a **reading**, never an answer. A fused score is exact only when
//! every stream that could still name a candidate has named it, and with no
//! declaration "could still name it" is true of every open stream — so over
//! strata whose candidate sets do not overlap, a top-ten drains both streams to
//! their ends, because no confirmation is ever coming. A declaration lets the
//! fusion skip the streams that *provably* cannot name a candidate and only
//! those: the finality test does not get weaker, its quantifier gets smaller.
//! The rows, the scores and the provenance are identical either way; what
//! changes is how many ranks were pulled to reach them, which the answer reports
//! under `"observed_resolution"` and `"statuses"`.
//!
//! `None` — or an omitted fourth element — is `Unrestricted`: "this producer may
//! name anything", the honest value for a host that does not know how its
//! indexes partition, and exactly the behaviour this binding had before the
//! parameter existed. An **empty list** is refused by name: a promise to name
//! nothing is not a narrow producer, it is a producer that should not be
//! registered, and a consumer holding it to that declaration row by row would
//! refuse its first row.
//!
//! Nothing is defaulted from a stratum or from a graph, here or below. Which
//! entities a text index names is a fact about the host's corpus that neither
//! this layer nor the relation can see, and a tag derived per stratum would hand
//! two producers over one entity space a pair of tags a consumer reads as
//! disjoint. That mistake is not conservative in either direction: it refuses a
//! valid query where both producers name one entity, and certifies a score
//! missing the other's contribution where they do not.
//!
//! # The evidence an answer carries about the indexes that served it
//!
//! [`search`]'s answer carries, beside its rows, what each stratum's index said
//! about itself. `"attestations"` maps a stratum to `{"generation", "incomplete"}`,
//! both independently `None`; `"exactness"` says whether the scores are exact or
//! floors, naming the strata that came up short; `"domains"` reports the
//! declaration each stream actually fused under; and `"evidence_id"` is the
//! content identity of the attestation map, the third of the three identities
//! beside `"plan_id"` and `"profile_id"`.
//!
//! The one thing none of them says is "the index was whole". `None` under
//! `"incomplete"` is silence, not a certificate: the engine-side service level
//! has no `Whole` variant, because a producer stopped at the engine's row
//! ceiling never looked at the rows it was licensed to skip and so could not
//! honestly certify anything about them. The seam asks the one question with an
//! honest answer on every path — *was your index NOT whole?* — and reading its
//! silence as certification would put a claim in the mouth of every producer
//! that never spoke.
//!
//! # Two identities, and only one of them survives the call
//!
//! Registration is **per call** here, exactly as it is on the query surface: the
//! producers a call names are the producers it evaluates under, and the next
//! call starts from nothing. A plan is pinned to the registry **instance** it was
//! planned against — that is what stops a plan from being replayed against a
//! registry whose IRIs declare the same shape but resolve to different code — so
//! `"plan_id"` names one call's plan and is not comparable across calls.
//!
//! What IS durable is the registry's declared **shape**, and the plan carries it
//! separately as `"registry_content_fingerprint"`: a pure function of what the
//! producers declare, identical for two calls that declare the same ones, here or
//! in another process. `"profile_id"` is durable for the same kind of reason —
//! the fusion law is content-addressed, so the same weights, smoothing constant
//! and ceiling always name the same law.
//!
//! # Hard-fail
//!
//! Every typed engine error (a codec `RdfDiagnostic`, a `PlanError`,
//! `AdmissionError`, `ExecutionError`, `FusionError`, a `TextError` from an
//! index that cannot declare a ranked order) is a Python `ValueError` carrying
//! the engine's message; a malformed argument shape is a `TypeError`. The cores
//! below are panic-free, so nothing unwinds across the FFI boundary.
//!
//! Two of those messages are a producer being held to its own declaration, and
//! both name the stratum so a host holding several knows which one to fix:
//! *"stream for stratum S emitted item I more than once"* is a producer that
//! declared unique candidates and repeated one, and *"stream for stratum S named
//! item I, which its declared candidate domains cannot reach; stratum T already
//! named it"* is a pair of domain declarations that place one candidate in two
//! disjoint blocks. Neither is silently repaired: the declaration has already
//! been used to certify rows, so a merged late contribution would be a score its
//! own provenance contradicts. The second is fixed by declaring the tag the two
//! producers share, or `None` — both fuse the overlapping entity into one row
//! carrying both contributions.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::retrieval::{
    AdmissionEnvironment, ClassWidth, CompiledRetrieval, DecayRule, Fixed, FusionProfile, Iri,
    Metric, Plan, PlannedResolution, ProducerDecision, ProducerStatus, RejectionReason,
    RequestTerm, RetrievalRequest, ScoreExactness, SearchResult, Statistics, Term, ToleratedDepth,
    TopK, UnservedReason,
};
use crate::text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};
use crate::{NativeRdfFormat, RdfDataset, TermValue, parse_dataset};
use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, IndexGeneration, PropertyFunctionRegistry, ServiceLevel,
};

/// The number of decimal digits in one whole fixed-point unit.
///
/// A weight is carried as a Python `int` of raw units, so a weight of one whole
/// unit is `10 ** SCALE_DIGITS` — the value [`SCALE`] names.
const SCALE_DIGITS: u32 = crate::text::SCALE_DIGITS;

/// One whole fixed-point unit, in raw units: the weight `Fixed::ONE`.
const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

// ── the host's declarations, as owned Rust data ──────────────────────────────

/// Which graphs one text producer's index reads literals from.
#[derive(Clone, Debug, PartialEq, Eq)]
enum GraphSpec {
    /// Every graph, default and named alike.
    Any,
    /// The default graph only.
    Default,
    /// The one named graph this IRI identifies.
    Named(String),
}

impl GraphSpec {
    /// Read the host's spelling: `"any"`, `"default"`, or a graph IRI.
    fn parse(producer: &str, spelling: &str) -> Result<Self, String> {
        match spelling {
            "any" => Ok(Self::Any),
            "default" => Ok(Self::Default),
            other if other.starts_with("http") || other.contains(':') => {
                Ok(Self::Named(other.to_owned()))
            }
            other => Err(format!(
                "text producer <{producer}>: unknown graph selector {other:?}; a selector is \
                 \"any\" (every graph), \"default\" (the default graph), or the IRI of one \
                 named graph"
            )),
        }
    }

    /// The engine's selector for this spec.
    fn selector(&self) -> GraphSelector {
        match self {
            Self::Any => GraphSelector::Any,
            Self::Default => GraphSelector::Default,
            Self::Named(iri) => GraphSelector::Named(TermValue::iri(iri.clone())),
        }
    }
}

/// One ranked text producer, exactly as the host declared it.
#[derive(Clone, Debug)]
struct TextProducer {
    /// The IRI the relation is registered under.
    producer: String,
    /// The caller-supplied stratum its rows rank within.
    stratum: String,
    /// The one predicate its index is built over, and the predicate its accepted
    /// request terms must name.
    predicate: String,
    /// Which graphs the index reads.
    graph: GraphSpec,
    /// Which blocks of the candidate universe this producer promises its rows
    /// lie in, as the host spelled them, or `None` for the unrestricted
    /// declaration.
    ///
    /// Host-supplied and never derived. Which entities a text index names is a
    /// fact about the host's corpus, and neither this layer nor the relation can
    /// see it: deriving a tag per stratum would hand two producers over one
    /// entity space a pair of tags a consumer reads as disjoint, which makes a
    /// fusion refuse a valid query in one direction and certify a score missing
    /// a contribution in the other. `None` is the honest value where the host
    /// does not know, and it is exactly what this binding declared before the
    /// parameter existed.
    domains: Option<Vec<String>>,
}

/// The statistics provider the host supplied, as owned data.
///
/// The planner asks per **stratum**, so both maps are keyed by stratum IRI;
/// selectivity is additionally keyed by the request term, which the host names
/// by its index into the request it passed alongside.
#[derive(Clone, Debug, Default)]
struct HostStatistics {
    /// The provider label, recorded verbatim in the plan snapshot.
    source: String,
    /// The provider-declared revision, recorded verbatim.
    revision: String,
    /// Reported cardinalities, by stratum.
    cardinality: BTreeMap<String, u64>,
    /// Reported selectivities, as `(stratum, term, parts per million)`. A `Vec`
    /// rather than a map because the key is a request term, which is not `Ord`.
    selectivity: Vec<(String, RequestTerm, u64)>,
}

impl Statistics for HostStatistics {
    fn source(&self) -> &str {
        &self.source
    }

    fn revision(&self) -> &str {
        &self.revision
    }

    fn cardinality(&self, stratum: &Iri) -> Option<u64> {
        self.cardinality.get(stratum.as_str()).copied()
    }

    fn selectivity_ppm(&self, stratum: &Iri, term: &RequestTerm) -> Option<u64> {
        self.selectivity
            .iter()
            .find(|(declared, declared_term, _)| {
                declared == stratum.as_str() && declared_term == term
            })
            .map(|(_, _, value)| *value)
    }
}

/// Everything one call needs, converted out of Python before the GIL is released.
#[derive(Clone, Debug)]
struct Call {
    /// The RDF document the whole ladder runs against.
    data: String,
    /// The media type `data` is written in.
    media_type: &'static str,
    /// The document base relative references resolve against.
    base: Option<String>,
    /// The request terms, in the host's own order.
    request: RetrievalRequest,
    /// The ranked producers to register, in the host's own order.
    producers: Vec<TextProducer>,
    /// The statistics planning consults.
    statistics: HostStatistics,
}

// ── pure-Rust cores (unit-tested without a Python interpreter) ───────────────

/// Map the Python-surface data format name onto the native codec's media type.
fn data_media_type(format: &str) -> Result<&'static str, String> {
    match format {
        "turtle" => Ok(NativeRdfFormat::Turtle.media_type()),
        "ntriples" => Ok(NativeRdfFormat::NTriples.media_type()),
        "nquads" => Ok(NativeRdfFormat::NQuads.media_type()),
        other => Err(format!(
            "unknown data format `{other}` (expected \"turtle\", \"ntriples\", or \"nquads\")"
        )),
    }
}

/// Parse one caller-supplied IRI through the layer's own validator.
fn retrieval_iri(role: &str, text: &str) -> Result<Iri, String> {
    Iri::parse(text).map_err(|e| format!("{role} <{text}>: {e}"))
}

/// Read one producer's host-declared candidate domains.
///
/// `None` is `CandidateDomains::Unrestricted` — "this producer may name
/// anything" — which licenses a consumer to skip nothing and is the behaviour
/// every answer this binding produced before domains could be declared. A list
/// is `CandidateDomains::Within`, each entry validated as an IRI by the same
/// parser every other IRI on this surface goes through.
///
/// An EMPTY list is refused here rather than read as the unrestricted
/// declaration, and the two are not neighbours that could be quietly merged: an
/// empty restriction promises the producer names nothing at all, a consumer
/// holds a producer to that row by row, and so every row it emitted would
/// contradict its own declaration. `register_ranked` refuses it on the Rust side
/// by panicking, which must never reach the FFI boundary, so the same refusal is
/// raised here — before the registry is touched — as a `ValueError` naming the
/// producer that declared it.
fn candidate_domains(
    producer: &str,
    declared: Option<&[String]>,
) -> Result<CandidateDomains, String> {
    let Some(tags) = declared else {
        return Ok(CandidateDomains::Unrestricted);
    };
    if tags.is_empty() {
        return Err(format!(
            "text producer <{producer}>: `domains` is an empty list, which promises that this \
             producer names no candidate at all rather than narrowing the ones it names. A \
             consumer holds a producer to that declaration row by row, so every row this \
             producer emitted would contradict it. Name the domains it really draws from, or \
             pass `domains=None`, which is the honest statement that it may name anything"
        ));
    }
    let mut blocks = BTreeSet::new();
    for tag in tags {
        let iri = crate::iri::parse(tag)
            .map_err(|e| format!("text producer <{producer}>: domain tag <{tag}>: {e}"))?;
        blocks.insert(DomainTag::new(iri));
    }
    Ok(CandidateDomains::Within(blocks))
}

/// Parse the document every stage runs against.
fn build_dataset(call: &Call) -> Result<Arc<RdfDataset>, String> {
    parse_dataset(call.data.as_bytes(), call.media_type, call.base.as_deref())
        .map_err(|e| e.to_string())
}

/// Register every declared text producer as a ranked producer over `data`.
///
/// The declaration is the relation's own
/// (`TextSearchRelation::ranked_declaration`), so the capability recorded in the
/// registry is the one the relation can honestly make: an index that resolves to
/// more than one partition refuses to claim a ranked order, and that refusal
/// arrives here as a `ValueError` rather than as a ranking that is not one.
fn build_registry(
    data: &RdfDataset,
    producers: &[TextProducer],
) -> Result<PropertyFunctionRegistry, String> {
    if producers.is_empty() {
        return Err(
            "no ranked producers declared; PurRDF mints no vocabulary, so there is no default \
             producer to fall back on"
                .to_owned(),
        );
    }
    let mut registry = PropertyFunctionRegistry::new();
    let mut seen: Vec<&str> = Vec::with_capacity(producers.len());
    for producer in producers {
        if seen.contains(&producer.producer.as_str()) {
            return Err(format!(
                "property function <{}> is declared twice; a relation may not be silently \
                 shadowed",
                producer.producer
            ));
        }
        seen.push(&producer.producer);

        let config = TextIndexConfig::new(
            vec![TermValue::iri(producer.predicate.clone())],
            producer.graph.selector(),
        )
        .map_err(|e| format!("text producer <{}>: {e}", producer.producer))?;
        let index = TextIndex::from_dataset(data, &config)
            .map_err(|e| format!("text producer <{}>: {e}", producer.producer))?;
        let relation = TextSearchRelation::new(Arc::new(index));
        let stratum = crate::iri::parse(&producer.stratum).map_err(|e| {
            format!(
                "text producer <{}>: stratum <{}>: {e}",
                producer.producer, producer.stratum
            )
        })?;
        let domains = candidate_domains(&producer.producer, producer.domains.as_deref())?;
        let declaration = relation
            .ranked_declaration(stratum, Some(producer.predicate.clone()), domains)
            .map_err(|e| format!("text producer <{}>: {e}", producer.producer))?;
        registry.register_ranked(&producer.producer, Arc::new(relation), declaration);
    }
    Ok(registry)
}

/// Build the fusion law from the host's weights and the decay rule it named.
///
/// The rule arrives already resolved, carrying the smoothing constant it was
/// read alongside, because the two are never independently chosen: `K` means
/// something different in each rule's arithmetic, so a profile is built from the
/// pair or from neither.
///
/// How many contributions a candidate may receive is not an argument: it is the
/// number of weighted strata, because a candidate surfaces at most once in each.
fn build_profile(weights: &[(String, i128)], decay: DecayRule) -> Result<FusionProfile, String> {
    let mut declared = BTreeMap::new();
    for (stratum, raw) in weights {
        declared.insert(
            retrieval_iri("fusion weight stratum", stratum)?,
            Fixed::from_raw(*raw),
        );
    }
    FusionProfile::with_decay(declared, decay).map_err(|e| e.to_string())
}

/// Plan the call's request against a registry built from its producers.
fn run_plan(call: &Call) -> Result<(Arc<RdfDataset>, PropertyFunctionRegistry, Plan), String> {
    let data = build_dataset(call)?;
    let registry = build_registry(&data, &call.producers)?;
    let planned = crate::retrieval::plan(&call.request, &registry, &call.statistics)
        .map_err(|e| e.to_string())?;
    Ok((data, registry, planned))
}

/// Plan, then admit and emit the per-stratum SPARQL units.
///
/// `profile` is the fusion law the host means to fuse under, when it has named
/// one. It is not a planning input and never becomes one — the plan is already
/// built when it is read — but admission is the one stage that holds the plan
/// and the law at the same time, so it is where each planned depth can be
/// measured against the rank resolution that law delivers. A host that has not
/// chosen a law passes `None` and is told nothing about resolution, rather than
/// being handed evidence measured against a law it never named.
fn run_compile(
    call: &Call,
    profile: Option<&FusionProfile>,
) -> Result<(Plan, CompiledRetrieval), String> {
    let (_, registry, planned) = run_plan(call)?;
    let environment = AdmissionEnvironment {
        registry: &registry,
        statistics: &call.statistics,
        fusion_profile: profile,
    };
    let compiled = crate::retrieval::compile(&planned, &environment).map_err(|e| e.to_string())?;
    Ok((planned, compiled))
}

/// Run the whole ladder and return the fused answer.
fn run_search(call: &Call, profile: &FusionProfile, top_k: TopK) -> Result<SearchResult, String> {
    let data = build_dataset(call)?;
    let registry = build_registry(&data, &call.producers)?;
    let environment = AdmissionEnvironment {
        registry: &registry,
        statistics: &call.statistics,
        fusion_profile: None,
    };
    block_on(crate::retrieval::search(
        &call.request,
        &registry,
        &call.statistics,
        &*data,
        &environment,
        profile,
        top_k,
    ))
    .map_err(|e| e.to_string())
}

/// Drive one future to completion on this thread.
///
/// The ladder's futures are awaited in a single task and never cross a thread
/// boundary — `search` documents exactly that on its own `future_not_send`
/// reasoning — so a parking waker is the whole runtime they need. This binding
/// therefore drives the real async entry point rather than asking the Rust side
/// for a second, synchronous one.
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

// ── argument conversion ──────────────────────────────────────────────────────

/// Read one request term out of its `(kind, …)` tuple.
///
/// The kinds are the whole of the Rust request lattice, so a host can ask for a
/// modality no registered producer accepts and read back the typed evidence that
/// nothing served it:
///
/// * `("lexical", text, language | None, predicate | None)`
/// * `("vector", [component, …], "cosine" | "dot" | "euclidean", hint | None)`
/// * `("spatial", geometry, predicate, max_distance_raw | None)`
/// * `("temporal", predicate, lower | None, upper | None)`
/// * `("numeric", predicate, lower_raw | None, upper_raw | None)`
/// * `("entity", term)` — the term in its canonical lexical, `<iri>` or `"lex"@en`.
fn request_term(index: usize, value: &Bound<'_, PyAny>) -> PyResult<RequestTerm> {
    let fields: Vec<Bound<'_, PyAny>> = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "request term {index}: a term is a tuple whose first element names its kind"
        ))
    })?;
    let Some(kind) = fields.first() else {
        return Err(PyTypeError::new_err(format!(
            "request term {index}: a term tuple is not empty; its first element names its kind"
        )));
    };
    let kind: String = kind.extract().map_err(|_| {
        PyTypeError::new_err(format!("request term {index}: the kind must be a string"))
    })?;
    let rest = &fields[1..];
    match kind.as_str() {
        "lexical" => {
            let [text, language, predicate] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::Lexical {
                text: text_field(index, &kind, "text", &text)?,
                language: optional_text(index, &kind, "language", &language)?,
                predicate: optional_iri(index, &kind, "predicate", &predicate)?,
            })
        }
        "vector" => {
            let [embedding, metric, hint] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::Vector {
                embedding: embedding.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "request term {index} (vector): `embedding` must be a sequence of floats"
                    ))
                })?,
                metric: metric_field(index, &metric)?,
                index_hint: optional_text(index, &kind, "index_hint", &hint)?,
            })
        }
        "spatial" => {
            let [geometry, predicate, max_distance] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::Spatial {
                geometry: text_field(index, &kind, "geometry", &geometry)?,
                predicate: required_iri(index, &kind, "predicate", &predicate)?,
                max_distance: optional_fixed(index, &kind, "max_distance", &max_distance)?,
            })
        }
        "temporal" => {
            let [predicate, lower, upper] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::Temporal {
                predicate: required_iri(index, &kind, "predicate", &predicate)?,
                lower: optional_text(index, &kind, "lower", &lower)?,
                upper: optional_text(index, &kind, "upper", &upper)?,
            })
        }
        "numeric" => {
            let [predicate, lower, upper] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::NumericRange {
                predicate: required_iri(index, &kind, "predicate", &predicate)?,
                lower: optional_fixed(index, &kind, "lower", &lower)?,
                upper: optional_fixed(index, &kind, "upper", &upper)?,
            })
        }
        "entity" => {
            let [entity] = term_fields(index, &kind, rest)?;
            Ok(RequestTerm::EntitySeed {
                entity: Term::new(text_field(index, &kind, "entity", &entity)?),
            })
        }
        other => Err(PyValueError::new_err(format!(
            "request term {index}: unknown kind {other:?}; a term is \"lexical\", \"vector\", \
             \"spatial\", \"temporal\", \"numeric\", or \"entity\""
        ))),
    }
}

/// Take a term tuple's payload as exactly `N` fields, naming the kind that
/// expected them.
fn term_fields<'py, const N: usize>(
    index: usize,
    kind: &str,
    rest: &[Bound<'py, PyAny>],
) -> PyResult<[Bound<'py, PyAny>; N]> {
    <[Bound<'py, PyAny>; N]>::try_from(rest.to_vec()).map_err(|_| {
        PyTypeError::new_err(format!(
            "request term {index} ({kind}): expected {N} field(s) after the kind, got {}",
            rest.len()
        ))
    })
}

/// Read one mandatory string field.
fn text_field(index: usize, kind: &str, field: &str, value: &Bound<'_, PyAny>) -> PyResult<String> {
    value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "request term {index} ({kind}): `{field}` must be a string"
        ))
    })
}

/// Read one optional string field, where `None` means the term does not
/// constrain it.
fn optional_text(
    index: usize,
    kind: &str,
    field: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<String>> {
    if value.is_none() {
        return Ok(None);
    }
    text_field(index, kind, field, value).map(Some)
}

/// Read one mandatory IRI field, validated by the layer's own parser.
fn required_iri(index: usize, kind: &str, field: &str, value: &Bound<'_, PyAny>) -> PyResult<Iri> {
    let text = text_field(index, kind, field, value)?;
    retrieval_iri(&format!("request term {index} ({kind}) `{field}`"), &text)
        .map_err(PyValueError::new_err)
}

/// Read one optional IRI field.
fn optional_iri(
    index: usize,
    kind: &str,
    field: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<Iri>> {
    if value.is_none() {
        return Ok(None);
    }
    required_iri(index, kind, field, value).map(Some)
}

/// Read one optional exact fixed-point field, in raw units.
///
/// An `int` and nothing else: a `float` endpoint would be a binary
/// approximation of the decimal the host wrote, which is precisely what the
/// exact type exists to prevent.
fn optional_fixed(
    index: usize,
    kind: &str,
    field: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<Fixed>> {
    if value.is_none() {
        return Ok(None);
    }
    let raw: i128 = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "request term {index} ({kind}): `{field}` must be an integer of raw fixed-point \
             units (one whole unit is retrieval.SCALE), never a float"
        ))
    })?;
    Ok(Some(Fixed::from_raw(raw)))
}

/// Read a vector term's metric.
fn metric_field(index: usize, value: &Bound<'_, PyAny>) -> PyResult<Metric> {
    let spelling: String = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "request term {index} (vector): `metric` must be a string"
        ))
    })?;
    match spelling.as_str() {
        "cosine" => Ok(Metric::Cosine),
        "dot" => Ok(Metric::Dot),
        "euclidean" => Ok(Metric::Euclidean),
        other => Err(PyValueError::new_err(format!(
            "request term {index} (vector): unknown metric {other:?}; a metric is \"cosine\", \
             \"dot\", or \"euclidean\""
        ))),
    }
}

/// Read the fusion law's decay rule, with the smoothing constant it carries.
///
/// Spelled as a string tag, which is how this binding names every closed set it
/// exposes: a vector term's `metric`, a producer's `graph` and the call's own
/// `data_format` are all chosen by their spelling, and an unknown one is refused
/// by a message that names the accepted ones. There is deliberately no spelling
/// meaning "whichever" — the two rules compute different numbers and reach
/// different depths, so a rule is something a caller states, never something
/// this binding resolves on its behalf.
///
/// `k` belongs to the rule rather than beside it: the same constant is used
/// differently by each, so the two are read together and carried together.
fn decay_rule(spelling: &str, k: u32) -> Result<DecayRule, String> {
    match spelling {
        "reciprocal_rank" => Ok(DecayRule::ReciprocalRank { k }),
        "weighted_reciprocal_rank" => Ok(DecayRule::WeightedReciprocalRank { k }),
        other => Err(format!(
            "unknown decay rule {other:?}; a rule is \"reciprocal_rank\" (the reciprocal is \
             truncated to the declared scale before the weight is applied, so the depth it \
             separates to is the same for every weight) or \"weighted_reciprocal_rank\" (the \
             weight is folded into the numerator as one exactly-rounded division, so the depth \
             it separates to grows with the weight)"
        )),
    }
}

/// The refusal for a call that named part of a fusion law and not the rest.
///
/// A law is three things together — the per-stratum `weights`, the smoothing
/// constant `k`, and the `decay` rule saying how a contribution falls off with
/// rank — and any subset of them names no law at all. Supplying the absent part
/// here would report resolution measured against arithmetic the host never
/// wrote, so the refusal says which parts arrived and which did not.
fn partial_fusion_law(weights: bool, k: bool, decay: bool) -> String {
    let parts = [
        ("`weights`", weights),
        ("the smoothing constant `k`", k),
        ("the `decay` rule", decay),
    ];
    // A fixed array in a fixed order, so the two lists a message carries are a
    // pure function of which arguments arrived.
    let list = |supplied: bool| {
        parts
            .iter()
            .filter(|(_, present)| *present == supplied)
            .map(|(label, _)| *label)
            .collect::<Vec<_>>()
            .join(" and ")
    };
    format!(
        "a fusion law is three things together: `weights`, the smoothing constant `k`, and the \
         `decay` rule naming how a contribution falls off with rank. This call named {}, and left \
         {} unnamed. Pass all three to learn what the planned depths cost in rank resolution, or \
         none of them to compile without naming a law — the missing part is not filled in here, \
         because a resolution measured against arithmetic the caller never wrote is evidence \
         about nothing",
        list(true),
        list(false)
    )
}

/// Collect the request term list.
fn collect_request(request: &Bound<'_, PyAny>) -> PyResult<RetrievalRequest> {
    let items = request
        .try_iter()
        .map_err(|_| PyTypeError::new_err("`request` must be a sequence of request-term tuples"))?;
    let mut terms = Vec::new();
    for (index, item) in items.enumerate() {
        terms.push(request_term(index, &item?)?);
    }
    Ok(RetrievalRequest::from_terms(terms))
}

/// Collect the `text_producers` dict into the ordered declarations one call
/// registers: `producer_iri -> (stratum_iri, predicate_iri, graph)`, or
/// `producer_iri -> (stratum_iri, predicate_iri, graph, domains)`.
///
/// The fourth element is the producer's candidate-domain declaration: `None`
/// for the unrestricted promise, or a list of domain-tag IRIs. Omitting it
/// entirely is the same declaration as `None` — the widest promise, which
/// licenses a consumer to skip nothing — so a host that never heard of domains
/// keeps exactly the reading it had.
fn collect_producers(producers: &Bound<'_, PyDict>) -> PyResult<Vec<TextProducer>> {
    let mut declared = Vec::with_capacity(producers.len());
    for (key, value) in producers {
        let producer: String = key
            .extract()
            .map_err(|_| PyTypeError::new_err("text producer keys must be IRI strings"))?;
        let shape = || {
            PyTypeError::new_err(format!(
                "text producer <{producer}>: the value is (stratum, predicate, graph) or \
                 (stratum, predicate, graph, domains)"
            ))
        };
        let mut fields: Vec<Bound<'_, PyAny>> = value.extract().map_err(|_| shape())?;
        // Read off the tail first: the three mandatory fields are destructured
        // as an array, which consumes the vector, so the optional fourth has to
        // leave before that happens.
        let domains = match fields.len() {
            3 => None,
            4 => Some(fields.remove(3)),
            _ => return Err(shape()),
        };
        let [stratum, predicate, graph] =
            <[Bound<'_, PyAny>; 3]>::try_from(fields).map_err(|_| shape())?;
        let field = |label: &str, value: &Bound<'_, PyAny>| -> PyResult<String> {
            value.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "text producer <{producer}>: `{label}` must be a string"
                ))
            })
        };
        let domains = match domains {
            Some(value) if !value.is_none() => Some(value.extract::<Vec<String>>().map_err(|_| {
                PyTypeError::new_err(format!(
                    "text producer <{producer}>: `domains` is a list of domain-tag IRI strings, \
                     or None for the unrestricted declaration (this producer may name anything)"
                ))
            })?),
            _ => None,
        };
        let graph =
            GraphSpec::parse(&producer, &field("graph", &graph)?).map_err(PyValueError::new_err)?;
        declared.push(TextProducer {
            stratum: field("stratum", &stratum)?,
            predicate: field("predicate", &predicate)?,
            graph,
            domains,
            producer,
        });
    }
    // The dict's iteration order is the host's insertion order, but the registry
    // it builds is a set: sorting makes the registration order a pure function
    // of the declarations, so the registry's content fingerprint — which the
    // plan records — cannot depend on how the dict was written.
    declared.sort_by(|left, right| left.producer.cmp(&right.producer));
    Ok(declared)
}

/// Collect the `weights` dict: `stratum_iri -> raw fixed-point units`.
fn collect_weights(weights: &Bound<'_, PyDict>) -> PyResult<Vec<(String, i128)>> {
    let mut declared = Vec::with_capacity(weights.len());
    for (key, value) in weights {
        let stratum: String = key
            .extract()
            .map_err(|_| PyTypeError::new_err("fusion weight keys must be stratum IRI strings"))?;
        let raw: i128 = value.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "fusion weight for <{stratum}> must be an integer of raw fixed-point units \
                 (one whole unit is retrieval.SCALE), never a float"
            ))
        })?;
        declared.push((stratum, raw));
    }
    Ok(declared)
}

/// Read the statistics provider the host supplied.
///
/// `source` and `revision` are mandatory: they name *which* statistics a plan
/// was built against, the planner records them verbatim, and a revision this
/// binding invented would make a replay against moved statistics undetectable.
/// `cardinality` and `selectivity` are optional — a provider that measured
/// nothing reports nothing, and the planner falls back to each producer's own
/// declared row bound rather than to a number nobody measured.
fn collect_statistics(
    statistics: &Bound<'_, PyDict>,
    request: &RetrievalRequest,
) -> PyResult<HostStatistics> {
    let text = |key: &str| -> PyResult<String> {
        let Some(value) = statistics.get_item(key)? else {
            return Err(PyValueError::new_err(format!(
                "`statistics` must name its `{key}`; the planner records it verbatim and \
                 invents none"
            )));
        };
        value
            .extract()
            .map_err(|_| PyTypeError::new_err(format!("`statistics[{key:?}]` must be a string")))
    };
    let mut collected = HostStatistics {
        source: text("source")?,
        revision: text("revision")?,
        ..HostStatistics::default()
    };

    if let Some(cardinality) = statistics.get_item("cardinality")? {
        let entries: BTreeMap<String, u64> = cardinality.extract().map_err(|_| {
            PyTypeError::new_err(
                "`statistics[\"cardinality\"]` maps a stratum IRI to a non-negative integer",
            )
        })?;
        collected.cardinality = entries;
    }

    if let Some(selectivity) = statistics.get_item("selectivity")? {
        // Parts per million, as an integer, for the reason the fusion weights
        // above are raw fixed-point integers: the value reaches a plan's
        // canonical identity, and a binary float has two spellings of zero and
        // one value that is not equal to itself. `1_000_000` is "every row
        // matches".
        let entries: Vec<((String, usize), u64)> = selectivity
            .cast::<PyDict>()
            .map_err(|_| {
                PyTypeError::new_err(
                    "`statistics[\"selectivity\"]` maps a (stratum IRI, request-term index) \
                     pair to an integer of parts per million in [0, 1000000], never a float",
                )
            })?
            .iter()
            .map(|(key, value)| Ok((key.extract()?, value.extract()?)))
            .collect::<PyResult<_>>()
            .map_err(|_: PyErr| {
                PyTypeError::new_err(
                    "`statistics[\"selectivity\"]` maps a (stratum IRI, request-term index) \
                     pair to an integer of parts per million in [0, 1000000], never a float",
                )
            })?;
        for ((stratum, index), value) in entries {
            let Some(term) = request.terms.get(index) else {
                return Err(PyValueError::new_err(format!(
                    "`statistics[\"selectivity\"]` names request term {index}, and the request \
                     carries {} term(s)",
                    request.terms.len()
                )));
            };
            collected.selectivity.push((stratum, term.clone(), value));
        }
    }

    Ok(collected)
}

/// Convert every Python argument the three entry points share into owned Rust
/// data, so the engine call itself runs with the GIL released.
fn collect_call(
    data: &str,
    request: &Bound<'_, PyAny>,
    text_producers: &Bound<'_, PyDict>,
    statistics: &Bound<'_, PyDict>,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Call> {
    let request = collect_request(request)?;
    let statistics = collect_statistics(statistics, &request)?;
    Ok(Call {
        data: data.to_owned(),
        media_type: data_media_type(data_format).map_err(PyValueError::new_err)?,
        base: base.map(ToOwned::to_owned),
        producers: collect_producers(text_producers)?,
        request,
        statistics,
    })
}

// ── result rendering ─────────────────────────────────────────────────────────

/// The wire spelling of a rejection dimension.
const fn rejection_reason(reason: RejectionReason) -> &'static str {
    match reason {
        RejectionReason::NotRanked => "not_ranked",
        RejectionReason::NoAcceptedTerm => "no_accepted_term",
        RejectionReason::DepthExceeded => "depth_exceeded",
        RejectionReason::UnsatisfiedConstraint => "unsatisfied_constraint",
        RejectionReason::DeclaresNoRows => "declares_no_rows",
    }
}

/// The wire spelling of a per-term unserved reason.
const fn unserved_reason(reason: UnservedReason) -> &'static str {
    match reason {
        UnservedReason::NoProducerAccepts => "no_producer_accepts",
        UnservedReason::EveryAcceptingProducerRejected => "every_accepting_producer_rejected",
        UnservedReason::AcceptedWithoutPlacement => "accepted_without_placement",
        UnservedReason::Unbound => "unbound",
    }
}

/// Render one plan as a dict.
fn plan_dict<'py>(py: Python<'py>, planned: &Plan) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("plan_id", planned.id().to_hex())?;
    out.set_item("version", planned.version)?;

    let bindings = PyList::empty(py);
    for binding in &planned.producer_bindings {
        let entry = PyDict::new(py);
        entry.set_item("producer", &binding.producer)?;
        entry.set_item("stratum", binding.stratum.as_str())?;
        entry.set_item("request_terms", binding.request_terms.clone())?;
        bindings.append(entry)?;
    }
    out.set_item("producer_bindings", bindings)?;

    let decisions = PyList::empty(py);
    for decision in &planned.producer_decisions {
        let entry = PyDict::new(py);
        match decision {
            ProducerDecision::Selected { producer, stratum } => {
                entry.set_item("producer", producer)?;
                entry.set_item("selected", true)?;
                entry.set_item("stratum", stratum.as_str())?;
                entry.set_item("reason", py.None())?;
            }
            ProducerDecision::Rejected { producer, reason } => {
                entry.set_item("producer", producer)?;
                entry.set_item("selected", false)?;
                entry.set_item("stratum", py.None())?;
                entry.set_item("reason", rejection_reason(*reason))?;
            }
        }
        decisions.append(entry)?;
    }
    out.set_item("producer_decisions", decisions)?;

    let depths = PyDict::new(py);
    for (stratum, depth) in &planned.stratum_depths {
        depths.set_item(stratum.as_str(), depth)?;
    }
    out.set_item("stratum_depths", depths)?;

    out.set_item(
        "unserved_terms",
        unserved_list(py, planned.unserved_evidence())?,
    )?;

    let snapshot = PyDict::new(py);
    snapshot.set_item("source", &planned.statistics_snapshot.source)?;
    snapshot.set_item("revision", &planned.statistics_snapshot.revision)?;
    let entries = PyList::empty(py);
    for entry in &planned.statistics_snapshot.entries {
        let rendered = PyDict::new(py);
        rendered.set_item("subject", &entry.subject)?;
        rendered.set_item("cardinality", entry.cardinality)?;
        rendered.set_item("selectivity_ppm", entry.selectivity_ppm)?;
        entries.append(rendered)?;
    }
    snapshot.set_item("entries", entries)?;
    out.set_item("statistics", snapshot)?;

    out.set_item(
        "registry_content_fingerprint",
        &planned.registry_content_fingerprint,
    )?;
    Ok(out)
}

/// Render the per-term unserved evidence.
fn unserved_list(
    py: Python<'_>,
    unserved: Vec<crate::retrieval::UnservedTerm>,
) -> PyResult<Bound<'_, PyList>> {
    let out = PyList::empty(py);
    for term in unserved {
        let entry = PyDict::new(py);
        entry.set_item("request_term", term.request_term)?;
        entry.set_item("reason", unserved_reason(term.reason))?;
        out.append(entry)?;
    }
    Ok(out)
}

/// Render what a plan's recorded depths will cost in rank resolution, per
/// stratum, as the admission waist measured it before anything ran.
///
/// Keyed by stratum IRI in ascending order, which is the order the engine's own
/// map carries, so the dict a host reads is a pure function of the plan and the
/// law. `separates_to` is `None` when the law never stops separating inside any
/// depth a plan can express — a saturation, deliberately not a very large number
/// a caller could mistake for a measurement — and `fully_separated` is the one
/// bit that follows from comparing it with `requested_depth`, answered here so a
/// host does not re-derive the comparison (and the saturating case) itself.
fn planned_resolution_dict<'py>(
    py: Python<'py>,
    planned: &BTreeMap<Iri, PlannedResolution>,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    for (stratum, entry) in planned {
        let rendered = PyDict::new(py);
        rendered.set_item("separates_to", entry.separation.rank())?;
        rendered.set_item("requested_depth", entry.requested_depth)?;
        rendered.set_item("fully_separated", entry.fully_separated())?;
        out.set_item(stratum.as_str(), rendered)?;
    }
    Ok(out)
}

/// Render one compiled plan as a dict: the plan document, the per-stratum
/// SPARQL, the two identities that pin the units, and what the plan's depths
/// will cost in rank resolution under the law the caller named.
fn compile_dict<'py>(
    py: Python<'py>,
    planned: &Plan,
    compiled: &CompiledRetrieval,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("plan", plan_dict(py, planned)?)?;
    let units = PyList::empty(py);
    for unit in &compiled.units {
        let entry = PyDict::new(py);
        entry.set_item("stratum", unit.stratum.as_str())?;
        entry.set_item("sparql", &unit.sparql)?;
        units.append(entry)?;
    }
    out.set_item("units", units)?;
    out.set_item("plan_id", compiled.plan_id.to_hex())?;
    out.set_item("registry_fingerprint", &compiled.registry_fingerprint)?;
    // The waist's whole point: the cost of the plan's depths, readable here,
    // without executing a single unit. Empty when the call named no fusion law,
    // because resolution is measured against one and this binding invents none.
    out.set_item(
        "planned_resolution",
        planned_resolution_dict(py, &compiled.resolution)?,
    )?;
    Ok(out)
}

/// Render one fused answer as a dict.
fn search_dict<'py>(py: Python<'py>, result: &SearchResult) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);

    let rows = PyList::empty(py);
    for row in &result.rows {
        let entry = PyDict::new(py);
        entry.set_item("entity", row.entity.as_str())?;
        entry.set_item("score", row.score.to_decimal_lexical())?;
        entry.set_item(
            "threshold_witness",
            row.threshold_witness.to_decimal_lexical(),
        )?;
        let contributions = PyList::empty(py);
        for (stratum, rank, value) in &row.contributions {
            let contribution = PyDict::new(py);
            contribution.set_item("stratum", stratum.as_str())?;
            contribution.set_item("rank", rank)?;
            contribution.set_item("contribution", value.to_decimal_lexical())?;
            contributions.append(contribution)?;
        }
        entry.set_item("contributions", contributions)?;
        rows.append(entry)?;
    }
    out.set_item("rows", rows)?;

    let statuses = PyDict::new(py);
    for (stratum, status) in &result.trailer.statuses {
        let entry = PyDict::new(py);
        match status {
            ProducerStatus::Exhausted { rows_emitted } => {
                entry.set_item("status", "exhausted")?;
                entry.set_item("rows_emitted", rows_emitted)?;
            }
            ProducerStatus::DepthReached { rank } => {
                entry.set_item("status", "depth_reached")?;
                entry.set_item("rank", rank)?;
            }
            ProducerStatus::CeilingReached { bound } => {
                entry.set_item("status", "ceiling_reached")?;
                entry.set_item("bound", bound.to_decimal_lexical())?;
            }
            ProducerStatus::ExecutionFailed { reason } => {
                entry.set_item("status", "execution_failed")?;
                entry.set_item("reason", reason)?;
            }
            ProducerStatus::TermsRejected => {
                entry.set_item("status", "terms_rejected")?;
            }
        }
        statuses.set_item(stratum.as_str(), entry)?;
    }
    out.set_item("statuses", statuses)?;

    // What the index behind each handed stream attested, keyed by stratum. The
    // two axes are independent and each is independently absent: `None` under
    // "generation" is "this producer declared no generation", and `None` under
    // "incomplete" is "this producer said nothing about whether its index was
    // whole". Neither absence may be read as a claim — least of all the second,
    // which has no opposite to be read as: the engine-side `ServiceLevel` has no
    // `Whole` variant, deliberately, because a producer stopped at the engine's
    // row ceiling never looked at the rows it was licensed to skip and so could
    // not certify wholeness even if asked. The seam asks the narrower question
    // that has an honest answer on every path — *was your index NOT whole?* — so
    // silence here is silence.
    //
    // The key set is the streams this fusion was handed, and a stratum that
    // never became a stream is absent rather than reported as `Undeclared`:
    // "declined to answer" and "was never asked" are different facts.
    let attestations = PyDict::new(py);
    for (stratum, attestation) in &result.trailer.attestations {
        let entry = PyDict::new(py);
        entry.set_item(
            "generation",
            match &attestation.generation {
                IndexGeneration::Undeclared => None,
                IndexGeneration::Declared(generation) => Some(generation.as_str()),
            },
        )?;
        entry.set_item(
            "incomplete",
            match &attestation.service {
                ServiceLevel::Undeclared => None,
                ServiceLevel::Incomplete { reason } => Some(reason.as_str()),
            },
        )?;
        attestations.set_item(stratum.as_str(), entry)?;
    }
    out.set_item("attestations", attestations)?;

    // Whether the scores are exact, read off the same attestations. `False` does
    // not make the answer wrong: every row in it is a real row in this fusion's
    // own certified order, and every score is a LOWER BOUND on the score the
    // whole index would have produced. What does not follow is that a row absent
    // from the answer would have stayed absent, or that the emitted order would
    // have survived the missing contributions. The strata named are exactly the
    // ones that attested an incomplete index, in canonical order, and each one's
    // verbatim reason is under the same key in "attestations" — so the list is
    // the set of indexes to rebuild rather than a flag to shrug at.
    let exactness = PyDict::new(py);
    match &result.trailer.exactness {
        ScoreExactness::Exact => {
            exactness.set_item("exact", true)?;
            exactness.set_item("lower_bounds_for", Vec::<&str>::new())?;
        }
        ScoreExactness::LowerBounds { strata } => {
            exactness.set_item("exact", false)?;
            exactness.set_item(
                "lower_bounds_for",
                strata.iter().map(Iri::as_str).collect::<Vec<_>>(),
            )?;
        }
    }
    out.set_item("exactness", exactness)?;

    // The candidate-domain declaration each handed stream fused under, verbatim:
    // `None` where the producer promised only that it may name anything, and the
    // tag IRIs in canonical order where it restricted itself. It is on the
    // answer because it is an input the answer cannot otherwise be audited
    // against — these declarations decide which streams fusion was allowed to
    // skip when it certified a row, so a reader asking why a stratum stopped at
    // a bound instead of being read to its end is asking about this map.
    let domains = PyDict::new(py);
    for (stratum, declared) in &result.trailer.domains {
        domains.set_item(
            stratum.as_str(),
            declared
                .tags()
                .map(|tags| tags.iter().map(DomainTag::as_str).collect::<Vec<_>>()),
        )?;
    }
    out.set_item("domains", domains)?;

    // Rank resolution at two altitudes, kept apart by name because they answer
    // two different questions. `planned_resolution` is what the admission waist
    // said the plan's depths would cost, knowable before any row was read;
    // `observed_resolution` is what the rows this run actually pulled did cost.
    // They legitimately disagree — a top-k that certified early never reaches
    // its planned depth — and neither corrects the other.
    out.set_item(
        "planned_resolution",
        planned_resolution_dict(py, &result.planned_resolution)?,
    )?;

    // `separates_to` is `None` when the profile's contributions never collide
    // inside any expressible depth — a saturation, deliberately not a very large
    // number a caller could mistake for a measurement.
    let observed = PyDict::new(py);
    for (stratum, measured) in &result.trailer.resolution {
        let entry = PyDict::new(py);
        entry.set_item("separates_to", measured.separation.rank())?;
        entry.set_item("ranks_pulled", measured.ranks_pulled)?;
        entry.set_item("collisions_observed", measured.collisions_observed)?;
        observed.set_item(stratum.as_str(), entry)?;
    }
    out.set_item("observed_resolution", observed)?;
    out.set_item("cut_on_a_tie", result.trailer.cut_on_a_tie)?;

    out.set_item(
        "unserved_terms",
        unserved_list(py, result.unserved_terms.clone())?,
    )?;
    out.set_item(
        "unweighted_strata",
        result
            .unweighted_strata
            .iter()
            .map(Iri::as_str)
            .collect::<Vec<_>>(),
    )?;
    out.set_item("plan_id", result.plan_id.to_hex())?;
    out.set_item("profile_id", result.profile_id.to_hex())?;
    // The third identity, rendered exactly as its two siblings are: 64 lowercase
    // hex characters. It is the content identity of the attestation map above,
    // so two answers that agree on all three were assembled from the same
    // indexes in the same state — a difference no other field on this dict can
    // show, because a rebuilt index moves none of them.
    out.set_item("evidence_id", result.evidence_id.to_hex())?;
    Ok(out)
}

// ── the PyO3 surface ─────────────────────────────────────────────────────────

/// Plan one retrieval request against the declared ranked producers.
///
/// The pure stage: which producers the request reaches, which it does not and
/// the dimension that refused each, the per-stratum depths and weights, the
/// statistics the planner actually consulted, and the plan's canonical identity.
/// Nothing is executed, so this is the call a host makes to find out *why* a
/// request would answer the way it will.
#[pyfunction]
#[pyo3(signature = (data, request, *, text_producers, statistics, data_format="turtle", base=None))]
fn plan<'py>(
    py: Python<'py>,
    data: &str,
    request: &Bound<'py, PyAny>,
    text_producers: &Bound<'py, PyDict>,
    statistics: &Bound<'py, PyDict>,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(data, request, text_producers, statistics, data_format, base)?;
    // Parsing, index construction and planning run detached (GIL released); the
    // result dict is built after the GIL is reacquired.
    let planned = py
        .detach(|| run_plan(&call).map(|(_, _, planned)| planned))
        .map_err(PyValueError::new_err)?;
    plan_dict(py, &planned)
}

/// Plan, admit and emit: the per-stratum SPARQL the request compiles to.
///
/// Returns the plan document under `"plan"` and, under `"units"`, one entry per
/// stratum carrying the SPARQL text that stratum runs — the per-stratum depth
/// is already a bound the plan carries, and the emitted text carries it as its
/// own `LIMIT`. The admitted `"plan_id"` and the
/// `"registry_fingerprint"` the units were compiled against are alongside, so a
/// host that logs a unit can say exactly which plan and which registry it came
/// from. A host can read, log or execute those units itself; [`search`] is what
/// runs them and fuses their rows.
///
/// `"planned_resolution"` is what those depths will cost in rank resolution,
/// per stratum, **before** anything is executed: each entry names the
/// `"separates_to"` depth this law still tells adjacent ranks apart at (`None`
/// when it never stops inside a depth a plan can express), the
/// `"requested_depth"` the plan recorded, and `"fully_separated"` — whether
/// every rank the plan reads is still ordered by score alone. A `False` there is
/// not an error: past that depth the declared tie-break is total, so the answer
/// stays correct and deterministic at a coarser resolution, and
/// `retrieval.weight_for_depth` says what a finer one costs.
///
/// Resolution is measured against a fusion law, so it is reported only when the
/// call names one: pass `weights`, `k` and `decay` together, exactly as
/// [`search`] takes them, and `"planned_resolution"` carries an entry per
/// weighted stratum. Omit all three and it is empty — this binding invents no law
/// to measure against, and a resolution attributed to a law the host never chose
/// would be evidence about nothing. Naming some of them and not the rest is a
/// `ValueError` that says which part is missing, because the three are one law
/// between them.
///
/// `decay` is `"reciprocal_rank"` or `"weighted_reciprocal_rank"`, and it is the
/// part of the law that most changes the answer here: the depth a plan is fully
/// separated to is a property of the rule first and of the weight second. A
/// stratum the truncated rule reports as coarse may be fully separated under the
/// folded one at the same weight, so the rule the measurement is taken under is
/// the caller's to name and is never assumed.
#[pyfunction]
#[pyo3(signature = (
    data,
    request,
    *,
    text_producers,
    statistics,
    weights=None,
    k=None,
    decay=None,
    data_format="turtle",
    base=None,
))]
#[allow(
    clippy::too_many_arguments,
    reason = "the waist's inputs are named, not bundled"
)]
fn compile<'py>(
    py: Python<'py>,
    data: &str,
    request: &Bound<'py, PyAny>,
    text_producers: &Bound<'py, PyDict>,
    statistics: &Bound<'py, PyDict>,
    weights: Option<&Bound<'py, PyDict>>,
    k: Option<u32>,
    decay: Option<&str>,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(data, request, text_producers, statistics, data_format, base)?;
    // A law is its weights, its smoothing constant *and* its decay rule; any
    // part of one names no law at all, and silently supplying the rest would
    // report a resolution measured against arithmetic the host never wrote.
    let law = match (weights, k, decay) {
        (Some(weights), Some(k), Some(decay)) => Some((
            collect_weights(weights)?,
            decay_rule(decay, k).map_err(PyValueError::new_err)?,
        )),
        (None, None, None) => None,
        (weights, k, decay) => {
            return Err(PyValueError::new_err(partial_fusion_law(
                weights.is_some(),
                k.is_some(),
                decay.is_some(),
            )));
        }
    };
    // Parsing, index construction, planning and admission run detached.
    let (planned, compiled) = py
        .detach(|| {
            let profile = law
                .as_ref()
                .map(|(declared, decay)| build_profile(declared, *decay))
                .transpose()?;
            run_compile(&call, profile.as_ref())
        })
        .map_err(PyValueError::new_err)?;

    compile_dict(py, &planned, &compiled)
}

/// Run the whole retrieval ladder and return one fused answer.
///
/// `fuse ∘ execute ∘ compile ∘ plan`, over the declared producers and the
/// caller's own fusion law. The answer carries the fused `"rows"` with each
/// row's per-stratum `"contributions"`, every applicable producer's own terminal
/// `"statuses"`, the per-term `"unserved_terms"` evidence, any
/// `"unweighted_strata"` the profile declined to score, and the `"plan_id"` /
/// `"profile_id"` pair that names exactly which plan and which law produced it.
///
/// It also carries what rank resolution cost, at both altitudes and under two
/// distinct keys. `"planned_resolution"` is the admission waist's own map —
/// exactly what [`compile`] reports for the same request under the same law,
/// carried onto the answer so the one call that plans and executes together is
/// not the one call that cannot see it: per weighted stratum, the
/// `"separates_to"` depth, the `"requested_depth"` the plan recorded, and
/// `"fully_separated"`. `"observed_resolution"` is what the rows this run really
/// pulled cost, with an entry for every weighted stratum a stream was fused for
/// — including one that ended without emitting a row, whose `"ranks_pulled"` is
/// zero, because a stream that yielded nothing was still pulled from: the same
/// `"separates_to"` depth
/// (`None` when this law never stops separating inside an expressible depth),
/// the `"ranks_pulled"` this run reached, and the `"collisions_observed"` —
/// adjacent ranks the fused score could not tell apart, counted by observation
/// rather than inferred. The two disagree whenever a top-k certified before
/// reaching its planned depth, and that gap is the point: a depth a fusion never
/// reached cost it nothing.
///
/// Every `"statuses"` entry spells its own ending, and there are exactly five
/// spellings. `"exhausted"` (with `"rows_emitted"`) is the ONLY completeness
/// claim of the five: that producer emitted every row it had. The other four
/// each name who stopped the read and where. `"depth_reached"` (with `"rank"`)
/// is the producer stopping at the depth the plan gave it, verified against the
/// rows fusion really pulled: ranks one through `"rank"` were read and nothing
/// below it was looked at. `"ceiling_reached"` (with `"bound"`, an exact decimal
/// `str`) is a contribution bound: every row at or above it was read and the
/// rows below were not — usually written by a fusion the caller's `top_k`
/// stopped. `"execution_failed"` (with `"reason"`) is the producer that could
/// not run at all, and `"terms_rejected"` is the producer that declined the
/// request terms it was handed. A stratum that answered with nothing and one
/// that could not answer stay distinguishable, because none of the five is
/// reduced to an aggregate flag.
///
/// `"attestations"` maps each stratum whose stream was handed to fusion to what
/// the index behind it attested, as `{"generation": str | None, "incomplete":
/// str | None}`. Both axes are independent, and each `None` is an ABSENCE: no
/// generation was declared, or nothing was said about whether the index was
/// whole. Neither is a claim, and the second has no opposite to be mistaken for
/// one — the engine-side service level has no "whole" variant, because a
/// producer stopped at the engine's row ceiling never looked at the rows it was
/// licensed to skip. Read at the instant each stream was opened, so a stratum
/// the `top_k` later stopped still reports both facts.
///
/// `"exactness"` is `{"exact": bool, "lower_bounds_for": list[str]}`, derived
/// from those attestations alone and therefore unmoved by how deep this call
/// read. When `"exact"` is `False`, every score in the answer is a LOWER BOUND
/// on the score a whole index would have produced; the rows are still real rows
/// in this fusion's own certified order, and what does not follow is that a row
/// absent from the answer would have stayed absent. `"lower_bounds_for"` names
/// exactly the strata that attested an incomplete index, in canonical order, and
/// each one's verbatim reason is under the same key in `"attestations"`.
///
/// `"domains"` reports the candidate-domain declaration each handed stream fused
/// under — `None` where the producer promised only that it may name anything, a
/// sorted list of tag IRIs where it restricted itself — because those
/// declarations decide which streams fusion was allowed to skip when it
/// certified a row.
///
/// `"evidence_id"` is the content identity of `"attestations"`, rendered like
/// `"plan_id"` and `"profile_id"`: 64 lowercase hex characters. Two answers that
/// agree on all three were assembled from the same indexes in the same state,
/// which no other field on the answer can show — a rebuilt index moves the
/// dataset snapshot, the query text and the registry fingerprint not at all.
///
/// `"cut_on_a_tie"` says whether the last row in the answer beat a *settled*
/// rival it tied with exactly, so the final place was settled by the declared
/// tie-break rather than by relevance. Only rivals already final when that row
/// was emitted are counted, so `False` means "no settled rival tied with it" and
/// not "the cut was decided on a strict score difference" — a rival still live
/// could have risen to the same score had fusion read past the top-k, and
/// reading that far would move the `"ranks_pulled"` this same answer reports.
/// None of these is an error: past its separating depth a law still answers
/// correctly and deterministically, only more coarsely.
/// `retrieval.weight_for_depth` says what a finer answer costs.
///
/// `weights` maps a stratum IRI to its weight in raw fixed-point units, where
/// `retrieval.SCALE` is one whole unit; see this module's own documentation for
/// why every weight in one dict must be written in the same spelling.
///
/// `k`, `decay` and `top_k` are required: fused enumeration is top-k by
/// construction and the fusion law is the caller's, so none of them has a value
/// this binding could supply on the host's behalf. How many contributions a
/// candidate may receive is *not* a parameter — it is the number of weighted
/// strata.
///
/// `decay` names the rank-decay rule the whole answer is computed under, and it
/// reaches every number in that answer rather than only the law's identity.
/// `"reciprocal_rank"` truncates the reciprocal to the declared scale before the
/// weight is applied; `"weighted_reciprocal_rank"` folds the weight into the
/// numerator as one exactly-rounded division. The two produce different
/// contributions from the same weights, different `"planned_resolution"` and
/// `"observed_resolution"` maps, and different `"profile_id"` values, because
/// the rule is part of what the law's content identity fixes. An unknown
/// spelling is a `ValueError` naming both accepted ones.
#[pyfunction]
#[pyo3(signature = (
    data,
    request,
    *,
    text_producers,
    weights,
    statistics,
    k,
    decay,
    top_k,
    data_format="turtle",
    base=None,
))]
#[allow(
    clippy::too_many_arguments,
    reason = "the ladder's inputs are named, not bundled"
)]
fn search<'py>(
    py: Python<'py>,
    data: &str,
    request: &Bound<'py, PyAny>,
    text_producers: &Bound<'py, PyDict>,
    weights: &Bound<'py, PyDict>,
    statistics: &Bound<'py, PyDict>,
    k: u32,
    decay: &str,
    top_k: usize,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(data, request, text_producers, statistics, data_format, base)?;
    let declared = collect_weights(weights)?;
    // Read before the GIL is released, with every other Python-side argument:
    // the rule is owned Rust data by the time the ladder runs.
    let decay = decay_rule(decay, k).map_err(PyValueError::new_err)?;
    // The whole ladder runs detached (GIL released): parse, index, plan, admit,
    // execute and fuse. The answer dict is built after the GIL is reacquired.
    let result = py
        .detach(|| {
            let profile = build_profile(&declared, decay)?;
            run_search(&call, &profile, TopK::new(top_k))
        })
        .map_err(PyValueError::new_err)?;
    search_dict(py, &result)
}

/// The smallest stratum weight, in raw fixed-point units, that still separates
/// every adjacent pair of ranks up to `depth` **under the rule `decay` names**.
///
/// The design calculus read in the direction a profile author needs: name the
/// depth you must read to, get the weight that buys it. The answer is the true
/// minimum rather than a safe over-estimate: whether a weight reaches a depth
/// oscillates from one raw unit to the next, so the search walks the exact
/// candidate weights each adjacent-rank constraint admits instead of bisecting a
/// predicate that is not monotone. Weights are read as ratios, and an
/// over-estimate here would silently re-scale the stratum's share of every fused
/// score.
///
/// The rule is the caller's, because the two rules answer this question
/// differently and one of them cannot answer it at all past a point. `decay` is
/// `"reciprocal_rank"` or `"weighted_reciprocal_rank"`; an unknown spelling
/// raises `ValueError` naming both.
///
/// Raises `ValueError` for four further reasons, and the message says which.
///
/// A `k` of zero describes no law at all — the rule is refused before `depth` is
/// read, including at a `depth` of one, which has no adjacent pair to separate
/// and would otherwise hand back a real price under a rule that cannot be
/// evaluated. `retrieval.class_width` and `retrieval.deepest_rank_within_width`
/// refuse it identically, and none of the three reports it as the decay rule
/// running out of separation: switching rules is the remedy for a rule that
/// saturated, and it does nothing for a constant of zero.
///
/// A `depth` of zero names no rank to separate, so it is rejected rather than
/// answered. A `depth` past `2**32 - 1` is deeper than a plan can record — a
/// plan carries a per-stratum depth as a 32-bit rank — which is a limit of that
/// encoding and not of the arithmetic, and of the two walls below it is the only
/// one `"weighted_reciprocal_rank"` ever meets. And no weight at all reaches the
/// depth, which only `"reciprocal_rank"` raises and which is a real wall rather
/// than a budget: that rule rounds the reciprocal before the weight is applied,
/// so once two adjacent ranks collide there no weight can part them again. The
/// message reports the exact depth it does reach — the deepest any weight
/// reaches, not the depth of some particular one — and the remedy it points at
/// is reachable from right here: ask the same depth again under
/// `"weighted_reciprocal_rank"`, whose reachable depth grows with the weight.
///
/// Remember that weights are read as *ratios*. Raising one stratum to reach a
/// depth changes its share of every fused score; this reports what the depth
/// costs, not whether to pay it.
#[pyfunction]
#[pyo3(signature = (depth, k, *, decay))]
fn weight_for_depth(depth: u64, k: u32, decay: &str) -> PyResult<i128> {
    decay_rule(decay, k)
        .map_err(PyValueError::new_err)?
        .weight_for_depth(depth)
        .map(Fixed::into_raw)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// How many consecutive ranks around `rank` a weight of `weight_raw` cannot tell
/// apart **under the rule `decay` names**.
///
/// One means the rank is still separated from both neighbours by score alone. A
/// width of `w` means `w` consecutive ranks share a contribution, so their order
/// in the answer is settled by the declared tie-break rather than by relevance.
///
/// The answer is `None` when the class is still running at the deepest rank a
/// plan can express, exactly as `retrieval.deepest_rank_within_width` reports
/// its own saturation. A plan records a per-stratum depth as a 32-bit rank, so
/// there is no end inside its reach to count to, and `2**32 - 1` would be the
/// search's ceiling wearing a width's shape: under `"reciprocal_rank"` with a
/// raw weight of one every contribution truncates to zero and rank one's class
/// is the whole expressible range, and at a raw weight of fifty — fifty times
/// heavier — it still is. An `int` there would say those two classes are the
/// same size and invite a caller to log it, plot it, or divide by it.
///
/// This is the resolution curve, not the single point where it first exceeds
/// one: knowing a depth is coarse by four ranks rather than ten thousand is the
/// difference between an answer that is usable and one that is not. The curve
/// belongs to the rule, so the width the two rules report at one weight and one
/// rank legitimately differs, and `decay` — `"reciprocal_rank"` or
/// `"weighted_reciprocal_rank"` — says which curve was read.
///
/// This asks a question about arithmetic and takes no stratum, because a width
/// is a property of the rule, its smoothing constant, the weight and the rank and
/// of nothing else. `weight_raw` is in raw fixed-point units, where
/// `retrieval.SCALE` is one whole unit.
///
/// An operand the law cannot evaluate raises `ValueError` rather than returning
/// a width: a rank of zero, which is not a rank; a smoothing constant of zero;
/// an unknown `decay` spelling; and a weight that is not strictly positive,
/// which a fusion law refuses where it is declared and which therefore has no
/// resolution to report here either. A width of one is the claim that a rank is
/// perfectly separated from its neighbours, and returning it where nothing was
/// measured would report the most favourable resolution there is at exactly the
/// point no resolution was computed.
#[pyfunction]
#[pyo3(signature = (weight_raw, k, rank, *, decay))]
fn class_width(weight_raw: i128, k: u32, rank: u64, decay: &str) -> PyResult<Option<u64>> {
    if weight_raw <= 0 {
        return Err(PyValueError::new_err(format!(
            "a stratum weight is strictly positive, and {weight_raw} raw fixed-point units is \
             not; a fusion law refuses a non-positive weight where it is declared, so there is \
             no rank resolution to report for one here"
        )));
    }
    decay_rule(decay, k)
        .map_err(PyValueError::new_err)?
        .class_width(Fixed::from_raw(weight_raw), rank)
        // The saturating case is rendered the way every other wall on this
        // surface is: `None`, because the class has no end inside any plan's
        // reach to count to, not an enormous width.
        .map(ClassWidth::width)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// The deepest depth that can be read with every rank in it sitting in a
/// class no wider than `max_width`, for a weight of `weight_raw` **under the
/// rule `decay` names**.
///
/// This inverts [`class_width`]: name the tolerance you can live with, get the
/// depth it buys. `max_width` of one is the separating depth itself — the
/// deepest depth a read can stop at with every rank it *actually read*
/// separated from both of its neighbours within that read. It is a depth, not a
/// rank property, and the difference is one call away: `retrieval.class_width`
/// at that rank never reports the one a "still separated from both neighbours"
/// reading would predict, because the unbounded curve it walks also looks at the
/// one rank the bounded read never reaches. Where it counts a width at all that
/// width is at least `max_width + 1`, and it is exactly `max_width + 1` only
/// where the run that ends the walk is one rank longer than the tolerance — the
/// smooth case, not the rule. A light weight is where the difference shows:
/// under `"reciprocal_rank"` with `k` of 60 and a raw weight of `10**2`, a
/// tolerance of one lands on depth one, whose class is forty ranks wide. Where
/// that run reaches the end of the expressible range `retrieval.class_width` is
/// `None` there, having no width to compare. The two agree; they are answering
/// a depth question and a rank question. Larger tolerances
/// answer the question a caller reading deeply actually has: not "where does
/// this stop being exact" but "how far can I read and still have ranks ordered
/// to within the resolution I can live with".
///
/// The answer is `None` when no depth a plan can express ever exceeds the
/// tolerance, exactly as `"separates_to"` is `None` on a `retrieval.search`
/// answer for a law that never stops separating. A plan records a per-stratum
/// depth as a 32-bit rank, so there is no bound inside its reach to report, and
/// `2**32 - 1` is a saturation point rather than a reading: two weights fifty
/// times apart both land on it, and an `int` there would say they reach the
/// same depth and invite a caller to log it, plot it, or divide by it.
///
/// This asks a question about arithmetic and takes no stratum, exactly as
/// [`class_width`] does: the answer is a property of the rule, its smoothing
/// constant, the weight and the tolerance, and of nothing else. `weight_raw`
/// is in raw fixed-point units, where `retrieval.SCALE` is one whole unit.
///
/// The curve belongs to the rule, so the depth the two rules report at one
/// weight and one tolerance legitimately differs, and `decay` —
/// `"reciprocal_rank"` or `"weighted_reciprocal_rank"` — says which curve was
/// read.
///
/// An operand the law cannot evaluate raises `ValueError` rather than
/// returning a depth: a smoothing constant of zero; a `max_width` of zero,
/// because a class always contains its own rank and so a tolerance of zero is
/// not a tolerance; an unknown `decay` spelling; and a weight that is not
/// strictly positive, which a fusion law refuses where it is declared and which
/// therefore has no resolution to report here either. The constant and the
/// tolerance are both checked before anything is measured, so neither refusal
/// depends on the other argument — a tolerance of one does no walking, and
/// letting it answer where a larger tolerance refuses would make the same
/// unusable rule usable or not according to a question asked of it.
///
/// # What it costs to ask
///
/// The answer is walked rank by rank, because the class width is not monotone
/// in the rank and bisecting it would silently over-report. The walk starts at
/// the separating depth `retrieval.weight_for_depth` prices — every class below
/// that is a singleton by definition — and stops at the first run of
/// `max_width + 1` ranks sharing one contribution. The answer lands near
/// `sqrt(max_width)` times the separating depth, so the walk is about
/// `sqrt(max_width) - 1` times that depth, one integer division per step. A
/// `max_width` of one does no walking at all and a small tolerance costs a
/// fraction of the separating depth; a large tolerance at a heavy weight under
/// `"weighted_reciprocal_rank"`, where the separating depth itself grows with
/// the weight, walks very far.
///
/// The walk stops at the deepest depth a plan can record — reporting `None`
/// there rather than that ceiling — which bounds it at fewer than `2^32` steps
/// however it is asked; it cannot fail to terminate, and the GIL is not
/// released while it runs. It is a design-time question all the same — price a
/// depth budget once while choosing weights — and not something to put in a hot
/// loop.
#[pyfunction]
#[pyo3(signature = (weight_raw, k, max_width, *, decay))]
fn deepest_rank_within_width(
    weight_raw: i128,
    k: u32,
    max_width: u64,
    decay: &str,
) -> PyResult<Option<u64>> {
    if weight_raw <= 0 {
        return Err(PyValueError::new_err(format!(
            "a stratum weight is strictly positive, and {weight_raw} raw fixed-point units is \
             not; a fusion law refuses a non-positive weight where it is declared, so there is \
             no rank resolution to report for one here"
        )));
    }
    decay_rule(decay, k)
        .map_err(PyValueError::new_err)?
        .deepest_rank_within_width(Fixed::from_raw(weight_raw), max_width)
        // The saturating case is rendered the way `"separates_to"` renders it
        // on a `search` answer: `None`, because there is no bound inside any
        // plan's reach to report, not an enormous one.
        .map(ToleratedDepth::rank)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Register the `purrdf-retrieval` surface on a Python module. Called by the
/// unified `purrdf_native` cdylib to populate the `purrdf_native.retrieval`
/// submodule.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("SCALE_DIGITS", SCALE_DIGITS)?;
    m.add("SCALE", SCALE)?;
    m.add_function(wrap_pyfunction!(plan, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(weight_for_depth, m)?)?;
    m.add_function(wrap_pyfunction!(class_width, m)?)?;
    m.add_function(wrap_pyfunction!(deepest_rank_within_width, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "https://example.org/note";
    const TITLE: &str = "https://example.org/title";
    const TEXT_PRODUCER: &str = "https://example.org/pf/search";
    const TITLE_PRODUCER: &str = "https://example.org/pf/title";
    const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
    const TITLE_STRATUM: &str = "https://example.org/stratum/title";

    /// The fixture law's rule: the reciprocal truncated before the weight lands.
    const TRUNCATED: DecayRule = DecayRule::ReciprocalRank { k: 60 };
    /// The other rule, at the same smoothing constant: the weight folded into
    /// the numerator.
    const FOLDED: DecayRule = DecayRule::WeightedReciprocalRank { k: 60 };

    const DATA: &str = concat!(
        "<https://example.org/a> <https://example.org/note> \"the quick brown fox\" ;\n",
        "  <https://example.org/title> \"quick notes\" .\n",
        "<https://example.org/b> <https://example.org/note> \"a quick red fox\" ;\n",
        "  <https://example.org/title> \"red herrings\" .\n",
    );

    fn producer(iri: &str, stratum: &str, predicate: &str) -> TextProducer {
        TextProducer {
            producer: iri.to_owned(),
            stratum: stratum.to_owned(),
            predicate: predicate.to_owned(),
            graph: GraphSpec::Any,
            domains: None,
        }
    }

    fn call(terms: Vec<RequestTerm>, producers: Vec<TextProducer>) -> Call {
        Call {
            data: DATA.to_owned(),
            media_type: NativeRdfFormat::Turtle.media_type(),
            base: None,
            request: RetrievalRequest::from_terms(terms),
            producers,
            statistics: HostStatistics {
                source: "host-statistics".to_owned(),
                revision: "r1".to_owned(),
                ..HostStatistics::default()
            },
        }
    }

    fn lexical(text: &str, predicate: &str) -> RequestTerm {
        RequestTerm::Lexical {
            text: text.to_owned(),
            language: None,
            predicate: Some(Iri::parse(predicate).expect("fixture predicate")),
        }
    }

    fn unit_weights(strata: &[&str]) -> Vec<(String, i128)> {
        strata
            .iter()
            .map(|stratum| ((*stratum).to_owned(), SCALE))
            .collect()
    }

    /// The headline: one request reaches two real producers over real data and
    /// comes back as one fused ranking whose rows carry both provenances.
    #[test]
    fn two_text_producers_fuse_into_one_ranking() {
        let call = call(
            vec![lexical("quick fox", NOTE), lexical("quick", TITLE)],
            vec![
                producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE),
                producer(TITLE_PRODUCER, TITLE_STRATUM, TITLE),
            ],
        );
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM, TITLE_STRATUM]), TRUNCATED)
            .expect("the fixture profile is valid");
        let result = run_search(&call, &profile, TopK::new(10)).expect("the producers answer");

        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| {
                    (
                        row.entity.as_str().to_owned(),
                        row.contributions
                            .iter()
                            .map(|(stratum, rank, _)| (stratum.as_str().to_owned(), *rank))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "<https://example.org/a>".to_owned(),
                    vec![(TEXT_STRATUM.to_owned(), 1), (TITLE_STRATUM.to_owned(), 1),],
                ),
                (
                    "<https://example.org/b>".to_owned(),
                    vec![(TEXT_STRATUM.to_owned(), 2)],
                ),
            ],
            "ex:a holds the needle in both indexed fields; ex:b only in the note"
        );
        assert!(
            result.unserved_terms.is_empty(),
            "every request term reached a producer"
        );
        assert_eq!(result.profile_id, profile.id());
    }

    /// A modality no registered producer accepts is reported per term, never
    /// silently dropped.
    #[test]
    fn an_unaccepted_modality_is_reported_per_term() {
        let call = call(
            vec![
                lexical("quick fox", NOTE),
                RequestTerm::EntitySeed {
                    entity: Term::new("<https://example.org/a>"),
                },
            ],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), TRUNCATED)
            .expect("the fixture profile is valid");
        let result = run_search(&call, &profile, TopK::new(10)).expect("the producer answers");

        assert_eq!(result.unserved_terms.len(), 1);
        assert_eq!(result.unserved_terms[0].request_term, 1);
        assert_eq!(
            unserved_reason(result.unserved_terms[0].reason),
            "no_producer_accepts"
        );
        assert!(
            !result.rows.is_empty(),
            "the term that WAS served still answers; an unserved neighbour is not a refusal"
        );
    }

    /// The compile stage emits the stratum's SPARQL with the needle as a
    /// rendered constant, so a host can read exactly what will run.
    #[test]
    fn compile_emits_the_stratum_sparql() {
        let call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        let (planned, compiled) = run_compile(&call, None).expect("a fresh plan is admitted");
        assert_eq!(compiled.units.len(), 1);
        assert_eq!(compiled.units[0].stratum.as_str(), TEXT_STRATUM);
        assert!(
            compiled.units[0].sparql.contains("\"quick fox\""),
            "the needle is a rendered constant: {}",
            compiled.units[0].sparql
        );
        assert!(
            compiled.units[0].sparql.contains(TEXT_PRODUCER),
            "the unit calls the producer the plan bound: {}",
            compiled.units[0].sparql
        );
        assert_eq!(planned.statistics_snapshot.source, "host-statistics");
        assert!(
            compiled.resolution.is_empty(),
            "a call that named no fusion law is told nothing about resolution, rather than being \
             handed evidence measured against a law it never chose"
        );
    }

    /// The waist answers what a plan will cost, without executing it: a call
    /// that names the law it means to fuse under gets the resolution evidence
    /// back from `compile`, not only from `search`.
    #[test]
    fn compile_reports_what_the_planned_depths_cost_under_a_named_law() {
        let call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), TRUNCATED)
            .expect("the fixture profile is valid");
        let (planned, compiled) =
            run_compile(&call, Some(&profile)).expect("a fresh plan is admitted under a law");
        let stratum = Iri::parse(TEXT_STRATUM).expect("fixture stratum");
        let recorded = compiled
            .resolution
            .get(&stratum)
            .copied()
            .expect("a weighted stratum's resolution is recorded at the waist");
        assert!(
            recorded.fully_separated(),
            "a unit weight separates every rank this small plan reads"
        );
        assert_eq!(
            recorded.requested_depth, planned.stratum_depths[&stratum],
            "the evidence names the depth the plan recorded"
        );
    }

    /// Statistics are the host's, recorded verbatim, and a measured cardinality
    /// lowers a stratum's depth rather than raising it.
    #[test]
    fn host_statistics_bound_the_planned_depth() {
        let mut call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        let (_, _, unbounded) = run_plan(&call).expect("the request plans");
        let declared = unbounded
            .stratum_depths
            .values()
            .copied()
            .next()
            .expect("the plan places one stratum");

        call.statistics
            .cardinality
            .insert(TEXT_STRATUM.to_owned(), 1);
        let (_, _, bounded) = run_plan(&call).expect("the request plans");
        assert_eq!(
            bounded.stratum_depths.values().copied().next(),
            Some(1),
            "a measured cardinality of one caps the stratum at one row (declared {declared})"
        );
        assert_eq!(bounded.statistics_snapshot.revision, "r1");
        assert_eq!(
            bounded.statistics_snapshot.entries.len(),
            1,
            "the planner records the statistic it consulted"
        );
    }

    /// An undeclared domain is the widest promise, a declared one is read
    /// through the layer's own IRI parser, and an EMPTY declaration is refused
    /// where it is written.
    ///
    /// The refusal is the interesting one, and the two valid neighbours are why:
    /// an empty restriction is one line away from the unrestricted declaration
    /// and means the opposite of it, so reading it as "no restriction" would be
    /// the silent repair that registers a producer promising to name nothing.
    #[test]
    fn candidate_domains_read_the_hosts_declaration_and_refuse_an_empty_one() {
        assert_eq!(
            candidate_domains(TEXT_PRODUCER, None).expect("an absent declaration is the widest"),
            CandidateDomains::Unrestricted
        );

        let declared = candidate_domains(
            TEXT_PRODUCER,
            Some(&[
                "https://example.org/domain/notes".to_owned(),
                "https://example.org/domain/titles".to_owned(),
            ]),
        )
        .expect("two tags are a restriction");
        let tags: Vec<&str> = declared
            .tags()
            .expect("a restriction names its blocks")
            .iter()
            .map(DomainTag::as_str)
            .collect();
        assert_eq!(
            tags,
            [
                "https://example.org/domain/notes",
                "https://example.org/domain/titles"
            ],
            "the blocks are carried in canonical order, not the host's"
        );

        let empty = candidate_domains(TEXT_PRODUCER, Some(&[]))
            .expect_err("a promise to name nothing is refused");
        assert!(empty.contains(TEXT_PRODUCER), "got {empty}");
        assert!(empty.contains("empty list"), "got {empty}");

        let bad = candidate_domains(TEXT_PRODUCER, Some(&["not an iri".to_owned()]))
            .expect_err("a tag that is not an IRI is refused where it is written");
        assert!(bad.contains("domain tag"), "got {bad}");
    }

    /// A declared domain reaches the registry, and the answer reports it back:
    /// the declaration is an input the answer can be audited against.
    #[test]
    fn a_declared_domain_reaches_the_fused_answer() {
        let mut declared = producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE);
        declared.domains = Some(vec!["https://example.org/domain/notes".to_owned()]);
        let declared_call = call(vec![lexical("quick fox", NOTE)], vec![declared]);
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), TRUNCATED)
            .expect("the fixture profile is valid");
        let result =
            run_search(&declared_call, &profile, TopK::new(10)).expect("the declared run answers");
        let stratum = Iri::parse(TEXT_STRATUM).expect("fixture stratum");
        assert_eq!(
            result.trailer.domains.get(&stratum).and_then(|d| d
                .tags()
                .map(|tags| tags.iter().map(DomainTag::as_str).collect::<Vec<_>>())),
            Some(vec!["https://example.org/domain/notes"])
        );

        // The neighbouring undeclared run answers identically, which is the
        // whole claim: a declaration changes the reading, never the answer.
        let plain_call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        let unrestricted =
            run_search(&plain_call, &profile, TopK::new(10)).expect("the undeclared run answers");
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| (row.entity.as_str(), row.score.to_decimal_lexical()))
                .collect::<Vec<_>>(),
            unrestricted
                .rows
                .iter()
                .map(|row| (row.entity.as_str(), row.score.to_decimal_lexical()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            unrestricted.trailer.domains.get(&stratum),
            Some(&CandidateDomains::Unrestricted)
        );
    }

    /// A registry with no producer is refused by name rather than answered with
    /// an empty ranking, because PurRDF has no default producer to fall back on.
    #[test]
    fn an_empty_producer_set_is_refused() {
        let call = call(vec![lexical("quick fox", NOTE)], Vec::new());
        let error = run_plan(&call).expect_err("no producer is a refusal");
        assert!(error.contains("no ranked producers"), "got {error}");
    }

    /// A multi-partition index cannot honestly declare a ranked order, and the
    /// relation's own refusal reaches the host.
    #[test]
    fn a_multi_partition_index_refuses_to_declare_a_ranking() {
        const TAGGED: &str = concat!(
            "<https://example.org/a> <https://example.org/note> \"the quick fox\"@en .\n",
            "<https://example.org/b> <https://example.org/note> \"le renard vif\"@fr .\n",
        );
        let mut call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        call.data = TAGGED.to_owned();
        let error = run_plan(&call).expect_err("two languages are two partitions");
        assert!(error.contains("partitions"), "got {error}");
    }

    /// The neighbouring valid case: the same corpus in one language declares a
    /// ranked order and answers.
    #[test]
    fn a_single_partition_index_declares_a_ranking_and_answers() {
        const TAGGED: &str = concat!(
            "<https://example.org/a> <https://example.org/note> \"the quick fox\"@en .\n",
            "<https://example.org/b> <https://example.org/note> \"a quick hound\"@en .\n",
        );
        let mut call = call(
            vec![lexical("quick", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        call.data = TAGGED.to_owned();
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), TRUNCATED)
            .expect("the fixture profile is valid");
        let result = run_search(&call, &profile, TopK::new(10)).expect("one partition answers");
        assert_eq!(result.rows.len(), 2, "both documents hold the needle");
    }

    /// Weights cross as exact raw units, and a non-positive one is refused by
    /// the fusion law rather than silently ordering nothing.
    #[test]
    fn weights_are_exact_and_a_non_positive_one_is_refused() {
        let profile = build_profile(&[(TEXT_STRATUM.to_owned(), SCALE / 2)], TRUNCATED)
            .expect("half a unit is a valid weight");
        assert_eq!(
            profile
                .weight(&Iri::parse(TEXT_STRATUM).expect("fixture stratum"))
                .map(Fixed::to_decimal_lexical)
                .as_deref(),
            Some("0.500000000000")
        );
        assert!(
            build_profile(&[(TEXT_STRATUM.to_owned(), 0)], TRUNCATED).is_err(),
            "a zero weight is not a weight"
        );
    }

    /// Graph selectors route by their spelling, and an unknown one is a typed
    /// refusal naming what is accepted.
    #[test]
    fn graph_selectors_route_by_spelling() {
        assert_eq!(
            GraphSpec::parse(TEXT_PRODUCER, "any").expect("any"),
            GraphSpec::Any
        );
        assert_eq!(
            GraphSpec::parse(TEXT_PRODUCER, "default").expect("default"),
            GraphSpec::Default
        );
        assert_eq!(
            GraphSpec::parse(TEXT_PRODUCER, "https://example.org/g").expect("named"),
            GraphSpec::Named("https://example.org/g".to_owned())
        );
        assert!(GraphSpec::parse(TEXT_PRODUCER, "every").is_err());
    }

    /// Decay rules route by their spelling, and an unknown one is a typed
    /// refusal naming both accepted ones — there being no spelling that means
    /// "whichever".
    #[test]
    fn decay_rules_route_by_spelling() {
        assert_eq!(
            decay_rule("reciprocal_rank", 60).expect("the truncated rule"),
            TRUNCATED
        );
        assert_eq!(
            decay_rule("weighted_reciprocal_rank", 60).expect("the folded rule"),
            FOLDED
        );
        let error = decay_rule("rrf", 60).expect_err("an unknown rule is refused");
        assert!(error.contains("unknown decay rule"), "got {error}");
        assert!(
            error.contains("\"reciprocal_rank\"") && error.contains("\"weighted_reciprocal_rank\""),
            "the refusal names both accepted spellings: {error}"
        );
    }

    /// The rule reaches the arithmetic, not only the profile's identity: the two
    /// rules compute different contributions from the same weight and rank.
    #[test]
    fn the_two_rules_fuse_the_same_corpus_to_different_scores() {
        let call = call(
            vec![lexical("quick fox", NOTE)],
            vec![producer(TEXT_PRODUCER, TEXT_STRATUM, NOTE)],
        );
        // A weight of one and a half units: the folded rule's single division
        // keeps a raw unit the truncated rule's inner rounding discards.
        let weights = [(TEXT_STRATUM.to_owned(), SCALE + SCALE / 2)];
        let under = |decay| {
            let profile = build_profile(&weights, decay).expect("the fixture profile is valid");
            let result = run_search(&call, &profile, TopK::new(10)).expect("the producer answers");
            (
                profile.id(),
                result.rows[0].score.to_decimal_lexical(),
                result.profile_id,
            )
        };
        let (truncated_id, truncated_score, truncated_answer_id) = under(TRUNCATED);
        let (folded_id, folded_score, folded_answer_id) = under(FOLDED);

        assert_ne!(
            truncated_score, folded_score,
            "the rule decides the number, so naming it has to change the answer"
        );
        assert_ne!(
            truncated_id, folded_id,
            "the rule is part of what the law's content identity fixes"
        );
        assert_eq!(truncated_answer_id, truncated_id);
        assert_eq!(folded_answer_id, folded_id);
    }

    /// The refusal for a half-named law says which part arrived and which did
    /// not, for every partial combination, and never fills the absent one in.
    #[test]
    fn a_partly_named_fusion_law_names_the_part_that_is_missing() {
        let message = partial_fusion_law(true, true, false);
        assert!(
            message.contains("named `weights` and the smoothing constant `k`"),
            "got {message}"
        );
        assert!(
            message.contains("left the `decay` rule unnamed"),
            "got {message}"
        );

        let message = partial_fusion_law(false, true, false);
        assert!(
            message.contains("named the smoothing constant `k`"),
            "got {message}"
        );
        assert!(
            message.contains("left `weights` and the `decay` rule unnamed"),
            "got {message}"
        );

        let message = partial_fusion_law(true, false, false);
        assert!(
            message.contains("left the smoothing constant `k` and the `decay` rule unnamed"),
            "got {message}"
        );
    }

    /// Data format names route to media types, and an unknown one is refused.
    #[test]
    fn data_format_names_route_to_media_types() {
        assert_eq!(data_media_type("turtle").expect("turtle"), "text/turtle");
        assert_eq!(
            data_media_type("ntriples").expect("ntriples"),
            "application/n-triples"
        );
        assert_eq!(
            data_media_type("nquads").expect("nquads"),
            "application/n-quads"
        );
        assert!(data_media_type("trix").is_err());
    }
}
