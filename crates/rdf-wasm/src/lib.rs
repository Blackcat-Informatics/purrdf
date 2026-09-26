// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! # purrdf — a wasm32, in-memory RDF 1.2 engine with an idiomatic RDF/JS API
//!
//! Parcel **P10** of the purrdf program (`docs/design/PurRDF-PLAN.md`).
//! This crate compiles the oxigraph-free, PyO3-free [`purrdf`] kernel to
//! `wasm32-unknown-unknown` and exposes it to JavaScript/TypeScript through the
//! [RDF/JS](https://rdf.js.org/) community spec — `DataFactory`, `DatasetCore`, and
//! `Stream`/`Sink` — packaged for npm/ESM as **`purrdf`**.
//!
//! ## Scope (by charter)
//!
//! - **In-memory only.** The oxigraph `Store` (RocksDB) and `crates/logic` do not
//!   compile to wasm and are deliberately excluded — this is the
//!   value-interned IR + the COW [`MutableDataset`](purrdf::ir::MutableDataset),
//!   not a persistent quad store.
//! - **Two SPARQL lanes.** The native, oxigraph-free multiset evaluator
//!   ([`purrdf_sparql_eval`]) binds to the wasm [`Dataset`] (see the `query` module),
//!   so SELECT / ASK / CONSTRUCT / DESCRIBE run client-side with no server. The
//!   synchronous lane is offline: it installs no remote source, so `SERVICE` / `LOAD`
//!   hard-fails there rather than silently returning a partial answer — except for the
//!   `SILENT` forms, which SPARQL 1.1 requires to succeed with nothing fetched. The
//!   asynchronous lane (the `async_query` module) runs the same evaluator as a job that
//!   suspends through JSPI on host-resolved `SERVICE` and `LOAD` effects and yields to
//!   the event loop, so the host owns the I/O and its policy while PurRDF keeps the
//!   parsing, evaluation, joins, `SILENT` semantics and result encoding.
//! - **Separate from the C-ABI (P8).** WASM has its own ownership model,
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
//   * `async_query` — the asynchronous operation runtime: every evaluating `query`
//                 and SHACL surface as a job that suspends on host-resolved SERVICE /
//                 LOAD effects and yields to the event loop, through JSPI
//   * `protocol` — the SPARQL 1.1 Protocol request surface (`SparqlProtocolRequest`):
//                 an HTTP request read into an operation, its dataset parameters
//                 applied, its response format negotiated
//   * `panic_poison` — the panic hook that poisons the instance before a panic's trap
//                 unwinds, so no later call runs on the state the panic left behind
mod async_query;
mod codec;
mod convert;
mod dataset;
pub mod entail;
mod factory;
mod jsonld;
mod panic_poison;
mod projection;
mod protocol;
mod query;
pub mod shacl;
mod stream;
mod term;

#[cfg(target_arch = "wasm32")]
pub use async_query::purrdf_jspi_run;
pub use async_query::{
    AsyncEffect, AsyncEffectKind, AsyncEvidence, AsyncJob, AsyncJobOptions, AsyncOperationKind,
    LoadAuthorization, ServiceCatalog, ShaclAsyncOperation,
};
pub use dataset::Dataset;
pub use entail::RegimeClosure;
pub use factory::DataFactory;
pub use jsonld::CompiledJsonLdContext;
pub use projection::{ProjectionLift, ProjectionPackage, lift_projection};
pub use protocol::SparqlProtocolRequest;
pub use query::{
    CancellationToken, EntailmentQueryOutcome, GovernorEvidence, NegotiatedOutcome, PartialAnswers,
    ProvenanceInfo, QueryEngine, QueryOutcome, QueryResult, SelectResult, SelectRow,
    TrippedGovernor, UpdateOutcome, governor_dimensions, provenance_from_json, provenance_from_xml,
};
pub use stream::Sink;
pub use term::{Quad, Term};

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    /// The low end of this module's shadow stack, where `wasm-ld` placed it. Only its
    /// address is ever taken; the byte is never read.
    safe static __stack_low: u8;
}

/// Runs once, when the instance starts: installs the synchronous shadow stack's floor
/// in [`purrdf_stack`], the measurement the SPARQL parser's and evaluator's stack guards
/// refuse against, and the panic hook that poisons the instance (the `panic_poison` module).
///
/// The floor's default is address 0, the floor rustc's `--stack-first` layout gives the
/// stack; this reads the linker's own record of it instead, so the guards measure against
/// wherever the stack really ends however this module was linked. The
/// asynchronous lane switches away from this floor onto each job's region and back
/// (`async_query`), and puts this value back whenever a job suspends or returns.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn install_stack_floor() {
    // `black_box`: the low end may be address 0, and nothing may be inferred from an
    // address the compiler assumes is not null.
    let low = core::hint::black_box(&raw const __stack_low) as usize;
    purrdf_stack::replace_floor(low);
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
    let armed = js_sys::Reflect::get(
        &js_sys::global(),
        &JsValue::from_str(panic_poison::TEST_PANIC_FLAG),
    )
    .is_ok_and(|flag| flag.as_bool() == Some(true));
    if armed {
        panic!("the armed test panic fired");
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
