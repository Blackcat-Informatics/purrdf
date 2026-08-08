// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
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
//! - **Hard-fail, no degraded fallback.** `SERVICE` federation ([`remote`],
//!   [`remote_http`]), `LATERAL` (`binop`), host-injected **property-function**
//!   relations ([`property_fn`], `property_fn_plan`, `property_fn_eval`), and SPARQL
//!   `UPDATE` ([`update`]) are all evaluated in-engine — none of them is out of scope.
//!   What remains a typed [`EvalError::Unsupported`] is a narrow, enumerated residue:
//!   a variable-bound quoted-triple-term component in a BGP or property-path pattern
//!   (`convert`), an unresolved custom SPARQL function or aggregate IRI (`expr`,
//!   `modifier`), `heldIn` called without a caller-supplied standpoint-predicate
//!   configuration, and a manually constructed graph pattern whose nesting exceeds the
//!   parser's safety bound (`governor::soundness`). A call into a relation the host did
//!   not register, or one no declared access pattern admits, is not in that residue
//!   either: it is a typed [`EvalError::Function`], because the construct is supported
//!   and the host's table is what does not answer it. Never a wrong answer, and never a
//!   partial one *offered as complete* (the project `no-optionality` doctrine).
//! - **Governed execution, when a caller asks for it.** A caller may attach ceilings
//!   and a stop signal ([`governor::QueryGovernors`]) and run
//!   [`NativeSparqlEngine::query_governed`], which either completes or returns
//!   [`GovernedOutcome::BudgetExhausted`] — the rows already reached, plus a
//!   machine-checked [`PartialAnswers`] certificate saying whether they are a lower
//!   bound, an upper bound, or neither. This does not soften the pillar above; it is
//!   what lets the pillar stay absolute. A ceiling changes only the **outcome**, never
//!   the query's complete answer: different ceilings may expose different sides of that
//!   interval, but none labels an uncertified row as an answer. And a truncation is
//!   unrepresentable in the shape of a complete result — it is a distinct type, reachable
//!   only while carrying its certificate — so the engine still never hands anyone a
//!   partial answer they could mistake for the whole one. An ungoverned query takes the
//!   direct evaluator path before any governor charge, ledger, or stop probe. See
//!   [`governor`] and `docs/SPARQL-GOVERNOR-PROFILE.md`.
//!
//! The crate carries **zero oxigraph-family dependencies** and builds for
//! `wasm32-unknown-unknown` (the wasm query path); both invariants are
//! gated by `make rdf-core-hygiene`.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
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
mod fallible;
mod governed;
pub mod governor;
mod list_fn;
mod modifier;
pub(crate) mod parallel;
#[cfg(test)]
mod parallel_determinism_gate;
mod path;
pub mod property_fn;
mod property_fn_eval;
mod property_fn_plan;
pub mod remote;
// HTTP-shaped SERVICE source. The actual POST transport is host-injected so this
// crate stays wasm-portable.
pub mod remote_http;
mod row_ingest;
pub mod scratch;
pub mod solution;
mod substitute;
mod template;
pub mod update;
pub mod user_fn;

pub use engine::{NativeSparqlEngine, PlanCache, PreparedQuery, QueryOptions, ShaclPrebinding};
pub use error::EvalError;
pub use eval::{
    EvalCtx, EvalOptions, LossVocabulary, Outcome, StandpointPredicates, eval, evaluate_query,
};
pub use fallible::{CompleteSparqlResult, FallibleSparqlError, FallibleSparqlResult};
pub use governed::{
    BudgetExhausted, GovernedEvidence, GovernedOutcome, GovernedUpdateOutcome, PartialAnswers,
    PartialSparqlResult, RelationIdentity,
};
pub use governor::{
    CHARGE_SCHEDULE, CancellationFlag, ChargePoint, GOVERNOR_CORPUS_DIGEST,
    GOVERNOR_PROFILE_DIGEST, GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, GovernorState,
    ItemCharge, NodeCharges, NonMonotoneBarrier, PlanEstimate, ProfileIdentity, QueryExplanation,
    QueryGovernors, STOP_POLL_FUEL, StopSignal, WallDeadline, resolve_precedence,
};
// The kernel's governor vocabulary, re-exported so a host that governs queries through
// this crate can NAME what it gets back — the ceilings it set, what was spent, and which
// governor stopped the execution — without also depending on `purrdf-core` directly. A
// governed surface whose outcome types are unnameable from the crate that produces them
// is one no consumer can match on.
pub use purrdf_core::{GovernorEvidence, ResourceDimension, StopCause, TrippedGovernor};
// The adornment lattice, re-exported for the same reason: it appears in
// [`PropertyFunction`]'s own signature (`modes`, `rows_per_invocation`, `admits`), so a
// host implementing the trait cannot write the impl without naming it.
pub use purrdf_core::binding_pattern::BindingPattern;
// Re-exported so engine hosts can configure the extension-function namespace set
// (see [`NativeSparqlEngine::with_parser_options`]) without depending on the
// front-end crate directly.
pub use purrdf_sparql_algebra::ParserOptions;
// The property-function seam: the relation trait a host implements, the argument /
// row / arity types its calls speak in, the registry evaluation resolves a predicate
// IRI against, and the in-memory reference relation. Re-exported so a host wires a
// relation into the engine without naming the module path.
pub use property_fn::{
    MemoryRelation, PfArgs, PfArity, PfCursor, PfDescriptor, PfMode, PfRow, PropertyFunction,
    PropertyFunctionRegistry,
};
pub use remote::{LocalRemoteQuerySource, RemoteError, RemoteQuerySource, ResolvedBindings};
pub use remote_http::{HttpRemoteQuerySource, HttpRequest, HttpTransport};
pub use scratch::{ScratchId, ScratchInterner, SolutionTerm};
pub use solution::{Solution, SolutionSeq, VarSchema, compatible};
pub use update::{GraphResolveRequest, GraphResolver};
pub use user_fn::{
    Arity, NativeFnBody, NativeFunction, NodeKind, TypeConstraint, UserFnBody, UserFnParam,
    UserFunction, UserFunctionRegistry, Volatility,
};

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
