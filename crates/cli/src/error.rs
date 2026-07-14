// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The single CLI error type and its process-exit-code mapping.
//!
//! Every fallible step in the pipeline funnels its error into [`CliError`], whose
//! [`CliError::exit_code`] classifies it into the three-way exit contract the shell
//! sees:
//!
//! * **2** — argument / usage errors ([`CliError::Usage`]). This matches clap's own
//!   exit code for a malformed command line, so a usage error the pipeline detects
//!   (e.g. stdin with no explicit format) is indistinguishable to a caller from one
//!   clap rejects.
//! * **3** — an entailment-regime boundary the CLI cannot cross
//!   ([`CliError::UnsupportedRegime`]): the regime needs inputs (query class
//!   expressions or a rule set) the CLI has no way to supply.
//! * **1** — every other runtime failure ([`CliError::Runtime`]): a parse/serialize
//!   diagnostic, a pack-integrity failure, an I/O error, or a results-serialization
//!   error.
//!
//! The `From` conversions below let the pipeline propagate library errors with `?`
//! while preserving that classification: an [`EntailError::Unsupported`] becomes an
//! [`CliError::UnsupportedRegime`] (exit 3), and everything else becomes
//! [`CliError::Runtime`] (exit 1).

use std::fmt;

use purrdf_core::{PackError, RdfDiagnostic};
use purrdf_entail::EntailError;

/// A CLI-level failure, carrying its rendered message and its exit classification.
#[derive(Debug)]
pub(crate) enum CliError {
    /// An argument / usage error (exit code 2).
    Usage(String),
    /// An entailment regime the CLI cannot materialize because it needs extra
    /// inputs (exit code 3).
    UnsupportedRegime(String),
    /// Any other runtime failure — parse, serialize, pack integrity, or I/O
    /// (exit code 1).
    Runtime(String),
}

impl CliError {
    /// The process exit code for this error's category (2 usage / 3 unsupported
    /// regime / 1 runtime).
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::UnsupportedRegime(_) => 3,
            Self::Runtime(_) => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(msg) | Self::UnsupportedRegime(msg) | Self::Runtime(msg) => {
                f.write_str(msg)
            }
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
        match &error {
            EntailError::Unsupported(_) => Self::UnsupportedRegime(error.to_string()),
            _ => Self::Runtime(error.to_string()),
        }
    }
}
