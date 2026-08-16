// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `consistency` subcommand: `Source → decide → verdict + certificate`.
//!
//! This is the one DL question `reason` and `entails` cannot reach. `reason --regime
//! owl-direct` runs the tableau's query-independent augmentation and REFUSES an
//! inconsistent ontology outright (an inconsistent knowledge base entails every triple,
//! so there is no closure to materialize); `entails` decides whether a premise entails a
//! conclusion, which presupposes a premise WITH a model. Neither can be asked "does this
//! ontology have a model at all", because both are built on top of an answer to that
//! question rather than able to give it. Before this subcommand existed the only way to
//! reach it from the command line was indirectly — run `reason` or `entails` and read the
//! refusal — which yields a bare verdict with no certificate behind it, and no way to
//! exercise the consistency search on its own, apart from the entailment machinery layered
//! on top of it.
//!
//! # The same boundary every other host calls
//!
//! [`purrdf_validate::regime::consistency_to_string`] is the one string boundary the
//! Python, WebAssembly and C-ABI hosts already reach for this question, so a verdict this
//! binary prints is byte-for-byte the verdict those three print for the same document.
//! There is no second decision here, only the CLI's own format resolution in front of it.
//!
//! # Reading the input: the same lossless N-Quads crossing `entails` uses
//!
//! The boundary parses N-Quads (which accepts N-Triples unchanged), so a caller handing
//! this command Turtle, RDF/XML, JSON-LD or a verified pack crosses into it exactly as
//! `entails` crosses its premise: resolved through `--from`/the path's extension, parsed
//! with the native codecs, and re-serialized into N-Quads. That crossing is lossless by
//! construction — N-Quads carries named graphs, the RDF 1.2 statement layer and literal
//! base direction — and a REALIZED drop is refused rather than recorded, because a lossily
//! transcoded ontology is a different ontology and the verdict would be about that one.
//!
//! # What comes out, and why the certificate is not behind `--report`
//!
//! Both the one-line verdict and the full DL certificate go to stdout, unconditionally.
//! Every other reasoning subcommand puts its certificate behind `--report`, decoded through
//! a silent/stderr/file tri-state, because their primary output is a DOCUMENT (a closure, a
//! verdict against a caller's own conclusion) that a script consumes and the certificate is
//! secondary evidence a caller may or may not want. `consistency` has no document: the
//! certificate — completeness, the reverse mapping's boundary list, the search-cost
//! counters — IS the second half of the answer, not evidence about a first half sitting
//! beside it. Hiding it behind a flag on a command whose whole purpose is a one-shot,
//! by-hand reproduction would recreate exactly the ambiguity ("the reasoner says no" —
//! but a decided no, or a budget it ran out of?) the certificate exists to remove, for the
//! one caller who most needs it in hand without an extra flag to remember.
//!
//! # Exit codes: `true`/`false` are both decided, `unknown` is a trip
//!
//! `true` and `false` both exit **0**: each is a DECIDED verdict, and a decided `false` is
//! no more a failure of this command than a `false` ASK answer is a failure of `query`.
//! `unknown` exits **3**, the same code `query`/`update` use when a caller-set governor
//! stops a run short — the certificate's `completeness budget-exhausted` line says a
//! hypertableau run reached its round cap or its work cap before saturating, which is a run
//! that stopped incomplete rather than one that failed, exactly the distinction exit 3 exists
//! to carry. The certificate's four budget lines say WHICH cap: an exhausted run has `steps`
//! at `budget`, or `work` at `work-budget`.

use purrdf_rdf::{JsonLdSerializeOptions, NativeRdfFormat, serialize_dataset_to_format};
use purrdf_validate::regime::consistency_to_string;

use crate::cli::{CliRdfFormat, LedgerTarget};
use crate::error::{CliError, CliOutcome};
use crate::format;
use crate::sink;
use crate::source;

/// The resolved `consistency` flags.
///
/// Grouped for the reason [`crate::convert::ConvertOptions`] is: it keeps [`run`]'s
/// signature small enough to read.
pub(crate) struct ConsistencyOptions<'a> {
    /// The input path `IN`, or `-` for stdin (which requires `--from`).
    pub(crate) input: &'a str,
    /// `--from`: the input-format override; inferred from `input`'s extension when absent.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the base IRI relative IRIs in the input resolve against.
    pub(crate) base: Option<&'a str>,
    /// `--step-cap`: narrows the per-decision round cap the ontology's own size already
    /// derives. `0` (clap's default) applies no narrowing, mirroring the `step_cap`
    /// parameter [`consistency_to_string`] takes.
    pub(crate) step_cap: u32,
    /// `--work-cap`: narrows the per-decision WORK cap the ontology's own size already
    /// derives, on the same `0`-means-no-narrowing rule and mirroring the `work_cap`
    /// parameter [`consistency_to_string`] takes. It bounds the matcher, scan, closure and
    /// clone work done INSIDE a round, which the round cap cannot see.
    pub(crate) work_cap: u32,
}

/// Run the `consistency` subcommand.
pub(crate) fn run(
    options: &ConsistencyOptions<'_>,
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<CliOutcome, CliError> {
    refuse_document_flags(ledger_target, jsonld_options)?;
    let document = read_as_nquads(options)?;
    let answer = consistency_to_string(&document, options.step_cap, options.work_cap)
        .map_err(CliError::Runtime)?;

    let mut rendered = String::with_capacity(answer.answer().len() + answer.certificate().len());
    rendered.push_str(answer.answer());
    rendered.push_str(answer.certificate());
    sink::write_out("-", rendered.as_bytes())?;

    // `unknown` is the certificate's own word for "at least one hypertableau run reached
    // its round cap" — read from the rendered certificate rather than re-parsing the
    // verdict line, so this reads the exact fact `completeness` states rather than
    // inferring it from the three-valued text a second time.
    if answer
        .certificate()
        .contains("\ncompleteness budget-exhausted\n")
    {
        Ok(CliOutcome::BudgetExhausted)
    } else {
        Ok(CliOutcome::Complete)
    }
}

/// Refuse the two global document flags, which name outputs this command does not produce.
///
/// Identical rationale to `entails`'s `refuse_document_flags`: `--loss-ledger` records what
/// a CONVERSION dropped, and this command converts nothing for the operator — it decides a
/// question and writes a verdict plus a certificate, neither of which is RDF, and its own
/// crossing into the boundary's N-Quads is lossless by construction or the run is refused
/// (see [`read_as_nquads`]), so there is no ledger. `--jsonld-options` configures a JSON-LD/
/// YAML-LD serializer, and no serializer runs here. Both flags are GLOBAL — clap accepts
/// them on every subcommand — so an unrefused one would be silently ignored, which is the
/// no-op this repository refuses everywhere else.
fn refuse_document_flags(
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    if !matches!(ledger_target, LedgerTarget::Silent) {
        return Err(CliError::Usage(
            "--loss-ledger records what a conversion dropped, and `consistency` converts \
             nothing for you: it decides a question and prints a verdict plus its \
             certificate. The document it reads crosses into the boundary's N-Quads \
             losslessly or the run is refused, so there is no ledger to surface"
                .to_owned(),
        ));
    }
    if jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--jsonld-options configures a JSON-LD/YAML-LD serializer, and `consistency` \
             runs none: its output is a line-oriented verdict and certificate, not RDF"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Read `options.input` through the CLI's own format resolution and re-serialize it as
/// N-Quads — the one media type [`consistency_to_string`]'s boundary parses.
///
/// Identical in shape to `entails`'s `read_as_nquads`: the resolution is `convert`'s and
/// `reason`'s ([`format::resolve`]), the parse is the native codecs' (including a verified
/// pack), and a REALIZED drop (an RDF-1.2 statement-layer row or a base-direction literal
/// the target format cannot carry) is refused rather than recorded, because a lossily
/// transcoded ontology is a different ontology and the verdict would be about that one.
fn read_as_nquads(options: &ConsistencyOptions<'_>) -> Result<String, CliError> {
    let format = format::resolve(options.from, options.input)?;
    format::refuse_base_with_pack(format, options.base, "a pack --from source")?;
    let dataset = source::load_dataset(options.input, format, options.base)?;
    let outcome = serialize_dataset_to_format(&*dataset, NativeRdfFormat::NQuads, None)?;
    if outcome.statement_rows_dropped > 0 || outcome.directional_literals_dropped > 0 {
        return Err(CliError::Runtime(format!(
            "{}: reading this document into the consistency boundary's N-Quads dropped {} \
             statement-layer row(s) and {} literal base direction(s). The decided document \
             would not be the one you named, so the run is refused rather than answered about \
             something else",
            options.input, outcome.statement_rows_dropped, outcome.directional_literals_dropped
        )));
    }
    String::from_utf8(outcome.bytes).map_err(|error| {
        CliError::Runtime(format!(
            "{}: N-Quads output is not UTF-8: {error}",
            options.input
        ))
    })
}
