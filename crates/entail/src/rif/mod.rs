// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RIF-Core rule-entailment engine.
//!
//! Two layers compose here: [`model`] is the triple-shaped Horn rule model
//! (facts, atoms, rules, rule sets) that a RIF-in-XML reader produces, and
//! [`eval`] is the deterministic semi-naive forward-chaining evaluator that
//! materializes a rule set over an [`RdfDataset`](purrdf_core::RdfDataset) to its
//! least fixpoint.
//!
//! The engine is pure `purrdf-core`, `wasm32`-clean, and mints no vocabulary: the
//! RIF frame `o[p->v]` is simply the RDF triple `(o, p, v)`, so no RIF-specific
//! IRIs are fabricated. It covers the monotonic definite-Horn fragment (no
//! built-ins, negation, membership, or `External`) that the RIF SPARQL-entailment
//! conformance cases exercise.

pub mod eval;
pub mod model;

pub use eval::materialize_rif;
pub use model::{Atom, Fact, RifTerm, Rule, RuleSet};
