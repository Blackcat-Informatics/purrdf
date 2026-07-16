// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf` -- PyO3-free RDF 1.2 kernel for the PurRDF Rust workspace.
//!
//! The crate is the narrow waist between transport/runtime stores (GTS and future
//! logic stores) and consumers such as SHACL, validate, and LOGIC. It models RDF 1.2
//! terms directly, preserves source/location context where adapters can provide it,
//! and keeps reporting structured but SARIF-free.
//!
//! # Crate boundary (purrdf P2b)
//!
//! The oxigraph-free, PyO3-free kernel — the immutable IR, the owned value model,
//! diagnostics, dataset capability flags, the loss ledger, provenance, the FnO and
//! SSSOM codecs, the content store, and the GTS reader path — lives in the
//! ring-fenced sibling crate [`purrdf_core`]. `purrdf` **re-exports** every one
//! of those modules at its own root so that both the public `purrdf::…` API and
//! the crate's own internal `crate::…` paths keep resolving unchanged. What remains
//! *here* is the native text/statement/normalize surface ([`native_codecs`],
//! [`native_quads`], [`statements`], [`turtle_normalize`]), the [`gts_compose`]
//! author, and the `flattened_dataset_from_bytes` GTS helper in [`gts`]. The
//! Python bindings live in `bindings/python`, and the last oxigraph adapters
//! have been removed, so the entire crate is oxigraph-free.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

// ---------------------------------------------------------------------------
// Re-exported kernel modules (live in `purrdf-core`). The re-export keeps the
// public `purrdf::ir::…` surface AND this crate's internal `crate::ir::…`
// references resolving against the ring-fenced core, so the oxigraph/py adapters
// below need no path edits.
// ---------------------------------------------------------------------------
pub mod gts_write;
pub use purrdf_core::{
    backend, bundle, content_store, dataset_view, diagnostic, fno, ir, lookaside, loss, model,
    provenance, sssom, store, turtle, turtle_render,
};

pub mod gts;
// Streamable-compaction certificates (GTS-SPEC §10.1/§10.2, Task
// 5): content projection + refold digest, `verify_compaction`, `compose`, and
// the certifying authoring wrapper `compact_and_certify`.
pub mod gts_certify;
mod gts_core;
mod gts_import_graph;
mod gts_import_sink;
mod gts_resolve;
// Full content-chain verification: COSE signatures + expected-head replay +
// digest inclusion, re-exported through the `gts` adapter surface below.
mod gts_verify;
// Per-subject Symmetric-CBD subgraph extraction: the subgraph that *describes* a
// term/slice, used by the docs multi-format export AND by the native engine's DESCRIBE
// evaluation. It lives in `purrdf-core` (pure IR, no codec/gts) so both the higher
// `purrdf` surface and the lower `purrdf-sparql-eval` engine share one CBD authority;
// re-exported here for existing `purrdf::describe::*` callers.
pub use purrdf_core::describe;
pub mod gts_view;
// The native RDF text codecs (S3): the codec-only `GtsCodecBackend`
// over the `purrdf-gts` Turtle/TriG/NT/NQ/RDF-XML codecs, oxigraph-free.
pub mod native_codecs;
/// Deterministic graph/tabular projection foundations and codecs.
pub mod projections;
// Oxigraph-free `RdfQuad` ⇄ `RdfDataset` conversions: the native twins of
// the oxigraph-quad helpers, available to every Rust consumer without pulling the
// oxigraph Store adapter.
pub mod native_quads;
// The PyO3-free GTS snapshot compose core (P6): SnapshotBuilder + emit_gts +
// BlobRow, lifted out of the Python binding surface so purrdf-pipeline can
// author a full multi-named-graph snapshot without pulling pyo3. Oxigraph-free
//.
pub mod dataset_io;
pub mod gts_compose;
// The native OWL ↔ RDF 1.2 statement codec is fully oxigraph-free (it folds over the
// native flat-quad stream).
pub mod statements;
// Shared corpus-classification helpers ( Task 2): the pure corpus
// enumeration / classification helpers the native golden-capture binary
// (src/bin/capture_sparql_goldens.rs) uses. Oxigraph-free.
pub mod capture_support;
// Canonical, review-friendly Turtle serializer over the IR (Task 9): the
// native replacement for rdflib `longturtle` in `purrdf normalize`. Oxigraph-free.
pub mod turtle_normalize;
/// Statement-centric RDF 1.2 visualization projection and SVG export support.
pub mod viz;

// Mirror the kernel's root-level re-exports so `purrdf::RdfTerm`,
// `purrdf::RdfDiagnostic`, … keep resolving exactly as before. The two
// IR import helpers are re-exported here.
pub use dataset_io::dataset_from_bytes;
pub use gts_import_graph::import_gts_graph;
pub use gts_import_sink::import_gts_events;
pub use native_codecs::okf::{
    OkfBundle, OkfConfig, OkfError, OkfReadOutcome, OkfWriteOutcome, OkfWriter, lift_okf_bundle,
    write_okf_bundle,
};
pub use native_codecs::{
    GtsCodecBackend, NativeRdfFormat, ParseOptions, SerializeOutcome, SpanTable, classify,
    parse_dataset, parse_dataset_with, serialize_dataset, serialize_dataset_base_only,
    serialize_dataset_to_format,
};
pub use native_quads::{
    canonical_flat_nquads, canonical_flat_nquads_with, dataset_from_quad_sources,
    dataset_from_quads, flat_dataset_from_quad_sources, flat_dataset_from_quads,
    flat_rdf_quads_from_dataset,
};
pub use projections::{
    CsvwAction, CsvwAnnotations, CsvwCell, CsvwColumn, CsvwConfig, CsvwContext, CsvwDatatype,
    CsvwDatatypeFormat, CsvwDialect, CsvwExactProjection, CsvwExactReadOutcome, CsvwForeignKey,
    CsvwInheritedProperties, CsvwInput, CsvwMappedTableGroup, CsvwMode, CsvwNaturalLanguage,
    CsvwNumericFormat, CsvwRdfTableMapping, CsvwReadOutcome, CsvwReference, CsvwRow, CsvwSchema,
    CsvwTable, CsvwTableDirection, CsvwTableGroup, CsvwTextDirection, CsvwTransformation, CsvwTrim,
    CsvwValue, CsvwVocabulary, CsvwWarning, CsvwWarningKind, CsvwWriteOutcome, CsvwWritePlan,
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgGraphContext, LpgLabel, LpgLiftOutcome,
    LpgNode, LpgPackageProjection, LpgProjection, LpgProperty, LpgPropertyAtom, LpgRdfQuad,
    LpgReifier, OboDomainRangeAxiom, OboEdge, OboEquivalentNodesSet, OboExistentialRestriction,
    OboGraph, OboGraphDocument, OboGraphsConfig, OboGraphsProjection, OboGraphsVocabulary,
    OboLogicalDefinitionAxiom, OboMeta, OboMetadataRoles, OboNode, OboNodeType, OboOwlRoles,
    OboPropertyChainAxiom, OboPropertyType, OboPropertyValue, OboRdfRoles, OboSynonym, OboXref,
    ProjectionDirection, ProjectionError, ProjectionErrorKind, ProjectionLimits, ProjectionPackage,
    ProjectionTerm, escape_cypher_identifier, escape_cypher_string, escape_xml_attribute,
    escape_xml_text, lift_lpg, project_csvw, project_csvw_exact, project_lpg, project_lpg_csv,
    project_lpg_cypher, project_lpg_graphml, project_neo4j_csv, project_obo_graphs, read_csvw,
    read_csvw_exact, read_lpg_csv, read_lpg_cypher, read_lpg_graphml, read_neo4j_csv,
    stable_identifier, validate_absolute_iri, write_csvw, write_lpg_csv, write_lpg_cypher,
    write_lpg_graphml, write_neo4j_csv,
};
pub use purrdf_core::{
    ArtifactId, ArtifactIndex, ArtifactInterner, ArtifactRecord, AssertionOccurrence, Attribution,
    AttributionRole, BlankScope, BudgetExceeded, BundleError, Bytes, CanonHash, Canonicalized,
    ContentDigest, ContentStore, ContentStoreError, DatasetDiff, DatasetMut, DatasetProvenance,
    DatasetSink, DatasetView, FallibleDatasetView, FastHasher, FastMap, FastSet, FnFunction,
    FnImpl, FnMapping, FnOutput, FnParam, FnParamMapping, FnReturnMapping, FnoCatalog,
    FrozenDatasetSource, GraphMatch, GraphMatchValue, GtsBundle, HandleEntry, HandleKey, IdSet,
    IdVec, LossEntry, LossLedger, MutableDataset, OriginKind, OriginSetId, OriginSetInterner,
    PROJECTION_CODECS, PageFault, PageFaultKind, PageGeneration, PageId, PageMaterialization,
    PagePart, PageProvider, PageTranslation, PagedDataset, PagedFreezeError, PagedQuadOverlap,
    PagedQuadTable, PagedQueryError, PagedQueryEvidence, PagedQueryLimits, PagedQueryView,
    PipelineBundle, PipelineBundleError, ProvenanceError, QuadHandle, QuadIds, QuadPatternCursor,
    QuadRef, QuadValues, RdfAnnotation, RdfBlobOrigin, RdfBlobRecord, RdfBundle, RdfDataset,
    RdfDatasetBuilder, RdfDatasetVisitor, RdfDiagnostic, RdfEnvelope, RdfListError, RdfLiteral,
    RdfLocation, RdfLookaside, RdfLookasideKind, RdfLookasideResource, RdfMetadataEntry,
    RdfMetadataValue, RdfOpaqueNodeRecord, RdfParseRequest, RdfParserBackend, RdfQuad, RdfReifier,
    RdfSegmentRecord, RdfSerializeRequest, RdfSerializer, RdfSeverity, RdfSignatureRecord,
    RdfStoreCapabilities, RdfSuppressionRecord, RdfTerm, RdfTermKind, RdfTextDirection, RdfTriple,
    SSSOM_DEFAULT_VALIDATION_TYPES, SegmentUnitMap, SerializeGraph, SmallVec, SparqlEngine,
    SparqlRequest, SparqlResult, SssomDiagnostic, SssomMapping, SssomMappingSet, SssomMeta,
    SubsetPageProvider, TermFactory, TermId, TermRef, TermValue, UnitCatalog, UnitId, UnitInterner,
    UnitMetadata, ViewOperationStatus, assert_ledger_complete, assert_ledger_sound, canonicalize,
    canonicalize_with, check_ledger_complete, check_ledger_sound, check_provenance, dataset_diff,
    datasets_isomorphic, emit_annotation, emit_quad, emit_reifier, emit_resource, emit_term,
    fno_to_ntriples, fno_to_quads, gts_to_rdf_loss_ledger, loss_matrix_json,
    lpg_to_rdf_loss_ledger, okf_to_rdf_loss_ledger, pair_loss_ledger, profile_for,
    rdf_gts_loss_matrix_json, rdf_to_gts_loss_ledger, rdf_to_lpg_loss_ledger,
    rdf_to_obo_graphs_loss_ledger, rdf_to_okf_loss_ledger, rdf_to_skos_loss_ledger,
    registered_pairs, rule_iri, smallvec, try_canonicalize, try_canonicalize_with,
};
pub use purrdf_core::{
    PackBuilder, PackDigest, PackError, PackId, PackView, dataset_from_view, pack_digest,
    restore_pack, verify_pack,
};

// Shared USTAR (tar) codec: byte-deterministic writer + reader used by both the
// snapshot stage (writer) and the validate path (reader). Unconditional — no
// oxigraph or PyO3 dependency.
pub mod ustar;

/// The common purrdf surface, for `use purrdf::prelude::*;`.
///
/// Pulls in the owned value model, the immutable IR + builder, term identity,
/// capability flags, and the diagnostic type — the set a typical consumer reaches
/// for first. Mirrors
/// the ring-fenced kernel's own [`purrdf_core::prelude`].
pub mod prelude {
    pub use purrdf_core::prelude::*;
}
