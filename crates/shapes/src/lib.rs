// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-shapes` — the Rust SHACL Core validator for purrdf.
//!
//! Validates an oxigraph RDF 1.2 data graph against a SHACL shapes graph with
//! NO inference (parity with pySHACL `inference="none"`). The engine core is
//! PyO3-free so the rlib links into the future Rust compiler over its own Store.
//! SHACL-AF SPARQL-based constraints (`sh:sparql`/`sh:SPARQLConstraint`) and
//! targets (`sh:SPARQLTarget`) are implemented in the [`sparql`] module,
//! delegated to oxigraph's SPARQL 1.1 engine.

pub(crate) mod components;
pub mod constraints;
pub mod data;
pub mod engine;
pub mod expression;
pub mod instance;
pub mod json_schema;
pub mod model;
pub mod path;
pub(crate) mod prebinding;
pub mod report;
pub mod shape_union;
pub mod shapes;
pub mod sparql;
pub mod term;
pub mod text_ingest;

pub use json_schema::Namespaces;

/// Crate version string for cache/toolchain salt parity with Python package
/// versions (`metadata.version("purrdf-shapes")`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
