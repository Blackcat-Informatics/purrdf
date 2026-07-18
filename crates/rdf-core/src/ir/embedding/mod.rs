// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic embedding companions for exact PurRDF pack artifacts.
//!
//! PURREMB keeps model-specific, lossy vector projections separate from RDF
//! canonical identity while binding them to both the exact source pack and its
//! independently certified RDFC digest. The reader borrows caller-owned bytes;
//! heap buffers, immutable memory maps, and WebAssembly linear memory therefore
//! use one format and one validation path.

mod contract;
mod error;
mod identity;
mod metadata;
mod target;
mod verify;
mod view;
mod wire;
mod writer;

pub use contract::*;
pub use error::{DigestKind, EmbeddingError, EmbeddingWriteError};
pub use identity::*;
pub use metadata::*;
pub use target::*;
pub use verify::*;
pub use view::*;
pub use wire::*;
pub use writer::*;
