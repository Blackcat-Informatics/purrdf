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
//! `top_k` is required on **all three** entry points, including the two that
//! execute nothing, because the row bound is a planning input rather than a
//! trailing preference. It is what each stratum's depth is derived from wherever
//! the producers' own `domains` declarations make that sound, so a `plan` or
//! `compile` call without it would report a depth, a `LIMIT` and an identity for a
//! read nobody asked for.
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
//! A list of **more than one** tag is refused by name as well, and the refusal is
//! about what this binding can register rather than about the declaration. One
//! tag says where every row of this producer lies, so a consumer reads the block
//! off the declaration and no row has to repeat it. Several tags say only that
//! the rows lie somewhere in that set, which obliges the producer to name each
//! row's own block — and the two ranked relations this surface builds project a
//! candidate and a score and declare no column such a block could be read out
//! of, because a tag describes how a host's corpora partition and only the host
//! knows that. Refused at registration, where a caller can act on it, rather
//! than at the first row pulled, where it arrives as a failure of the fusion.
//! The exits are the ones the refusal names: one tag, one producer per block, or
//! `None`.
//!
//! Nothing is defaulted from a stratum or from a graph, here or below. Which
//! entities a text index names is a fact about the host's corpus that neither
//! this layer nor the relation can see, and a tag derived per stratum would hand
//! two producers over one entity space a pair of tags a consumer reads as
//! disjoint. That mistake is not conservative in either direction: it refuses a
//! valid query where both producers name one entity, and certifies a score
//! missing the other's contribution where they do not.
//!
//! # What only the host can say about the index behind a producer
//!
//! A fifth element may follow the four above — `(stratum, predicate, graph,
//! domains, (generation, incompleteness))` — and it is the one part of a producer
//! that nothing crossing this boundary can express. Two facts can change an
//! answer while every input the engine sees stays identical: WHICH version of the
//! host's index answered, and whether that index was WHOLE. A corpus read out of
//! a search index mid-rebuild is the same document as one read out of a whole
//! index, so if the host does not say, nothing can.
//!
//! Each member is a `str` or `None`, recorded verbatim and never parsed, and each
//! axis is independently absent. `None` is SILENCE on both: never a claim that
//! the index was current, and never a certificate that it was whole. A spec that
//! writes no attestation position declares exactly that silence, which is what
//! every `text_producers` value declared before this position existed.
//!
//! The two axes reach the answer differently, and only one of them is a
//! shortfall:
//!
//! * `incompleteness` — the host's own reason the index was not whole, e.g.
//!   `"shard 3 of 4 is still rebuilding"` — is reported verbatim under
//!   `"attestations"[stratum]["incomplete"]`, and it makes `"exactness"` name
//!   that stratum under BOTH `"deficit"` and `"inflation"`. Every score in that
//!   answer is then an ESTIMATE rather than a value, and the error runs in both
//!   directions: fusion scores by rank, so a row the short index never named is
//!   summed too LOW, while every row behind it moved up a rank and is summed too
//!   HIGH. This lane REPORTS it rather than refusing, because its answer has a
//!   slot to say it in — the same rule the SPARQL lane follows, decided by what
//!   the return type can carry.
//! * `generation` — the host's own name for the index version that answered — is
//!   reported under `"attestations"[stratum]["generation"]` and is NOT a
//!   shortfall: an answer whose producers named only generations is still exact.
//!   It REPLACES what the shipped text relation would otherwise attest, which is
//!   the content digest of the index this call built, because the kernel pins
//!   exactly one generation per invocation and two distinct ones are its
//!   diagnostic for an index that moved under the query. Declaring one is
//!   therefore a choice to identify the index by the host's own spelling; a
//!   spelling that does NOT move when the host's corpus does makes two answers
//!   from two index states carry one `"evidence_id"`, which is the whole thing
//!   that identity exists to prevent. Declaring an incompleteness alone changes
//!   no generation: an axis left silent delegates to the relation's own.
//!
//! [`plan`] and [`compile`] read the same value, because one producer
//! declaration serves all three entry points, and neither reports it: they
//! execute nothing, so no index has answered yet and there is nothing to attest
//! about. It reaches no plan, no compiled unit and no identity either of them
//! returns — including the registry's content fingerprint, which is a function of
//! what each producer declares to the PLANNER, and an attestation declares
//! nothing there.
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
//!
//! A third refusal in that family never reaches the fusion at all: a `domains`
//! list of **more than one** tag is refused where it is written, before a
//! registry exists, naming the producer that declared it and the three exits
//! (one tag, one producer per block, `domains=None`). It used to arrive at the
//! first row pulled instead, as the fusion's *"restricted its candidates to …
//! but the row at rank R names no block, so nothing backs that restriction"* —
//! a registration-time defect in a query-time failure's clothes.
//!
//! That leaves the engine's three row-level block refusals unreachable from this
//! binding today, which is a property of what this surface can register and not
//! a claim that they are unreal: each is a live refusal for a host writing its
//! own relation on the Rust surface. The *"nothing backs that restriction"* one
//! is the message the registration refusal above forecloses. The other two —
//! *"named item I in block B, which its declared domains … do not include"* and
//! *"name item I from two different blocks"* — need a producer whose rows DO
//! name their own block, and both relations this module builds declare no block
//! column, so nothing registered here can emit such a row. What a Python host
//! can actually meet is the two stratum-level messages above and the
//! registration refusal.
//!
//! Two terminal *statuses* are unreachable here for the same kind of reason, and
//! they are recorded next to those refusals because a reader checking whether a
//! status is testable from Python will look in one place for all of them.
//!
//! `"supplied_query_ended"` — a unit running a query text the host wrote rather
//! than one this layer rendered, whose own internal bound the layer cannot see —
//! needs a caller-assembled bundle. This surface compiles every unit it runs and
//! accepts no bundle from a caller, so no stratum a Python host can configure can
//! end that way. It is mapped here for the same reason the next one is: a host
//! fusing streams from the Rust surface can be handed it.
//!
//! `"row_bound_reached"`
//! — the producer stopping at the row count it declared it can serve per
//! invocation — needs a **self-bounding** producer: one whose declaration places
//! the depth as an argument the producer itself reads, so the read cannot reach
//! for the row past it and how that read ended is not observable. The only
//! relation this module registers is the text-search one, whose ranked
//! declaration places no depth argument, so every stratum a Python host can
//! configure is bounded by the unit's own emitted `LIMIT` and ends
//! `"exhausted"`, `"depth_reached"` or `"ceiling_reached"` instead. The status is
//! documented on [`search`] and mapped here because a host fusing streams from
//! the Rust surface can be handed it, and reading it as "that was all of it"
//! would be the exact mistake the seven spellings exist to prevent. Making it
//! reachable from Python means letting a caller register a producer of its own,
//! which this surface does not do.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pyo3::create_exception;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString};

use crate::attestation::Attestation;
use crate::retrieval::{
    AdmissionEnvironment, ClassWidth, CompiledRetrieval, DecayRule, DepthCause, Fixed,
    FusionProfile, Iri, Metric, Plan, PlanError, PlanId, PlannedResolution, ProducerDecision,
    ProducerStatus, RejectionReason, RequestTerm, RetrievalRequest, ScoreExactness, ScoreInterval,
    SearchResult, Statistics, Term, ToleratedDepth, TopK, UnservedReason,
};
use crate::text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};
use crate::{NativeRdfFormat, RdfDataset, TermValue, parse_dataset};
use purrdf_sparql_eval::{
    CandidateDomains, Completeness, DomainTag, IndexGeneration, OrderFidelity,
    PropertyFunctionRegistry, RankFidelity, ServiceLevel,
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
    /// What the host declares about the index this producer's rows came from:
    /// which version of it answered, and whether it was **not** whole.
    ///
    /// [`Attestation::UNDECLARED`] — silence on both axes — is what a spec that
    /// wrote no attestation position declares, and it leaves the registration
    /// byte-for-byte what it was before that position existed. See this module's
    /// header for what each axis does to the answer.
    attestation: Attestation,
    /// What this producer's own search promises about the rows it can name, on
    /// both axes, as the host declared them.
    ///
    /// Host-supplied for the reason `domains` is, and the reason is the same
    /// shape. BM25 over the index this relation holds is exhaustive and ranks by
    /// exact scores: every document carrying a query term is scored, with no
    /// pruning and no early exit, and nothing is compared in an approximated
    /// space. Over the document this call was handed, both axes are therefore
    /// facts rather than claims.
    ///
    /// What the relation cannot see is whether that document is itself the whole
    /// of what the host means. A host that handed in a sample, one partition of a
    /// larger collection, or a snapshot it knows has fallen behind has a
    /// genuinely lossy producer; a host whose text was transliterated, truncated
    /// or machine-translated before it got here has a genuinely order-perturbed
    /// one, because the values being compared are approximations of the ones the
    /// ranking is meant to be over. Neither is visible from inside, and this is
    /// the only place either can be said.
    ///
    /// Distinct from `attestation`, which is about the INDEX behind the rows: an
    /// attestation says which version answered and whether that version was
    /// whole, while this says whether the producer's own search over it names
    /// every row it should and ranks them as they were due. A host can be silent
    /// on one and explicit on the other.
    fidelity: RankFidelity,
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

// ── pure-Rust cores (PyO3-free, exercised through the pytest suite) ──────────

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
///
/// A list of MORE THAN ONE tag is refused here too, and for a reason that is
/// specific to this binding rather than to the declaration itself. A
/// several-block restriction says only that the producer's candidates lie
/// *somewhere* in that set, so a consumer that wants to hold it to a row has to
/// be told which block that row came from — the per-row fact
/// `RankedDeclaration::block_position` points at. The ranked relations this
/// module can build project a candidate and a score and nothing else, and both
/// declare no block column, because a domain tag describes how a host's corpora
/// partition and only the host knows that. So a several-block list from Python
/// is a restriction that no row this binding can produce is able to back, and a
/// fusion holding the stream to it refuses the very first row it pulls. That is
/// a registration-time defect wearing a query-time failure's clothes, so it is
/// refused where the caller wrote it, with the exits that do work: one tag (the
/// block whose rows this producer really ranks), one producer per block, or
/// `None`.
///
/// A one-tag list needs no per-row fact and is fully supported: the block is
/// *entailed* by the declaration, and the executor reads it straight off the
/// declaration for every row. The count that decides between the two is taken
/// after the tags become a set, so a list repeating one tag is the one-block
/// declaration it means; and the blocks the refusal quotes are quoted in the
/// set's own canonical order, so two hosts that wrote the same blocks in
/// different orders read the same message about the same declaration.
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
    // Counted after the set absorbs them, so a list that spells one block twice
    // is the satisfiable one-block declaration it means rather than a refusal
    // over an arity the declaration does not actually have.
    if blocks.len() > 1 {
        let listed = blocks
            .iter()
            .map(|tag| format!("<{}>", tag.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "text producer <{producer}>: `domains` names {count} blocks [{listed}], and a \
             several-block declaration only says this producer's candidates lie somewhere in that \
             set — it obliges the producer to say, row by row, which of those blocks each row came \
             from. The ranked relations this surface builds project a candidate and a score and \
             declare no block column, because a domain tag describes how a host's corpora \
             partition and only the host knows that. So this is a restriction no row this producer \
             can emit is able to back, and a fusion holding it to the declaration refuses its \
             first row. Three exits work: pass exactly ONE domain tag, naming the block this \
             producer's rows really lie in — a single tag entails the per-row fact and needs no \
             block column; or register one producer per block, each with its own single tag and \
             its own stratum; or pass `domains=None`, which restricts nothing and costs only the \
             earlier certification a narrower claim would have bought",
            count = blocks.len()
        ));
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
        let fidelity = producer.fidelity.clone();
        // Read off the relation itself, BEFORE the host's attestation wraps it:
        // what a producer declares to the planner is what the relation can
        // honestly declare, and an attestation says nothing about arity, modes or
        // ranked order. The wrapper delegates every one of those, so the
        // declaration would be identical either way; taking it from the relation
        // is what makes that true by construction rather than by inspection.
        let declaration = relation
            .ranked_declaration(stratum, Some(producer.predicate.clone()), fidelity, domains)
            .map_err(|e| format!("text producer <{}>: {e}", producer.producer))?;
        registry.register_ranked(
            &producer.producer,
            producer.attestation.clone().wrap(Arc::new(relation)),
            declaration,
        );
    }
    Ok(registry)
}

/// The fidelity a `(completeness, order)` position declares, or the top of the
/// lattice where it said nothing.
///
/// Two independent members, each a `str` or `None`, shaped exactly like the
/// attestation position beside it and read on exactly the same terms: a member
/// that is `None` is SILENCE on that axis, and a member that is a string is that
/// axis declared degraded, with the host's own words carried **verbatim**.
/// Nothing here parses either string. There is no tag to spell, no prefix to
/// strip and no whitespace to lose, because the position of the member is what
/// says which axis it is about.
///
/// The axes fail independently and a host may know about one and not the other:
/// * `completeness` — the search does not name every row that was due. A host
///   that handed in a sample, one partition, or a snapshot that has fallen
///   behind.
/// * `order` — a row it does name can arrive at a rank BETTER than it earned,
///   which is what breaks every score bound. A host whose text was
///   transliterated, truncated or machine-translated before it arrived is
///   ranking over approximations of the values the ranking is meant to be over.
///
/// Silence on both is [`RankFidelity::EXACT`], and that is a fact rather than a
/// fabricated default: this binding builds the index in this very call, out of
/// the document it was handed, and BM25 over it scores every document carrying a
/// query term with no pruning and compares nothing in an approximated space. The
/// relation is exhaustive and order-faithful **over what it was given**. What it
/// cannot see — whether what it was given is the whole of what the host means —
/// is precisely what a member says, and the host is the only party who knows it.
///
/// # Why an empty member is refused, and refused here rather than below
///
/// A declared degradation with nothing behind it reports a degraded stratum
/// while saying nothing a reader can act on, and it is indistinguishable in a
/// rendered answer from a producer that declared none. `register_ranked` refuses
/// it by **panicking**, which must never cross the FFI boundary, so the refusal
/// is made here as an ordinary Python error — the same reason an empty domain
/// list is refused here.
///
/// The neighbouring valid cases are deliberately close: `None` on a member
/// registers, and so does any member with real prose in it, whatever it spells.
fn read_fidelity(subject: &str, value: &Bound<'_, PyAny>) -> PyResult<Option<RankFidelity>> {
    // A bare string is not destructured into its own characters. `"ab"` extracts
    // as a well-formed two-member sequence, so accepting it would report `"a"`
    // back to an operator as the completeness evidence they never wrote.
    if value.is_instance_of::<PyString>() || value.is_instance_of::<PyBytes>() {
        return Ok(None);
    }
    let Ok(members) = value.extract::<Vec<Bound<'_, PyAny>>>() else {
        return Ok(None);
    };
    let Ok([completeness, order]) = <[Bound<'_, PyAny>; 2]>::try_from(members) else {
        return Ok(None);
    };
    let read = |member: &Bound<'_, PyAny>, axis: &str| -> PyResult<Option<String>> {
        let declared = member.extract::<Option<String>>().map_err(|_| {
            PyTypeError::new_err(format!(
                "{subject}: a fidelity's `{axis}` must be a str or None"
            ))
        })?;
        if declared.as_deref().is_some_and(|d| d.trim().is_empty()) {
            return Err(PyValueError::new_err(format!(
                "{subject}: `fidelity` declares a degraded `{axis}` but supplies no evidence \
                 for it. A consumer carries this string into its answer verbatim, so an empty \
                 one reports a degraded stratum while saying nothing a reader can act on. \
                 State what the producer does not promise, or write None for this axis"
            )));
        }
        Ok(declared)
    };
    Ok(Some(RankFidelity {
        // Verbatim on both halves: `Arc::from` the string as the host wrote it,
        // with no trim. A disclosure that is indented, multi-line, or ends in a
        // newline reaches the consumer as those bytes, because the test that
        // proves the Rust leg survives a `\u{1}`, a `;` and a `\n` is a claim
        // about this surface too.
        completeness: read(&completeness, "completeness")?.map_or(
            Completeness::Complete,
            |evidence| Completeness::Lossy {
                evidence: Arc::from(evidence),
            },
        ),
        order: read(&order, "order")?.map_or(OrderFidelity::Faithful, |evidence| {
            OrderFidelity::Perturbed {
                evidence: Arc::from(evidence),
            }
        }),
    }))
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
fn run_search(call: &Call, profile: &FusionProfile) -> Result<SearchResult, String> {
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
fn collect_request(request: &Bound<'_, PyAny>, top_k: usize) -> PyResult<RetrievalRequest> {
    let items = request
        .try_iter()
        .map_err(|_| PyTypeError::new_err("`request` must be a sequence of request-term tuples"))?;
    let mut terms = Vec::new();
    for (index, item) in items.enumerate() {
        terms.push(request_term(index, &item?)?);
    }
    // The bound is part of the request, not an argument of the last stage: the
    // planner derives every stratum's depth from it, so a request that asks for a
    // different number of rows is a different plan with different depths and a
    // different identity. Every entry point on this surface therefore takes it,
    // including the two that execute nothing.
    Ok(RetrievalRequest::bounded(terms, TopK::new(top_k)))
}

/// Collect the `text_producers` dict into the ordered declarations one call
/// registers: `producer_iri -> (stratum_iri, predicate_iri, graph)`, or
/// `producer_iri -> (stratum_iri, predicate_iri, graph, domains)`, or the same
/// four followed by one `(generation, incompleteness)` attestation.
///
/// The fourth element is the producer's candidate-domain declaration: `None`
/// for the unrestricted promise, or a list of domain-tag IRIs. Omitting it
/// entirely is the same declaration as `None` — the widest promise, which
/// licenses a consumer to skip nothing — so a host that never heard of domains
/// keeps exactly the reading it had.
///
/// # The attestation is the FIFTH position, and that is not an accident
///
/// A `domains` value and an attestation are both sequences, so on a four-element
/// value the two are genuinely ambiguous: `("a", "b")` is a well-formed
/// two-tag restriction AND a well-formed attestation, and nothing in either value
/// says which the host meant. Guessing between them is the silent-wrong reading
/// this whole surface exists to refuse — one guess registers a producer whose
/// rows cannot back a restriction it never made, the other reports a domain tag
/// back to an operator as an index generation. So the position is fixed: a spec
/// that attests writes its `domains` position explicitly, and `None` there
/// restricts nothing. The shape refusal spells all three accepted widths.
///
/// # Errors
///
/// `TypeError` naming the accepted shapes when the value is not a sequence of
/// three, four, five or six positions, or when the fifth is not a two-member
/// sequence;
/// `TypeError` naming the field when an attestation member is neither `str` nor
/// `None`, or when a mandatory position is not a string; `ValueError` naming both
/// producers and the stratum when two entries claim one stratum.
fn collect_producers(producers: &Bound<'_, PyDict>) -> PyResult<Vec<TextProducer>> {
    let mut declared = Vec::with_capacity(producers.len());
    for (key, value) in producers {
        let producer: String = key
            .extract()
            .map_err(|_| PyTypeError::new_err("text producer keys must be IRI strings"))?;
        let shape = || {
            PyTypeError::new_err(format!(
                "text producer <{producer}>: the value is (stratum, predicate, graph), \
                 (stratum, predicate, graph, domains), (stratum, predicate, graph, domains, \
                 (generation, incompleteness)), or (stratum, predicate, graph, domains, \
                 (generation, incompleteness), (completeness, order)) — an attestation is the \
                 fifth position, because a fourth-position sequence is already a `domains` list \
                 and guessing between the two would report one back as the other; a fidelity is \
                 the sixth, because it speaks about the producer's search rather than about the \
                 index the attestation names"
            ))
        };
        let mut fields: Vec<Bound<'_, PyAny>> = value.extract().map_err(|_| shape())?;
        // Read off the tail first, deepest position first: the three mandatory
        // fields are destructured as an array, which consumes the vector, so every
        // optional position has to leave before that happens.
        let fidelity = match fields.len() {
            3..=5 => None,
            6 => Some(fields.remove(5)),
            _ => return Err(shape()),
        };
        let attestation = match fields.len() {
            3 | 4 => Attestation::UNDECLARED,
            5 => {
                let trailing = fields.remove(4);
                // A fifth position that is not even SHAPED like an attestation
                // reports the accepted widths rather than a diagnostic about a
                // position the caller may never have meant to write; one that is
                // shaped like an attestation but carries the wrong member types
                // keeps its own precise diagnostic, which `Attestation::read`
                // raises.
                Attestation::read(&format!("text producer <{producer}>"), &trailing)?
                    .ok_or_else(shape)?
            }
            _ => return Err(shape()),
        };
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
                     or None for the unrestricted declaration (this producer may name anything). \
                     An attestation is the FIFTH position, written after a `domains` position of \
                     its own"
                ))
            })?),
            _ => None,
        };
        let fidelity = match fidelity {
            Some(value) if !value.is_none() => {
                // A sixth position that is not even SHAPED like a fidelity
                // reports the accepted widths rather than a diagnostic about a
                // position the caller may never have meant to write; one that is
                // shaped like a fidelity but carries the wrong member types keeps
                // its own precise diagnostic, which `read_fidelity` raises.
                read_fidelity(&format!("text producer <{producer}>"), &value)?.ok_or_else(shape)?
            }
            _ => RankFidelity::EXACT,
        };
        let graph =
            GraphSpec::parse(&producer, &field("graph", &graph)?).map_err(PyValueError::new_err)?;
        declared.push(TextProducer {
            stratum: field("stratum", &stratum)?,
            predicate: field("predicate", &predicate)?,
            graph,
            domains,
            attestation,
            fidelity,
            producer,
        });
    }
    // The dict's iteration order is the host's insertion order, but the registry
    // it builds is a set: sorting makes the registration order a pure function
    // of the declarations, so the registry's content fingerprint — which the
    // plan records — cannot depend on how the dict was written.
    declared.sort_by(|left, right| left.producer.cmp(&right.producer));
    // One stratum carries one producer, and the registry enforces that with a
    // panic — the right shape for a Rust caller assembling a registry in code, and
    // the wrong one here: a panic crosses the boundary as `PanicException`, which
    // derives from `BaseException` and slips past a host's `except Exception`.
    // Every other misconfiguration on this surface raises `ValueError` by name, so
    // this one does too, before the registry is touched. Scanned after the sort,
    // so the two names reported are a function of the declarations and not of the
    // order the host wrote the dict in.
    for (index, later) in declared.iter().enumerate().skip(1) {
        if let Some(earlier) = declared[..index]
            .iter()
            .find(|earlier| earlier.stratum == later.stratum)
        {
            return Err(PyValueError::new_err(format!(
                "text producers <{}> and <{}> both claim stratum <{}>: one stratum carries one \
                 producer, because a rank is meaningful only inside the list that assigned it \
                 and two lists concatenated rank the second producer's best row below every row \
                 of the first. Shards or segments whose scores are already comparable belong \
                 inside ONE producer that merges them by score; producers that score by \
                 different laws belong in two strata, where the weighted sum across strata is \
                 the point of the fusion",
                earlier.producer, later.producer, later.stratum
            )));
        }
    }
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
///
/// `top_k` is one of those shared arguments rather than `search`'s alone, because
/// the row bound is a planning input: it decides how deep each stratum is read and
/// therefore which plan a request is. A `plan` or `compile` call that did not carry
/// it would report a depth, a `LIMIT` and an identity for a read nobody asked
/// for.
fn collect_call(
    data: &str,
    request: &Bound<'_, PyAny>,
    text_producers: &Bound<'_, PyDict>,
    statistics: &Bound<'_, PyDict>,
    top_k: usize,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Call> {
    let request = collect_request(request, top_k)?;
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
        UnservedReason::AcceptedWithoutPlacement => "accepted_without_placement",
        UnservedReason::Unbound => "unbound",
    }
}

/// The stable lowercase name of a depth's binding cause.
///
/// Spelled here rather than derived from `Debug`, so the Python surface's
/// vocabulary is a decision this file makes and not a rename away from changing.
/// Exhaustive, like the two renderings above it: a cause this match does not
/// name is a compile error rather than a word this function invented.
const fn depth_cause_name(cause: DepthCause) -> &'static str {
    match cause {
        DepthCause::Declaration => "declaration",
        DepthCause::Cardinality => "cardinality",
        DepthCause::Selectivity => "selectivity",
        DepthCause::LicensedPrefix => "licensed_prefix",
        DepthCause::Floor => "floor",
        DepthCause::Unbounded => "unbounded",
        DepthCause::ReadCeiling => "read_ceiling",
    }
}

// ── the plan-document boundary ───────────────────────────────────────────────

create_exception!(
    retrieval,
    PlanDocumentError,
    PyValueError,
    "A refusal from the plan-document boundary — `retrieval.certify_plan` and \
     `retrieval.explain_depth`, the two entry points that read a plan document this \
     process did not produce.\n\
     \n\
     Carries a `.refusal` attribute: one of the engine's pinned kebab-case names for \
     the refusal. The decode-side names are `version` (the document was written under \
     a plan layout this build does not write), `truncated`, `trailing-bytes`, \
     `invalid-tag`, `invalid-utf8`, `invalid-iri`, `non-ascending-keys` (a keyed \
     section did not arrive strictly ascending, so the bytes are not an encoding of \
     any plan), `non-ascending-selectivity-terms` (a record's run of contributing \
     request-term indices did not ascend either, which is the same fact one nesting \
     level in), `duplicate-stratum-depth`, `duplicate-stratum-derivation`, \
     `duplicate-statistics-subject` (one subject answered twice, with nothing saying \
     which answer the plan was planned against) and `duplicate-selectivity-term` (one \
     term counted twice into a sum the arithmetic reached once). The \
     certificate-side names are `depth-not-derivable` (a recorded depth is not the \
     depth its own recorded inputs derive), `depth-without-derivation`, \
     `derivation-without-depth`, `derivation-without-statistics-entry` (a depth was \
     derived for a stratum the snapshot names nowhere), \
     `statistics-entry-contradicts-derivation` (the plan's two records of one \
     stratum's statistics disagree), `selectivity-term-out-of-range` (a recorded \
     selectivity domain indexes a term the plan's own request does not carry) and \
     `unconsulted-statistics-subject` (the snapshot names a subject that is neither a \
     stratum nor a predicate of any of the plan's request terms, so it is evidence \
     about a consultation that did not happen).\n\
     \n\
     Branch on `.refusal`, never on `str(exc)`: the name is the pinned contract and \
     the message is prose that may be reworded. A `version` refusal means the \
     document came from another build and cannot be reinterpreted under this one; \
     every other name means the document in hand says something it cannot also \
     mean, and no repair is offered because a plan whose depth and evidence \
     disagree has no reading under which one of them is the truth.\n\
     \n\
     Subclasses `ValueError`, so code that already catches this module's \
     `ValueError` keeps working."
);

/// Raise a plan-document refusal as [`PlanDocumentError`], with `.refusal`
/// always present.
///
/// The name comes from the engine's own `PlanError::refusal`, never from a match
/// written here: `PlanError` is `#[non_exhaustive]`, so a match in this crate
/// would need a wildcard arm, and a wildcard arm over a refusal's *name* has
/// nothing honest to put there — it would hand a caller an invented word at the
/// one moment the caller is asking which refusal it got.
fn plan_document_error(py: Python<'_>, refusal: &PlanError) -> PyErr {
    let error = PlanDocumentError::new_err(refusal.to_string());
    // A failure to set the attribute would mean the exception object refused an
    // ordinary `setattr`, which cannot happen for a Python-level exception class;
    // it is ignored rather than replacing a precise refusal with a vaguer one.
    let _ = error.value(py).setattr("refusal", refusal.refusal());
    error
}

/// Decode a plan document, refusing it by name.
fn decode_plan(py: Python<'_>, document: &[u8]) -> PyResult<Plan> {
    Plan::from_canonical_bytes(document).map_err(|refusal| plan_document_error(py, &refusal))
}

/// Render one plan as a dict.
fn plan_dict<'py>(py: Python<'py>, planned: &Plan) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    // Encoded once and read twice: the identity is the digest of exactly these
    // bytes, so deriving the hex from the same buffer the caller is handed makes
    // "this document's id" a property of the code rather than of two calls that
    // happen to agree.
    let canonical = planned.canonical_bytes();
    out.set_item("plan_id", PlanId::from_canonical(&canonical).to_hex())?;
    out.set_item("version", planned.version)?;
    // The plan's canonical, length-framed encoding: what a host stores, sends, or
    // hands back to `retrieval.certify_plan`. It is the plan's identity in the
    // literal sense — `"plan_id"` is its digest — so a plan can leave this
    // process and be checked on the way back in.
    out.set_item("canonical_bytes", PyBytes::new(py, &canonical))?;

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
        // An absent cardinality renders as `None`, never as `0`. Zero is a
        // measurement and absence is not, and collapsing them at the binding
        // boundary would undo the distinction the record is built on.
        rendered.set_item("cardinality", entry.cardinality)?;
        rendered.set_item("selectivity_ppm", entry.selectivity_ppm)?;
        rendered.set_item("selectivity_terms", entry.selectivity_terms.clone())?;
        entries.append(rendered)?;
    }
    snapshot.set_item("entries", entries)?;
    out.set_item("statistics", snapshot)?;

    // What each recorded depth was derived from, so a host can recompute the
    // number rather than take the plan's word for it.
    let derivations = PyDict::new(py);
    for (stratum, inputs) in &planned.stratum_derivations {
        let rendered = PyDict::new(py);
        rendered.set_item("declared", inputs.declared)?;
        rendered.set_item("cardinality", inputs.cardinality)?;
        rendered.set_item("selectivity_ppm", inputs.selectivity_ppm)?;
        rendered.set_item("selectivity_terms", inputs.selectivity_terms.clone())?;
        rendered.set_item("licensed_prefix", inputs.licensed_prefix)?;
        // Asked of the PLAN, through the method the engine offers for exactly
        // this question, rather than computed beside it from the inputs this
        // loop happens to be holding. The two spellings would answer the same
        // question today and would be one edit away from not doing so, and the
        // one a caller of the Rust surface reads is the method — so a divergence
        // would show up first as the binding quietly disagreeing with
        // `Plan::explain_depth` about a plan they both hold.
        let Some(cause) = planned.explain_depth(stratum) else {
            // Unreachable: `stratum` is a key of `stratum_derivations`, which is
            // the map `explain_depth` reads, so it answers `None` only for a
            // stratum this loop is not iterating. Reported rather than unwrapped
            // because a panic here would cross the FFI boundary, and reported as
            // a disagreement inside this build rather than as anything the
            // caller did.
            return Err(PyValueError::new_err(format!(
                "plan records a depth derivation for stratum {} that it then \
                 reports no depth cause for; this build disagrees with itself",
                stratum.as_str()
            )));
        };
        rendered.set_item("cause", depth_cause_name(cause))?;
        derivations.set_item(stratum.as_str(), rendered)?;
    }
    out.set_item("stratum_derivations", derivations)?;

    // Nothing is certified here. This renders a plan and adds no semantics to
    // it, and certifying would add one: `plan` and `compile` both reach this
    // function, so every Python `compile` would be able to raise a refusal
    // `purrdf_retrieval::plan` never raises — a surface that answers a
    // different question from the engine underneath it.
    //
    // It would also be the wrong place to pay for the answer. Certifying is a
    // per-stratum re-derivation of numbers planning computed a moment earlier,
    // and it is documented as the COLD path for exactly that reason; admission
    // deliberately does not call it either. The check is not absent, it is
    // `retrieval.certify_plan`, asked once by whoever received a document
    // rather than on every call by whoever produced one.

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
///
/// Each unit carries its `"depth"` beside its `"sparql"`, because the emitted
/// text is written exactly one row deeper than the plan reads: that last row is a
/// probe the executor reads to
/// tell an exhausted producer from a depth-cut one, and it is never a value. A
/// host that runs the text itself has no other honest source for the number of
/// rows it may keep — the depth is not recoverable from the text, and
/// `planned_resolution` answers only when the call named a fusion law — so the
/// bound travels with the text it bounds.
///
/// `"declared_rows"` travels beside it because the depth alone does not say which
/// of two situations a host is in. A depth *below* the declaration leaves rows
/// underneath the read; a depth *on* it means the producer has promised there is
/// nothing further, and the probe row is what checks that promise. Those are
/// different facts about the same run, and the difference is not recoverable from
/// the depth, the text or the plan — only from the number the registry declared.
/// It is `None` for a producer that declared no access mode and therefore no row
/// count at all, because "declared nothing" and "declared zero" are different
/// facts here too: an absent declaration can refuse nothing, while a zero is a
/// measurement of the producer's data.
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
        // Rendered rather than read off a field: the unit carries the query body
        // and appends its own bound, so the text a host runs cannot disagree with
        // the depth beside it. The value is byte-identical to what the field held.
        entry.set_item("sparql", unit.sparql())?;
        // The unit's own reportable bound, read off the field that carries it
        // rather than re-derived from the text or looked up again in the plan:
        // the text's `LIMIT` is the emitted bound, which includes the probe.
        entry.set_item("depth", unit.depth())?;
        // The declaration the depth above was checked against, projected and never
        // defaulted: `None` stays `None` all the way out to the host, because a
        // producer that declared no access mode declared no row count, and a zero
        // put there in its place would be a measurement nobody took.
        entry.set_item("declared_rows", unit.declared_rows())?;
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

/// Render one stratum's declared fidelity as a dict.
///
/// A **tagged** shape on each axis: the key `"completeness"` is always present
/// and carries `"complete"` or `"lossy"`, and `"evidence"` appears only with the
/// degraded tag. The alternative -- a `{"lossy": bool, "evidence": str | None}`
/// product -- makes `{"lossy": True, "evidence": None}` a perfectly well-formed
/// dict, which is exactly the "declared a loss but disclosed nothing" state the
/// Rust side refuses at registration. Re-introducing it here would put the
/// ambiguity back at the binding, one layer below where it was removed.
///
/// There is no `None` anywhere in the result, and no absent key means "exact":
/// silence is what this whole surface exists to stop a consumer having to
/// interpret.
fn fidelity_dict<'py>(py: Python<'py>, fidelity: &RankFidelity) -> PyResult<Bound<'py, PyDict>> {
    let entry = PyDict::new(py);
    match &fidelity.completeness {
        Completeness::Complete => entry.set_item("completeness", "complete")?,
        Completeness::Lossy { evidence } => {
            entry.set_item("completeness", "lossy")?;
            entry.set_item("completeness_evidence", &**evidence)?;
        }
    }
    match &fidelity.order {
        OrderFidelity::Faithful => entry.set_item("order", "faithful")?,
        OrderFidelity::Perturbed { evidence } => {
            entry.set_item("order", "perturbed")?;
            entry.set_item("order_evidence", &**evidence)?;
        }
    }
    Ok(entry)
}

/// Render one row's score interval as a dict.
///
/// Tagged for the reason above. `"bound"` is `True` with `"deficit"` and
/// `"inflation"` beside it, or `False` with `"perturbed"` naming the strata for
/// which no finite bound exists. The two shapes carry different keys rather than
/// one shape with nullable numbers, because a `None` deficit and a zero deficit
/// mean opposite things -- "no bound could be computed" and "nothing was
/// withheld" -- and a caller reading a nullable field will eventually conflate
/// them.
fn interval_dict<'py>(py: Python<'py>, interval: &ScoreInterval) -> PyResult<Bound<'py, PyDict>> {
    let entry = PyDict::new(py);
    match interval {
        ScoreInterval::Bounded { deficit, inflation } => {
            entry.set_item("bounded", true)?;
            entry.set_item("deficit", deficit.to_decimal_lexical())?;
            entry.set_item("inflation", inflation.to_decimal_lexical())?;
        }
        ScoreInterval::Unbounded { perturbed } => {
            entry.set_item("bounded", false)?;
            entry.set_item(
                "perturbed",
                perturbed.iter().map(Iri::as_str).collect::<Vec<_>>(),
            )?;
        }
    }
    Ok(entry)
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
        // How far a degraded stratum could have moved THIS row, in both
        // directions. Zero-width on both terms when every stratum was
        // exhaustive, which is every answer this surface produced before the
        // term existed.
        entry.set_item("interval", interval_dict(py, &row.interval)?)?;
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
            // The producer read to the row count it registered and could not be
            // asked for the row past it, so how the read ended was not observable.
            // It carries a rank like `"depth_reached"` and claims nothing about what
            // lies below it, which is the whole difference between the two.
            ProducerStatus::RowBoundReached { rank } => {
                entry.set_item("status", "row_bound_reached")?;
                entry.set_item("rank", rank)?;
            }
            // A unit running a query text the host supplied rather than one this
            // layer rendered. The layer bounds only the outside of such a text, so
            // what that text bounds inside itself — and therefore what it left
            // unread — was not observable. It carries a rank like
            // `"depth_reached"` and, like `"row_bound_reached"`, claims nothing
            // about what lies below it.
            ProducerStatus::SuppliedQueryEnded { rank } => {
                entry.set_item("status", "supplied_query_ended")?;
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
                IndexGeneration::Declared(generation) => Some(&**generation),
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

    // Whether the scores are exact, read off what the producers declared and
    // what their indexes attested. `False` does not make the answer wrong: every
    // row in it is a real row in this fusion's own certified order. What it means
    // is that a score is an ESTIMATE rather than a value, and the error runs in
    // BOTH directions -- which is why there is no "lower_bounds_for" key here.
    //
    // Fusion scores by RANK and nothing else, so a stratum that fails to name a
    // row does not merely withhold that row's contribution: every row behind the
    // missing one moves up a rank and collects a larger contribution than it
    // earned. A candidate the degraded stratum missed is summed too LOW; one it
    // named is summed too HIGH. A consumer handed a one-sided name would be
    // confidently wrong in the direction the name told it not to look.
    //
    // "deficit" and "inflation" name the strata responsible on each side, in
    // canonical order. "unbounded" names strata whose declared ORDER is
    // perturbed, for which no finite bound exists at all -- empty for every
    // producer this workspace ships, because an HNSW graph compares exact
    // distances and fails only to visit. Each stratum's verbatim reason is under
    // the same key in "fidelities" or "attestations", so the lists are what to
    // act on rather than flags to shrug at.
    let exactness = PyDict::new(py);
    fn strata_list(strata: &BTreeSet<Iri>) -> Vec<&str> {
        strata.iter().map(Iri::as_str).collect()
    }
    match &result.trailer.exactness {
        ScoreExactness::Exact => {
            exactness.set_item("exact", true)?;
            exactness.set_item("deficit", Vec::<&str>::new())?;
            exactness.set_item("inflation", Vec::<&str>::new())?;
            exactness.set_item("unbounded", Vec::<&str>::new())?;
        }
        ScoreExactness::Estimated {
            deficit,
            inflation,
            unbounded,
        } => {
            exactness.set_item("exact", false)?;
            exactness.set_item("deficit", strata_list(deficit))?;
            exactness.set_item("inflation", strata_list(inflation))?;
            exactness.set_item("unbounded", strata_list(unbounded))?;
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

    // What each handed stream declared about the rows it can name, keyed like
    // "attestations" and "domains" beside it. This is where a consumer learns a
    // stratum was served approximately, and where that producer's own words
    // about the loss are read -- verbatim, never parsed or re-worded here.
    //
    // It is read WITH "statuses", never instead of it. A status says how the
    // read ENDED; a fidelity says whether the rows that ended it were all the
    // rows that were DUE. "exhausted" beside a "lossy" declaration is neither a
    // contradiction nor a completeness claim: the producer emitted every row its
    // search produced, and the declaration says the search does not produce
    // every row there was.
    let fidelities = PyDict::new(py);
    for (stratum, fidelity) in &result.trailer.fidelities {
        fidelities.set_item(stratum.as_str(), fidelity_dict(py, fidelity)?)?;
    }
    out.set_item("fidelities", fidelities)?;

    // How many leading rows keep their places whatever the degraded strata did
    // or did not find. This is the answer a consumer with a completeness
    // obligation actually has: told only that a stratum was approximate, its
    // one safe move is to downgrade the whole answer, and this lets it present
    // the certain part as settled and mark the rest.
    //
    // It claims membership and never absence: a row PAST the prefix is
    // possible rather than excluded.
    out.set_item(
        "certain_prefix",
        result.trailer.certain_prefix(&result.rows),
    )?;

    // The evidence the verdict above rests on, so a caller can audit it rather
    // than take it: the most any candidate outside the answer could be worth.
    // A leading row is certain exactly when its own floor clears this, which is
    // what lets the prefix speak about candidates no producer ever named -- a
    // lossy stratum's whole failure mode is not naming things, so a bound that
    // covered only the rows in hand would be a claim about the ranking rather
    // than about the answer.
    //
    // `None` where a stratum declared a PERTURBED order: that breaks the one
    // inequality every bound here rests on, so no finite ceiling exists and
    // reporting a number would be the fabrication this channel exists to
    // prevent. `certain_prefix` is then zero, for the same reason.
    out.set_item(
        "unemitted_ceiling",
        result
            .trailer
            .unemitted_ceiling
            .map(Fixed::to_decimal_lexical),
    )?;

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
///
/// `top_k` is required here even though nothing runs. The bound decides how deep
/// each stratum is read, so it decides what `"stratum_depths"` says and what
/// `"plan_id"` is: a top-five request and a top-five-hundred request are two
/// plans, not one plan read twice. Whether it actually narrows a depth is decided
/// by the producers' own `domains` — over strata whose declared blocks do not
/// overlap, each is planned to `top_k` rows and no deeper; over anything else the
/// declared-or-measured bound stands. It never widens a depth and never changes an
/// answer.
#[pyfunction]
#[pyo3(signature = (
    data,
    request,
    *,
    text_producers,
    statistics,
    top_k,
    data_format="turtle",
    base=None,
))]
#[allow(
    clippy::too_many_arguments,
    reason = "the pure stage's inputs are named, not bundled"
)]
fn plan<'py>(
    py: Python<'py>,
    data: &str,
    request: &Bound<'py, PyAny>,
    text_producers: &Bound<'py, PyDict>,
    statistics: &Bound<'py, PyDict>,
    top_k: usize,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(
        data,
        request,
        text_producers,
        statistics,
        top_k,
        data_format,
        base,
    )?;
    // Parsing, index construction and planning run detached (GIL released); the
    // result dict is built after the GIL is reacquired.
    let planned = py
        .detach(|| run_plan(&call).map(|(_, _, planned)| planned))
        .map_err(PyValueError::new_err)?;
    plan_dict(py, &planned)
}

/// Decode a plan document and check that every depth it records follows from the
/// inputs it records beside them.
///
/// This is the other half of `plan`'s `"canonical_bytes"`: a plan can leave this
/// process — stored, logged, sent to another host — and the check runs on the way
/// back in. `plan_bytes` is exactly what `"canonical_bytes"` handed out, and the
/// answer is the decoded plan rendered as `plan` renders it, so a host reads the
/// document it received rather than the one it believes it sent.
///
/// A plan is untrusted input: it can be edited and it can be forged, and its
/// depths are the numbers that decide how deep each stratum is actually read. The
/// plan records every input those depths were derived from, so this recomputes
/// each one with the engine's own arithmetic and refuses a plan the two disagree
/// about — the depth is a checkable claim rather than an asserted one.
///
/// It is the COLD path, and deliberately on no hot one. [`plan`], [`compile`]
/// and [`search`] do not run it, and neither does admission: planning just built
/// the plan those stages return, so certifying it there would re-derive, once
/// per stratum and on every call, a number this build had computed a moment
/// earlier. The question this answers — is this document internally coherent at
/// all — is a property of the bytes alone and has nothing to do with the
/// registry or the statistics in force now, which is why it is asked once, here,
/// by the party that received them.
///
/// Every refusal raises `retrieval.PlanDocumentError` carrying a pinned
/// `.refusal` name; branch on that, never on the message. A version this build
/// does not write is `version`; a keyed section out of order is
/// `non-ascending-keys`, and a record's run of contributing request-term indices
/// out of order is `non-ascending-selectivity-terms`; one stratum recorded twice
/// is `duplicate-stratum-depth` or `duplicate-stratum-derivation`, and one term
/// counted twice into one selectivity is `duplicate-selectivity-term`; a depth
/// that does not follow from its inputs is `depth-not-derivable`; a stratum the
/// snapshot does not name is `derivation-without-statistics-entry`, and a
/// snapshot row for a subject nothing consulted is
/// `unconsulted-statistics-subject`; a snapshot row saying something else than
/// the derivation beside it is `statistics-entry-contradicts-derivation`; and a
/// recorded selectivity domain indexing a term the plan's own request does not
/// carry is `selectivity-term-out-of-range`. The class lists them all.
#[pyfunction]
#[pyo3(signature = (plan_bytes))]
fn certify_plan<'py>(py: Python<'py>, plan_bytes: &[u8]) -> PyResult<Bound<'py, PyDict>> {
    // Decoding and certifying run detached (GIL released): both are pure
    // functions of a buffer whose length is the sender's choice, and the
    // rendering is built after the GIL is reacquired.
    let decoded = py.detach(|| {
        let decoded = Plan::from_canonical_bytes(plan_bytes)?;
        decoded.certify()?;
        Ok::<Plan, PlanError>(decoded)
    });
    let planned = decoded.map_err(|refusal| plan_document_error(py, &refusal))?;
    plan_dict(py, &planned)
}

/// Which recorded input bound `stratum`'s depth in the plan document
/// `plan_bytes`, or `None` when that plan records no derivation for it.
///
/// A depth of one is the motivating case. It arrives by four different roads — a
/// declaration of zero or one row, a measured cardinality, a selectivity that
/// scaled the bound to nothing, or the floor that stops any of them reaching zero
/// — and a caller looking at the number alone cannot tell which, though the four
/// have completely different remedies. The answer is one of `"declaration"`,
/// `"cardinality"`, `"selectivity"`, `"licensed_prefix"`, `"floor"`,
/// `"unbounded"` or `"read_ceiling"`: the same closed vocabulary `plan` renders
/// under each derivation's `"cause"`, from the same engine call.
///
/// This reads the derivation the document records and does **not** certify it.
/// The two are different questions — "which leg bound this number" and "does this
/// number follow from those legs" — and answering the first says nothing about
/// the second, which is `certify_plan`'s to answer over the whole plan at once. A
/// host that has not certified a document it received is reading an explanation
/// of a depth that may not follow from it.
///
/// Raises `retrieval.PlanDocumentError` for every way the document itself is
/// refused, with the same pinned `.refusal` names `certify_plan` raises, plus
/// `invalid-iri` when `stratum` is not an IRI.
#[pyfunction]
#[pyo3(signature = (plan_bytes, stratum))]
fn explain_depth(
    py: Python<'_>,
    plan_bytes: &[u8],
    stratum: &str,
) -> PyResult<Option<&'static str>> {
    let planned = decode_plan(py, plan_bytes)?;
    let stratum = Iri::parse(stratum).map_err(|refusal| plan_document_error(py, &refusal))?;
    Ok(planned.explain_depth(&stratum).map(depth_cause_name))
}

/// Plan, admit and emit: the per-stratum SPARQL the request compiles to.
///
/// Returns the plan document under `"plan"` and, under `"units"`, one entry per
/// stratum carrying the `"sparql"` text that stratum runs and the `"depth"` that
/// text is keyed to. The admitted `"plan_id"` and the
/// `"registry_fingerprint"` the units were compiled against are alongside, so a
/// host that logs a unit can say exactly which plan and which registry it came
/// from. A host can read, log or execute those units itself; [`search`] is what
/// runs them and fuses their rows.
///
/// `"depth"` is the reportable bound and it is **not** the `LIMIT` in
/// `"sparql"`: the text is emitted exactly one row deeper, and that extra row is
/// a probe that exists only so a reader can tell a producer that ran out from a
/// read the depth cut. A host that runs the text itself keeps at most `"depth"`
/// rows and reports none of what came after. "Exactly one row deeper" is exact and
/// unconditional: the declared row bound does not cap it, including a declared bound
/// of zero, because a bound equal to its own depth admits no row for the probe to
/// arrive in and every such read would be reported as an exhaustion.
///
/// `"declared_rows"` is that declared bound, on the unit beside the depth it was
/// checked against, and `None` for a producer that declared no access mode and so
/// declared no row count at all. It is the one number that distinguishes a depth
/// with rows still under it from a depth sitting *on* the producer's own promise
/// that there are none — the case the probe row exists to check — and a host
/// reading `"depth"` to know how many rows it may report is entitled to know which
/// of the two it has.
///
/// One relation shape is bounded by something the text does not carry: one that
/// takes the depth as an argument bounds itself by the number it was handed, which
/// is never raised past the row count it registered. Where the depth already sits on
/// that registration such a relation returns at most `"depth"` rows whatever its
/// index holds, so a host running the text itself learns nothing about what lay
/// below — and [`search`], which runs it, reports that stratum
/// `"row_bound_reached"` rather than `"exhausted"`.
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
/// `top_k` is required, as it is on [`plan`] and for the same reason: the depths
/// this stage emits a `LIMIT` for were derived from it. This stage narrows nothing
/// of its own — a `LIMIT` below `"depth"` would leave `"depth"` and
/// `"planned_resolution"` describing a read nobody took.
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
    top_k,
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
    top_k: usize,
    weights: Option<&Bound<'py, PyDict>>,
    k: Option<u32>,
    decay: Option<&str>,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let call = collect_call(
        data,
        request,
        text_producers,
        statistics,
        top_k,
        data_format,
        base,
    )?;
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
/// Every `"statuses"` entry spells its own ending, and there are exactly seven
/// spellings. `"exhausted"` (with `"rows_emitted"`) is the only one of the seven
/// that names no stopper: that producer emitted every row ITS SEARCH PRODUCED.
/// On its own that is not a claim that everything matching was returned — whether
/// those were every row that was DUE is what `"fidelities"` says under the same
/// stratum key, and the two are read together. The other six
/// each name who stopped the read and where. `"depth_reached"` (with `"rank"`)
/// is the producer stopping at the depth the plan gave it, verified against the
/// rows fusion really pulled: ranks one through `"rank"` were read and nothing
/// below it was looked at. `"row_bound_reached"` (with `"rank"`) is the producer
/// stopping at the row count IT declared it can serve per invocation: it takes its
/// depth as an argument, the depth was already on that declaration, so the row past
/// it could not be asked for and whether one exists was NOT observable — which is
/// why it is not `"exhausted"`, and why reading deeper means raising that producer's
/// declared bound rather than re-planning. `"ceiling_reached"` (with `"bound"`, an
/// exact decimal `str`) is a contribution bound: every row at or above it was read
/// and the rows below were not — usually written by a fusion the caller's `top_k`
/// stopped. `"supplied_query_ended"` (with `"rank"`) is a unit running a query
/// text the host wrote rather than one this layer rendered: the layer bounds only
/// the outside of such a text, so what that text bounded inside itself — and
/// therefore what it left unread — was not observable either, which is why it is
/// its own word and not `"exhausted"`. `"execution_failed"` (with `"reason"`) is
/// the producer that could not run at all, and `"terms_rejected"` is the producer
/// that declined the request terms it was handed. A stratum that answered with
/// nothing and one that could not answer stay distinguishable, because none of
/// the seven is reduced to an aggregate flag.
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
/// Either axis may be the HOST's word rather than the relation's: a
/// `text_producers` value may carry a fifth `(generation, incompleteness)`
/// position, each member a `str` or `None`, recorded verbatim. It is the only way
/// an incompleteness reaches this answer at all, because the shipped text
/// relation indexes the document it was handed and has no way to know what was
/// missing from it. A declared generation replaces the content digest that
/// relation would otherwise attest; a declared incompleteness is added beside it
/// and leaves it alone. See this module's own documentation for both.
///
/// `"fidelities"` maps a stratum to what its producer declared about the rows it
/// can name, on two independent axes. `"completeness"` is `"complete"` or
/// `"lossy"`; `"order"` is `"faithful"` or `"perturbed"`. Where an axis is
/// degraded, `"completeness_evidence"` / `"order_evidence"` carries that
/// producer's OWN words for it, verbatim — never parsed here, never re-worded.
/// The evidence key is ABSENT, not `None`, when the axis is not degraded:
/// silence is the thing this surface exists to stop a caller interpreting. A
/// stratum whose stream never opened — one that failed before fusion was handed
/// anything — has no entry at all, which is the third state and the only one a
/// caller has to test for.
///
/// It is read WITH `"statuses"`, never instead of it. A status says how the read
/// ENDED; a fidelity says whether the rows that ended it were all the rows that
/// were DUE. `"exhausted"` beside a `"lossy"` declaration is neither a
/// contradiction nor a completeness claim: the producer emitted every row its
/// search produced, and the declaration says that search does not produce every
/// row there was.
///
/// `"exactness"` is `{"exact": bool, "deficit": list[str], "inflation":
/// list[str], "unbounded": list[str]}`, derived from those declarations and
/// attestations alone and therefore unmoved by how deep this call read. `True`
/// says no stratum in this fusion declared itself degraded — the narrow true
/// thing, not a certificate that every index was whole and every search
/// exhaustive.
///
/// When `"exact"` is `False`, every score is an ESTIMATE rather than a value and
/// the error runs in BOTH directions — which is why there is no
/// `"lower_bounds_for"` key. Fusion scores by RANK and nothing else, so a
/// stratum that fails to name a row does not merely withhold that row's
/// contribution: every row behind the missing one moves up a rank and collects a
/// larger one than it earned. A candidate the degraded stratum missed is summed
/// too LOW; one it named is summed too HIGH, and a consumer handed a one-sided
/// name would be confidently wrong in the direction the name told it not to
/// look. `"deficit"` and `"inflation"` name the strata responsible on each side,
/// in canonical order; `"unbounded"` names strata whose declared ORDER is
/// perturbed, for which no finite bound exists at all. The rows are still real
/// rows in this fusion's own certified order; what does NOT follow is that a row
/// absent from the answer would have stayed absent.
///
/// Each row's `"interval"` carries the size of its own error: `{"bounded": True,
/// "deficit": str, "inflation": str}` in the same fixed-point lexical as
/// `"score"`, or `{"bounded": False, "perturbed": list[str]}` where no finite
/// bound exists. `"certain_prefix"` is how many LEADING rows keep their places
/// whatever the degraded strata did or did not find — the answer a caller with a
/// completeness obligation actually has, since without it the only safe move is
/// to downgrade the whole answer. It claims membership and never absence: a row
/// PAST the prefix is possible rather than excluded.
///
/// `"unemitted_ceiling"` is the evidence that verdict rests on, in the same
/// fixed-point lexical as `"score"`: the most any candidate outside the answer
/// could be worth, counting both the candidates no stream ever named and the
/// ones a bounded read abandoned. A leading row is certain exactly when its own
/// floor clears it. It is `None` where a stratum declared a perturbed order,
/// because that breaks the one inequality every bound here rests on and no
/// finite ceiling exists — `"certain_prefix"` is then `0`.
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
    let call = collect_call(
        data,
        request,
        text_producers,
        statistics,
        top_k,
        data_format,
        base,
    )?;
    let declared = collect_weights(weights)?;
    // Read before the GIL is released, with every other Python-side argument:
    // the rule is owned Rust data by the time the ladder runs.
    let decay = decay_rule(decay, k).map_err(PyValueError::new_err)?;
    // The whole ladder runs detached (GIL released): parse, index, plan, admit,
    // execute and fuse. The answer dict is built after the GIL is reacquired.
    let result = py
        .detach(|| {
            let profile = build_profile(&declared, decay)?;
            run_search(&call, &profile)
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
    m.add("PlanDocumentError", m.py().get_type::<PlanDocumentError>())?;
    m.add_function(wrap_pyfunction!(plan, m)?)?;
    m.add_function(wrap_pyfunction!(certify_plan, m)?)?;
    m.add_function(wrap_pyfunction!(explain_depth, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(weight_for_depth, m)?)?;
    m.add_function(wrap_pyfunction!(class_width, m)?)?;
    m.add_function(wrap_pyfunction!(deepest_rank_within_width, m)?)?;
    Ok(())
}
