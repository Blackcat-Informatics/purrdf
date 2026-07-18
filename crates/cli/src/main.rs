// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `purrdf` command-line interface.
//!
//! A single `Source → [transform] → Sink` pipeline exposed as five subcommands:
//!
//! * `convert` — transcode RDF between the native syntaxes and the pack container;
//! * `query` — evaluate a SPARQL query over an RDF or pack source;
//! * `reason` — materialize an entailment regime's closure over a source graph;
//! * `project` — materialize a deterministic graph/tabular carrier archive;
//! * `lift` — reconstruct RDF from a strict bidirectional carrier.
//!
//! plus the global `--loss-ledger` flag, which surfaces the machine-readable
//! loss ledger for a conversion, projection, or lift.
//!
//! Exit codes: clap rejects a malformed command line with **2**; the pipeline maps
//! its own failures the same way — usage errors → **2**, an unsupported entailment
//! regime → **3**, every other runtime failure → **1** (see [`error::CliError`]).
//! Nothing is swallowed: the error's message is printed to stderr and its category
//! becomes the process exit code.

mod cli;
mod convert;
mod error;
mod format;
mod ledger;
mod projection;
mod query;
mod reason;
mod sink;
mod source;

use clap::Parser;
use purrdf_rdf::JsonLdSerializeOptions;

use crate::cli::{Cli, Command};
use crate::error::CliError;

fn main() {
    let parsed = Cli::parse();
    if let Err(error) = dispatch(&parsed) {
        eprintln!("purrdf: {error}");
        std::process::exit(error.exit_code());
    }
}

/// Route a parsed command line to its subcommand, threading the decoded global
/// `--loss-ledger` target through.
fn dispatch(cli: &Cli) -> Result<(), CliError> {
    let ledger_target = cli.ledger_target();
    let jsonld_options = cli
        .jsonld_options
        .as_ref()
        .map(|path| {
            let bytes = std::fs::read(path)?;
            JsonLdSerializeOptions::from_json(&bytes).map_err(CliError::from)
        })
        .transpose()?;
    match &cli.cmd {
        Command::Convert {
            from,
            to,
            base,
            entailment,
            canonical,
            input,
            output,
        } => convert::run(
            &convert::ConvertOptions {
                from: *from,
                to: *to,
                base: base.as_deref(),
                entailment: *entailment,
                canonical: *canonical,
                jsonld_options: jsonld_options.as_ref(),
            },
            input,
            output,
            &ledger_target,
        ),
        Command::Query {
            data,
            base,
            entailment,
            results_format,
            query,
        } => query::run(
            data,
            base.as_deref(),
            *entailment,
            *results_format,
            query,
            jsonld_options.as_ref(),
            &ledger_target,
        ),
        Command::Reason {
            regime,
            from,
            to,
            base,
            input,
            output,
        } => reason::run(
            *regime,
            *from,
            *to,
            base.as_deref(),
            input,
            output,
            jsonld_options.as_ref(),
            &ledger_target,
        ),
        Command::Project {
            profile,
            config,
            from,
            base,
            input,
            output,
        } => projection::run_project(
            *profile,
            config,
            *from,
            base.as_deref(),
            input,
            output,
            jsonld_options.as_ref(),
            &ledger_target,
        ),
        Command::Lift {
            profile,
            config,
            to,
            base,
            input,
            output,
        } => projection::run_lift(
            *profile,
            config,
            *to,
            base.as_deref(),
            input,
            output,
            jsonld_options.as_ref(),
            &ledger_target,
        ),
    }
}
