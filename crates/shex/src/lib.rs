// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-shex` — the native **ShEx 2.1** engine for PurRDF: the schema
//! layer and the shape-map validator.
//!
//! A pure-Rust, wasm-clean leaf crate implementing the Shape Expressions
//! language, 2.1 (<https://shex.io/shex-semantics/>):
//!
//! * [`parse_shexc`] — a hand-rolled lexer + recursive-descent parser for the
//!   compact syntax (spec §6): directives, `start`, shape declarations, the
//!   `AND`/`OR`/`NOT` algebra, node constraints and facets, value sets with
//!   stems/ranges/exclusions, triple expressions with every cardinality form,
//!   `$`/`&` labels and inclusions, `^` inverse, annotations and semantic
//!   actions, with relative-IRI resolution against `BASE` via `purrdf-iri`.
//! * [`parse_shexj`] / [`to_shexj`] — the ShExJ JSON wire format (spec
//!   Appendix A), byte-compatible with the `shexTest` ground-truth corpus.
//! * [`check_structure`] — the spec §5.7 structural requirements (dangling
//!   references, label collisions, reference-only cycles, the negation
//!   requirement), what the `negativeStructure` suite exercises.
//! * [`validate()`] — the shape-map validator (spec §5.2–§5.5): fixed
//!   `(node, shape)` associations checked over a frozen
//!   `purrdf_core::RdfDataset` in interned `TermId` space, with node
//!   constraints, `EXTRA`/`CLOSED` triple-expression matching, and
//!   typing-based recursion; gated against the shexTest `validation/`
//!   manifest by `tests/validation_conformance.rs`.
//!
//! # Hard-fail
//!
//! Per the repo `no-optionality` doctrine, every malformed schema is a typed
//! [`ShexError`] / [`StructureError`]; there is no lenient mode and no panic
//! on any input (parsers are fuzz-safe and depth-bounded).
//!
//! # Conformance
//!
//! `tests/syntax_conformance.rs` runs the vendored shexTest v2.1.0 corpus
//! (`vectors/shexTest`): every `negativeSyntax/` document must fail
//! [`parse_shexc`], every `negativeStructure/` document must parse and fail
//! [`check_structure`], and every `schemas/` pair must parse in both
//! syntaxes; `tests/shexj_roundtrip.rs` proves `parse_shexj → to_shexj →
//! parse_shexj` is the identity on the corpus;
//! `tests/validation_conformance.rs` runs the full `validation/` manifest
//! (with an exact-count trait skip list and xfail ledger).

#![forbid(unsafe_code)]

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod shexj;
pub mod structure;
pub mod validate;

pub use ast::{
    Annotation, IriExclusion, LanguageExclusion, LiteralExclusion, NodeConstraint, NodeKind,
    NumericLiteral, ObjectLiteral, ObjectValue, Schema, SemAct, Shape, ShapeDecl, ShapeExpr,
    ShapeLabel, StemValue, TripleConstraint, TripleExpr, TripleExprGroup, ValueSetValue,
};
pub use error::{Result, ShexError};
pub use parser::parse_shexc;
pub use shexj::{parse_shexj, to_shexj};
pub use structure::{check_structure, StructureError};
pub use validate::{
    validate, validate_with, ConformanceStatus, ExternalResolver, ResultEntry, ResultShapeMap,
    ShapeSelector, ValidationOptions,
};
