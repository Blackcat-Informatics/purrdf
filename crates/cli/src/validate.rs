// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `validate` subcommand: `Data + Shapes → decide → validation report`.
//!
//! SHACL validation on the command line, over the same `purrdf_shapes::engine` every other
//! PurRDF host reaches. There is no CLI-local validator, no CLI-local SARIF mapping and no
//! second opinion about what conforms: this module resolves formats, reads the two graphs,
//! calls the engine once, and serializes what it hands back.
//!
//! # The canonical output is the SHACL results GRAPH; SARIF is a named projection
//!
//! `--format` chooses between two artifacts that already ship, and the default is the results
//! graph. The SHACL specification defines the validation process to produce a **validation
//! report** — an RDF graph of `sh:ValidationResult` nodes under a `sh:ValidationReport` — and
//! that graph is the answer written in the language of the question: focus node, value node,
//! result path, source shape and source constraint component are RDF terms, not strings. It
//! also composes with the rest of this binary, because it is ordinary RDF: `purrdf query` can
//! query a report, `purrdf convert` can transcode one, and `purrdf validate` can validate one.
//!
//! `sarif` projects the same report through [`purrdf_validate::report_to_sarif_string`] — the
//! writer the WebAssembly and C-ABI hosts already use — into the vocabulary an editor or a
//! code-scanning dashboard reads. That projection is deliberately lossy in the direction that
//! decides this question: several SHACL severities collapse onto SARIF's three levels (the
//! verbatim severity IRI survives only in a property bag) and the RDF term structure becomes
//! strings. Lossy-for-a-purpose is exactly right for the CI consumer and exactly wrong for the
//! artifact everything else derives from, so SARIF is an opt-in rather than the default.
//!
//! The SARIF arm calls `report_to_sarif_string` over the report this lane already holds rather
//! than [`purrdf_validate::validate_to_sarif_string`], which takes a Turtle shapes string and
//! an N-Triples data string. That is the same function `validate_to_sarif_string` calls
//! internally — so no behavior diverges — and reaching it directly is what lets `--format
//! sarif` work over a verified PACK data source and a non-Turtle shapes graph, neither of
//! which the string-in boundary can express.
//!
//! # Both verdicts exit 0, and the verdict is on stderr regardless
//!
//! A non-conforming graph is a DECIDED verdict, not a failure of this command — the same
//! position `consistency true|false` and a `false` ASK already hold in this binary, and the
//! same reason: the report on stdout is the answer either way, and mapping "the data violates
//! your shapes" onto an error code would put it in the same bucket as a corrupt pack. So exit
//! **0** covers both, **1** is a malformed document or an unsupported SHACL construct, **2** is
//! a usage error, and **3** is a tripped governor.
//!
//! Because stdout carries a well-formed RDF or SARIF document, the verdict cannot be
//! interleaved into it — so it is written to **stderr**, always, as two `key value` lines
//! (`shacl conforms true|false`, `shacl results N`). Unconditional rather than behind a flag,
//! for the reason `consistency` prints its certificate unconditionally: an operator running
//! this by hand needs the answer in hand, and a shell that wants to branch on conformance
//! should not have to parse the artifact to find it.
//!
//! # A tripped governor produces no report at all
//!
//! `purrdf_shapes::engine::GovernedValidation` deliberately has no partial-report variant.
//! Every SHACL constraint is a negative claim, so a truncated solution bag and a complete one
//! that found nothing produce the identical sentence and a `conforms` computed from the first
//! means nothing. This lane carries that through unflattened: on a trip stdout gets NOTHING,
//! stderr gets the governor receipt, and the exit code is 3.
//!
//! # Reading the two graphs
//!
//! The DATA graph is read through the pipeline's own format resolution into the frozen IR —
//! any of the nine native syntaxes or a verified pack — and handed to the engine as an
//! `RdfDataset`. Nothing is transcoded on the way in, so unlike `entails`/`consistency` there
//! is no lossless-or-refused crossing to make: the engine reads the same IR the parser built.
//!
//! The SHAPES graph is read through [`load_shapes`], which routes Turtle to
//! [`purrdf_shapes::engine::parse_shapes`] — byte-for-byte the boundary every other host uses,
//! and the one that additionally recovers the shapes document's own `@prefix`/`PREFIX` map as
//! the fallback prefix environment for SHACL-AF `sh:select` queries. Every other syntax is
//! parsed by the native codec into the same IR and given to
//! [`purrdf_shapes::shapes::from_dataset`], which carries no document-prefix fallback because
//! that fallback is a recovery from Turtle SOURCE TEXT and there is none to recover from. That
//! difference is stated on `--shapes-from`'s help rather than discovered.
//!
//! The prefix fallback is the ONLY difference between the two routes. Both derive the
//! shapes document's own `file://` retrieval IRI and resolve relative IRI references
//! against it, so `<PersonShape>` in a shapes graph means the same term whether the file
//! is read as Turtle or as TriG. It did not always: the Turtle route reached a
//! `parse_shapes` that took no base at all, so the one document in this command that
//! could not resolve a relative IRI was the one describing the constraints.
//!
//! # `--shapes-graph` is command-line text, and its refusal says so
//!
//! `--shapes-graph` names the graph the shapes document is exposed under, overriding a
//! `sh:shapesGraph` that document declares. That declaration is an IRI *inside* the shapes
//! document, so it resolves against the shapes document's base — and the flag that overrides
//! it resolves against the SAME base, through [`resolve_shapes_graph`]. `--shapes-graph
//! sg` therefore names what `sh:shapesGraph <sg>` written in that document names, and an
//! absolute value is carried lexical-verbatim (`BaseScope::resolve`'s own contract), so
//! nothing about an already-absolute invocation changes.
//!
//! A relative value with NO base in scope — a `--shapes -` stdin shapes graph, or a pack —
//! is a hard usage error (exit 2) decided before a byte of either document is read. It used
//! to travel all the way into `RdfDatasetBuilder::freeze`, which refused it as an
//! un-internable IRI TERM and advised adding an `@base`/`xml:base` DOCUMENT directive. No
//! directive an operator writes in any document can fix an argv string, so the remedy named
//! a fix that could not be applied; the refusal here names the flag, keeps the shared
//! `purrdf_iri` diagnostic code, and names a remedy that exists on the surface the value
//! came from.

use std::sync::Arc;

use purrdf::shapes::engine::{self, GovernedValidation};
use purrdf::shapes::report::ValidationReport;
use purrdf::shapes::shapes::Shapes;
use purrdf_core::RdfDataset;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
use purrdf_rdf::{JsonLdSerializeOptions, NativeRdfFormat, SourceFormat, parse_dataset};
use purrdf_validate::SarifOptions;

use crate::cli::{CliRdfFormat, LedgerTarget, ValidateFormat};
use crate::error::{CliError, CliOutcome};
use crate::governors::{self, GovernorFlags};
use crate::{format, ledger, sink, source};

/// The resolved `validate` flags.
///
/// Grouped for the reason [`crate::convert::ConvertOptions`] is: it keeps [`run`]'s signature
/// small enough to read, and every field is borrowed from the parsed command line.
pub(crate) struct ValidateOptions<'a> {
    /// The data-graph path `IN`, or `-` for stdin (which requires `--from`).
    pub(crate) input: &'a str,
    /// The report path `OUT`, or `-` for stdout.
    pub(crate) output: &'a str,
    /// `--shapes`: the SHACL shapes graph, or `-`.
    pub(crate) shapes: &'a str,
    /// `--shapes-from`: the shapes-graph format override.
    pub(crate) shapes_from: Option<CliRdfFormat>,
    /// `--shapes-graph`: the IRI the shapes graph is exposed under to SHACL-SPARQL paths,
    /// as the operator wrote it. [`resolve_shapes_graph`] turns it into the absolute IRI
    /// the engine is handed.
    pub(crate) shapes_graph: Option<&'a str>,
    /// `--from`: the data-graph format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the base IRI relative IRIs in the DATA graph resolve against.
    pub(crate) base: Option<&'a str>,
    /// `--format`: which artifact the report is serialized as.
    pub(crate) format: ValidateFormat,
    /// The five execution governors this subcommand carries.
    pub(crate) governors: GovernorFlags,
    /// Explicit JSON-LD/YAML-LD serialization configuration for a JSON-LD/YAML-LD `--format`.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Run the `validate` subcommand.
pub(crate) fn run(
    options: &ValidateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    refuse_two_stdins(options)?;
    refuse_inapplicable_flags(options, ledger_target)?;

    let data_format = format::resolve(options.from, options.input)?;
    // The DATA parse is the only leg `--base` has here: the shapes graph resolves against
    // its own retrieval IRI, and the validation report is a fresh graph `emit` serializes
    // with no base at all (it passes `None`). So a data syntax that admits no relative IRI
    // leaves the flag with nowhere to go.
    format::refuse_unconsumable_base(
        options.base,
        &[format::BaseUse::parse(data_format, "the --from data graph")],
    )?;
    let shapes_format = format::resolve(options.shapes_from, options.shapes)?;
    // The shapes document's base is derived ONCE and spent twice: the document parses under
    // it, and `--shapes-graph` resolves against it. Deriving it separately per consumer is
    // how the flag would come to name a graph the shapes document cannot.
    let shapes_base = shapes_document_base(options.shapes, shapes_format)?;
    // Decided BEFORE either document is read: a `--shapes-graph` that names no graph is a
    // malformed request, and it should fail against the command line rather than after the
    // data has been parsed and validated.
    let shapes_graph = resolve_shapes_graph(options.shapes_graph, shapes_base.as_deref())?;

    let data = source::load_dataset(options.input, data_format, options.base)?;
    let shapes = load_shapes(options.shapes, shapes_format, shapes_base.as_deref())?;

    let Some(report) = validate(&data, &shapes, options, shapes_graph.as_deref())? else {
        // A tripped governor: the receipt is already on stderr and there is no report to
        // write, by the engine's own design. Exit 3 carries that to the shell.
        return Ok(CliOutcome::BudgetExhausted);
    };

    emit(&report, options, ledger_target)?;
    // The verdict AFTER the artifact, so a `> file` redirect has the report on disk by the
    // time the operator reads the line that describes it.
    eprintln!("shacl conforms {}", report.conforms);
    eprintln!("shacl results {}", report.results.len());
    Ok(CliOutcome::Complete)
}

/// Run the engine, returning `None` when a governor stopped it (having written the receipt).
///
/// Both lanes call the `_with_shapes_graph` shape of the engine entry point, so engaging a
/// governor cannot change WHAT is validated — only whether the run is bounded. Passing
/// `--shapes-graph`'s `None` still honors a `sh:shapesGraph` the shapes document declares,
/// which is the engine's own default and not a CLI decision.
fn validate(
    data: &Arc<RdfDataset>,
    shapes: &Shapes,
    options: &ValidateOptions<'_>,
    shapes_graph: Option<&str>,
) -> Result<Option<ValidationReport>, CliError> {
    if !options.governors.is_engaged() {
        return engine::validate_dataset_with_shapes_graph(data, shapes, shapes_graph)
            .map(Some)
            .map_err(CliError::Runtime);
    }

    let governed = engine::validate_dataset_with_governors(
        data,
        shapes,
        shapes_graph,
        &options.governors.to_governors(),
    )
    .map_err(CliError::Runtime)?;

    match governed {
        GovernedValidation::Complete { report, .. } => Ok(Some(report)),
        GovernedValidation::BudgetExhausted { tripped, evidence } => {
            eprint!("{}", governors::render_validation_trip(tripped, &evidence));
            Ok(None)
        }
    }
}

/// Serialize `report` to `--format` and write it to `OUT`.
///
/// The RDF arm re-reads the engine's own `to_ntriples` rendering into the IR and hands it to
/// the shared [`sink`], so every one of the nine syntaxes is produced by the SAME serializer
/// every other subcommand uses — including the loss ledger, which records what the target
/// syntax could not carry. Going through the IR even for `--format ntriples` is deliberate:
/// one path means `ntriples` and `turtle` describe the same graph rather than one being the
/// engine's rendering and the other a transcode of it.
fn emit(
    report: &ValidationReport,
    options: &ValidateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let Some(target) = options.format.to_rdf_format() else {
        // SARIF: not RDF, so no sink, no ledger, no serializer configuration.
        let sarif = purrdf_validate::report_to_sarif_string(report, &SarifOptions::default());
        return sink::write_out(options.output, sarif.as_bytes());
    };

    let nt = report.to_ntriples();
    let graph = parse_dataset(nt.as_bytes(), NativeRdfFormat::NTriples.media_type(), None)?;
    let ledger = sink::write_rdf(
        &*graph,
        options.output,
        SourceFormat::Native(target),
        None,
        NativeRdfFormat::NTriples.loss_codec_name(),
        options.jsonld_options,
    )?;
    ledger::surface(ledger_target, &ledger)
}

/// Read the shapes graph at `path` into a parsed [`Shapes`].
///
/// See the module documentation for why Turtle takes a different (and privileged) route than
/// the other syntaxes.
///
/// # Both routes resolve relative IRIs against the same base
///
/// A shapes graph is an RDF document like any other, and its author may write
/// `<PersonShape>`. Both arms therefore derive the shapes document's own RFC-8089
/// `file://` retrieval IRI through [`source::effective_base`] — the SAME derivation the
/// data graph gets — so identical bytes resolve identically whichever syntax they are
/// labelled with. `--shapes -` has no retrieval IRI, so a relative reference there is a
/// hard `iri-relative-no-base` naming the remedy, never a vacuous pass.
///
/// The Turtle arm used to call `parse_shapes` with no base at all while the non-Turtle
/// arm passed a literal `None`, so the shapes graph was the one input in this binary that
/// could not resolve a relative IRI. Threading the base is what deletes that asymmetry;
/// `--base` is deliberately NOT used here, because it names the base of the DATA graph
/// and the two documents are independent.
///
/// The base is a parameter rather than derived here because [`run`] spends the same value
/// on `--shapes-graph` as well (see [`resolve_shapes_graph`]): one derivation is what keeps
/// the flag and the document agreeing about what a relative IRI denotes.
fn load_shapes(path: &str, format: SourceFormat, base: Option<&str>) -> Result<Shapes, CliError> {
    if format == SourceFormat::Native(NativeRdfFormat::Turtle) {
        let bytes = source::read_bytes(path)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            CliError::Runtime(format!("--shapes {path}: not UTF-8 text: {error}"))
        })?;
        return engine::parse_shapes(&text, base)
            .map_err(|error| CliError::Runtime(format!("--shapes {path}: {error}")));
    }

    let dataset = source::load_dataset(path, format, base)?;
    purrdf::shapes::shapes::from_dataset(&dataset)
        .map_err(|error| CliError::Runtime(format!("--shapes {path}: {error}")))
}

/// The base the SHAPES document parses under, and the base `--shapes-graph` resolves
/// against.
///
/// It is [`source::effective_base`] with an explicit `None` for `--base`: that flag names
/// the base of the DATA graph, and silently retargeting a second document with it is the
/// confusion `--base`'s own help text promises this command does not create. A pack/GTS
/// container stores resolved IRIs and has no document base to derive.
fn shapes_document_base(path: &str, format: SourceFormat) -> Result<Option<String>, CliError> {
    match format {
        SourceFormat::Native(native) => source::effective_base(path, native, None),
        SourceFormat::Pack | SourceFormat::Gts => Ok(None),
    }
}

/// Resolve `--shapes-graph` against the shapes document's base.
///
/// An ABSOLUTE value is carried lexical-verbatim — [`BaseScope::resolve`]'s own contract —
/// so an already-absolute invocation is byte-for-byte what it always was. A RELATIVE one
/// resolves against the base the shapes document itself parses under, so `--shapes-graph sg`
/// names exactly what `sh:shapesGraph <sg>` written in that document names, which is the
/// declaration this flag overrides. A relative one with nothing in scope is refused.
fn resolve_shapes_graph(raw: Option<&str>, base: Option<&str>) -> Result<Option<String>, CliError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let scope = match base {
        // A derived retrieval IRI is produced by its own parse, so a failure here is not
        // reachable from the command line; it is still reported rather than unwrapped,
        // because an unreachable panic in a CLI is a crash report.
        Some(base) => BaseScope::rooted(
            BaseIri::parse(base).map_err(|error| {
                CliError::Usage(format!(
                    "the shapes graph's base `{base}` is not a usable base IRI: {error}"
                ))
            })?,
            BaseOrigin::Caller,
        ),
        None => BaseScope::empty(),
    };
    scope
        .resolve(raw)
        .map(|iri| Some(iri.as_str().to_owned()))
        .map_err(|error| shapes_graph_refusal(raw, &error))
}

/// The refusal for a `--shapes-graph` that names no graph.
///
/// It carries the shared [`purrdf_iri::IriError::diagnostic_code`] so it groups with every
/// other IRI failure in this toolkit, and it does NOT carry the library's own remedy for a
/// missing base: that one names `@base` and `xml:base`, which are DOCUMENT directives, and a
/// `--shapes-graph` value is argv text that no document can reach. Naming a fix the operator
/// cannot apply is worse than naming none — this is the same refusal shape `describe --iri`
/// carries, for the same reason.
fn shapes_graph_refusal(raw: &str, error: &purrdf_iri::IriError) -> CliError {
    let code = error.diagnostic_code();
    if code == "iri-relative-no-base" {
        return CliError::Usage(format!(
            "--shapes-graph `{raw}`: {code}: a relative IRI reference has no base in scope, so \
             it names no graph to expose the shapes under. This is a command-line value, so no \
             `@base` you write in a document resolves it: give --shapes a PATH, whose `file://` \
             retrieval IRI this flag resolves against exactly as a `sh:shapesGraph` inside that \
             document would, or write the graph name in absolute form"
        ));
    }
    CliError::Usage(format!("--shapes-graph `{raw}`: {code}: {error}"))
}

/// Refuse a command line that reads standard input twice.
///
/// The data graph and the shapes graph may each be `-`, and at most ONE of them may be: a
/// process has one standard input, so two documents naming it would each get part of one
/// stream. Refused naming both, exactly as `entails` refuses it, rather than mis-read.
fn refuse_two_stdins(options: &ValidateOptions<'_>) -> Result<(), CliError> {
    if options.input == "-" && options.shapes == "-" {
        return Err(CliError::Usage(
            "IN and --shapes both read standard input, and there is only one: a process has a \
             single stdin stream, so the data graph and the shapes graph would each get part \
             of one document. Give one of them a path"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Refuse the flags that name something this `--format` does not produce.
///
/// `--jsonld-options` configures a JSON-LD/YAML-LD serializer; under `--format sarif` no RDF
/// serializer runs at all, and under an RDF `--format` the shared
/// [`sink::validate_jsonld_options`] refuses every syntax but those two — the same check
/// `convert`/`reason` make, applied to the same target kind.
///
/// `--loss-ledger` records what a SERIALIZATION dropped. Under `--format sarif` nothing is
/// serialized as RDF, so a ledger would be structurally empty; an empty ledger written on
/// request is the silent no-op this repository refuses everywhere else, so the combination is
/// refused by name instead. Under an RDF `--format` the flag is fully live, because the
/// results graph crosses a real serializer and a star-incapable target really can drop rows.
fn refuse_inapplicable_flags(
    options: &ValidateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let Some(target) = options.format.to_rdf_format() else {
        if ledger_target.is_requested() {
            return Err(CliError::Usage(
                "--loss-ledger records what an RDF serialization dropped, and `--format sarif` \
                 runs none: the SARIF log is JSON, so its ledger would always be empty"
                    .to_owned(),
            ));
        }
        if options.jsonld_options.is_some() {
            return Err(CliError::Usage(
                "--jsonld-options configures a JSON-LD/YAML-LD serializer, and `--format sarif` \
                 runs none: the SARIF log is its own JSON schema, not JSON-LD"
                    .to_owned(),
            ));
        }
        return Ok(());
    };
    sink::validate_jsonld_options(SourceFormat::Native(target), options.jsonld_options)
}
