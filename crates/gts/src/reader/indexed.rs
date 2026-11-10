// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicitly bound seek input for selective dictionary operations. Structural
//! discovery does not certify unvisited payloads or perform full verification.

mod access;
mod catalog;
mod dictionary;
pub use access::{
    AccessBinding, AccessOwner, BindingChange, BoundCursor, BoundRange, DecodeCapabilities,
    IndexedError, IndexedInput, IndexedWork, LookupPolicy,
};
pub use catalog::{CatalogError, IndexedCatalog, SegmentCatalog, TermBlock};
pub use dictionary::{IndexedDictionary, SelectedBinding, SelectedTerm};
