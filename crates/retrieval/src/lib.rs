// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-retrieval` — the composition layer over the ranked
//! property-function producers.
//!
//! PurRDF answers ranked retrieval through several relations reached via the
//! evaluator's property-function seam: exact BM25 rows from `purrdf-text`,
//! nearest-neighbour rows from the evaluator's `knn` module, spatial matches
//! from `purrdf-geo`. Each returns its own ranked list under a caller-supplied
//! IRI. What does not exist elsewhere is the layer that turns one request into
//! one answer across all of them: deciding which producers a request reaches,
//! running them, and combining their lists without destroying what each
//! producer knew about its own result.
//!
//! This crate is that layer, composed in four stages:
//!
//! ```text
//! plan(request, registry, statistics) -> Plan
//! compile(Plan, environment)          -> per-stratum SPARQL units
//! execute(units, registry, dataset)   -> per-stratum ranked streams
//! fuse(streams, profile, k)           -> the top k of one ordered answer
//! ```
//!
//! This build ships all four stages. [`plan`] is the pure planner, with the
//! stage's value ([`Plan`]), the typed request lattice ([`RetrievalRequest`],
//! [`RequestTerm`]), the statistics input planning consults ([`Statistics`]),
//! and the plan's canonical identity ([`PlanId`]). [`compile`] is the semantic
//! admission waist ([`AdmissionEnvironment`], [`AdmissionError`]) that emits the
//! per-stratum SPARQL units a caller can run directly ([`CompiledRetrieval`],
//! [`StratumUnit`]). [`execute`] runs those units independently through
//! `purrdf-sparql-eval` against the caller's own dataset, isolating a stratum's
//! failure in its own
//! [`ProducerStatus`] ([`ExecutionResult`], [`StratumStream`],
//! [`RankedStreamImpl`]). Finally [`fuse`] is the exact fixed-point fusion over
//! the verified ranked-stream protocol ([`RankedStream`], [`FusionStream`],
//! [`FusionProfile`]), bounded by the caller's [`TopK`] because fused
//! enumeration is top-k by construction.
//!
//! [`search`] is those four stages run as one — `fuse ∘ execute ∘ compile ∘
//! plan` — returning a [`SearchResult`] that names both the pinned plan and the
//! fusion profile. It adds no policy of its own: a caller that wants to stop
//! between stages calls the stage functions directly, and [`SearchError`] carries
//! whichever stage refused. A caller that stopped at [`execute`] and later wants
//! to fuse resumes through the same exported bridge `search` itself uses,
//! [`RankedStreamAdapter`], so the two compositions cannot drift.
//!
//! # Nothing is lost between the stages
//!
//! Each stage knows something the next one structurally cannot, and the answer
//! carries all of it rather than whatever survived to the end. Every applicable
//! producer's own status is in the [`FusionTrailer`] — the strata fusion
//! verified, and the strata that never became a stream because their unit failed
//! or the profile declares no weight for them. Every request term that reached
//! no producer is in [`SearchResult::unserved_terms`] with a typed
//! [`UnservedReason`], because statuses answer per producer and a caller asks
//! per term. Completeness is asserted only by the trailer, which exists only
//! once every stream has reached a terminal status — its own receipt, or the
//! contribution bound the fused top-k stopped reading it at.
//!
//! That includes what each producer declared about its *own* rows. A ranked
//! producer states its duplicate handling where it is registered, and fusion is
//! the consumer that declaration was written for, so it is carried from the
//! registry to that consumer as a [`StreamContract`] — read at the admission
//! waist into [`StratumUnit`], tagged onto every [`StratumStream`], reported
//! through [`RankedStream::contract`]. A producer that declared
//! [`DuplicatePolicy::Allowed`] is de-duplicated, which is what that policy says
//! its consumer must do; one that declared [`DuplicatePolicy::Unique`] is
//! believed and costs no per-stream identity set at all.
//!
//! A producer's [`RankOrdering`] is deliberately *not* carried here. Its claim
//! is about ranks, and ranks are held contiguous and ascending for every stream
//! regardless of what was declared. The only quantity fusion can observe is the
//! contribution, which it computes itself and refuses on mismatch, so the
//! producer supplies no term of it; a declaration read in contribution space
//! would refuse conforming streams for the consumer's own quantization.
//!
//! # Fusion is a law, not a knob
//!
//! A [`FusionProfile`] fixes the decay rule and its smoothing constant, every
//! stratum's weight and the total tie-break; the admitted contribution count
//! and the score ceiling follow from the weights, because a candidate may
//! surface at most once per stratum. Weights are read as ratios and never as
//! absolute quantities, so a map built with two different [`Fixed`]
//! constructors is a silent factor-of-`10^12` error that refuses nothing — see
//! [`FusionProfile::with_decay`] before writing one.
//! It is content-addressed, so an answer names exactly which law produced it.
//! Contributions are exact [`Fixed`] values computed with checked arithmetic;
//! an intermediate that does not fit is a loud [`FusionError::Overflow`], never
//! a wrapped score masquerading as an order.
//!
//! The law's decay rule, its weights, the fixed-point scale and a stratum's
//! depth are one coupled quantity: past a depth those first three decide,
//! adjacent ranks stop producing distinct contributions and the fused score
//! stops separating them. Nothing errors there and nothing becomes
//! nondeterministic — the declared tie-break is total, so the order simply falls
//! through to its later keys — which is exactly the kind of quiet degradation
//! this crate refuses to leave unsaid. It is said rather than refused:
//! [`FusionProfile::monotone_depth`] reports the exact bound before a plan runs,
//! and the fused trailer reports, per stratum, how deep the answer in hand was
//! actually read against it.
//!
//! Which rule a profile names decides how much depth a weight can buy.
//! [`DecayRule::ReciprocalRank`] truncates the reciprocal before applying the
//! weight, so every weight at or above one shares a bound near a million ranks.
//! [`DecayRule::WeightedReciprocalRank`] folds the weight into the numerator —
//! the same quantity, one exactly-rounded division instead of two truncations —
//! and its bound runs to roughly `10^6 · sqrt(w)`, so a stratum that must be
//! read fourteen million ranks deep is admissible at a weight of two hundred.
//! Read in reverse, that relation is a design calculus: under the weighted rule
//! a deep stratum *needs* a heavy enough weight, and a profile that underweights
//! one answers at a coarser rank resolution, which it reports. See
//! [`FusionProfile`] for the derivation in both directions.
//!
//! # A composition outside the kernel
//!
//! Nothing in `purrdf-core` or the property-function seam changes to admit this
//! layer, and no relation implements anything extra to participate. The
//! declaration lives beside the registry, supplied where a producer is wired up
//! (`purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked`) and read
//! back by IRI, so a producer states which request terms it accepts and how its
//! rows rank — declarative data, never a function pointer — while every relation
//! that has nothing to do with ranked retrieval says nothing at all.
//!
//! # One stratum, one producer
//!
//! A stratum is served by exactly one producer, refused at the point the
//! configuration is committed
//! (`purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked`) and again at
//! the admission waist for an edited plan.
//!
//! A rank is meaningful only inside the list that assigned it, so merging two
//! ranked lists needs either a comparable score — which a rank is not — or a
//! fusion rule, and this crate **is** the fusion rule. Two producers under one
//! stratum have neither, so their rows could only be concatenated: the second
//! producer's best row would surface below the whole of the first's output and
//! decay as though it had lost to rows it never competed with, a candidate both
//! produced would appear twice in one stratum's stream, and a first producer that
//! filled the depth would leave the second contributing nothing.
//!
//! The rule costs no configuration, because every configuration splits cleanly.
//! Co-stratum ranks are well defined iff the producers' ranks are comparable;
//! ranks are comparable iff the producers share a scoring law; and producers
//! sharing a scoring law can merge internally, by their own scores, below the
//! seam. So there are exactly two ways to express what a shared stratum was
//! reaching for, and the refusal names both:
//!
//! * **shards, per-language segments, a partitioned index** — anything whose
//!   scores are already comparable, with no weight meant to stand between them —
//!   merge inside ONE producer, which owns that comparability;
//! * **different scoring laws** — one stratum each, where the weighted sum across
//!   strata is the design rather than an accident. Taking this exit for shards
//!   would distort the score rather than merge it, because each stratum is a
//!   summand: a candidate held by two shards-as-strata would collect two
//!   contributions where the host meant one family's worth.
//!
//! The first exit takes **two** things, not one: comparable scores, *and* a
//! weighting that can ride inside the score. Sharing a law buys only the first,
//! and this is the case the rule is most often read past. Two classes over one
//! embedding space — a heading class and a body class — share a law in every
//! sense, but if the host means one to **outweigh** the other and the shared
//! score is a *bounded* metric (a cosine distance in `[0, 2]`, lower-better), the
//! merge cannot carry the weight. Sorting merged by `d / w`, a heading beats a
//! body hit only past a similarity edge of `(1 - s_c)(1 - w_h / w_c)`: at a
//! weight ratio of one half that is `0.45` against a body similarity of `0.1` but
//! `0.0005` against `0.999` and exactly zero against a perfect match. The edge
//! vanishes precisely where the top-k contest is decided and is largest for the
//! worst matches, so the weighting is both backwards and silently lost — an
//! unweighted merge still returns a plausible ranking. Such a pair takes the
//! **second** exit despite the shared law, because a [`FusionProfile`] weight
//! acts in rank space — and rank space is what a stratum is. An unbounded score
//! needs none of this: BM25F's field weights are natively a multiplicative weight
//! the score carries at every magnitude, so that case merges inside one producer
//! as the first exit says. The question to ask over a shared law is therefore:
//! *do you want these two weighted differently, and is the score bounded?*
//!
//! # It mints no vocabulary
//!
//! Producers, strata and weights are **caller-supplied configuration**. There is
//! no default registry, no built-in producer, and no fabricated namespace.
//! Fixtures use `example.org`. A configuration that names nothing is refused;
//! it is never guessed.
//!
//! # Exact where identity is concerned
//!
//! A plan's identity is a domain-separated BLAKE3 digest over a versioned,
//! canonical, length-framed encoding ([`Plan::canonical_bytes`]) — never over a
//! serde document or a `Hash`. The encoding sorts map entries, so it is a pure
//! function of the plan's fields and is byte-identical on every target.
//!
//! # No float is ever computed with
//!
//! Every number this layer derives — a stratum weight, a reciprocal-rank
//! contribution, a fused score, a per-stratum depth and the selectivity that
//! bounds it — is exact integer or base-10 fixed-point arithmetic, and the
//! crate root carries `#![deny(clippy::float_arithmetic)]` so no second path
//! can be reintroduced. This is the same denial `purrdf-text` and `purrdf-geo`
//! carry, for the same reason: an order computed in binary floating point is
//! not reproducible across targets, and it cannot be an identity — `0.0` and
//! `-0.0` are two spellings of one value and `NaN` is not equal to itself.
//!
//! The denial has **no exception**, not even a narrow one. A caller-supplied
//! embedding is the only float in the crate ([`RequestTerm::Vector`]), and it
//! is carried, never computed with: it is compared by
//! [`f32::to_bits`](f32::to_bits) — a total, reflexive function, which is what
//! makes `RequestTerm`'s `Eq` genuine — encoded into a plan's canonical bytes by
//! the same bit pattern, and written into the emitted query as that bit pattern
//! in hex ([`encode_embedding`]). All three readings are the one identity, which
//! is why two plans that are equal compile to one query and two that are not
//! never do. Storing a float and doing arithmetic on one are different acts, and
//! only the second is refused.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

mod admission;
mod canonical;
mod compile;
mod embedding;
mod error;
mod execute;
mod fixed;
mod fuse;
mod fusion_profile;
mod fusion_stream;
mod id;
mod iri;
mod matching;
mod plan;
mod planner;
mod ranked_stream;
mod reciprocal_rank;
mod render;
mod request;
mod search;
mod statistics;

pub use admission::{AdmissionEnvironment, AdmissionError};
pub use compile::{CompiledRetrieval, PlannedResolution, StratumUnit, compile};
pub use embedding::{EmbeddingError, decode_embedding, encode_embedding};
pub use error::{FusionError, PlanError};
pub use execute::{ExecutionError, ExecutionResult, RankedStreamImpl, StratumStream, execute};
pub use fuse::{FusionResult, TopK, fuse};
pub use fusion_profile::{DecayRule, FusionProfile, TieBreak};
pub use fusion_stream::{CandidateId, FusedRow, FusionStream, FusionTrailer, ProducerStatus};
pub use id::{
    FUSION_PROFILE_ID_BYTES, FUSION_PROFILE_ID_DOMAIN, FUSION_PROFILE_VERSION, FusionProfileId,
    PLAN_ID_BYTES, PLAN_ID_DOMAIN, PLAN_VERSION, PlanId,
};
pub use iri::{Iri, Term};
pub use plan::{
    Plan, PlanOrigin, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry,
    StatisticsSnapshot, UnservedReason, UnservedTerm,
};
pub use planner::plan;
pub use ranked_stream::{ProducerReceipt, ProtocolError, RankedStream, StreamContract};
pub use reciprocal_rank::MonotoneDepth;
pub use reciprocal_rank::{contribution, contribution_under, weighted_contribution};
pub use request::{Metric, RequestTerm, RetrievalRequest};
pub use search::{RankedStreamAdapter, SearchError, SearchResult, search};
pub use statistics::Statistics;

// The exact fixed-point type a fusion profile's weights and the fused scores
// are expressed in, and the fixed-point scale itself. Re-exported so a caller
// building a plan or a fusion profile can name those values without depending
// on `purrdf-text` directly.
pub use fixed::{Fixed, RECIP_K};
pub use purrdf_text::SCALE_DIGITS;
// The registry instance identity a plan records. Re-exported for the same
// reason: `Plan::registry_instance_id` is a value a caller compares against a
// live Registry.
pub use purrdf_sparql_eval::RegistryId;
// The two halves of a producer's declared stream contract. Re-exported because
// `StreamContract` is built from them and a caller assembling a stream of its
// own must be able to name them without depending on the evaluator crate.
pub use purrdf_sparql_eval::{DuplicatePolicy, RankOrdering};
