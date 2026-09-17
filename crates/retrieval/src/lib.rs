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
//! This build ships the first stage's value — [`Plan`] — together with the
//! typed request lattice ([`RetrievalRequest`], [`RequestTerm`]) and the plan's
//! canonical identity ([`PlanId`]). The later stages consume these types; they
//! are not implemented here yet.
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

mod canonical;
mod error;
mod id;
mod iri;
mod plan;
mod request;

pub use error::PlanError;
pub use id::{PLAN_ID_BYTES, PLAN_ID_DOMAIN, PLAN_VERSION, PlanId};
pub use iri::{Iri, Term, Weight};
pub use plan::{
    Plan, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry, StatisticsSnapshot,
};
pub use request::{Metric, RequestTerm, RetrievalRequest};

// The exact fixed-point type stratum weights are expressed in. Re-exported so a
// caller building a plan can name the values it puts in `Plan::stratum_weights`
// without depending on `purrdf-text` directly.
pub use purrdf_text::{Fixed, SCALE_DIGITS};
// The registry instance identity a plan records. Re-exported for the same
// reason: `Plan::registry_instance_id` is a value a caller compares against a
// live Registry.
pub use purrdf_sparql_eval::RegistryId;
