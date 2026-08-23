// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `purrdf` command-line interface.
//!
//! A single `Source → [transform] → Sink` pipeline exposed as eight subcommands:
//!
//! * `convert` — transcode RDF between the native syntaxes and the pack container;
//! * `query` — evaluate a SPARQL query over an RDF or pack source;
//! * `update` — atomically apply a SPARQL UPDATE to an RDF source;
//! * `reason` — materialize an entailment regime's closure over a source graph;
//! * `entails` — decide whether a premise entails a conclusion, or answer a basic
//!   graph pattern's certain answers, under an entailment regime;
//! * `consistency` — decide whether an OWL-Direct ontology has a model at all;
//! * `project` — materialize a deterministic graph/tabular carrier archive;
//! * `lift` — reconstruct RDF from a strict bidirectional carrier.
//!
//! `reason` and `entails` are the two halves of entailment and neither is the
//! other: `reason` computes a CLOSURE, which is what a caller wants who will go on
//! asking many questions of one premise, and `entails` decides ONE question, which
//! is not the membership test in that closure it looks like — see
//! [`entails`] for why. `consistency` is the question neither of those two can
//! answer: an inconsistent ontology has no closure for `reason` to materialize and
//! no closure for `entails` to decide a conclusion against, so it is its own
//! subcommand rather than a mode of either — see [`consistency`].
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
//! A `query` or `update` whose caller-set governor tripped is not a failure and exits
//! **3**. A query carries its certified answers on stdout; an update emits no dataset
//! because the mutation was not applied — see [`error::CliOutcome`]. `consistency`
//! reuses the same **3**, for the same reason, when its answer is `unknown`: the
//! hypertableau reached its round cap rather than saturating, and `true`/`false` both
//! exit **0** as decided verdicts.

mod cli;
mod consistency;
mod convert;
mod entails;
mod error;
mod format;
mod governors;
pub mod immutable;
mod ledger;
mod projection;
mod query;
mod reason;
mod report;
mod sink;
mod source;
mod update;

use std::fs::File;
use std::io::Read as _;

use clap::Parser as _;
use purrdf_rdf::{JsonLdContextLimits, JsonLdSerializeOptions};

use crate::cli::{Cli, Command, ReportTarget};
use crate::error::{CliError, CliOutcome};
use crate::governors::GovernorFlags;

/// The `purrdf` pipeline entry point: parse the command line, dispatch it, and map
/// the outcome to a process exit code.
///
/// The thin `main.rs` bin is exactly a call to this; keeping the whole pipeline in
/// the library is what makes it reachable by the crate's benchmarks (the pack input
/// path) and integration tests, not only by the binary.
pub fn run() {
    let parsed = Cli::parse();
    match dispatch(&parsed) {
        // The ordinary success path returns, so stdout's own drop-time flush still
        // runs; every write the pipeline makes is already flushed by `sink::write_out`.
        Ok(CliOutcome::Complete) => {}
        // A tripped governor is not an error: the lane that tripped already wrote its
        // report to stderr; a query wrote its answers to stdout and an update wrote no
        // dataset. Only the exit code is left to carry.
        Ok(outcome) => std::process::exit(outcome.exit_code()),
        Err(error) => {
            eprintln!("purrdf: {error}");
            std::process::exit(error.exit_code());
        }
    }
}

/// Route a parsed command line to its subcommand, threading the decoded global
/// `--loss-ledger` target through.
///
/// Every arm but `query` and `update` reports [`CliOutcome::Complete`]: a governor bounds
/// a SPARQL evaluation or mutation, and the remaining subcommands run neither, so there
/// is no outcome of theirs a third exit code could describe.
fn dispatch(cli: &Cli) -> Result<CliOutcome, CliError> {
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
        )
        .map(|()| CliOutcome::Complete),
        Command::Query {
            data,
            base,
            entailment,
            rules,
            report,
            results_format,
            fuel,
            deadline,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
            explain,
            aggregate_namespace,
            provenance_namespace,
            query,
        } => query::run(
            &query::QueryOptions {
                data,
                base: base.as_deref(),
                entailment: *entailment,
                rules: rules.as_deref(),
                results_format: *results_format,
                query,
                governors: GovernorFlags {
                    fuel: *fuel,
                    deadline: *deadline,
                    max_answers: *max_answers,
                    max_intermediate_cells: *max_intermediate_cells,
                    max_scratch_bytes: *max_scratch_bytes,
                    max_remote_requests: *max_remote_requests,
                },
                explain: *explain,
                jsonld_options: jsonld_options.as_ref(),
                aggregate_namespace: aggregate_namespace.as_deref(),
                provenance_namespace: provenance_namespace
                    .as_ref()
                    .map(|(prefix, iri)| (prefix.as_str(), iri.as_str())),
            },
            &ledger_target,
            &ReportTarget::decode(report.as_ref()),
        ),
        Command::Update {
            data,
            from,
            output,
            to,
            base,
            fuel,
            deadline,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
            aggregate_namespace,
            update,
        } => update::run(
            &update::UpdateOptions {
                data,
                from: *from,
                output,
                to: *to,
                base: base.as_deref(),
                update,
                governors: GovernorFlags {
                    fuel: *fuel,
                    deadline: *deadline,
                    max_answers: None,
                    max_intermediate_cells: *max_intermediate_cells,
                    max_scratch_bytes: *max_scratch_bytes,
                    max_remote_requests: *max_remote_requests,
                },
                jsonld_options: jsonld_options.as_ref(),
                aggregate_namespace: aggregate_namespace.as_deref(),
            },
            &ledger_target,
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
        )
        .map(|()| CliOutcome::Complete),
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
        )
        .map(|()| CliOutcome::Complete),
        Command::Consistency {
            step_cap,
            work_cap,
            from,
            base,
            input,
        } => consistency::run(
            &consistency::ConsistencyOptions {
                input,
                from: *from,
                base: base.as_deref(),
                step_cap: *step_cap,
                work_cap: *work_cap,
            },
            &ledger_target,
            jsonld_options.as_ref(),
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
        )
        .map(|()| CliOutcome::Complete),
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
        )
        .map(|()| CliOutcome::Complete),
    }
}
