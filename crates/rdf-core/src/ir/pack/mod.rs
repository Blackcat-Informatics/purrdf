// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Succinct, dependency-free bit-packing codecs (the `pack` kernel).
//!
//! This module tree hosts the primitives a later on-disk/serialized dataset
//! encoding (FoQ inverted lists, the value dictionary, bitmap-triples) builds on:
//! fixed-width integer vectors, rank/select bitmaps, and varint/zigzag/delta byte
//! codecs. Everything here is `std`-only (no new dependencies),
//! `wasm32-unknown-unknown`-clean (no threads, no filesystem, no wall-clock, no
//! RNG), and byte-deterministic.
//!
//! [`bits`] lands the whole primitive layer (Task 1 of the succinct-pack-codec
//! feature): [`bits::IntVector`] (fixed-width bit-packed integers),
//! [`bits::BitVec`]/[`bits::RankSelect`] (rank1/select1/rank0/select0 succinct
//! bitmaps), and the varint/zigzag/delta-list byte helpers.
//!
//! [`dict`] lands the unified value dictionary (Task 2):
//! [`dict::PackDict`] assigns ONE unified id per distinct term value —
//! regardless of role (subject, predicate, object, graph name, or structural
//! reference) — scanned from an `RdfDataset`'s base quads and side tables (see
//! that module's docs), PFC-compresses the whole sorted value list on disk,
//! and decodes into an owned, query-ready form.
//!
//! [`triples`] lands the graph-partitioned succinct bitmap-triples + FoQ
//! auxiliary indexes (Task 3): [`triples::Triples::encode`] builds one
//! self-contained bitmap-triples structure per graph (partition 0 = default
//! graph) from a [`dict::PackDict`] + `RdfDataset`, and
//! [`triples::TriplesRef::from_bytes`] is the borrowed, zero-copy reader that
//! answers all 8 `(s, p, o)` pattern shapes without decompression.
//!
//! [`side`] lands the RDF 1.2 reifier/annotation side-tables (Task 4):
//! [`side::SideTables::encode`] builds a self-contained encoding of every
//! reifier binding and statement annotation (in unified [`dict::PackTermId`]s)
//! from a [`dict::PackDict`] + `RdfDataset`, and [`side::SideTablesRef::from_bytes`]
//! is the borrowed, zero-copy reader that reproduces `RdfDataset::reifier_quads`,
//! `RdfDataset::annotation_quads`, and `RdfDataset::annotations_of_with_graph`
//! byte-for-byte over the unified id space.
//!
//! [`container`] lands the on-disk pack container (Task 5): [`PackBuilder`]
//! frames the dict/triples/side sections' bytes into ONE fixed-layout,
//! mmap-friendly file with a section-directory integrity check (each section's
//! SHA-256, plus the dataset's RDFC-1.0 canonical digest), and [`PackView`] is
//! the borrowed, zero-copy, fail-closed reader over that file — the module's
//! public face for everything this tree assembles. See [`container`]'s doc
//! comment for the exact byte layout.
//!
//! [`view`] wires [`PackView`] into the [`crate::DatasetView`] seam (Task 6):
//! [`view::PackId`] is the id newtype a `PackView`-backed read mints, and
//! `impl DatasetView for PackView<'_>` answers every read-side query straight off
//! the pack's decoded dictionary + borrowed sections, with no materialization step.
//!
//! `#[doc(hidden)]` (see [`super::pack`]'s declaration): this whole tree is an
//! internal-codec surface, not a SemVer-guaranteed part of the crate's public API.

#[doc(hidden)]
pub mod bits;
#[doc(hidden)]
pub mod container;
#[doc(hidden)]
pub mod dict;
#[doc(hidden)]
pub mod side;
#[doc(hidden)]
pub mod triples;
#[doc(hidden)]
pub mod view;

#[doc(hidden)]
pub use container::{PackBuilder, PackError, PackView};
#[doc(hidden)]
pub use view::PackId;
