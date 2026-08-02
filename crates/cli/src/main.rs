// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `purrdf` command-line interface.
//!
//! A single `Source → [transform] → Sink` pipeline exposed as six subcommands:
//!
//! * `convert` — transcode RDF between the native syntaxes and the pack container;
//! * `query` — evaluate a SPARQL query over an RDF or pack source;
//! * `reason` — materialize an entailment regime's closure over a source graph;
//! * `entails` — decide whether a premise entails a conclusion, or answer a basic
//!   graph pattern's certain answers, under an entailment regime;
//! * `project` — materialize a deterministic graph/tabular carrier archive;
//! * `lift` — reconstruct RDF from a strict bidirectional carrier.
//!
//! `reason` and `entails` are the two halves of entailment and neither is the
//! other: `reason` computes a CLOSURE, which is what a caller wants who will go on
//! asking many questions of one premise, and `entails` decides ONE question, which
//! is not the membership test in that closure it looks like — see
//! [`entails`] for why.
//!
//! plus the global `--loss-ledger` flag, which surfaces the machine-readable
//! loss ledger for a conversion, projection, or lift, and the `--report` flag the
//! four reasoning subcommands carry, which surfaces the reasoning certificate:
//! which rules fired, which constructs the run could not fully handle, what it
//! cost, and the contract hash of the calculus that produced the closure.
//!
//! Exit codes: clap rejects a malformed command line with **2**; the pipeline maps
//! its own failures the same way — usage errors → **2**, every other runtime
//! failure → **1** (see [`error::CliError`]). Nothing is swallowed: the error's
//! message is printed to stderr and its category becomes the process exit code.

mod cli;
mod convert;
mod entails;
mod error;
mod format;
mod ledger;
mod projection;
mod query;
mod reason;
mod report;
mod sink;
mod source;

use std::fs::File;
use std::io::Read as _;

use clap::Parser;
use purrdf_rdf::{JsonLdContextLimits, JsonLdSerializeOptions};

use crate::cli::{Cli, Command, ReportTarget};
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
            let limit = JsonLdContextLimits::default().max_options_bytes();
            let mut bytes = Vec::new();
            File::open(path)?
                .take(u64::try_from(limit).expect("options byte limit fits u64") + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                return Err(CliError::Runtime(format!(
                    "JSON-LD options document exceeds the {limit}-byte limit"
                )));
            }
            JsonLdSerializeOptions::from_json(&bytes).map_err(CliError::from)
        })
        .transpose()?;
    match &cli.cmd {
        Command::Convert {
            from,
            to,
            base,
            entailment,
            rules,
            report,
            canonical,
            input,
            output,
        } => convert::run(
            &convert::ConvertOptions {
                from: *from,
                to: *to,
                base: base.as_deref(),
                entailment: *entailment,
                rules: rules.as_deref(),
                canonical: *canonical,
                jsonld_options: jsonld_options.as_ref(),
            },
            input,
            output,
            &ledger_target,
            &ReportTarget::decode(report.as_ref()),
        ),
        Command::Query {
            data,
            base,
            entailment,
            rules,
            report,
            results_format,
            query,
        } => query::run(
            data,
            base.as_deref(),
            *entailment,
            rules.as_deref(),
            *results_format,
            query,
            jsonld_options.as_ref(),
            &ledger_target,
            &ReportTarget::decode(report.as_ref()),
        ),
        Command::Reason {
            regime,
            rules,
            report,
            from,
            to,
            base,
            input,
            output,
        } => reason::run(
            *regime,
            rules.as_deref(),
            *from,
            *to,
            base.as_deref(),
            input,
            output,
            jsonld_options.as_ref(),
            &ledger_target,
            &ReportTarget::decode(report.as_ref()),
        ),
        Command::Entails {
            regime,
            premise,
            conclusion,
            pattern,
            verify,
            imports,
            report,
            from,
            base,
            output,
        } => entails::run(
            &entails::EntailsOptions {
                regime: *regime,
                premise,
                conclusion: conclusion.as_deref(),
                pattern: pattern.as_deref(),
                verify: *verify,
                imports,
                from: *from,
                base: base.as_deref(),
                jsonld_options: jsonld_options.as_ref(),
            },
            output,
            &ledger_target,
            &ReportTarget::decode(report.as_ref()),
        ),
        Command::Project {
            profile,
            config,
            assets,
            from,
            base,
            input,
            output,
        } => projection::run_project(
            *profile,
            config,
            assets.as_deref(),
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
