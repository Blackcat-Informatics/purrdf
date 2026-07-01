// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! POD value types that cross the C boundary by value.
//!
//! Everything here is `#[repr(C)]` / `#[repr(i32)]` and FFI-stable. The
//! [`PurrdfStatus`] enum is **append-only** — new variants get new numbers,
//! existing ones never change, so the ABI stays SemVer-frozen.
//!
//! The structured term-view types ([`crate::term::PurrdfTermView`] and friends)
//! live in the [`crate::term`] module, alongside the term-crossing logic that
//! produces them.

/// Status returned by every fallible entry point as `i32`. `Ok == 0`.
///
/// Append-only: never renumber a variant. `Panic` is parked at 100 to leave room
/// for ordinary status codes to grow contiguously from 10.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PurrdfStatus {
    /// Success.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// A C string argument was not valid UTF-8.
    InvalidUtf8 = 2,
    /// An argument was structurally invalid (e.g. an unknown enum tag).
    InvalidArgument = 3,
    /// The requested media type / format id is not supported.
    UnsupportedFormat = 4,
    /// Parsing the input bytes failed.
    ParseError = 5,
    /// Serializing the dataset failed.
    SerializeError = 6,
    /// Evaluating the SPARQL query failed.
    QueryError = 7,
    /// Freezing a mutable graph into a dataset failed.
    FreezeError = 8,
    /// A cursor has no more rows (a non-error terminal signal, returned > 0).
    CursorExhausted = 9,
    /// A GTS container read/write operation failed.
    GtsError = 10,
    /// A panic was caught at the FFI boundary (should never reach the caller in
    /// normal operation).
    Panic = 100,
}

/// `#[repr(C)]` twin of `purrdf_core`'s `RdfStoreCapabilities` (each `bool`
/// rendered as `0`/`1`). The seven flags mirror the kernel exactly.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfCapabilities {
    /// The dataset distinguishes named graphs from the default graph.
    pub named_graphs: u8,
    /// The dataset carries RDF-1.2 quoted triples (the star layer).
    pub quoted_triples: u8,
    /// The dataset carries reifier bindings.
    pub reifiers: u8,
    /// The dataset carries annotation triples.
    pub annotations: u8,
    /// The dataset carries per-quad source locations.
    pub source_locations: u8,
    /// The dataset carries projection loss records.
    pub loss_records: u8,
    /// The dataset carries an out-of-band lookaside.
    pub lookaside: u8,
}

/// The SemVer ABI version reported by `purrdf_abi_version`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfAbiVersion {
    /// Major version: incompatible ABI change.
    pub major: u32,
    /// Minor version: backward-compatible additions.
    pub minor: u32,
    /// Patch version: backward-compatible fixes.
    pub patch: u32,
}
