// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared small-vector primitives for the workspace's hot, short-lived id rows.
//!
//! [`SmallVec`] stores its first few elements inline (no heap allocation) and
//! spills to the heap only when it grows past the inline capacity, which suits
//! the many tiny, transient id vectors on the IR and evaluator paths. The
//! `union` and `serde` features stay OFF workspace-wide (see the root
//! `[workspace.dependencies]` note), so this stays wasm-clean and `unsafe`-free.
//!
//! Only the generic [`IdVec`] alias lives here. Domain-named aliases (solution
//! rows, path frontiers, …) belong to the downstream crate that owns the
//! concept, not to this shared kernel module.

pub use smallvec::{smallvec, SmallVec};

/// A small-vector of interned [`TermId`](crate::TermId)s, inline up to 4 ids.
///
/// The generic id-row primitive: most id sequences on hot paths (quad rows,
/// short frontiers) fit inline, avoiding a heap allocation.
pub type IdVec = SmallVec<[crate::TermId; 4]>;
