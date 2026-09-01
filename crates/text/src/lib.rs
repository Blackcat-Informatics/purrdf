// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-text` — deterministic full-text search over RDF 1.2 literals.
//!
//! This crate builds an in-memory inverted index over the literals of a frozen
//! `purrdf-core` dataset and answers ranked retrieval queries against it. Text
//! is put through one pipeline — compatibility normalization plus full Unicode
//! case folding (`UAX #15`, `UAX #21`), then segmentation at Unicode word
//! boundaries (`UAX #29`) — so tokenization is the standard's rather than an
//! ad-hoc run of alphanumeric characters, and it behaves the same in every
//! script. See [`Analyzer`] for why the fold is a fold rather than a
//! lowercasing, and for how a script written without spaces is segmented.
//!
//! The index side and the query side run that same pipeline. They have to: a
//! needle is matched against the dictionary by equality, so two pipelines that
//! merely resemble each other produce a search that returns nothing and reports
//! nothing.
//!
//! # It mints no vocabulary
//!
//! Ranked retrieval reaches SPARQL through the evaluator's property-function
//! seam, and the predicate IRIs a query calls it by are **caller-supplied
//! configuration**. PurRDF is not an ontology: there is no namespace of this
//! project's own, no default IRI, and no fabricated fallback for a caller who
//! supplies none. A configuration with no IRI is a [`TextError::Config`], not a
//! guess — the same rule that governs every other configurable vocabulary in
//! this workspace. Which predicates' literals are indexed is likewise the
//! caller's decision.
//!
//! # Every score is exact, and identical on every target
//!
//! Ranking is done entirely in base-10 fixed-point integer arithmetic
//! ([`Fixed`], [`SCALE_DIGITS`] fractional digits). No floating-point value
//! enters this crate; the crate root denies `clippy::float_arithmetic`, so none
//! can.
//!
//! That is a correctness requirement rather than a preference. BM25 needs a
//! natural logarithm, and a libm `ln` may differ by a unit in the last place
//! between implementations — which is enough to reverse the order of two
//! near-tied documents. The same query over the same data would then return rows
//! in one order from a native build and another from a
//! `wasm32-unknown-unknown` build of the same engine: an answer divergence, and
//! one nothing downstream could detect. [`Fixed::ln`] is instead a fixed-length
//! integer series — a fixed iteration count, never a convergence test — so its
//! result is a pure function of its input on every target, and a ranking is
//! reproducible byte for byte.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

mod analysis;
mod error;
mod fixed;
mod index;
mod term_bytes;

pub use analysis::{Analyzer, Token, UnicodeVersion, UnicodeVersions, unicode_versions};
pub use error::TextError;
pub use fixed::{Fixed, SCALE_DIGITS};
pub use index::{
    Document, GraphSelector, PartitionKey, PartitionStats, TextIndex, TextIndexConfig,
};
pub use term_bytes::{FINGERPRINT_BYTES, fingerprint_terms};
