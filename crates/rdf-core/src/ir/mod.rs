// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The immutable, value-interned RDF 1.2 dataset IR (C1).
//!
//! This module tree realizes the normative C0 semantic contract from
//! `docs/design/819-rdf-ir-dataflow.md`. Task 2 (C1.a) landed the **interning
//! half** (typed term ids in [`term`] and the `intern_*` entry points in
//! [`builder`]); Task 3 (C1.b) completes C1 with the quad/reifier/annotation/
//! location builder methods, the validate-then-freeze path ([`validate`]), and the
//! frozen, infallible, zero-allocation [`dataset`] iteration surface. The
//! GTS-bundle bridge arrives in later tasks (C2+).

pub mod builder;
pub mod bundle;
// The pipeline carrier (C1): the frozen hot graph + lookaside + blob store +
// provenance + a typed-handle lane, generic over the kernel-opaque handle payload.
pub mod pipeline_bundle;
// Native full W3C RDFC-1.0 dataset canonicalization: stable canonical blank
// labels + canonical N-Quads, extended for the RDF-1.2 reifier/annotation overlay.
// The canonicalization authority for the purrdf family — explicitly NOT oxigraph.
pub mod canon;
// The `RdfDataset`-direct, blank-aware structural comparator (C1/C2): the
// equality oracle for importer equivalence — explicitly NOT oxigraph.
pub mod compare;
pub mod dataset;
// The copy-on-write, suppression-delta mutable dataset + `DatasetMut` impl (P5).
pub mod mutable;
// Evented, ID-addressed OUTPUT of a frozen dataset (C6): the dual of the
// permissive ingestion protocol, for chase / SHACL-result / projection consumers.
pub mod event_sink;
// The u64-scaled GLOBAL term-identity layer (backend seam): a separate id space
// (`GlobalTermId`) and its value-interner (`GlobalDictionary`), for paged /
// cross-segment backends. NEVER widens the frozen dataset's u32 `TermId` niche.
pub mod global;
// The permissive-ingestion adapter (purrdf P6): an `RdfEventSink` (the
// `purrdf-events` protocol) that buffers forward references and freezes a dataset
// at `finish()`, plus the frozen-IR-replay `RdfEventSource` that drives it.
pub mod ingest;
// A reference, in-memory, demand-paged dataset (backend seam): `PagedDataset`
// composes many frozen `RdfDataset` pages into one logical `DatasetView` keyed on
// `GlobalTermId`, plus the `PageProvider` demand-paging hook and per-page
// `PageTranslation` local↔global id map.
pub mod paged;
pub mod term;
pub mod validate;

pub use builder::{RdfDatasetBuilder, ValidatedRdfDatasetBuilder};
pub use bundle::{GtsBundle, RdfEnvelope};
pub use canon::{CanonHash, Canonicalized, canonicalize, canonicalize_with};
pub use compare::{DatasetDiff, dataset_diff, datasets_isomorphic};
pub use dataset::{
    QuadHandle, QuadIds, QuadPatternCursor, QuadProbePlan, QuadRef, RdfDataset, RdfDatasetIter,
    TermRef,
};
pub use event_sink::RdfDatasetVisitor;
pub use global::{GlobalDictionary, GlobalTermId};
pub use ingest::{DatasetSink, FrozenDatasetSource};
pub use mutable::{MutableDataset, QuadValues};
pub use paged::{
    CountingDemandProvider, InMemoryPageProvider, PageFault, PageId, PageProvider, PageTranslation,
    PagedDataset, PagedFreezeError, PagedQuadOverlap, SubsetPageProvider,
};
pub use pipeline_bundle::{HandleEntry, HandleKey, PipelineBundle, PipelineBundleError};
pub use term::{BlankScope, TermId, TermValue};
