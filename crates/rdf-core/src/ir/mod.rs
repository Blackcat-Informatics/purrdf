// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The immutable, value-interned RDF 1.2 dataset IR (C1).
//!
//! Typed term IDs and the [`builder`] interning entry points feed the
//! quad/reifier/annotation/location builder methods, the validate-then-freeze
//! path ([`validate`]), and the frozen, infallible, zero-allocation [`dataset`]
//! iteration surface.

// The IR-boundary absoluteness invariant: the one gate every term table's IRI arm
// runs on its MISS path, so a relative IRI reference is unrepresentable in the IR
// from any ingress rather than being re-checked once per codec seam.
pub(crate) mod absolute;
pub mod builder;
pub mod bundle;
// The pipeline carrier (C1): the frozen hot graph + lookaside + blob store +
// provenance + a typed-handle lane, generic over the kernel-opaque handle payload.
pub mod pipeline_bundle;
// Native full W3C RDFC-1.0 dataset canonicalization: stable canonical blank
// labels + canonical N-Quads, extended for the RDF-1.2 reifier/annotation overlay.
// The canonicalization authority for the purrdf family.
pub mod canon;
// The `RdfDataset`-direct, blank-aware structural comparator (C1/C2): the
// equality oracle for importer equivalence.
pub mod compare;
pub mod composite;
// Compact, allocation-free probe cursors shared by the layered views.
mod cursor;
pub mod dataset;
/// Deterministic, mmap-native embedding companions bound to exact pack bytes.
pub mod embedding;
// The copy-on-write, suppression-delta mutable dataset + `DatasetMut` impl.
pub mod mutable;
pub mod view_accounting;
// Evented, ID-addressed OUTPUT of a frozen dataset (C6): the dual of the
// permissive ingestion protocol, for chase / SHACL-result / projection consumers.
pub mod event_sink;
// The u64-scaled GLOBAL term-identity layer (backend seam): a separate id space
// (`GlobalTermId`) and its value-interner (`GlobalDictionary`), for paged /
// cross-segment backends. NEVER widens the frozen dataset's u32 `TermId` niche.
pub mod global;
// The permissive-ingestion adapter: an `RdfEventSink` (the
// `purrdf-events` protocol) that buffers forward references and freezes a dataset
// at `finish()`, plus the frozen-IR-replay `RdfEventSource` that drives it.
pub mod import;
pub mod ingest;
// A reference, in-memory, demand-paged dataset (backend seam): `PagedDataset`
// composes many frozen `RdfDataset` pages into one logical `DatasetView` keyed on
// `GlobalTermId`, plus the `PageProvider` demand-paging hook and per-page
// `PageTranslation` local↔global id map.
pub mod paged;
pub mod segmented;
// The succinct, dependency-free dataset pack. Its builder/view/restore surface is
// public; the bit-packing implementation modules remain doc-hidden.
pub mod pack;
// Caller-invoked blank-node recourse: RDF 1.2 skolemize/deskolemize under a
// caller-supplied authority, plus the whole-dataset term-rewrite driver that
// `canon::canonical_relabel` shares. Never a serializer mode.
pub mod skolem;
pub mod term;
mod term_walk;
pub mod validate;

pub use builder::{NativeBuildError, RdfDatasetBuilder, ValidatedRdfDatasetBuilder};
pub use bundle::{GtsBundle, RdfEnvelope};
pub use canon::{
    BudgetExceeded, CANON_CORPUS_DIGEST, CANON_PRESENTATION_FLAT_ASSERTION_ID,
    CANON_PRESENTATION_FLAT_ASSERTION_VERSION, CANON_PRESENTATION_OVERLAY_ID,
    CANON_PRESENTATION_OVERLAY_VERSION, CANON_PROFILE_ID, CANON_PROFILE_VERSION, CanonError,
    CanonHash, CanonPresentation, CanonicalRelabeling, Canonicalized, DatasetStateDigest,
    DatasetStateError, RDFC_CALL_LIMIT, RESERVED_NAMESPACE, ReservedVocabulary, TermPosition,
    ViewCanonError, blank_count_view, canonical_relabel, canonical_relabel_with_mapping,
    canonicalize, canonicalize_graph_view, canonicalize_view, canonicalize_with, check_admissible,
    check_admissible_flat_view, check_admissible_view, graph_digest_view, try_blank_count_view,
    try_canonicalize, try_canonicalize_flat_graph_view, try_canonicalize_flat_view,
    try_canonicalize_graph_view, try_canonicalize_view, try_canonicalize_with,
    try_flat_digest_view, try_graph_digest_view,
};
pub use compare::{DatasetDiff, dataset_diff, datasets_isomorphic};
pub use dataset::{
    QuadHandle, QuadIds, QuadPatternCursor, QuadProbePlan, QuadRef, QueryIndexAllocationError,
    RdfDataset, RdfDatasetIter, TermRef,
};
pub use embedding::*;
pub use event_sink::RdfDatasetVisitor;
pub use global::{GlobalDictionary, GlobalTermId};
pub use ingest::{DatasetSink, FrozenDatasetSource};
pub use mutable::{
    DeltaDatasetView, DeltaViewId, GraphExistenceMode, MutableDataset, QuadValues, RecordKind,
    RecordValues,
};
pub use pack::{
    PackBuilder, PackCheckpoint, PackDigest, PackError, PackId, PackView, dataset_from_view,
    pack_digest, restore_pack, verify_pack,
};
pub use paged::{
    CanonicalPagedError, CountingDemandProvider, InMemoryPageProvider, PageFault, PageFaultKind,
    PageGeneration, PageId, PageMaterialization, PagePart, PageProvider, PageTranslation,
    PagedDataset, PagedFreezeError, PagedQuadOverlap, PagedQuadTable, PagedQueryError,
    PagedQueryEvidence, PagedQueryLimits, PagedQueryView, PagedStack, PagedStackError,
    PagedStackEvidence, PagedStackQueryView, PagedStackSnapshot, StackPageOrigin, StackSource,
    SubsetPageProvider, canonical_paged_seal,
};
pub use pipeline_bundle::{
    BundleDigestWork, CanonScopeName, GraphLayer, HandleEntry, HandleKey, PIPELINE_ROOT_DOMAIN,
    PipelineBundle, PipelineBundleError, PipelineViewBundle,
};
pub use segmented::{
    SegmentedBuildLimits, SegmentedBuilder, SegmentedBytes, SegmentedError, SegmentedEvidence,
    SegmentedExportError, SegmentedHandle, SegmentedImage, SegmentedProvider, SegmentedReadLimits,
    SegmentedReceipt, SegmentedReceiptAuthority, SegmentedRequest, SegmentedReservation,
    SegmentedSession, SegmentedSnapshot, SegmentedTermGuard,
};
pub use skolem::{GENID_WELL_KNOWN_PATH, SkolemError, deskolemize, skolemize};
pub use term::{BlankScope, NonIriPredicate, TermId, TermValue};
pub(crate) use term_walk::try_owned_text;
pub use term_walk::{
    Nested, OwnedTermFoldError, TermBox, TermVisit, fold_term, fold_workspace_bound,
    try_fold_nested, try_fold_nested_with_memory, try_fold_term, try_fold_term_with_memory,
    try_fold_term_with_storage, visit_nested,
};

pub use composite::{
    CompositeDatasetView, CompositeSource, CompositeViewId, GraphPlacement, ScopeBinding,
};
pub use view_accounting::{
    OwnerKey, OwnerMutability, RetainedCharge, RetentionGuard, RetentionLedger, RetentionSnapshot,
    ViewAccountingReport, ViewLimits, ViewStats, ViewWork,
};
