// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-jsonschema` — native JSON Schema validation for drafts 2020-12,
//! 2019-09 and 07.
//!
//! Every schema resource is evaluated in the dialect its `$schema` names
//! ([`Dialect`]), and a `$ref` from one dialect into another reads the
//! target by its own rules:
//!
//! * **2020-12** — every keyword of the Core, Applicator, Unevaluated,
//!   Validation, Meta-Data, Format and Content vocabularies; `$dynamicRef` and
//!   `$dynamicAnchor` with the dynamic scope; `$vocabulary` in custom
//!   meta-schemas.
//! * **2019-09** — its vocabularies: `$recursiveRef` and `$recursiveAnchor`,
//!   array-form `items` with `additionalItems`, `unevaluatedItems` and
//!   `unevaluatedProperties` (which `contains` does not feed), `$anchor`,
//!   `$vocabulary`.
//! * **draft-07** — its keyword set: `definitions`, `dependencies`, array-form
//!   `items` with `additionalItems`, `$ref` overriding its siblings, plain-name
//!   `$id` fragments, and `contentEncoding` (base64) / `contentMediaType`
//!   (`application/json`) as assertions.
//!
//! Output is available as the `flag`, `basic` and `detailed` formats. The
//! crate is checked against the official JSON-Schema-Test-Suite for all three
//! drafts, `optional/` and `optional/format/` included, with no case ignored.
//! It depends on `serde_json`, `regex`, `purrdf-iri` and `purrdf-hash` only, runs on
//! `wasm32-unknown-unknown`, and needs no network.
//!
//! # Meta-schemas are the caller's
//!
//! No meta-schema document is compiled into this crate. Every compiled
//! document is checked against its meta-schema, so the caller registers the
//! published meta-schemas of the dialects it uses — once, as a shared
//! [`Metaschemas`] set — or a custom meta-schema with
//! [`Registry::add_resource`]. A meta-schema that is needed and not
//! registered is [`SchemaError::MissingMetaschema`], naming it. The dialects'
//! identifiers, vocabularies and keyword semantics are code.
//!
//! # Example
//!
//! ```
//! use purrdf_jsonschema::{Metaschemas, OutputFormat, Registry, SchemaError};
//! use serde_json::{Value, json};
//!
//! # fn draft_2020_12() -> Vec<(&'static str, Value)> {
//! #     purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
//! #         .iter()
//! #         .map(|&(uri, text)| (uri, serde_json::from_str(text).unwrap()))
//! #         .collect()
//! # }
//! // The nine published draft 2020-12 meta-schema documents, as
//! // `(URI, document)` pairs, from wherever the application keeps them.
//! let metaschemas = Metaschemas::new(draft_2020_12())?;
//!
//! let mut registry = Registry::with_metaschemas(&metaschemas);
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
//! assert!(schema.is_valid(&json!({"name": "Ada"})).expect("evaluation"));
//! assert!(!schema.is_valid(&json!({"name": "Ada", "age": 36})).expect("evaluation"));
//!
//! let output = schema.evaluate(&json!({"age": 36})).expect("evaluation");
//! let basic = output.to_json(OutputFormat::Basic);
//! assert_eq!(basic["valid"], false);
//! assert!(basic["errors"].as_array().is_some_and(|errors| !errors.is_empty()));
//!
//! // Without the meta-schemas, the document cannot be checked:
//! let mut bare = Registry::new();
//! bare.add_resource("https://example.org/s.json", json!({"type": "string"}))?;
//! assert!(matches!(
//!     bare.compile("https://example.org/s.json"),
//!     Err(SchemaError::MissingMetaschema { .. })
//! ));
//! # Ok::<(), SchemaError>(())
//! ```
//!
//! # Semantics worth knowing
//!
//! * **Three dialects.** A `$schema` naming draft-06 or earlier, or an
//!   unreleased successor of 2020-12, is refused with
//!   [`SchemaError::UnsupportedDialect`] — at compile time, including when a
//!   `$ref` reaches such a document. A custom meta-schema is accepted when it
//!   is itself written in a supported dialect, and its `$vocabulary` decides
//!   which keywords are in force; a *required* vocabulary this crate does not
//!   implement is [`SchemaError::UnsupportedVocabulary`]. A schema with no
//!   `$schema` is read in the registry's default dialect (2020-12 unless
//!   [`Registry::set_default_dialect`] says otherwise). A 2019-09
//!   `$recursiveRef` other than `"#"` is refused.
//! * **Numbers are exact.** `1` and `1.0` are equal and both integers;
//!   `multipleOf` divides in decimal, so `0.0075` is a multiple of `0.0001`.
//! * **Patterns are ECMA-262** with the `u` flag. [`ecma`] uses `regex` for
//!   regular patterns and a bounded explicit-stack matcher for lookaround,
//!   backreferences and scoped modifiers. Unicode properties use vendored
//!   Unicode 17 ranges. [`Schema::is_valid`] and [`Schema::evaluate`] return
//!   [`EvaluationError`] if matching exhausts its resource budget.
//! * **Depth costs memory, not stack.** Subschemas in progress live on a heap
//!   work stack, and `const`, `enum` and `uniqueItems` compare and hash values
//!   on one too, so an instance of any nesting depth is evaluated without
//!   growing the thread's stack. More than [`MAX_REF_CHAIN`] references
//!   followed in a row at one instance location stops with an
//!   [`EvaluationError`] naming the [`EvaluationCause`], never a guessed
//!   verdict.
//! * **`format` is an annotation** by default, as every supported draft
//!   specifies. [`Registry::set_format_assertion`] makes every format the
//!   dialect defines assert — `hostname`, `idn-hostname` and `idn-email`
//!   through the IDNA2008 implementation in `purrdf_iri::idna` — and a
//!   2020-12 meta-schema declaring the Format-Assertion vocabulary, or a
//!   2019-09 one requiring the Format vocabulary, does the same; under the
//!   Format-Assertion vocabulary a format name no draft defines is
//!   [`SchemaError::UnsupportedFormat`].
//! * **Unknown keywords are annotations** carrying their value, as are the
//!   keywords of a vocabulary the meta-schema does not declare and the
//!   keywords of other dialects.
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
mod dialect;
pub mod ecma;
mod equal;
mod error;
mod format;
mod meta_set;
mod number;
mod output;
mod pointer;
mod registry;
mod schema;
mod validate;

pub use dialect::Dialect;
pub use error::{EvaluationCause, EvaluationError, SchemaError};
pub use meta_set::Metaschemas;
pub use output::{Output, OutputFormat, OutputUnit};
pub use registry::{DRAFT_2020_12, Registry};
pub use schema::Schema;
pub use validate::MAX_REF_CHAIN;

impl Registry {
    /// Compile the schema at the absolute URI `uri`. A fragment selects a
    /// subschema, by JSON Pointer (`…#/$defs/name`) or anchor (`…#name`).
    pub fn compile(&self, uri: &str) -> Result<Schema, SchemaError> {
        compile::compile(self, uri)
    }
}

impl Schema {
    /// Compile a single document registered under `uri` in a registry that
    /// holds `metaschemas` — the shorthand for a schema that references
    /// nothing outside itself and the meta-schemas.
    ///
    /// The set's documents and compiled meta-validators are shared, so
    /// calling this repeatedly with one set parses and compiles only
    /// `document`.
    pub fn from_document(
        metaschemas: &Metaschemas,
        uri: &str,
        document: serde_json::Value,
    ) -> Result<Self, SchemaError> {
        let mut registry = Registry::with_metaschemas(metaschemas);
        registry.add_resource(uri, document)?;
        registry.compile(uri)
    }
}
