// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-jsonschema` — native JSON Schema draft 2020-12 validation.
//!
//! An implementation of the 2020-12 dialect: every keyword of the Core,
//! Applicator, Unevaluated, Validation, Meta-Data, Format-Annotation and
//! Content vocabularies, and the Format-Assertion vocabulary for every format
//! but the three whose complete check needs IDNA2008 tables (`hostname`,
//! `idn-hostname`, `idn-email`, refused as assertions); `$ref`, `$dynamicRef` and
//! `$dynamicAnchor` with the dynamic scope; `unevaluatedItems` and
//! `unevaluatedProperties` over annotations collected through every in-place
//! applicator; `$vocabulary` in custom meta-schemas; and the `flag`, `basic`
//! and `detailed` output formats. It is checked against the official
//! JSON-Schema-Test-Suite for draft 2020-12, optional tests included, and
//! passes every case but one: the case that asks for a draft 2019-09 document
//! to be evaluated, which this crate refuses by design (see below).
//!
//! It depends on `serde_json`, `regex` and `purrdf-iri` only, runs on
//! `wasm32-unknown-unknown`, and needs no network: the 2020-12 meta-schemas
//! are vendored and registered in every [`Registry`].
//!
//! # Example
//!
//! ```
//! use purrdf_jsonschema::{OutputFormat, Registry};
//! use serde_json::json;
//!
//! let mut registry = Registry::new();
//! registry.add_resource(
//!     "https://example.org/person.json",
//!     json!({
//!         "$schema": "https://json-schema.org/draft/2020-12/schema",
//!         "type": "object",
//!         "properties": {"name": {"type": "string"}},
//!         "required": ["name"],
//!         "unevaluatedProperties": false
//!     }),
//! )?;
//! let schema = registry.compile("https://example.org/person.json")?;
//!
//! assert!(schema.is_valid(&json!({"name": "Ada"})));
//! assert!(!schema.is_valid(&json!({"name": "Ada", "age": 36})));
//!
//! let output = schema.evaluate(&json!({"age": 36}));
//! let basic = output.to_json(OutputFormat::Basic);
//! assert_eq!(basic["valid"], false);
//! assert!(basic["errors"].as_array().is_some_and(|errors| !errors.is_empty()));
//! # Ok::<(), purrdf_jsonschema::SchemaError>(())
//! ```
//!
//! # Semantics worth knowing
//!
//! * **One dialect.** A `$schema` naming any other published dialect
//!   (draft-04, -06, -07, 2019-09, …) is refused with
//!   [`SchemaError::UnsupportedDialect`] — at compile time, including when a
//!   `$ref` reaches such a document. A custom meta-schema is accepted when it
//!   is itself a 2020-12 schema, and its `$vocabulary` decides which keywords
//!   are in force; a *required* vocabulary this crate does not implement is
//!   [`SchemaError::UnsupportedVocabulary`]. A schema with no `$schema` is
//!   2020-12.
//! * **Numbers are exact.** `1` and `1.0` are equal and both integers;
//!   `multipleOf` divides in decimal, so `0.0075` is a multiple of `0.0001`.
//! * **Patterns are ECMA-262** with the `u` flag, translated by [`ecma`]; see
//!   there for the three constructs refused rather than approximated.
//! * **`format` is an annotation** unless the meta-schema declares the
//!   Format-Assertion vocabulary; then it asserts, and a format this crate
//!   cannot check completely is [`SchemaError::UnsupportedFormat`].
//! * **Unknown keywords are annotations** carrying their value, as are the
//!   keywords of a vocabulary the meta-schema does not declare.
//! * **Schemas are checked against their meta-schema** when compiled; a
//!   schema that fails is [`SchemaError::InvalidSchema`].
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

mod compile;
pub mod ecma;
mod equal;
mod error;
mod format;
mod number;
mod output;
mod pointer;
mod registry;
mod schema;
mod validate;

pub use error::SchemaError;
pub use output::{Output, OutputFormat, OutputUnit};
pub use registry::{DRAFT_2020_12, Registry};
pub use schema::Schema;

impl Registry {
    /// Compile the schema at the absolute URI `uri`. A fragment selects a
    /// subschema, by JSON Pointer (`…#/$defs/name`) or anchor (`…#name`).
    pub fn compile(&self, uri: &str) -> Result<Schema, SchemaError> {
        compile::compile(self, uri)
    }
}

impl Schema {
    /// Compile a single document registered under `uri` in a fresh
    /// [`Registry`] — the shorthand for a schema that references nothing
    /// outside itself and the 2020-12 meta-schemas.
    pub fn from_document(uri: &str, document: serde_json::Value) -> Result<Self, SchemaError> {
        let mut registry = Registry::new();
        registry.add_resource(uri, document)?;
        registry.compile(uri)
    }
}
