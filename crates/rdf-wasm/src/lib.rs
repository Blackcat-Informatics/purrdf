// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! # purrdf — a wasm32, in-memory RDF 1.2 engine with an idiomatic RDF/JS API
//!
//! Parcel **P10** of the purrdf program (`docs/design/PurRDF-PLAN.md`).
//! This crate compiles the PyO3-free [`purrdf`] kernel to
//! `wasm32-unknown-unknown` and exposes it to JavaScript/TypeScript through the
//! [RDF/JS](https://rdf.js.org/) community spec — `DataFactory`, `DatasetCore`, and
//! `Stream`/`Sink` — packaged for npm/ESM as **`purrdf`**.
//!
//! ## Scope (by charter)
//!
//! - **In-memory only.** A persistent (RocksDB) store and `crates/logic` do not
//!   compile to wasm and are deliberately excluded — this is the
//!   value-interned IR + the COW [`MutableDataset`](purrdf::ir::MutableDataset),
//!   not a persistent quad store.
//! - **Two SPARQL lanes.** The native multiset evaluator
//!   ([`purrdf_sparql_eval`]) binds to the wasm [`Dataset`] (see the `query` module),
//!   so SELECT / ASK / CONSTRUCT / DESCRIBE run client-side with no server. The
//!   synchronous lane is offline: it installs no remote source, so `SERVICE` / `LOAD`
//!   hard-fails there rather than silently returning a partial answer — the `SILENT`
//!   forms included, because `SILENT` tolerates an endpoint or source that was contacted
//!   and failed, and none is contacted there. The
//!   asynchronous lane (the `async_query` module) runs the same evaluator as a job that
//!   suspends through JSPI on host-resolved `SERVICE` and `LOAD` effects and yields to
//!   the event loop, so the host owns the I/O and its policy while PurRDF keeps the
//!   parsing, evaluation, joins, `SILENT` semantics and result encoding.
//! - **Separate from the C-ABI.** WASM has its own ownership model,
//!   packaging, and async I/O; it is not a C-ABI consumer and does not depend on the
//!   `no_std` track.
//!
//! ## The RDF-1.2 wedge
//!
//! No incumbent RDF/JS library carries RDF-1.2 quoted-triple terms or directional
//! literals. purrdf's `DataFactory` accepts a quoted triple anywhere a term is
//! expected (`termType: "Quad"` as subject/object) and round-trips base direction on
//! literals — the deliberate "overcome, don't inherit" extension to stock RDF/JS
//! (`.goals`: SUBSUME, EXTEND, ENHANCE).
//!
//! ## Architecture
//!
//! The `#[wasm_bindgen]` surface is a thin shim over `purrdf` seams that already
//! exist: `TermFactory` (the DataFactory 1:1 map), `DatasetMut`/`MutableDataset` (the
//! mutable `DatasetCore`), `native_codecs` (parse/serialize), and the
//! `purrdf-events` protocol (the `Stream`/`Sink`). Mapping logic lives in plain
//! Rust so it unit-tests on the native workspace gate; the wasm-bindgen wrappers are
//! exercised as real wasm under `wasm-pack test --node`.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

use wasm_bindgen::prelude::*;

// The idiomatic RDF/JS surface, built up parcel by parcel:
//   * `term`    — RDF/JS Term types (NamedNode/BlankNode/Literal/Variable/DefaultGraph
//                 + the RDF-1.2 Quad-as-term wedge)
//   * `factory` — the RDF/JS DataFactory over the engine's owned term model
//   * `codec`   — format-name resolution for the native codecs
//   * `convert` — Quad/Term ↔ engine value space (QuadValues/TermValue)
//   * `dataset` — the mutable RDF/JS DatasetCore over `MutableDataset`/`DatasetMut`
//                 (parse/serialize/size/add/delete/has/match/quads)
//   * `entail`  — SPARQL entailment-regime materialization + the rule inventories
//                 (`entailMaterialize`/`entailRules`/`entailImplementedRules`)
//   * `query`   — the offline SPARQL surface (`Dataset.query`) over the native
//                 evaluator, plus the GOVERNED lane (`queryGoverned` /
//                 `updateGoverned` / `explainQuery`) whose ceilings, wall
//                 deadline and cancellation token return a typed outcome rather
//                 than throwing when one of them trips
//   * `shacl`   — SHACL validation to SARIF + SHACL-AF entailment
//                 (`shaclValidateToSarif`/`shaclEntail`)
//   * `stream`  — the RDF/JS Sink over the `purrdf-events` ingestion protocol
//   * `operation` — the one implementation of every SPARQL operation kind, run
//                 offline by `query` and under a job's signal and sources by
//                 `async_query`, and the coded errors both lanes throw
//   * `async_query` — the asynchronous operation runtime: every evaluating `query`
//                 and SHACL surface as a job that suspends on host-resolved SERVICE /
//                 LOAD effects and yields to the event loop, through JSPI
//   * `interleaving` — the ledger of every `thread_local!` in the workspace and why
//                 each is safe while a job is suspended; `scripts/check-thread-locals.py`
//                 keeps it equal to the source
//   * `protocol` — the SPARQL 1.1 Protocol request surface (`SparqlProtocolRequest`):
//                 an HTTP request read into an operation, its dataset parameters
//                 applied, its response format negotiated, and a failure's HTTP problem
//   * `panic_poison` — the panic hook that poisons the instance before a panic's trap
//                 unwinds, so no later call runs on the state the panic left behind
mod async_query;
mod codec;
mod convert;
mod dataset;
pub mod entail;
mod factory;
#[doc(hidden)]
pub mod host;
pub mod interleaving;
mod jsonld;
mod operation;
mod panic_poison;
mod projection;
mod protocol;
mod query;
pub mod shacl;
mod shadow_stack;
mod stream;
mod term;
mod xpath_regex;

#[cfg(target_arch = "wasm32")]
pub use async_query::purrdf_jspi_run;
#[cfg(all(test, target_arch = "wasm32"))]
pub use async_query::{__purrdf_test_exchange_terminal, __purrdf_test_open_exchange};
pub use async_query::{
    AsyncEffect, AsyncEvidence, AsyncJob, AsyncJobOptions, AsyncOperationKind, DeliveryStatus,
    EffectKind, RunStatus, ServiceCatalog, SuspendStatus, async_stack_region_bytes,
};
#[cfg(all(test, target_arch = "wasm32"))]
pub use dataset::__purrdf_test_large_dataset_identity;
pub use dataset::Dataset;
pub use entail::RegimeClosure;
pub use factory::DataFactory;
pub use jsonld::CompiledJsonLdContext;
pub use projection::{ProjectionLift, ProjectionPackage, lift_projection};
pub use protocol::SparqlProtocolRequest;
pub use query::{
    CancellationToken, EntailmentQueryOutcome, GovernorEvidence, NegotiatedOutcome, PartialAnswers,
    ProvenanceInfo, QueryEngine, QueryOutcome, QueryResult, SelectResult, SelectRow,
    SilencedInvocation, TrippedGovernor, UpdateOutcome, governor_dimensions, provenance_from_json,
    provenance_from_xml,
};
pub use stream::Sink;
pub use term::{Quad, Term};

/// Runs once, when the instance starts: installs the synchronous shadow stack's floor
/// in [`purrdf_stack`], the measurement the SPARQL evaluator's stack guards refuse
/// against, and the panic hook that poisons the instance (the `panic_poison` module).
///
/// The floor's default is address 0, the floor rustc's `--stack-first` layout gives the
/// stack; this reads the linker's own record of it instead (`shadow_stack`), so the
/// guards measure against wherever the stack really ends however this module was linked.
/// The asynchronous lane switches away from this floor onto each job's region and back
/// (`async_query`), and puts this value back whenever a job suspends or returns.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn install_stack_floor() {
    purrdf_stack::replace_floor(shadow_stack::read().low);
    panic_poison::install();
}

/// Test-only: panics when the host has armed it, so the Node lane can prove that a panic
/// out of a synchronous call poisons the instance. Returns 0, doing nothing, otherwise.
///
/// Every PurRDF entry point is written not to panic, so no input reaches one; this is
/// the one way a test can raise a genuine Rust panic through a synchronous export. It is
/// a raw export, not a `#[wasm_bindgen]` one: the glue generates no wrapper for it and
/// the package root does not export it, so it is reachable only through the instance's
/// raw exports. And it is inert unless the JavaScript global
/// `__purrdfArmTestPanic` is `true` when it is called — a flag only the test fixture that
/// exercises it sets.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn __purrdf_test_panic() -> u32 {
    let armed =
        test_flag(panic_poison::TEST_PANIC_FLAG).is_ok_and(|flag| flag.as_bool() == Some(true));
    assert!(!armed, "the armed test panic fired");
    0
}

/// Test-only: executes `unreachable` when the host has armed it, so the Node lane can prove
/// that a trap which is not a panic, out of a synchronous call, poisons the instance.
/// Returns 0, doing nothing, otherwise.
///
/// No PurRDF entry point traps on any input, and a panic goes through the panic hook
/// (see [`__purrdf_test_panic`]); this raises the trap directly, with no hook running
/// first, exactly as allocation failure or a stray `unreachable` would. The trap is the
/// only thing it does: the JavaScript side has to recognize it from the error alone. Like
/// the panic export it is a raw export the package root never exports, inert unless the
/// JavaScript global `__purrdfArmTestTrap` is `true` when it is called.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn __purrdf_test_trap() -> u32 {
    let armed =
        test_flag(panic_poison::TEST_TRAP_FLAG).is_ok_and(|flag| flag.as_bool() == Some(true));
    if armed {
        core::arch::wasm32::unreachable();
    }
    0
}

/// The purrdf engine version (the crate's SemVer), exposed to JS as `version()`.
///
/// A liveness probe for the wasm build + the npm package: importing `purrdf` and
/// calling `version()` proves the module instantiated and the engine linked.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_the_crate_semver() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(thread_local_v2, js_name = globalThis)]
    static GLOBAL_THIS: JsValue;
}

/// Read the current test flag; only the stable global object is cached.
#[cfg(target_arch = "wasm32")]
fn test_flag(name: &str) -> Result<JsValue, JsValue> {
    GLOBAL_THIS.with(|global| host::reflect_get(global, name))
}
