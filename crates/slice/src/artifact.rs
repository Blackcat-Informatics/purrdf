// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed artifact inventory: roles, records, and content-addressed digests.

/// The role (kind) of a file artifact within a slice.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ArtifactRole {
    /// The required `manifest.ttl` describing the slice.
    Manifest,
    /// The optional `module.ttl` ontology module.
    Module,
    /// The optional `shapes.ttl` SHACL shapes.
    Shapes,
    /// A file under `mappings/`.
    Mapping,
    /// A SPARQL competency query under `queries/competency/`.
    CompetencyQuery,
    /// A SPARQL verification query under `queries/verify/`.
    VerifyQuery,
    /// A test DSL file under `tests/` (excluding counter-examples).
    TestDsl,
    /// An example file under `examples/`.
    Example,
    /// A counter-example file under `tests/counter-examples/`.
    CounterExample,
    /// The `docs.md` documentation file.
    Documentation,
    /// A translation catalog under `i18n/`.
    TranslationCatalog,
    /// The `CITATION.cff` citation metadata.
    Citation,
    /// Any file not matched by the above roles (forward-compat open variant).
    Other(String),
}

/// A single artifact within a slice: role, path, MIME type, and digests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactRecord {
    /// What kind of file this is.
    pub role: ArtifactRole,
    /// Normalized, relative path within the slice directory (no `..`, no leading `/`).
    pub logical_path: String,
    /// MIME type (e.g. `"text/turtle"`, `"application/sparql-query"`, `"text/markdown"`).
    pub media_type: String,
    /// SHA-256 hex digest of the raw file bytes (64 lowercase hex chars).
    pub raw_digest: String,
    /// For RDF artifacts: SHA-256 hex of the canonical N-Triples (sorted).
    /// `None` for non-RDF files.
    pub semantic_digest: Option<String>,
    /// The raw bytes of the artifact (content cache).
    pub content: Vec<u8>,
}
