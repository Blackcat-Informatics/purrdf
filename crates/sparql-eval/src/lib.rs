// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native, RDF-1.2-first **multiset SPARQL evaluator** (purrdf S6).
//!
//! This crate is the evaluation runtime that consumes the
//! [`purrdf_sparql_algebra`] front-end (S5) and evaluates it over the
//! [`purrdf_core`] IR's [`DatasetView`](purrdf_core::DatasetView) read trait
//! **entirely in interned [`TermId`](purrdf_core::TermId) space**. It is the
//! native replacement for the oxigraph-family `spareval` on the query path and
//! the single required impl of the
//! [`SparqlEngine`](purrdf_core::SparqlEngine) seam.
//!
//! ## Design pillars
//!
//! - **TermId hot path.** Basic-graph-pattern matching and joins never leave
//!   interned-id space: constants resolve to a dataset
//!   [`purrdf_core::TermId`] once (via `term_id_by_value`, P4) and
//!   solutions carry [`SolutionTerm`]s that are a single integer compare apart.
//!   Computed terms (FILTER/BIND results not already in the dataset) are interned
//!   in a per-query scratch table — but a computed value that *does* exist in the
//!   dataset is **promoted** to [`SolutionTerm::Existing`] at mint time, so
//!   cross-case join keys are unequal purely by construction (no structural
//!   fallback at join time). See
//!   [`scratch`].
//! - **Multiset (bag) semantics.** Solutions are a bag, preserved until
//!   `DISTINCT`/`REDUCED`. See [`solution`].
//! - **Property paths in-engine (S8).** The `Path` graph pattern is evaluated
//!   over the same indexed surface, wasm-safe, covering the full algebra
//!   (`* + ? / | ^ !()` and the PurRDF `{n,m}` / `<any>` extensions) — see the
//!   `path` module.
//! - **Hard-fail, no degraded fallback.** A well-formed but out-of-scope algebra
//!   node (`SERVICE`, `LATERAL`, SPARQL `UPDATE`) or an unimplemented builtin is a
//!   typed [`EvalError::Unsupported`] — never a partial or wrong answer (the project
//!   `no-optionality` doctrine).
//!
//! The crate carries **zero oxigraph-family dependencies** and builds for
//! `wasm32-unknown-unknown` (the wasm query path); both invariants are
//! gated by `make rdf-core-hygiene`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bgp;
mod binop;
mod clock;
mod construct;
mod convert;
mod dataset_spec;
mod describe_query;
pub mod engine;
pub mod error;
pub mod eval;
mod expr;
mod list_fn;
mod modifier;
pub(crate) mod parallel;
#[cfg(test)]
mod parallel_determinism_gate;
mod path;
pub mod remote;
// HTTP-shaped SERVICE source. The actual POST transport is host-injected so this
// crate stays wasm-portable.
pub mod remote_http;
pub mod scratch;
pub mod solution;
mod substitute;
mod template;
pub mod update;

pub use engine::{NativeSparqlEngine, PlanCache, PreparedQuery};
pub use error::EvalError;
pub use eval::{eval, evaluate_query, EvalCtx, EvalOptions, Outcome, StandpointPredicates};
// Re-exported so engine hosts can configure the extension-function namespace set
// (see [`NativeSparqlEngine::with_parser_options`]) without depending on the
// front-end crate directly.
pub use purrdf_sparql_algebra::ParserOptions;
pub use remote::{LocalRemoteQuerySource, RemoteError, RemoteQuerySource, ResolvedBindings};
pub use remote_http::{HttpRemoteQuerySource, HttpRequest, HttpTransport};
pub use scratch::{ScratchId, ScratchInterner, SolutionTerm};
pub use solution::{compatible, Solution, SolutionSeq, VarSchema};
pub use update::GraphResolver;

/// A deterministic, seed-free hasher builder (`AHasher` with fixed keys).
///
/// Used for every internal map/set whose construction order or membership could
/// otherwise depend on a per-process random seed. Two reasons:
///
/// 1. **Determinism.** SPARQL multiset output must be reproducible; a randomly
///    seeded hasher could reorder hash-iteration-driven steps and leak into the
///    result. We always drive *output* order from `Vec`s, but fixed-key hashing
///    removes the hazard entirely (cf. the repo `mappings-determinism` lesson).
/// 2. **wasm-cleanliness.** `std`'s default `RandomState` would pull a random
///    source; fixed-key `AHasher` needs none, keeping the crate clean on
///    `wasm32-unknown-unknown`.
///
/// This mirrors `purrdf-core`'s own fixed-key value-index hashing.
pub(crate) type DetHasher = std::hash::BuildHasherDefault<ahash::AHasher>;

/// A deterministic, seed-free [`HashMap`](std::collections::HashMap). See [`DetHasher`].
pub(crate) type DetHashMap<K, V> = std::collections::HashMap<K, V, DetHasher>;

/// A deterministic, seed-free [`HashSet`](std::collections::HashSet). See [`DetHasher`].
pub(crate) type DetHashSet<K> = std::collections::HashSet<K, DetHasher>;
