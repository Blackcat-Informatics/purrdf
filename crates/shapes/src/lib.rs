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
//!
//! The crate also owns the bidirectional schema boundary. [`compile_schema`]
//! explicitly selects shaped-only or ontology-complete developer-schema
//! projection and returns a deterministic coverage manifest plus cache key.
//! [`import_json_schema`] and [`import_linkml`] read native documents;
//! [`import_pydantic_package`], [`import_typescript_package`], and
//! [`import_graphql_package`] verify intact PurRDF-generated packages before
//! lowering through the same deterministic schema-import engine. Every reader
//! requires caller-owned namespace and datatype configuration and returns an
//! always-computed reverse loss ledger.
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
pub mod graphql;
pub mod instance;
pub mod json_schema;
pub mod linkml;
pub mod model;
pub(crate) mod parallel;
pub mod path;
pub(crate) mod prebinding;
pub mod pydantic;
pub mod report;
pub mod rules;
mod schema_catalog;
pub mod schema_import;
mod schema_surface;
pub mod shape_union;
pub mod shapes;
pub mod sparql;
pub mod term;
pub mod text_ingest;
pub mod typescript;

pub use graphql::{
    GRAPHQL_DIALECT, GRAPHQL_NAME_MAP_PATH, GRAPHQL_SCHEMA_PATH, GraphqlConfig,
    GraphqlDefinitionMap, GraphqlEnumValueMap, GraphqlError, GraphqlNameMap, GraphqlPackage,
    emit_graphql, import_graphql_package,
};
pub use json_schema::{
    Namespaces, SchemaClassPropertyCoverage, SchemaCompilation, SchemaCompilationInput,
    SchemaCompilationKey, SchemaCompileError, SchemaCompileRequest, SchemaCoveragePrecision,
    SchemaCoverageProvenance, SchemaCoverageReport, SchemaCoverageStatus, SchemaPropertyCoverage,
    SchemaSurfaceMode, ValueVocab, ValueVocabProjection, compile_schema, compile_with_value_vocab,
};
pub use linkml::{
    LinkmlConfig, LinkmlDocument, LinkmlError, LinkmlPackage, LinkmlSlotDiagnostic,
    LinkmlSlotDisposition, LinkmlSlotReason, LinkmlSlotRename, SanitizePolicy, emit_linkml,
    import_linkml, import_linkml_package, parse_linkml, write_linkml,
};
pub use pydantic::{
    PYDANTIC_DIALECT, PydanticClassConfig, PydanticConfig, PydanticError, PydanticModuleConfig,
    PydanticPackage, PydanticPackageTopology, PydanticVersionStamp, emit_pydantic,
    import_pydantic_package,
};
pub use rules::{apply_rules, entail_dataset};
pub use schema_import::{
    ImportedShapes, SchemaDatatypeMap, SchemaImportConfig, SchemaImportError,
    import_compiled_schema, import_json_schema,
};
pub use typescript::{
    TYPESCRIPT_DECLARATION_PATH, TYPESCRIPT_DIALECT, TypeScriptConfig, TypeScriptError,
    TypeScriptPackage, emit_typescript, import_typescript_package,
};

/// Crate version string for cache/toolchain salt parity with Python package
/// versions (`metadata.version("purrdf-shapes")`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
