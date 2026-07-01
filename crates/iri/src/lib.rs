// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-iri` — the native **IRI/URI value space** for the RDF 1.2 query stack.
//!
//! A pure-Rust, **zero-runtime-dependency**, wasm-clean leaf crate: the drop-in
//! replacement for the oxigraph-family `oxiri`, and the second foundation slice of
//! the native SPARQL engine (purrdf S2, EPIC #906). It is deliberately decoupled
//! from `purrdf-core` (no dependency in either direction yet); the IR keeps
//! IRIs **lexical-verbatim** (Constitution C0.1) and this crate is the
//! validation/resolution layer beside it.
//!
//! # Coverage (a superset of `oxiri`)
//!
//! * **Parse + validate** — RFC-3987 IRIs ([`parse`]) and the strict-ASCII RFC-3986
//!   URI subset ([`parse_uri`]). Component spans (scheme/authority/path/query/
//!   fragment) are exposed without re-encoding.
//! * **Reference resolution** — RFC-3986 §5 strict resolution ([`Iri::resolve`]).
//! * **Syntax normalization** — RFC-3986 §6.2.2 ([`Iri::normalize`]): case, percent-
//!   encoding, and dot-segment normalization. Idempotent.
//! * **CURIE/prefix** — [`expand_curie`]/[`resolve`]/[`contract`] over a
//!   [`PrefixMap`], subsuming the SSSOM serializer's hand-rolled prefix logic.
//!   `oxiri` has none of this — it is the EXTEND deliverable for this slice.
//!
//! # Hard-fail
//!
//! Malformed input is a typed [`IriError`], never a degraded fallback or silent
//! default (repo `no-optionality` doctrine). The one `Option`-returning surface is
//! CURIE expansion, where `None` is a *semantic* "not a CURIE / undeclared prefix"
//! signal, faithful to the SSSOM behavior this crate subsumes.

#![forbid(unsafe_code)]
#![feature(portable_simd)]

mod curie;
mod error;
mod normalize;
mod parse;
mod resolve;

pub use curie::{contract, curie_prefix, expand_curie, resolve, PrefixMap};
pub use error::{IriError, Result};
pub use parse::{parse, parse_uri, Iri};
