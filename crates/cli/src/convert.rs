// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `convert` subcommand: `Source → Sink` with no transform.
//!
//! Resolve the source and target formats, open the source as a view, and write it
//! through the [`sink`]. A pack→pack conversion is a verified byte
//! passthrough (re-encoding a pack would be pointless churn); every other
//! combination flows the source view into [`sink::write_rdf`], which serializes to
//! the target syntax or rebuilds the pack container. The resulting loss ledger is
//! surfaced under `--loss-ledger`.

use purrdf_core::{DatasetView, LossLedger, verify_pack};

use crate::cli::{CliRdfFormat, LedgerTarget};
use crate::error::CliError;
use crate::format::{self, CliFormat};
use crate::ledger;
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

/// Run the `convert` subcommand.
pub(crate) fn run(
    from: Option<CliRdfFormat>,
    to: Option<CliRdfFormat>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let source_format = format::resolve(from, input)?;
    let target_format = format::resolve(to, output)?;

    // Pack → pack: a verified byte passthrough (no decode/re-encode churn).
    if matches!(source_format, CliFormat::Pack) && matches!(target_format, CliFormat::Pack) {
        let bytes = source::read_bytes(input)?;
        verify_pack(&bytes)?;
        sink::write_out(output, &bytes)?;
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    let src_codec = source_format.loss_codec_name();
    let ledger = source::run_over_input(
        input,
        source_format,
        None,
        ConvertOp {
            out: output,
            target: target_format,
            base: None,
            src_codec,
        },
    )?;
    ledger::surface(ledger_target, &ledger)
}
