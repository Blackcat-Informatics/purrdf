// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-iri` — the native **IRI/URI value space** for the RDF 1.2 query stack.
//!
//! A pure-Rust, **zero-runtime-dependency**, wasm-clean leaf crate: the drop-in
//! replacement for the oxigraph-family `oxiri`, and the second foundation slice of
//! the native SPARQL engine (purrdf S2). It is deliberately decoupled
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
//!
//! # Examples
//!
//! Parse an absolute IRI, resolve a relative reference against it, and normalize
//! a messy spelling — the three core entry points:
//!
//! ```rust
//! use purrdf_iri::parse;
//!
//! // Parse + validate, with zero-copy component access.
//! let base = parse("http://example.org/a/b/c")?;
//! assert_eq!(base.scheme(), Some("http"));
//! assert_eq!(base.path(), "/a/b/c");
//!
//! // RFC-3986 §5 strict reference resolution.
//! let joined = base.resolve("../d?x=1")?;
//! assert_eq!(joined.as_str(), "http://example.org/a/d?x=1");
//!
//! // RFC-3986 §6.2.2 syntax normalization: case, percent-encoding, dot segments.
//! let messy = parse("HTTP://EXAMPLE.org/a/./b/../c/%7Ename")?;
//! assert_eq!(messy.normalize().as_str(), "http://example.org/a/c/~name");
//! # Ok::<(), purrdf_iri::IriError>(())
//! ```
//!
//! Expand and contract CURIEs over a caller-supplied [`PrefixMap`]:
//!
//! ```rust
//! use purrdf_iri::{PrefixMap, contract, expand_curie};
//!
//! let mut prefixes = PrefixMap::new();
//! prefixes.insert("ex", "http://example.org/ns#");
//!
//! assert_eq!(
//!     expand_curie("ex:Thing", &prefixes),
//!     Some("http://example.org/ns#Thing".to_owned())
//! );
//! assert_eq!(
//!     contract("http://example.org/ns#Thing", &prefixes),
//!     Some("ex:Thing".to_owned())
//! );
//! ```
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

mod curie;
mod error;
mod normalize;
mod parse;
pub mod pos;
mod resolve;

pub use curie::{PrefixMap, contract, curie_prefix, expand_curie, resolve};
pub use error::{IriError, Result};
pub use parse::{Iri, parse, parse_uri};
pub use pos::{LineIndex, Position};
