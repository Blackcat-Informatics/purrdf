// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-shapes` — the Rust SHACL Core validator for purrdf.
//!
//! Validates a PurRDF RDF 1.2 data graph against a SHACL shapes graph with
//! NO inference (parity with pySHACL `inference="none"`). The engine core is
//! PyO3-free and oxigraph-free, so the rlib links into any Rust consumer over
//! the interned `purrdf-core` IR. SHACL-SPARQL constraints
//! (`sh:sparql`/`sh:SPARQLConstraint`) and targets (`sh:SPARQLTarget`) are
//! implemented in the [`sparql`] module on the native `purrdf-sparql-eval`
//! engine.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

pub(crate) mod components;
pub mod constraints;
pub mod data;
pub mod engine;
pub mod expression;
pub mod instance;
pub mod json_schema;
pub mod linkml;
pub mod model;
pub mod path;
pub(crate) mod prebinding;
pub mod pydantic;
pub mod report;
pub mod rules;
mod schema_catalog;
pub mod shape_union;
pub mod shapes;
pub mod sparql;
pub mod term;
pub mod text_ingest;

pub use json_schema::{Namespaces, ValueVocab, ValueVocabProjection, compile_with_value_vocab};
pub use linkml::{
    LinkmlConfig, LinkmlDocument, LinkmlError, LinkmlPackage, emit_linkml, parse_linkml,
    write_linkml,
};
pub use pydantic::{PydanticConfig, PydanticError, PydanticPackage, emit_pydantic};
pub use rules::{apply_rules, entail_dataset};

/// Crate version string for cache/toolchain salt parity with Python package
/// versions (`metadata.version("purrdf-shapes")`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
