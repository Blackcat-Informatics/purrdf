// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `convert` subcommand: `Source → [entail] → [canonicalize] → Sink`.
//!
//! Resolve the source and target formats, open the source as a view, and write it
//! through the [`sink`]. A pack→pack conversion is a verified byte
//! passthrough (re-encoding a pack would be pointless churn); every other
//! combination flows the source view into [`sink::write_rdf`], which serializes to
//! the target syntax or rebuilds the pack container. The resulting loss ledger is
//! surfaced under `--loss-ledger`.
//!
//! ## Transforms: `--entailment` and `--canonical`
//!
//! Two optional transforms compose in a fixed order (entail first, then
//! canonicalize), and both need a concrete owned dataset — so when either is
//! present the pipeline reconstructs an `Arc<RdfDataset>` up front (a text source is
//! parsed, a pack source is rebuilt via [`source::load_dataset`]) instead of taking
//! the zero-copy view path:
//!
//! * `--entailment REGIME` materializes the regime's closure in memory (rejecting
//!   the non-materializable regimes on the same exit-3 path as `reason`).
//! * `--canonical` emits the RDFC-1.0 canonical N-Quads document
//!   ([`canonical_flat_nquads`]) rather than the `--to` format. Canonical output is
//!   always N-Quads, so `--canonical` OVERRIDES (and lets you omit) `--to`.

use std::sync::Arc;

use purrdf_core::{DatasetView, LossLedger, RdfDataset, verify_pack};
use purrdf_entail::materialize;
use purrdf_rdf::canonical_flat_nquads;

use crate::cli::{CliRdfFormat, CliRegime, LedgerTarget};
use crate::error::CliError;
use crate::format::{self, CliFormat};
use crate::ledger;
use crate::reason;
use crate::sink;
use crate::source::{self, ViewOp};

/// The generic sink operation for a convert: serialize whichever concrete view the
/// source resolved to into the target format.
struct ConvertOp<'a> {
    out: &'a str,
    target: CliFormat,
    base: Option<&'a str>,
    src_codec: Option<&'a str>,
}

impl ViewOp for ConvertOp<'_> {
    type Output = LossLedger;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<LossLedger, CliError> {
        sink::write_rdf(view, self.out, self.target, self.base, self.src_codec)
    }
}

/// The resolved `convert` flags: the format overrides plus the parse base and the
/// two optional transforms. Grouping them keeps [`run`]'s signature small and lets
/// the transform lane borrow them by reference.
pub(crate) struct ConvertOptions<'a> {
    /// The `--from` input-format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// The `--to` output-format override.
    pub(crate) to: Option<CliRdfFormat>,
    /// The `--base` parse/serialize base IRI.
    pub(crate) base: Option<&'a str>,
    /// The `--entailment` regime to materialize before serializing.
    pub(crate) entailment: Option<CliRegime>,
    /// Whether `--canonical` was set (emit RDFC-1.0 canonical N-Quads).
    pub(crate) canonical: bool,
}

/// Run the `convert` subcommand.
pub(crate) fn run(
    options: &ConvertOptions<'_>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let source_format = format::resolve(options.from, input)?;

    // The transform lane: either `--entailment` or `--canonical` needs a concrete
    // owned dataset, so reconstruct one and apply the transforms in order.
    if options.canonical || options.entailment.is_some() {
        return run_with_transforms(source_format, options, input, output, ledger_target);
    }

    // Pack → pack: a verified byte passthrough (no decode/re-encode churn). A DISK
    // pack is mmap-borrowed (no `Vec<u8>` copy of the pack contents); stdin has no
    // file to map, so it still buffers into a `Vec`.
    let target_format = format::resolve(options.to, output)?;
    if matches!(source_format, CliFormat::Pack) && matches!(target_format, CliFormat::Pack) {
        if input == "-" {
            let bytes = source::read_bytes(input)?;
            verify_pack(&bytes)?;
            sink::write_out(output, &bytes)?;
        } else {
            let mmap = source::verified_pack_mmap(input)?;
            sink::write_out(output, &mmap[..])?;
        }
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    let src_codec = source_format.loss_codec_name();
    let ledger = source::run_over_input(
        input,
        source_format,
        options.base,
        ConvertOp {
            out: output,
            target: target_format,
            base: options.base,
            src_codec,
        },
    )?;
    ledger::surface(ledger_target, &ledger)
}

/// The `--entailment` / `--canonical` lane: reconstruct an owned dataset, optionally
/// materialize its entailment closure, then either emit canonical N-Quads or
/// serialize to the `--to` target.
fn run_with_transforms(
    source_format: CliFormat,
    options: &ConvertOptions<'_>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let dataset = source::load_dataset(input, source_format, options.base)?;

    // Entail first: materialize the regime's closure (rejecting the
    // non-materializable regimes on the same exit-3 path `reason` uses).
    let dataset: Arc<RdfDataset> = match options.entailment {
        Some(regime) => {
            let regime = reason::resolve_materializable_regime(regime)?;
            materialize(&dataset, regime)?
        }
        None => dataset,
    };

    // Then canonicalize: RDFC-1.0 canonical N-Quads always override `--to`.
    if options.canonical {
        let nquads = canonical_flat_nquads(&dataset).map_err(CliError::Runtime)?;
        sink::write_out(output, nquads.as_bytes())?;
        // The RDFC-1.0 canonical N-Quads document flattens the RDF 1.2 statement
        // overlay into plain triples; it is a lossless re-rendering, so no ledger.
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    // No `--canonical`: serialize the (possibly entailed) closure to `--to`.
    let target_format = format::resolve(options.to, output)?;
    let src_codec = source_format.loss_codec_name();
    let ledger = sink::write_rdf(&*dataset, output, target_format, options.base, src_codec)?;
    ledger::surface(ledger_target, &ledger)
}
