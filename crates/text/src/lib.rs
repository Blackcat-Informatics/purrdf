// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-text` — deterministic full-text search over RDF 1.2 literals.
//!
//! This crate builds an in-memory inverted index over the literals of a frozen
//! `purrdf-core` dataset and answers ranked retrieval queries against it. Text
//! is put through one explicit resolved analyzer: compatibility caseless Unicode
//! normalization, script-scoped accents, costed dictionary segmentation and
//! optional English stemming. Atomic emoji, pre-stem surface words and
//! punctuation-bearing substring spans retain original-source evidence.
//! The standard law requires five caller-supplied baseline lexicon artifacts;
//! an empty lexicon is an explicit profile. See [`Analyzer`] and [`AnalyzerProfile`].
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
//! result is a pure function of its input on every target.
//!
//! Native Rust pins the **ranking** — row order and every score's decimal lexical —
//! against independent expectations. It also asserts byte identity of the
//! **serialized** answer from two independently built indexes queried through the
//! property-function seam. `make wasm` separately builds this release crate;
//! WASM execution is reserved for actual target paths and host interfaces.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

mod analysis;
pub mod character;
pub use character::{HanCharacterIndex, HanMatchEvidence};
mod error;
mod fixed;
mod index;
pub mod phonetic;
mod profile;
mod ranking;
mod relation;
mod score;
pub mod segment;
pub mod stem;
mod surface;
mod term_bytes;
pub mod unicode;
mod unicode_tables;

pub use analysis::{
    AlignedText, Analysis, Analyzer, AnalyzerScratch, Projection, Token, UnicodeVersion,
    UnicodeVersions, unicode_versions,
};
pub use error::TextError;
pub use fixed::{Fixed, SCALE_DIGITS};
pub use index::{
    Document, GraphSelector, PartitionKey, PartitionStats, SourceCoverage, TextIndex,
    TextIndexConfig,
};
pub use profile::{
    AccentFold, AccentScripts, AnalyzerProfile, InputMode, MAX_TOKEN_SCALARS, Segmentation,
    Stemming,
};
pub use relation::{
    SearchObservations, TermOccurrenceRelation, TextSearchRelation, verify_binding,
};
pub use score::{
    B, Constraint, K1, PartitionFilter, Scored, TermContribution, explain, rank_partition, select,
};
pub use surface::{
    MatchProjection, PhoneticMatch, SourceMatchEvidence, SubstringLimits, SubstringMatch,
    SubstringRefusal, SubstringRefusalReason, SubstringReport, SubstringWork, SurfaceIndex,
    SurfaceTerm,
};
pub use term_bytes::{FINGERPRINT_BYTES, fingerprint_terms};

/// Profiled analyzer identity. Its fingerprint binds every stage, bound,
/// dictionary and Unicode version across lexical and surface retrieval.
pub const ANALYZER_PROFILE_ID: &str = "purrdf-profiled-text-v4";

pub use ranking::{
    BoundedScore, FieldInput, INDEX_CORPUS_PROFILE_ID, PreparedCorpus, PreparedQuery,
    RANKING_PROFILE_ID, RANKING_PROFILE_VERSION, RankingField, RankingProfile, ScoreBound,
};
