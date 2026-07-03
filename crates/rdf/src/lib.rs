// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf` -- PyO3-free RDF 1.2 kernel for the PurRDF Rust workspace.
//!
//! The crate is the narrow waist between transport/runtime stores (GTS and future
//! logic stores) and consumers such as SHACL, validate, and LOGIC. It models RDF 1.2
//! terms directly, preserves source/location context where adapters can provide it,
//! and keeps reporting structured but SARIF-free.
//!
//! # Crate boundary (#885 / purrdf P2b)
//!
//! The oxigraph-free, PyO3-free kernel — the immutable IR, the owned value model,
//! diagnostics, dataset capability flags, the loss ledger, provenance, the FnO and
//! SSSOM codecs, the content store, and the GTS reader path — lives in the
//! ring-fenced sibling crate [`purrdf_core`]. `purrdf` **re-exports** every one
//! of those modules at its own root so that both the public `purrdf::…` API and
//! the crate's own internal `crate::…` paths keep resolving unchanged. What remains
//! *here* is the native text/statement/normalize surface ([`native_codecs`],
//! [`native_quads`], [`statements`], [`turtle_normalize`]), the [`gts_compose`]
//! author, the `flattened_dataset_from_bytes` GTS helper in [`gts`], and the
//! the Python bindings now live in `bindings/python`. EPIC #906 removed the last
//! oxigraph adapters, so the entire crate is now oxigraph-free.

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
mod gts_core;
mod gts_import_graph;
mod gts_import_sink;
mod gts_resolve;
// Full content-chain verification (content-addressed terms task 7): COSE
// signatures + expected-head replay + digest inclusion, re-exported through the
// `gts` adapter surface below.
mod gts_verify;
// Per-subject Symmetric-CBD subgraph extraction: the subgraph that *describes* a
// term/slice, used by the docs multi-format export AND by the native engine's DESCRIBE
// evaluation. It lives in `purrdf-core` (pure IR, no codec/gts) so both the higher
// `purrdf` surface and the lower `purrdf-sparql-eval` engine share one CBD authority;
// re-exported here for existing `purrdf::describe::*` callers.
pub use purrdf_core::describe;
pub mod gts_view;
// The native RDF text codecs (#909 / EPIC #906 S3): the codec-only `GtsCodecBackend`
// over the `purrdf-gts` Turtle/TriG/NT/NQ/RDF-XML codecs, oxigraph-free.
pub mod native_codecs;
// Oxigraph-free `RdfQuad` ⇄ `RdfDataset` conversions (EPIC #906): the native twins of
// the oxigraph-quad helpers, available to every Rust consumer without pulling the
// oxigraph Store adapter.
pub mod native_quads;
// The PyO3-free GTS snapshot compose core (#861 P6): SnapshotBuilder + emit_gts +
// BlobRow, lifted out of the Python binding surface so purrdf-pipeline can
// author a full multi-named-graph snapshot without pulling pyo3. Oxigraph-free
// (EPIC #906).
pub mod dataset_io;
pub mod gts_compose;
// The native OWL ↔ RDF 1.2 statement codec is fully oxigraph-free (it folds over the
// native flat-quad stream) (EPIC #906).
pub mod statements;
// Shared corpus-classification helpers (EPIC #906 Task 2): the pure corpus
// enumeration / classification helpers the native golden-capture binary
// (src/bin/capture_sparql_goldens.rs) uses. Oxigraph-free.
pub mod capture_support;
// Canonical, review-friendly Turtle serializer over the IR (#819 Task 9): the
// native replacement for rdflib `longturtle` in `purrdf normalize`. Oxigraph-free.
pub mod turtle_normalize;

// Mirror the kernel's root-level re-exports so `purrdf::RdfTerm`,
// `purrdf::RdfDiagnostic`, … keep resolving exactly as before. The two
// IR import helpers are re-exported here.
pub use dataset_io::dataset_from_bytes;
pub use gts_import_graph::import_gts_graph;
pub use gts_import_sink::import_gts_events;
pub use native_codecs::{
    classify, parse_dataset, serialize_dataset, serialize_dataset_base_only,
    serialize_dataset_to_format, GtsCodecBackend, NativeRdfFormat, SerializeOutcome,
};
pub use native_quads::{
    canonical_flat_nquads, canonical_flat_nquads_with, dataset_from_quads, flat_dataset_from_quads,
    flat_rdf_quads_from_dataset,
};
pub use purrdf_core::{
    canonicalize, canonicalize_with, check_provenance, dataset_diff, datasets_isomorphic,
    emit_annotation, emit_quad, emit_reifier, emit_resource, emit_term, fno_to_ntriples,
    fno_to_quads, gts_to_rdf_loss_ledger, loss_matrix_json, pair_loss_ledger,
    rdf_to_gts_loss_ledger, rule_iri, transcode_loss_matrix_json, ArtifactId, ArtifactIndex,
    ArtifactInterner, ArtifactRecord, AssertionOccurrence, Attribution, AttributionRole,
    BlankScope, BundleError, Bytes, CanonHash, Canonicalized, ContentDigest, ContentStore,
    ContentStoreError, DatasetDiff, DatasetMut, DatasetProvenance, DatasetSink, DatasetView,
    FnFunction, FnImpl, FnMapping, FnOutput, FnParam, FnParamMapping, FnReturnMapping, FnoCatalog,
    FrozenDatasetSource, GraphMatch, GraphMatchValue, GtsBundle, HandleEntry, HandleKey, LossEntry,
    LossLedger, MutableDataset, OriginKind, OriginSetId, OriginSetInterner, PipelineBundle,
    PipelineBundleError, ProvenanceError, QuadHandle, QuadIds, QuadRef, QuadValues, RdfAnnotation,
    RdfBlobOrigin, RdfBlobRecord, RdfBundle, RdfDataset, RdfDatasetBuilder, RdfDatasetVisitor,
    RdfDiagnostic, RdfEnvelope, RdfLiteral, RdfLocation, RdfLookaside, RdfLookasideKind,
    RdfLookasideResource, RdfLoss, RdfMetadataEntry, RdfMetadataValue, RdfOpaqueNodeRecord,
    RdfParseRequest, RdfParserBackend, RdfQuad, RdfReifier, RdfSegmentRecord, RdfSerializeRequest,
    RdfSerializer, RdfSeverity, RdfSignatureRecord, RdfStoreCapabilities, RdfSuppressionRecord,
    RdfTerm, RdfTermKind, RdfTextDirection, RdfTriple, SegmentUnitMap, SerializeGraph,
    SparqlEngine, SparqlRequest, SparqlResult, SssomDiagnostic, SssomMapping, SssomMappingSet,
    SssomMeta, TermFactory, TermId, TermRef, TermValue, UnitCatalog, UnitId, UnitInterner,
    UnitMetadata, PROJECTION_CODECS, SSSOM_DEFAULT_VALIDATION_TYPES,
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
