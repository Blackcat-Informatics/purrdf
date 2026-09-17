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
//! execute(units, registry)            -> per-stratum ranked streams
//! fuse(streams, profile)              -> one ordered answer
//! ```
//!
//! This build ships all four stages. [`plan`] is the pure planner, with the
//! stage's value ([`Plan`]), the typed request lattice ([`RetrievalRequest`],
//! [`RequestTerm`]), the statistics input planning consults ([`Statistics`]),
//! and the plan's canonical identity ([`PlanId`]). [`compile`] is the semantic
//! admission waist ([`AdmissionEnvironment`], [`AdmissionError`]) that emits the
//! per-stratum SPARQL units a caller can run directly ([`CompiledRetrieval`],
//! [`StratumUnit`]). [`execute`] runs those units independently through
//! `purrdf-sparql-eval`, isolating a stratum's failure in its own
//! [`ProducerStatus`] ([`ExecutionResult`], [`StratumStream`],
//! [`RankedStreamImpl`]). Finally [`fuse`] is the exact fixed-point fusion over
//! the verified ranked-stream protocol ([`RankedStream`], [`FusionStream`],
//! [`FusionProfile`]).
//!
//! # Fusion is a law, not a knob
//!
//! A [`FusionProfile`] fixes the decay rule and its smoothing constant, every
//! stratum's weight, the total tie-break and the admitted contribution ceiling.
//! It is content-addressed, so an answer names exactly which law produced it.
//! Contributions are exact [`Fixed`] values computed with checked arithmetic;
//! an intermediate that does not fit is a loud [`FusionError::Overflow`], never
//! a wrapped score masquerading as an order.
//!
//! # A composition outside the kernel
//!
//! Nothing in `purrdf-core` or the property-function seam changes to admit this
//! layer. The seam gains one additive, required capability declaration
//! (`purrdf_sparql_eval::PropertyFunction::retrieval_capability`) so a producer
//! can state which request terms it accepts and how its rows rank — declarative
//! data, never a function pointer.
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
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

mod admission;
mod canonical;
mod compile;
mod error;
mod execute;
mod fixed;
mod fuse;
mod fusion_profile;
mod fusion_stream;
mod id;
mod iri;
mod plan;
mod planner;
mod ranked_stream;
mod reciprocal_rank;
mod request;
mod statistics;

pub use admission::{AdmissionEnvironment, AdmissionError};
pub use compile::{CompiledRetrieval, StratumUnit, compile};
pub use error::{FusionError, PlanError};
pub use execute::{ExecutionError, ExecutionResult, RankedStreamImpl, StratumStream, execute};
pub use fuse::{FusionResult, fuse};
pub use fusion_profile::{DecayRule, FusionProfile, TieBreak};
pub use fusion_stream::{CandidateId, FusedRow, FusionStream, FusionTrailer, ProducerStatus};
pub use id::{
    FUSION_PROFILE_ID_BYTES, FUSION_PROFILE_ID_DOMAIN, FUSION_PROFILE_VERSION, FusionProfileId,
    PLAN_ID_BYTES, PLAN_ID_DOMAIN, PLAN_VERSION, PlanId,
};
pub use iri::{Iri, Term, Weight};
pub use plan::{
    Plan, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry, StatisticsSnapshot,
};
pub use planner::plan;
pub use ranked_stream::{ProducerReceipt, ProtocolError, RankedStream};
pub use reciprocal_rank::contribution;
pub use request::{Metric, RequestTerm, RetrievalRequest};
pub use statistics::Statistics;

// The exact fixed-point type stratum weights and fused scores are expressed in,
// and the fixed-point scale itself. Re-exported so a caller building a plan or
// a fusion profile can name those values without depending on `purrdf-text`
// directly.
pub use fixed::{Fixed, RECIP_K};
pub use purrdf_text::SCALE_DIGITS;
// The registry instance identity a plan records. Re-exported for the same
// reason: `Plan::registry_instance_id` is a value a caller compares against a
// live Registry.
pub use purrdf_sparql_eval::RegistryId;
