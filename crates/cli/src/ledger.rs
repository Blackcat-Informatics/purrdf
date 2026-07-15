// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surfacing the loss ledger under the global `--loss-ledger` flag.
//!
//! [`surface`] renders the ledger's stable JSON exactly where the flag's decoded
//! [`LedgerTarget`] directs: nowhere, stderr, or a file. The rendered JSON already
//! carries a trailing newline, so the stderr write emits it verbatim.

use purrdf_core::LossLedger;

use crate::cli::LedgerTarget;
use crate::error::CliError;

/// Surface `ledger` per the decoded `--loss-ledger` target.
///
/// * [`LedgerTarget::Silent`] — emit nothing.
/// * [`LedgerTarget::Stderr`] — write the JSON to stderr.
/// * [`LedgerTarget::File`] — write the JSON to the given path.
pub(crate) fn surface(target: &LedgerTarget, ledger: &LossLedger) -> Result<(), CliError> {
    match target {
        LedgerTarget::Silent => Ok(()),
        LedgerTarget::Stderr => {
            eprint!("{}", ledger.render_json());
            Ok(())
        }
        LedgerTarget::File(path) => {
            std::fs::write(path, ledger.render_json())?;
            Ok(())
        }
    }
}
