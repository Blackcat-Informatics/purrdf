// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The CLI's exit contract: the single error type, and the one outcome that is not an
//! error.
//!
//! Every fallible step in the pipeline funnels its error into [`CliError`], whose
//! [`CliError::exit_code`] classifies it into the exit contract the shell sees:
//!
//! * **2** — argument / usage errors ([`CliError::Usage`]). This matches clap's own
//!   exit code for a malformed command line, so a usage error the pipeline detects
//!   (e.g. stdin with no explicit format, or `--regime rif` without `--rules`) is
//!   indistinguishable to a caller from one clap rejects.
//! * **1** — every other runtime failure ([`CliError::Runtime`]): a parse/serialize
//!   diagnostic, a pack-integrity failure, an I/O error, or a results-serialization
//!   error.
//! * **3** — a caller-set governor stopped a query ([`CliOutcome::BudgetExhausted`]).
//!   This one is **not** a [`CliError`] and never becomes one; see below.
//!
//! # There is no unsupported-regime exit code, and a refused regime is still named
//!
//! A third code (**3**) used to classify an entailment-regime boundary the CLI could
//! not cross: `owl-direct` and `rif` were refused because a `Regime` value carried
//! neither the query's class expressions nor a rule set. `purrdf_entail::materialize`
//! takes a `Materialization` now, which carries both, and
//! [`EntailmentPlan`](crate::reason::EntailmentPlan) is where the CLI supplies them —
//! so every one of the seven regimes MATERIALIZES and the code that classified their
//! refusal has nothing left to classify. What remains is ordinary: `--regime rif`
//! without `--rules` is an incomplete command line (exit 2), and an unreadable or
//! malformed rule document is a runtime failure (exit 1), exactly like an unreadable
//! input.
//!
//! [`entails`](crate::entails) asks a different question, and it is total over five of
//! the seven rather than all of them: `owl-direct` is directed by a query's class
//! expressions and `rif` entails under the caller's rule document, and "premise,
//! conclusion, regime" carries neither. That refusal comes back from the shared
//! `purrdf-validate` boundary as a message NAMING the regime, and it is a
//! [`CliError::Runtime`] (exit 1) rather than a fourth code: the CLI does not keep its
//! own list of which regimes that service serves, because a second list is a second
//! opinion, and it prints the boundary's own diagnostic instead. What it never does is
//! answer under a weaker regime and label the answer with the one the operator asked
//! for.
//!
//! # Exit **3** is a governed query that was cut short, and it is not a failure
//!
//! The two categories above are categories of FAILURE, and a governor trip is not one.
//! `--fuel`, `--deadline`, `--max-answers` and their siblings are a POLICY the caller set
//! on their own query. When one of them trips, nothing went wrong: the engine did exactly
//! what it was told, evaluated as far as the policy allowed, and handed back a certificate
//! saying what the rows it reached bound. That is a SUCCESSFUL run that a caller-set
//! ceiling stopped, and neither existing code can carry it.
//!
//! * **1 would be a lie.** It would put a truncated answer in the same bucket as a corrupt
//!   pack and an unparseable document, and a shell pipeline would have no way to tell "your
//!   query was cut short — here is the certified prefix on stdout" from "your query failed
//!   and there is nothing on stdout to read". The distinction is exactly the one a caller
//!   who set the budget needs to act on: raise the ceiling and re-run, versus fix the data.
//! * **0 would be worse.** A truncated answer reported as a complete one is silently wrong,
//!   and every consumer downstream believes it. Making that unrepresentable is the whole
//!   reason the engine returns
//!   [`GovernedOutcome`](purrdf_sparql_eval::GovernedOutcome) rather than a `Result`, and a
//!   process boundary that flattens the two back together undoes it.
//!
//! So the trip is a third code, and it is carried by [`CliOutcome`] rather than by
//! [`CliError`]: it never travels the `?` path, it is never printed with the `purrdf: `
//! error prefix, and the answers it certified are still written to stdout in the requested
//! serialization. Three things — and only three — distinguish a tripped run: the exit code,
//! the governor report on stderr, and the fact that stdout may hold fewer rows than the
//! query has answers.
//!
//! ## This amends the section above; it does not reopen it
//!
//! The argument against a third code still binds, because it was an argument about
//! something else. The code that used to be **3** classified an entailment-regime boundary
//! *the CLI decided for itself*, and it was removed because the CLI had stopped keeping its
//! own taxonomy of what the library can do — the boundary's own diagnostic said it better,
//! as a plain runtime failure. A budget trip is not a taxonomy the CLI keeps. It is an
//! outcome the ENGINE reports, in a type whose entire design states that it is neither a
//! result nor an error, and the exit code is the only channel a process boundary has for
//! carrying that distinction to a shell. The rule is unchanged in both directions: never
//! invent a category the library does not have, and never flatten one it does.
//!
//! The `From` conversions below let the pipeline propagate library errors with `?`.

use std::fmt;

use purrdf_core::{PackError, RdfDiagnostic};
use purrdf_entail::EntailError;

/// How a run that did **not** fail ended.
///
/// The success side of the exit contract. A command that returns this ran to the end of
/// what it was asked to do; the only question left is whether a caller-set governor
/// stopped the query on the way, which is a fact about the caller's policy rather than
/// about the run's health — see the module documentation for why that is a third exit code
/// and not a [`CliError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOutcome {
    /// The command produced everything it was asked for (exit code 0).
    Complete,
    /// A governor stopped a query before it finished. The answers it certified are on
    /// stdout and the governor report is on stderr (exit code 3).
    BudgetExhausted,
}

impl CliOutcome {
    /// The process exit code for this outcome (0 complete / 3 budget-exhausted).
    pub(crate) const fn exit_code(self) -> i32 {
        match self {
            Self::Complete => 0,
            Self::BudgetExhausted => 3,
        }
    }
}

/// A CLI-level failure, carrying its rendered message and its exit classification.
#[derive(Debug)]
pub enum CliError {
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

impl From<purrdf_rdf::TransportError> for CliError {
    fn from(error: purrdf_rdf::TransportError) -> Self {
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
