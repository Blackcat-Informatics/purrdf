// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-jsonschema` — native JSON Schema validation: draft 2020-12, draft
//! 2019-09 and draft-07.
//!
//! An implementation of three dialects, each read by its own rules, per
//! schema resource: a 2020-12 schema may `$ref` a 2019-09 or draft-07 one, and
//! the referenced resource is evaluated as its own `$schema` says.
//!
//! * **Draft 2020-12**: every keyword of the Core, Applicator, Unevaluated,
//!   Validation, Meta-Data, Format-Annotation and Content vocabularies, and
//!   the Format-Assertion vocabulary; `$ref`, `$dynamicRef` and
//!   `$dynamicAnchor` with the dynamic scope; `prefixItems`.
//! * **Draft 2019-09**: every keyword of its Core, Applicator, Validation,
//!   Meta-Data, Format and Content vocabularies; `$recursiveRef` and
//!   `$recursiveAnchor` with the dynamic scope; array-form `items` with
//!   `additionalItems`; `unevaluatedItems` and `unevaluatedProperties` with
//!   2019-09's annotations (`contains` evaluates no items).
//! * **Draft-07**: `$ref` overriding its siblings, `$id` plain-name fragments
//!   as anchors, `definitions`, `dependencies`, array-form `items` with
//!   `additionalItems`, and the content assertions it permits (`base64` and
//!   JSON media types, see below).
//!
//! In every dialect: `unevaluatedItems` and `unevaluatedProperties` (where
//! the dialect has them) over annotations collected through every in-place
//! applicator; `$vocabulary` in custom meta-schemas (2019-09, 2020-12); format
//! assertion for every format but the three whose complete check needs
//! IDNA2008 tables (`hostname`, `idn-hostname`, `idn-email`, refused as
//! assertions); and the `flag`, `basic` and `detailed` output formats. It is
//! checked against the official JSON-Schema-Test-Suite for all three drafts,
//! optional tests included, and passes every case.
//!
//! It depends on `serde_json`, `regex` and `purrdf-iri` only, runs on
//! `wasm32-unknown-unknown`, and needs no network: the three dialects'
//! meta-schemas are vendored and registered in every [`Registry`].
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
//! * **Three dialects.** A `$schema` naming any other published dialect
//!   (draft-04, -06, …) is refused with [`SchemaError::UnsupportedDialect`] —
//!   at compile time, including when a `$ref` reaches such a document. A
//!   custom meta-schema is accepted when it is itself a schema of an
//!   implemented dialect, and its `$vocabulary` decides which keywords are in
//!   force; a *required* vocabulary this crate does not implement is
//!   [`SchemaError::UnsupportedVocabulary`]. A schema with no `$schema` is
//!   2020-12, or whatever [`Registry::set_default_dialect`] chose.
//! * **Numbers are exact.** `1` and `1.0` are equal and both integers;
//!   `multipleOf` divides in decimal, so `0.0075` is a multiple of `0.0001`.
//! * **Patterns are ECMA-262** with the `u` flag, translated by [`ecma`]; see
//!   there for the three constructs refused rather than approximated.
//! * **`format` is an annotation** unless the meta-schema declares the
//!   2020-12 Format-Assertion vocabulary, or requires the 2019-09 Format
//!   vocabulary; then it asserts, and a format this crate cannot check
//!   completely is [`SchemaError::UnsupportedFormat`]. Draft-07 `format` is an
//!   annotation.
//! * **Content is an annotation**, except in draft-07, which lets an
//!   implementation assert it: there `contentEncoding: base64` must decode
//!   (RFC 4648 §4) and a JSON `contentMediaType` must parse (RFC 8259); other
//!   encodings and media types stay annotations.
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
mod content;
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
pub use registry::{DRAFT_07, DRAFT_2019_09, DRAFT_2020_12, Registry};
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
    /// outside itself and the vendored meta-schemas.
    pub fn from_document(uri: &str, document: serde_json::Value) -> Result<Self, SchemaError> {
        let mut registry = Registry::new();
        registry.add_resource(uri, document)?;
        registry.compile(uri)
    }
}
