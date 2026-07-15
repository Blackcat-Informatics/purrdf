// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-core` -- oxigraph-free, PyO3-free RDF 1.2 kernel for the PurRDF Rust workspace.
//!
//! This crate is the ring-fenced core (purrdf P2b) extracted out of
//! `purrdf`: the immutable value-interned IR, the owned value model, structured
//! diagnostics, dataset capability flags, the loss ledger, and provenance. It
//! models RDF 1.2 terms directly, preserves
//! source/location context where adapters can provide it, and keeps reporting
//! structured but SARIF-free. The oxigraph adapters and the PyO3 extension surface
//! live in the sibling `purrdf` crate; **nothing here may pull oxigraph** — that
//! is the acceptance gate.
//!
//! # `no_std` readiness
//!
//! The immutable IR ([`ir`]) is the kernel's purest layer and is **file-IO-free**
//! (no `std::fs`/`std::io`), the first prerequisite for an eventual `alloc`-only
//! `no_std` core for embedded / C-ABI consumers. The remaining blocker is the
//! interner's `std::collections::{HashMap, HashSet}` (not in `alloc`); migrating it
//! to `hashbrown` is tracked as **P3c**. New IR code therefore prefers
//! `core::`/`alloc::` over `std::` where the item exists in both (e.g. `core::fmt`,
//! `alloc::sync::Arc`) so the eventual `#![no_std]` flip stays mechanical. Per the
//! purrdf plan, `no_std` is for embedded/C-ABI targets and is **not** a WASM
//! prerequisite. Common types are re-exported from [`prelude`].
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

pub mod bundle;
// Narrow purrdf backend traits (P2d): term interning, parser ingress,
// SPARQL execution, and serializer egress. PyO3-free, oxigraph-free — pure
// contract only; concrete adapters live in `purrdf`.
pub mod backend;
// RDF Collection (rdf:first/rest/nil) and Container (rdf:Seq/Bag/Alt) traversal:
// the malformed-list taxonomy and standard-`rdf:` const set backing the
// `DatasetView` walker methods.
pub mod collections;
pub mod content_id;
pub mod content_store;
// The static, allocation-free read view over an RDF dataset (purrdf P2):
// `DatasetView` + `GraphMatch`. PyO3-free, oxigraph-free — pure kernel.
pub mod dataset_view;
pub mod describe;
/// Structured diagnostics: severity, source/GTS locations, conversion losses,
/// and the [`RdfDiagnostic`] record callers translate to their reporting layer.
pub mod diagnostic;
// Native FnO (W3C Function Ontology) typed catalog model + serializer.
// PyO3-free; the `purrdf-slice` FnO emitter builds a `FnoCatalog` from the slice
// framework and serializes it here, replacing rdflib `emit_fno`/`_emit_fnom`.
pub mod fno;
// The workspace's single fixed-key ahash determinism policy: FastHasher and the
// FastMap/FastSet/IdSet lookup-table aliases (determinism comes from id-sorting,
// never hash order).
pub mod hash;
// The immutable, value-interned RDF 1.2 dataset IR (C1).
pub mod ir;
// Generic provenance sidecar for the immutable RDF 1.2 dataset (S2):
// UnitId/ArtifactId/OriginSetId newtypes, interners, AssertionOccurrence,
// DatasetProvenance, and the provenance gate. No PurRDF-specific concepts here.
/// Structured non-triple material ([`RdfLookaside`]) that travels with an RDF
/// store: typed sidecar resources, metadata entries, segment/blob records,
/// suppressions, opaque nodes, and signature records.
pub mod lookaside;
pub mod provenance;
// The machine-readable RDF↔GTS loss ledger and its drift-gated matrix (C0).
pub mod loss;
/// The owned RDF 1.2 value model: terms, literals (including base-direction
/// literals), triples, quads, reifiers, and statement annotations.
pub mod model;
// Native SSSOM (Simple Standard for Sharing Ontology Mappings) TSV codec +
// validator + RDF serializer. PyO3-free; replaces the `sssom` PyPI
// package's parse+validate behaviour for the PurRDF mapping artifacts.
// Shared small-vector primitives (SmallVec / IdVec) for hot, short-lived id rows.
pub mod small;
pub mod sssom;
/// Dataset/import capability flags ([`RdfStoreCapabilities`]).
pub mod store;
pub mod turtle;
// The canonical, review-friendly Turtle RENDERER over the IR — the oxigraph-free half
// of the on-disk normalizer (the oxigraph-coupled text parser stays in `purrdf`).
// The wasm-clean canonical-Turtle authority for the correspondence EDOAL lowering.
pub mod turtle_render;

pub use backend::{
    RdfParseRequest, RdfParserBackend, RdfSerializeRequest, RdfSerializer, SerializeGraph,
    SparqlEngine, SparqlRequest, SparqlResult, TermFactory,
};
pub use bundle::{
    ArtifactIndex, ArtifactRecord, BundleError, RdfBundle, SegmentUnitMap, UnitCatalog,
    UnitMetadata,
};
pub use collections::RdfListError;
pub use content_id::{Blake3ContentId, ContentIdScheme};
pub use content_store::{Bytes, ContentDigest, ContentStore, ContentStoreError};
pub use dataset_view::{DatasetMut, DatasetView, GraphMatch, GraphMatchValue, ViewTermId};
pub use describe::{Describer, describe};
pub use diagnostic::{RdfDiagnostic, RdfLocation, RdfSeverity};
pub use fno::{
    FnFunction, FnImpl, FnMapping, FnOutput, FnParam, FnParamMapping, FnReturnMapping, FnoCatalog,
    to_ntriples as fno_to_ntriples, to_quads as fno_to_quads,
};
pub use hash::{FastHasher, FastMap, FastSet, IdSet};
pub use ir::{
    BlankScope, BudgetExceeded, CanonHash, Canonicalized, CountingDemandProvider, DatasetDiff,
    DatasetSink, FrozenDatasetSource, GlobalDictionary, GlobalTermId, GtsBundle, HandleEntry,
    HandleKey, InMemoryPageProvider, MutableDataset, PageFault, PageId, PagePart, PageProvider,
    PageTranslation, PagedDataset, PagedFreezeError, PagedQuadOverlap, PagedQuadTable,
    PipelineBundle, PipelineBundleError, QuadHandle, QuadIds, QuadPatternCursor, QuadProbePlan,
    QuadRef, QuadValues, RdfDataset, RdfDatasetBuilder, RdfDatasetVisitor, RdfEnvelope,
    SubsetPageProvider, TermId, TermRef, TermValue, ValidatedRdfDatasetBuilder, canonicalize,
    canonicalize_with, dataset_diff, datasets_isomorphic, try_canonicalize, try_canonicalize_with,
};
#[doc(hidden)]
pub use ir::{
    PackBuilder, PackDigest, PackError, PackId, PackView, dataset_from_view, pack_digest,
    verify_pack,
};
pub use lookaside::{
    RdfBlobOrigin, RdfBlobRecord, RdfLookaside, RdfLookasideKind, RdfLookasideResource,
    RdfMetadataEntry, RdfMetadataValue, RdfOpaqueNodeRecord, RdfSegmentRecord, RdfSignatureRecord,
    RdfSuppressionRecord,
};
pub use loss::{
    LossEntry, LossLedger, PROJECTION_CODECS, assert_ledger_complete, assert_ledger_sound,
    check_ledger_complete, check_ledger_sound, gts_to_rdf_loss_ledger, loss_matrix_json,
    pair_loss_ledger, profile_for, rdf_gts_loss_matrix_json, rdf_to_gts_loss_ledger,
    registered_pairs,
};
pub use model::{
    RdfAnnotation, RdfLiteral, RdfQuad, RdfReifier, RdfTerm, RdfTermKind, RdfTextDirection,
    RdfTriple,
};
pub use provenance::{
    ArtifactId, ArtifactInterner, AssertionOccurrence, Attribution, AttributionRole,
    DatasetProvenance, OriginKind, OriginSetId, OriginSetInterner, ProvenanceError, UnitId,
    UnitInterner, check_provenance,
};
pub use small::{IdVec, SmallVec, smallvec};
pub use sssom::{
    SSSOM_DEFAULT_VALIDATION_TYPES, SssomDiagnostic, SssomMapping, SssomMappingSet, SssomMeta,
};
pub use store::RdfStoreCapabilities;
pub use turtle::{
    emit_annotation, emit_quad, emit_reifier, emit_resource, emit_term, rule_iri,
    write_dataset_annotation, write_dataset_quad, write_dataset_reifier, write_dataset_term,
};
pub use turtle_render::render as render_canonical_turtle;

/// The common purrdf-core surface, for `use purrdf_core::prelude::*;`.
///
/// Pulls in the owned value model, the immutable IR + builder, term identity,
/// capability flags, and the diagnostic type — the set a typical consumer reaches
/// for first.
pub mod prelude {
    pub use crate::backend::{
        RdfParseRequest, RdfParserBackend, RdfSerializeRequest, RdfSerializer, SerializeGraph,
        SparqlEngine, SparqlRequest, SparqlResult, TermFactory,
    };
    pub use crate::dataset_view::{DatasetView, GraphMatch};
    pub use crate::diagnostic::{RdfDiagnostic, RdfLocation, RdfSeverity};
    pub use crate::ir::{
        QuadIds, QuadPatternCursor, QuadRef, RdfDataset, RdfDatasetBuilder, TermId, TermRef,
        TermValue,
    };
    pub use crate::model::{
        RdfAnnotation, RdfLiteral, RdfQuad, RdfReifier, RdfTerm, RdfTermKind, RdfTextDirection,
        RdfTriple,
    };
    pub use crate::store::RdfStoreCapabilities;
}
