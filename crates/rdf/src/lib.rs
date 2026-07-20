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
// Streamable-compaction certificates (GTS-SPEC §10.1/§10.2): content
// projection + refold digest, `verify_compaction`, `compose`, and
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
/// Deterministic embedding companions bound to exact PurRDF packs.
pub use purrdf_core::embedding;
pub use purrdf_core::embedding::*;
pub mod gts_view;
// The native RDF text codecs (S3): the codec-only `GtsCodecBackend`
// over the `purrdf-gts` Turtle/TriG/NT/NQ/RDF-XML codecs, oxigraph-free.
pub mod native_codecs;
/// Deterministic graph/tabular/research-object projection foundations and codecs.
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
// Shared corpus-classification helpers: the pure corpus
// enumeration / classification helpers the native golden-capture binary
// (src/bin/capture_sparql_goldens.rs) uses. Oxigraph-free.
pub mod capture_support;
// Canonical, review-friendly Turtle serializer over the IR: the
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
pub use native_codecs::jsonld::{
    CompiledJsonLdContext, JSON_LD_SERIALIZE_OPTIONS_VERSION, JsonLdContainer, JsonLdContextLimits,
    JsonLdContextRegistry, JsonLdDirection, JsonLdNullable, JsonLdSerializeMode,
    JsonLdSerializeOptions, JsonLdTermDefinition, JsonLdTermSelection, JsonLdTermSelectionKind,
    JsonLdTypeMapping, derive_jsonld_context, serialize_dataset_to_jsonld,
    serialize_dataset_to_jsonld_with_context, serialize_dataset_to_jsonld_with_options,
    serialize_dataset_to_yamlld, serialize_dataset_to_yamlld_with_context,
    serialize_dataset_to_yamlld_with_options,
};
pub use native_codecs::okf::{
    OkfBundle, OkfConfig, OkfError, OkfReadOutcome, OkfWriteOutcome, OkfWriter, lift_okf_bundle,
    write_okf_bundle,
};
pub use native_codecs::{
    GtsCodecBackend, NativeRdfFormat, ParseOptions, SerializeOutcome, SpanTable, classify,
    parse_dataset, parse_dataset_with, serialize_dataset, serialize_dataset_base_only,
    serialize_dataset_to_format, serialize_dataset_to_format_with_jsonld_options,
    serialize_dataset_with_jsonld_options,
};
pub use native_quads::{
    canonical_flat_nquads, canonical_flat_nquads_with, dataset_from_quad_sources,
    dataset_from_quads, flat_dataset_from_quad_sources, flat_dataset_from_quads,
    flat_rdf_quads_from_dataset,
};
pub use projections::{
    CROISSANT_ARTIFACT, CROISSANT_PROFILE, CROISSANT_ROLES, CSVW_TERMS_PROFILE,
    ConstructViewConfig, ConstructViewProjection, CroissantConfig, CroissantRole,
    CroissantVocabulary, CsvwAction, CsvwAnnotations, CsvwCell, CsvwColumn, CsvwConfig,
    CsvwContext, CsvwDatatype, CsvwDatatypeFormat, CsvwDialect, CsvwExactProjection,
    CsvwExactReadOutcome, CsvwForeignKey, CsvwInheritedProperties, CsvwInput, CsvwMappedTableGroup,
    CsvwMode, CsvwNaturalLanguage, CsvwNumericFormat, CsvwRdfTableMapping, CsvwReadOutcome,
    CsvwReference, CsvwRow, CsvwSchema, CsvwTable, CsvwTableDirection, CsvwTableGroup,
    CsvwTermsCardinality, CsvwTermsColumn, CsvwTermsConfig, CsvwTermsGraphSelection,
    CsvwTermsIdentityColumn, CsvwTermsLimits, CsvwTermsProjection, CsvwTermsReport,
    CsvwTermsSelector, CsvwTermsTable, CsvwTermsValueMode, CsvwTextDirection, CsvwTransformation,
    CsvwTrim, CsvwValue, CsvwVocabulary, CsvwWarning, CsvwWarningKind, CsvwWriteOutcome,
    CsvwWritePlan, DATACITE_ARTIFACT, DATACITE_PROFILE, DCAT_ARTIFACT, DCAT_PROFILE, DCAT_ROLES,
    DataCiteConfig, DataCiteControlledValues, DcatConfig, DcatRdfConfig, DcatRdfMappingConfig,
    DcatRdfSource, DcatRole, DcatVocabulary, FRICTIONLESS_ARTIFACT, FRICTIONLESS_PROFILE,
    FrictionlessConfig, LiftProfile, LpgAnnotation, LpgConfig, LpgEdge, LpgExecutionLimits,
    LpgGraph, LpgGraphContext, LpgIriSelection, LpgLabel, LpgLiftOutcome, LpgNamedGraphSelection,
    LpgNode, LpgPackageProjection, LpgProgress, LpgProgressObserver, LpgProgressPhase,
    LpgProjection, LpgProjectionReport, LpgProperty, LpgPropertyAtom, LpgRdfQuad, LpgReifier,
    LpgScope, LpgStreamProjection, OKF_TERMS_PROFILE, OboDomainRangeAxiom, OboEdge,
    OboEquivalentNodesSet, OboExistentialRestriction, OboGraph, OboGraphDocument, OboGraphsConfig,
    OboGraphsProjection, OboGraphsVocabulary, OboLogicalDefinitionAxiom, OboMeta, OboMetadataRoles,
    OboNode, OboNodeType, OboOwlRoles, OboPropertyChainAxiom, OboPropertyType, OboPropertyValue,
    OboRdfRoles, OboSynonym, OboXref, OfflineJsonLdContext, OkfBodySection, OkfBodyStyle,
    OkfBodyValueMode, OkfCardinality, OkfCategory, OkfConceptSelector, OkfFieldMapping,
    OkfFrontmatterMappings, OkfGenerationConfig, OkfGenerationReport, OkfGraphSelection,
    OkfIndexConfig, OkfLinkPathStyle, OkfLinkSection, OkfLinkStyle, OkfLinkTargetMode,
    OkfPathStrategy, OkfProjection, OkfResourceMapping, OkfTermRendering, OkfValueMode,
    ProjectionArchive, ProjectionArtifactSink, ProjectionConfig, ProjectionDirection,
    ProjectionError, ProjectionErrorKind, ProjectionLift, ProjectionLimits, ProjectionPackage,
    ProjectionPackageSink, ProjectionProfile, ProjectionTerm, RESEARCH_ROLES, RO_CRATE_ARTIFACT,
    RO_CRATE_PREVIEW_ARTIFACT, RO_CRATE_PREVIEW_FILES_PREFIX, RO_CRATE_PROFILE, RO_CRATE_ROLES,
    RdfDescriptionProjection, ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset,
    ResearchField, ResearchObjectConfig, ResearchObjectIdentity, ResearchObjectModel,
    ResearchObjectPackageProjection, ResearchObjectPolicy, ResearchObjectProjection,
    ResearchObjectReadOutcome, ResearchObjectRoles, ResearchRecordSet, ResearchResource,
    ResearchRole, ResearchText, ResearchValue, RoCrateAssets, RoCrateConfig, RoCratePackaging,
    RoCrateRole, RoCrateVocabulary, SkosClassRoles, SkosConfig, SkosDocumentationRoles,
    SkosGraphSelection, SkosLabelRoles, SkosProjection, SkosRelationRoles, SkosSourceRoles,
    SkosTargetRoles, VOID_ROLES, VoidConfig, VoidDatasetPrefix, VoidExecutionLimits,
    VoidExternalLinkMapping, VoidGraphSelector, VoidRole, VoidSourceRoles, VoidStaticStatement,
    VoidStaticValue, VoidVocabulary, escape_cypher_identifier, escape_cypher_string,
    escape_xml_attribute, escape_xml_text, lift_archive, lift_lpg, lift_research_object,
    project_archive, project_archive_with_assets, project_construct_view, project_croissant,
    project_csvw, project_csvw_exact, project_csvw_terms, project_datacite, project_dcat,
    project_dcat_rdf, project_frictionless, project_lpg, project_lpg_artifacts_to_sink,
    project_lpg_csv, project_lpg_csv_to_sink, project_lpg_cypher, project_lpg_cypher_to_sink,
    project_lpg_graphml, project_lpg_graphml_to_sink, project_lpg_with_progress, project_neo4j_csv,
    project_neo4j_csv_to_sink, project_obo_graphs, project_okf_terms, project_research_object,
    project_ro_crate, project_ro_crate_with_assets, project_skos, project_void, read_croissant,
    read_csvw, read_csvw_exact, read_datacite, read_dcat, read_frictionless, read_lpg_csv,
    read_lpg_cypher, read_lpg_graphml, read_neo4j_csv, read_ro_crate, serialize_rdf_description,
    stable_identifier, validate_absolute_iri, write_csvw, write_lpg_csv, write_lpg_csv_to_sink,
    write_lpg_cypher, write_lpg_cypher_to_sink, write_lpg_graphml, write_lpg_graphml_to_sink,
    write_neo4j_csv, write_neo4j_csv_to_sink,
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
    QuadRef, QuadValues, RESEARCH_OBJECT_CODECS, RdfAnnotation, RdfBlobOrigin, RdfBlobRecord,
    RdfBundle, RdfDataset, RdfDatasetBuilder, RdfDatasetVisitor, RdfDiagnostic, RdfEnvelope,
    RdfListError, RdfLiteral, RdfLocation, RdfLookaside, RdfLookasideKind, RdfLookasideResource,
    RdfMetadataEntry, RdfMetadataValue, RdfOpaqueNodeRecord, RdfParseRequest, RdfParserBackend,
    RdfQuad, RdfReifier, RdfSegmentRecord, RdfSerializeRequest, RdfSerializer, RdfSeverity,
    RdfSignatureRecord, RdfStoreCapabilities, RdfSuppressionRecord, RdfTerm, RdfTermKind,
    RdfTextDirection, RdfTriple, SSSOM_DEFAULT_VALIDATION_TYPES, SegmentUnitMap, SerializeGraph,
    SmallVec, SparqlEngine, SparqlRequest, SparqlResult, SssomColumnLayout, SssomColumnLayoutError,
    SssomCommentError, SssomCommentKind, SssomCommentPlacement, SssomDiagnostic, SssomMapping,
    SssomMappingSet, SssomMeta, SssomSetComment, SubsetPageProvider, TermFactory, TermId, TermRef,
    TermValue, UnitCatalog, UnitId, UnitInterner, UnitMetadata, ViewOperationStatus,
    assert_ledger_complete, assert_ledger_sound, canonicalize, canonicalize_with,
    check_ledger_complete, check_ledger_sound, check_provenance, dataset_diff, datasets_isomorphic,
    emit_annotation, emit_quad, emit_reifier, emit_resource, emit_term, fno_to_ntriples,
    fno_to_quads, gts_to_rdf_loss_ledger, loss_matrix_json, lpg_to_rdf_loss_ledger,
    okf_to_rdf_loss_ledger, pair_loss_ledger, profile_for, rdf_gts_loss_matrix_json,
    rdf_to_gts_loss_ledger, rdf_to_lpg_loss_ledger, rdf_to_obo_graphs_loss_ledger,
    rdf_to_okf_loss_ledger, rdf_to_research_object_loss_ledger, rdf_to_skos_loss_ledger,
    registered_pairs, research_object_to_rdf_loss_ledger, rule_iri, smallvec, try_canonicalize,
    try_canonicalize_with,
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
