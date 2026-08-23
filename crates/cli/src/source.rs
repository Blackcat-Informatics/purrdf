// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The input source: reading a path (or stdin) into a queryable/serializable view.
//!
//! ## Dispatching over a non-object-safe `DatasetView`
//!
//! [`DatasetView`] uses return-position `impl Trait` in
//! its methods, so it is **not** object-safe: there is no `&dyn DatasetView`.
//! The pipeline therefore cannot erase the concrete view type behind a trait
//! object. Instead it dispatches by input KIND and runs a **generic operation**
//! monomorphized per arm:
//!
//! * a text/graph source is parsed into a concrete `RdfDataset` and the operation
//!   runs over `&RdfDataset`;
//! * a pack source is opened as a `PackView` and the operation runs over
//!   `&PackView` — zero materialization.
//!
//! Because a Rust closure cannot itself be generic, the operation is expressed as
//! a [`ViewOp`] trait whose single `run` method is generic over the view type;
//! [`run_over_input`] calls `op.run(&view)` in each arm so the compiler emits one
//! monomorphization per concrete view.
//!
//! ## Pack sources and immutable acquisition
//!
//! A pack source is acquired through [`ImmutableInput`](crate::immutable::ImmutableInput),
//! which yields bytes guaranteed **stable and un-truncatable** for the lifetime of
//! the owner: a disk pack is memory-mapped only when the mapping cannot be faulted
//! by a hostile concurrent pathname writer (a verified kernel seal, or our own
//! sealed `memfd` snapshot), and otherwise — and always for a **stdin** pack —
//! read into an owned buffer. The verified bytes are then handed to
//! [`PackView::from_bytes`] zero-copy, and the owner is held alive for the whole
//! operation, so the `PackView` reads it with no materialization.
//!
//! Integrity is verified **once**, unconditionally, via [`verify_pack`] over the
//! already-immutable bytes — *after* acquisition, closing the
//! time-of-check/time-of-use gap the previous "map then verify a mutable file" seam
//! left open. There is no longer any raw, memory-unsafe `mmap` of an unverified,
//! mutable file: the safety is enforced by [`ImmutableInput`](crate::immutable), not
//! promised by a contract on the caller.

use std::io::Read;
use std::sync::Arc;

use purrdf_core::{DatasetView, PackView, RdfDataset, dataset_from_view, verify_pack};
use purrdf_rdf::{SourceFormat, parse_dataset};

use crate::error::CliError;
use crate::immutable::ImmutableInput;

/// A generic operation to run over whichever concrete [`DatasetView`] the input
/// resolves to.
///
/// The one method is generic over the view type (`D`), which is exactly why this
/// is a trait rather than a closure: it lets [`run_over_input`] hand the operation
/// either a `&RdfDataset` or a `&PackView` and have the compiler monomorphize
/// `run` for each, sidestepping `DatasetView`'s lack of object safety.
pub(crate) trait ViewOp {
    /// What the operation produces on success.
    type Output;

    /// Run the operation over a borrowed concrete view.
    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError>;
}

/// Read every byte of a path, or of stdin when `path` is `-`.
pub(crate) fn read_bytes(path: &str) -> Result<Vec<u8>, CliError> {
    if path == "-" {
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;
        Ok(buffer)
    } else {
        Ok(std::fs::read(path)?)
    }
}

/// Acquire a pack `path` (or stdin when `path` is `-`) as an immutable, verified
/// byte owner.
///
/// The bytes are obtained through [`ImmutableInput`] — memory-safe against a hostile
/// concurrent pathname writer — and then run **once** through [`verify_pack`]
/// (fail-closed integrity) over those already-immutable bytes. The returned owner
/// must be held alive for as long as its bytes are used: a [`PackView`] borrows them
/// zero-copy, so callers keep the [`ImmutableInput`] in scope for the whole
/// operation. This is the single acquisition seam every pack consumer routes
/// through (the pipeline arms below, and `convert`'s pack→pack byte passthrough).
pub(crate) fn verified_pack_input(path: &str) -> Result<ImmutableInput, CliError> {
    let input = if path == "-" {
        ImmutableInput::from_stdin()?
    } else {
        ImmutableInput::from_disk_path(path)?
    };
    // Unconditional canonical integrity, once, over the stable bytes.
    verify_pack(input.as_bytes())?;
    Ok(input)
}

/// Open `path` as the concrete view its `format` implies and run `op` over it.
///
/// The text arm parses into an `RdfDataset`; the pack arm acquires a verified,
/// immutable byte owner ([`verified_pack_input`]) and opens a zero-copy `PackView`
/// over it. The owner is held alive for the whole `op.run` call.
pub(crate) fn run_over_input<Op: ViewOp>(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    op: Op,
) -> Result<Op::Output, CliError> {
    match format {
        SourceFormat::Native(rdf_format) => {
            let bytes = read_bytes(path)?;
            let dataset = parse_dataset(&bytes, rdf_format.media_type(), base)?;
            op.run(&*dataset)
        }
        SourceFormat::Pack => {
            let input = verified_pack_input(path)?;
            let view = PackView::from_bytes(input.as_bytes())?;
            op.run(&view)
        }
    }
}

/// Open `path` and reconstruct a concrete `Arc<RdfDataset>`, whatever its kind.
///
/// The text arm parses directly; the pack arm opens a verified zero-copy `PackView`
/// (over an immutable byte owner) and reconstructs a concrete dataset via
/// [`dataset_from_view`]. This is the entry point for steps that genuinely need an
/// owned MUTABLE dataset (e.g. SPARQL UPDATE), which cannot run over an immutable
/// view; read-only reasoning enters the reasoner over the `PackView` directly
/// through [`run_over_input`] instead.
pub(crate) fn load_dataset(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<Arc<RdfDataset>, CliError> {
    match format {
        SourceFormat::Native(rdf_format) => {
            let bytes = read_bytes(path)?;
            Ok(parse_dataset(&bytes, rdf_format.media_type(), base)?)
        }
        SourceFormat::Pack => {
            let input = verified_pack_input(path)?;
            let view = PackView::from_bytes(input.as_bytes())?;
            Ok(dataset_from_view(&view)?)
        }
    }
}
