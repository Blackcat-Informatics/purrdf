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
//!   depths and weights, the statistics snapshot the planner consulted, and
//!   every request term that reached no producer at all.
//! * [`compile`] — semantic admission plus emission. Returns the per-stratum
//!   SPARQL a host can read, run or log verbatim.
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
//! default weight, and no invented statistics revision: `k` and `top_k` are
//! required keywords for the same reason.
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

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::retrieval::{
    AdmissionEnvironment, CompiledRetrieval, Fixed, FusionProfile, Iri, Metric, Plan,
    ProducerDecision, ProducerStatus, RejectionReason, RequestTerm, RetrievalRequest, SearchResult,
    Statistics, Term, TopK, UnservedReason,
};
use crate::text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};
use crate::{NativeRdfFormat, RdfDataset, TermValue, parse_dataset};
use purrdf_sparql_eval::PropertyFunctionRegistry;

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
        let declaration = relation
            .ranked_declaration(stratum, Some(producer.predicate.clone()))
            .map_err(|e| format!("text producer <{}>: {e}", producer.producer))?;
        registry.register_ranked(&producer.producer, Arc::new(relation), declaration);
    }
    Ok(registry)
}

/// Build the fusion law from the host's weights and smoothing constant.
///
/// How many contributions a candidate may receive is not an argument: it is the
/// number of weighted strata, because a candidate surfaces at most once in each.
fn build_profile(weights: &[(String, i128)], k: u32) -> Result<FusionProfile, String> {
    let mut declared = BTreeMap::new();
    for (stratum, raw) in weights {
        declared.insert(
            retrieval_iri("fusion weight stratum", stratum)?,
            Fixed::from_raw(*raw),
        );
    }
    FusionProfile::new(declared, k).map_err(|e| e.to_string())
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
fn run_compile(call: &Call) -> Result<(Plan, CompiledRetrieval), String> {
    let (_, registry, planned) = run_plan(call)?;
    let environment = AdmissionEnvironment {
        registry: &registry,
        statistics: &call.statistics,
        fusion_profile: None,
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
/// registers: `producer_iri -> (stratum_iri, predicate_iri, graph)`.
fn collect_producers(producers: &Bound<'_, PyDict>) -> PyResult<Vec<TextProducer>> {
    let mut declared = Vec::with_capacity(producers.len());
    for (key, value) in producers {
        let producer: String = key
            .extract()
            .map_err(|_| PyTypeError::new_err("text producer keys must be IRI strings"))?;
        let fields: Vec<Bound<'_, PyAny>> = value.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "text producer <{producer}>: the value is (stratum, predicate, graph)"
            ))
        })?;
        let [stratum, predicate, graph] =
            <[Bound<'_, PyAny>; 3]>::try_from(fields).map_err(|_| {
                PyTypeError::new_err(format!(
                    "text producer <{producer}>: the value is (stratum, predicate, graph)"
                ))
            })?;
        let field = |label: &str, value: &Bound<'_, PyAny>| -> PyResult<String> {
            value.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "text producer <{producer}>: `{label}` must be a string"
                ))
            })
        };
        let graph =
            GraphSpec::parse(&producer, &field("graph", &graph)?).map_err(PyValueError::new_err)?;
        declared.push(TextProducer {
            stratum: field("stratum", &stratum)?,
            predicate: field("predicate", &predicate)?,
            graph,
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
    }
}

/// The wire spelling of a per-term unserved reason.
const fn unserved_reason(reason: UnservedReason) -> &'static str {
    match reason {
        UnservedReason::NoProducerAccepts => "no_producer_accepts",
        UnservedReason::EveryAcceptingProducerRejected => "every_accepting_producer_rejected",
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

    let weights = PyDict::new(py);
    for (stratum, weight) in &planned.stratum_weights {
        weights.set_item(stratum.as_str(), weight.into_raw())?;
    }
    out.set_item("stratum_weights", weights)?;

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
#[pyfunction]
#[pyo3(signature = (data, request, *, text_producers, statistics, data_format="turtle", base=None))]
fn compile<'py>(
    py: Python<'py>,
    data: &str,
    request: &Bound<'py, PyAny>,
    text_producers: &Bound<'py, PyDict>,
    statistics: &Bound<'py, PyDict>,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(data, request, text_producers, statistics, data_format, base)?;
    // Parsing, index construction, planning and admission run detached.
    let (planned, compiled) = py
        .detach(|| run_compile(&call))
        .map_err(PyValueError::new_err)?;

    let out = PyDict::new(py);
    out.set_item("plan", plan_dict(py, &planned)?)?;
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
    Ok(out)
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
/// `weights` maps a stratum IRI to its weight in raw fixed-point units, where
/// `retrieval.SCALE` is one whole unit; see this module's own documentation for
/// why every weight in one dict must be written in the same spelling.
///
/// `k` and `top_k` are required: fused enumeration is top-k by construction and
/// the fusion law is the caller's, so neither has a value this binding could
/// supply on the host's behalf. How many contributions a candidate may receive
/// is *not* a parameter — it is the number of weighted strata.
#[pyfunction]
#[pyo3(signature = (
    data,
    request,
    *,
    text_producers,
    weights,
    statistics,
    k,
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
    top_k: usize,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(data, request, text_producers, statistics, data_format, base)?;
    let declared = collect_weights(weights)?;
    // The whole ladder runs detached (GIL released): parse, index, plan, admit,
    // execute and fuse. The answer dict is built after the GIL is reacquired.
    let result = py
        .detach(|| {
            let profile = build_profile(&declared, k)?;
            run_search(&call, &profile, TopK::new(top_k))
        })
        .map_err(PyValueError::new_err)?;
    search_dict(py, &result)
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
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM, TITLE_STRATUM]), 60)
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
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), 60)
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
        let (planned, compiled) = run_compile(&call).expect("a fresh plan is admitted");
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
        let profile = build_profile(&unit_weights(&[TEXT_STRATUM]), 60)
            .expect("the fixture profile is valid");
        let result = run_search(&call, &profile, TopK::new(10)).expect("one partition answers");
        assert_eq!(result.rows.len(), 2, "both documents hold the needle");
    }

    /// Weights cross as exact raw units, and a non-positive one is refused by
    /// the fusion law rather than silently ordering nothing.
    #[test]
    fn weights_are_exact_and_a_non_positive_one_is_refused() {
        let profile = build_profile(&[(TEXT_STRATUM.to_owned(), SCALE / 2)], 60)
            .expect("half a unit is a valid weight");
        assert_eq!(
            profile
                .weight(&Iri::parse(TEXT_STRATUM).expect("fixture stratum"))
                .map(Fixed::to_decimal_lexical)
                .as_deref(),
            Some("0.500000000000")
        );
        assert!(
            build_profile(&[(TEXT_STRATUM.to_owned(), 0)], 60).is_err(),
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
