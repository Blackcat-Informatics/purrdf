// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-iri` — the native **IRI/URI value space** for the RDF 1.2 query stack.
//!
//! A pure-Rust, wasm-clean crate whose one dependency is the zero-dependency
//! lexical layer [`purrdf_lex`]: the drop-in
//! replacement for the oxigraph-family `oxiri`, and the second foundation slice of
//! the native SPARQL engine. It is deliberately decoupled
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
//! * **Base scoping** — [`BaseIri`] (an IRI whose absoluteness is checked once and
//!   then carried by the type) and [`BaseScope`], the in-scope base *stack* shared
//!   by every codec: `xml:base` per element, JSON-LD `@base` per context frame, and
//!   Turtle `@base` rebinding relative to the previous base. It separates the two
//!   grammar families — [`BaseScope::resolve`] for syntaxes that admit relative
//!   references, [`BaseScope::resolve_absolute_only`] for those that do not — and
//!   carries [`BaseOrigin`] provenance so a diagnostic can name *which* base was in
//!   force and where it came from. [`BaseIri::relativize`] is the inverse, letting a
//!   serializer emit `<>` or `<foo>` under a base.
//! * **Syntax normalization** — RFC-3986 §6.2.2 ([`Iri::normalize`]): case, percent-
//!   encoding, and dot-segment normalization. Idempotent.
//! * **CURIE/prefix** — [`expand_curie`]/[`resolve`]/[`contract`] over a
//!   [`PrefixMap`], subsuming the SSSOM serializer's hand-rolled prefix logic.
//!   `oxiri` has none of this — it is the EXTEND deliverable for this slice.
//! * **BCP 47 language tags** — [`langtag`], RFC 5646 `Language-Tag`
//!   well-formedness against the §2.1 ABNF and the closed §2.2.8 grandfathered
//!   list, shared by embedding metadata and CSVW validation. An accepted tag
//!   decomposes into its sections — including the two the grammar leaves
//!   hyphen-joined, extensions keyed by their singleton and the private-use
//!   subtags — in borrowing or owning form, and
//!   [`langtag::canonical_case`] rewrites it in the §2.1.1 case convention
//!   (language lower, region upper, script title, registered spelling for the
//!   grandfathered tags). The boundary is well-formedness: subtags are never
//!   checked against the IANA Language Subtag Registry, and RFC 4647
//!   language-range matching is outside this crate's scope entirely.
//! * **Grammar terminals** — [`terminals`], the exact Turtle/SPARQL character
//!   classes (`WS`, `PN_CHARS_BASE`, `PN_CHARS_U`, `PN_CHARS`, `VARNAME`) and
//!   the byte-class scanners built from them, re-exported from their home in
//!   [`purrdf_lex`], the lexical layer every grammar in the workspace shares.
//!   This crate's parser validates every component with them.
//! * **Host syntax** — [`host`], the RFC 3986 §3.2.2 `IPv4address`,
//!   `IPv6address` and `reg-name` productions as predicates. Every authority
//!   [`parse`] accepts has its host decided by them, and they are public so a
//!   surface that asks the same question (a JSON Schema `ipv4` format, a mail
//!   address literal) asks it here instead of carrying a second address parser.
//! * **IDNA2008** — [`idna`], host names under RFC 5891 over the RFC 5892
//!   derived property, the RFC 5892 Appendix A contextual rules and the
//!   RFC 5893 Bidi rule, with RFC 3492 Punycode between A-labels and U-labels
//!   and a local mapping step (NFKC_Casefold, then NFC). Every Unicode table is
//!   generated from the Unicode 17.0.0 database vendored under
//!   `crates/iri/unicode/`. [`Iri::to_uri`] applies it as RFC 3987 §3.1
//!   describes; [`parse`] never does, because RFC 3987 compares IRIs code point
//!   by code point.
//! * **JSON string escape law** — [`json_escape`], the one RFC 8259 §7 string
//!   body escaper every PurRDF JSON writer shares, re-exported from
//!   [`purrdf_lex`].
//!
//! # Hard-fail
//!
//! Malformed input is a typed [`IriError`], never a degraded fallback or silent
//! default (repo `no-optionality` doctrine). A relative reference with no base in
//! scope is [`IriError::NoBase`] — this crate implements RFC-3986 §5.1.1 and §5.1.2
//! and then hard-fails exactly where §5.1.4 says to. It never invents a base,
//! because it is handed BYTES and has no retrieval IRI to invent one from: §5.1.3 is
//! vacuous here, and so it is for every surface built on this crate that is likewise
//! handed bytes — the Rust library API, wasm, the C ABI, Python, and CLI stdin.
//! `purrdf-cli` is the one surface that *has* a retrieval IRI, and it implements
//! §5.1.3 in one place by deriving a file input's RFC-8089 `file://` IRI and passing
//! it in as a caller-supplied base like any other, only when nothing of higher
//! precedence was given. Every failure carries a stable
//! [`IriError::diagnostic_code`], which is the single owner of those strings for the
//! whole workspace.
//!
//! The `Option`-returning surfaces are these, and in each the `None` is a
//! *semantic* answer rather than a degraded failure:
//!
//! * [`expand_curie`] — `None` is "not a CURIE / undeclared prefix", faithful to the
//!   SSSOM behavior this crate subsumes.
//! * [`BaseIri::relativize`] — `None` is "no relative spelling of this target exists
//!   against this base" (different scheme, different authority, or a target outside
//!   the base's dot-normalized image); the caller's correct response is to emit the
//!   absolute IRI, not to raise an error.
//! * [`idna::to_ascii`], [`idna::to_ascii_mapped`], [`idna::punycode_encode`] and
//!   [`idna::punycode_decode`] — `None` is the protocol's own answer "not a
//!   valid name" (or "not Punycode"), each refusal a clause of RFC 3492 or
//!   RFC 5890–5893.
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

mod base;
mod curie;
mod error;
pub mod host;
pub mod idna;
mod idna_tables;
pub mod langtag;
mod normalize;
mod parse;
pub mod pos;
mod resolve;
pub mod vocab;

/// The lexical foundations this crate scans with, re-exported so the paths
/// `purrdf_iri::terminals`, `purrdf_iri::scan` and `purrdf_iri::json_escape`
/// name the same items as their home in [`purrdf_lex`].
pub use purrdf_lex::{json_escape, scan, terminals};

pub use base::{BaseInScope, BaseIri, BaseOrigin, BaseScope, ScopedBase};
pub use curie::{PrefixMap, contract, curie_prefix, expand_curie, resolve};
pub use error::{IriError, Result};
pub use parse::{Iri, is_absolute, parse, parse_uri};
pub use pos::{LineIndex, Position};
