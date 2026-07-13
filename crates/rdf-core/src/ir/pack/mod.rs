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
//! [`dict`] lands the four-section value dictionary (Task 2):
//! [`dict::PackDict`] assigns one unified id per distinct term value (scanned by
//! role from an `RdfDataset`'s base quads), PFC-compresses each section on disk, and
//! decodes into an owned, query-ready form. The bitmap-triples encoder that consumes
//! it arrives in a later task.
//!
//! `#[doc(hidden)]` (see [`super::pack`]'s declaration): this whole tree is an
//! internal-codec surface, not a SemVer-guaranteed part of the crate's public API.

#[doc(hidden)]
pub mod bits;
#[doc(hidden)]
pub mod dict;
