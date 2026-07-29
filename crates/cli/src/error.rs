// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The single CLI error type and its process-exit-code mapping.
//!
//! Every fallible step in the pipeline funnels its error into [`CliError`], whose
//! [`CliError::exit_code`] classifies it into the two-way exit contract the shell
//! sees:
//!
//! * **2** — argument / usage errors ([`CliError::Usage`]). This matches clap's own
//!   exit code for a malformed command line, so a usage error the pipeline detects
//!   (e.g. stdin with no explicit format, or `--regime rif` without `--rules`) is
//!   indistinguishable to a caller from one clap rejects.
//! * **1** — every other runtime failure ([`CliError::Runtime`]): a parse/serialize
//!   diagnostic, a pack-integrity failure, an I/O error, or a results-serialization
//!   error.
//!
//! # There is no unsupported-regime exit code, because there is no unsupported regime
//!
//! A third code (**3**) used to classify an entailment-regime boundary the CLI could
//! not cross: `owl-direct` and `rif` were refused because a `Regime` value carried
//! neither the query's class expressions nor a rule set. `purrdf_entail::materialize`
//! takes a `Materialization` now, which carries both, and
//! [`EntailmentPlan`](crate::reason::EntailmentPlan) is where the CLI supplies them —
//! so every one of the seven regimes runs and the code that classified their refusal
//! has nothing left to classify. What remains is ordinary: `--regime rif` without
//! `--rules` is an incomplete command line (exit 2), and an unreadable or malformed
//! rule document is a runtime failure (exit 1), exactly like an unreadable input.
//!
//! The `From` conversions below let the pipeline propagate library errors with `?`.

use std::fmt;

use purrdf_core::{PackError, RdfDiagnostic};
use purrdf_entail::EntailError;

/// A CLI-level failure, carrying its rendered message and its exit classification.
#[derive(Debug)]
pub(crate) enum CliError {
    /// An argument / usage error (exit code 2).
    Usage(String),
    /// Any other runtime failure — parse, serialize, pack integrity, or I/O
    /// (exit code 1).
    Runtime(String),
}

impl CliError {
    /// The process exit code for this error's category (2 usage / 1 runtime).
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Runtime(_) => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(msg) | Self::Runtime(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<RdfDiagnostic> for CliError {
    fn from(diagnostic: RdfDiagnostic) -> Self {
        Self::Runtime(diagnostic.to_string())
    }
}

impl From<PackError> for CliError {
    fn from(error: PackError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<purrdf_rdf::ProjectionError> for CliError {
    fn from(error: purrdf_rdf::ProjectionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<purrdf_sparql_results::Error> for CliError {
    fn from(error: purrdf_sparql_results::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<EntailError> for CliError {
    fn from(error: EntailError) -> Self {
        Self::Runtime(error.to_string())
    }
}
