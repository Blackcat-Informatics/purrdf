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
//! ## Pack sources and mmap
//!
//! A pack file on disk is opened **read-only** and memory-mapped, then handed to
//! [`PackView::from_bytes`] zero-copy; the mapping is held alive for the whole
//! operation. A pack arriving on **stdin** cannot be mmap'd, so it is read into a
//! `Vec<u8>` and viewed over that buffer instead. Either way the bytes are
//! **unconditionally** run through [`verify_pack`] (fail-closed integrity) before
//! any view is opened.
//!
//! SAFETY / caveat: the mmap is a read-only view of a file this process does not
//! mutate. `Mmap::map` is `unsafe` because the OS cannot guarantee another process
//! won't mutate the backing file underneath the mapping; the CLI's contract is
//! that a pack file it reads is not concurrently rewritten for the brief duration
//! of the operation. No concurrent mutation of the mapped file is performed or
//! tolerated.

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use purrdf_core::{DatasetView, PackView, RdfDataset, dataset_from_view, verify_pack};
use purrdf_rdf::parse_dataset;

use crate::error::CliError;
use crate::format::CliFormat;

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

/// Open a DISK pack `path` read-only, mmap it, and verify its integrity.
///
/// This is the mmap seam factored out of the pack arms of [`run_over_input`] and
/// [`load_dataset`] so a caller that only needs the verified bytes (not a
/// `PackView`) — e.g. `convert`'s pack→pack byte passthrough — can borrow them
/// without materializing a `Vec<u8>` copy of the pack. Callers on **stdin** cannot
/// use this (there is no file to map); they must fall back to [`read_bytes`] +
/// [`verify_pack`].
///
/// SAFETY / caveat: identical to the pack arms above — a read-only mapping of a
/// file this process does not mutate (nor tolerates concurrent external mutation
/// of) for the brief lifetime of the mapping, which the caller holds alive only as
/// long as it needs the borrowed bytes.
pub(crate) fn verified_pack_mmap(path: &str) -> Result<memmap2::Mmap, CliError> {
    let file = File::open(path)?;
    // SAFETY: see the module docs and the identical comment on the pack arms of
    // `run_over_input` / `load_dataset` — a read-only, non-concurrently-mutated
    // mapping confined to the caller's use of the returned `Mmap`.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    verify_pack(&mmap[..])?;
    Ok(mmap)
}

/// Open `path` as the concrete view its `format` implies and run `op` over it.
///
/// The text arm parses into an `RdfDataset`; the pack arm mmaps the file (or reads
/// stdin into a `Vec`), verifies its integrity, and opens a zero-copy `PackView`.
/// The mmap/`Vec` backing store is held alive for the whole `op.run` call.
pub(crate) fn run_over_input<Op: ViewOp>(
    path: &str,
    format: CliFormat,
    base: Option<&str>,
    op: Op,
) -> Result<Op::Output, CliError> {
    match format {
        CliFormat::Rdf(rdf_format) => {
            let bytes = read_bytes(path)?;
            let dataset = parse_dataset(&bytes, rdf_format.media_type(), base)?;
            op.run(&*dataset)
        }
        CliFormat::Pack => {
            if path == "-" {
                let bytes = read_bytes(path)?;
                verify_pack(&bytes)?;
                let view = PackView::from_bytes(&bytes)?;
                op.run(&view)
            } else {
                let mmap = verified_pack_mmap(path)?;
                let view = PackView::from_bytes(&mmap[..])?;
                op.run(&view)
            }
        }
    }
}

/// Open `path` and reconstruct a concrete `Arc<RdfDataset>`, whatever its kind.
///
/// The text arm parses directly; the pack arm opens a `PackView` (verified) and
/// reconstructs a concrete dataset via [`dataset_from_view`]. This is the entry
/// point for steps that genuinely need an owned dataset (e.g. entailment
/// materialization, whose `materialize` takes a `&RdfDataset`).
pub(crate) fn load_dataset(
    path: &str,
    format: CliFormat,
    base: Option<&str>,
) -> Result<Arc<RdfDataset>, CliError> {
    match format {
        CliFormat::Rdf(rdf_format) => {
            let bytes = read_bytes(path)?;
            Ok(parse_dataset(&bytes, rdf_format.media_type(), base)?)
        }
        CliFormat::Pack => {
            if path == "-" {
                let bytes = read_bytes(path)?;
                verify_pack(&bytes)?;
                let view = PackView::from_bytes(&bytes)?;
                Ok(dataset_from_view(&view)?)
            } else {
                let mmap = verified_pack_mmap(path)?;
                let view = PackView::from_bytes(&mmap[..])?;
                Ok(dataset_from_view(&view)?)
            }
        }
    }
}
