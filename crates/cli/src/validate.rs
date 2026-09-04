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
//! The SHAPES graph is read through [`load_shapes`], in two steps that
//! [`purrdf_shapes::engine::parse_shapes`] performs as one: [`read_shapes_document`] freezes
//! the document into a graph, and only then is that graph asked to be `Shapes`. Turtle
//! additionally carries its own `@prefix`/`PREFIX` map, recovered from the source text, as the
//! fallback prefix environment for SHACL-AF `sh:select` queries. Every other syntax is parsed
//! by the native codec into the same IR and carries no such fallback, because the fallback is
//! a recovery from Turtle SOURCE TEXT and there is none to recover from. That difference is
//! stated on `--shapes-from`'s help rather than discovered.
//!
//! The prefix fallback is the ONLY difference between the two routes. Both derive the
//! shapes document's own `file://` retrieval IRI and resolve relative IRI references
//! against it, so `<PersonShape>` in a shapes graph means the same term whether the file
//! is read as Turtle or as TriG. It did not always: the Turtle route reached a
//! `parse_shapes` that took no base at all, so the one document in this command that
//! could not resolve a relative IRI was the one describing the constraints.
//!
//! Splitting the read from the parse is what makes `owl:imports` resolvable at all: the
//! imports have to be read off the GRAPH, and the imported documents merged as graphs, before
//! anything is asked to be a shape. [`fold_shapes_imports`] walks that closure against the
//! `--import IRI=FILE` table — PurRDF fetches nothing — and a shapes graph with no imports
//! composes to exactly the `Shapes` `parse_shapes` produced before the seam existed.
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

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use purrdf::shapes::engine::{self, GovernedValidation};
use purrdf::shapes::report::ValidationReport;
use purrdf::shapes::shapes::Shapes;
use purrdf_core::RdfDataset;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
use purrdf_rdf::{JsonLdSerializeOptions, NativeRdfFormat, SourceFormat};
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
    /// `--import IRI=FILE`, repeatable: the local documents that resolve the shapes graph's
    /// `owl:imports`. Empty means the operator named none, which is the pre-flag behaviour
    /// plus a diagnostic — see [`fold_shapes_imports`].
    pub(crate) imports: &'a [String],
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
    let shapes = load_shapes(options, shapes_format, shapes_base.as_deref())?;

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
/// The RDF arm takes the engine's own report graph — `ValidationReport::to_dataset`, the IR
/// the report is built in — and hands it to the shared [`sink`], so every one of the nine
/// syntaxes is produced by the SAME serializer every other subcommand uses — including the
/// loss ledger, which records what the target syntax could not carry. Going through the IR
/// even for `--format ntriples` is deliberate: one path means `ntriples` and `turtle`
/// describe the same graph rather than one being the engine's rendering and the other a
/// transcode of it.
///
/// This used to serialize the report to N-Triples and re-parse that text. The parse was pure
/// waste — the report is *already* materialized as IR quads, and the text was only ever a
/// rendering of them — and it was lossy in principle: a round-trip relabels blank nodes and
/// is bounded by what the N-Triples grammar can carry.
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

    let graph = report.to_dataset();
    let ledger = sink::write_rdf(
        &graph,
        options.output,
        SourceFormat::Native(target),
        None,
        // The ledger's source codec stays `ntriples`: it names the expressiveness the
        // report graph is measured against, and a report graph is exactly N-Triples-shaped
        // (one default graph, no named graphs). Dropping the re-parse changed where the
        // quads come from, not what they can express.
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
fn load_shapes(
    options: &ValidateOptions<'_>,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<Shapes, CliError> {
    let root = read_shapes_document(options.shapes, format, base, "--shapes")?;
    let folded = fold_shapes_imports(root, options)?;
    purrdf::shapes::shapes::from_dataset_with_prefixes(&folded.dataset, &folded.prefixes).map_err(
        |error| {
            CliError::Runtime(format!(
                "--shapes {shapes}: {error}",
                shapes = options.shapes
            ))
        },
    )
}

/// A shapes document READ but not yet parsed into [`Shapes`]: the frozen graph plus the
/// `@prefix`/`PREFIX` map recovered from its source text.
///
/// The two travel together because they are only jointly meaningful. SHACL-AF `sh:select`
/// bodies may use prefixed names, the frozen IR does not retain a document's prefix map, and
/// an IMPORTED document's queries resolve against ITS OWN declarations — so folding a closure
/// has to carry every document's prefixes forward, not just the root's.
struct ShapesDocument {
    /// The document's quads.
    dataset: Arc<RdfDataset>,
    /// Its own prefix declarations, empty for any non-Turtle syntax (the recovery is a scan
    /// of Turtle source text, which no other syntax offers).
    prefixes: Vec<(String, String)>,
}

/// Read one shapes document, by the same two routes [`purrdf_shapes::engine::parse_shapes`]
/// takes, stopping one step short of parsing it into [`Shapes`].
///
/// Stopping short is what makes an import closure possible at all: the imports have to be
/// read off the GRAPH, and the documents merged as graphs, before anything is asked to be a
/// shape. The composition is deliberately the identical one `parse_shapes` performs —
/// `parse_turtle_to_dataset` + `extract_prefixes`, then `from_dataset_with_config(…, None)`,
/// which is what [`load_shapes`]'s `from_dataset_with_prefixes` call resolves to — so a
/// single document with no imports parses to exactly the `Shapes` it did before this seam
/// existed. `what` names the flag for the diagnostic, since this reads `--shapes` and
/// `--import` alike.
fn read_shapes_document(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    what: &str,
) -> Result<ShapesDocument, CliError> {
    if format == SourceFormat::Native(NativeRdfFormat::Turtle) {
        let bytes = source::read_bytes(path)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            CliError::Runtime(format!("{what} {path}: not UTF-8 text: {error}"))
        })?;
        let dataset = purrdf::shapes::text_ingest::parse_turtle_to_dataset(&text, base)
            .map_err(|errors| CliError::Runtime(format!("{what} {path}: {}", errors.join("\n"))))?;
        return Ok(ShapesDocument {
            dataset,
            prefixes: purrdf::shapes::text_ingest::extract_prefixes(&text),
        });
    }

    Ok(ShapesDocument {
        dataset: source::load_dataset(path, format, base)?,
        prefixes: Vec::new(),
    })
}

/// Fold the shapes graph's transitive `owl:imports` closure in from the `--import IRI=FILE`
/// table.
///
/// # Why this is not a fetch
///
/// Jena resolves `owl:imports` by dereferencing the IRI over HTTP. PurRDF cannot and will
/// not: it ships no HTTP client, every release crate must build for `wasm32-unknown-unknown`,
/// and a validation verdict that depends on what a URL served today is not reproducible. So
/// the closure is caller-supplied configuration, the same answer `entails --import` and
/// `shex --import` give — see `purrdf_entail::entails::imports`, whose doctrine paragraph is
/// the one this follows.
///
/// # Why an unresolved import is not always a refusal
///
/// Naming no `--import` at all leaves the imports UNRESOLVED but does not refuse them: it
/// reports each one on stderr and validates the shapes graph alone. That asymmetry is
/// deliberate and load-bearing. A shapes document may legitimately carry an `owl:Ontology`
/// header whose imports are irrelevant to its shapes — two vectors in this repo's own W3C
/// SHACL corpus do exactly that (`vectors/shacl/sparql/component/validator-001.ttl`,
/// `vectors/shacl/sparql/node/prefixes-001.ttl`) — and refusing them would reject input that
/// is valid, which is the mirror-image bug of the silent drop this whole function exists to
/// fix. The pre-flag behaviour was to say NOTHING, which is the actual defect:
/// a shapes graph whose shapes all live in an imported document validated everything
/// successfully against no shapes at all.
///
/// Naming ANY pair flips the closure to mandatory, because at that point the operator has
/// asserted that the imports matter and a half-resolved closure is a different shapes graph.
fn fold_shapes_imports(
    root: ShapesDocument,
    options: &ValidateOptions<'_>,
) -> Result<ShapesDocument, CliError> {
    let pairs = resolve_shapes_import_pairs(options.imports)?;
    let direct = purrdf_entail::entails::imports::imported_iris(&root.dataset);
    if direct.is_empty() {
        if let Some(pair) = pairs.first() {
            return Err(CliError::Usage(format!(
                "--import {spec}: the shapes graph has no owl:imports at all, so this \
                 document would be read and never used. Remove the pair, or import <{iri}> \
                 from the shapes graph",
                spec = pair.spec,
                iri = pair.iri
            )));
        }
        return Ok(root);
    }

    if pairs.is_empty() {
        // The diagnostic that replaces the silent drop. Not a refusal: see the doc comment.
        for iri in &direct {
            eprintln!(
                "shacl warning: the shapes graph owl:imports <{iri}>, which is not resolved. \
                 PurRDF fetches nothing — pass `--import <{iri}>=FILE` to fold it in. \
                 Validating against the shapes graph alone."
            );
        }
        return Ok(root);
    }

    // Breadth-first to a FIXPOINT over the import graph, each document read once. Two
    // properties are inherited from `purrdf_entail::entails::imports::resolve`, and both
    // matter: an imported document's OWN imports are followed, and a CYCLE terminates
    // rather than looping — OWL 2 §3.4 defines the closure as the transitive one and
    // explicitly permits `A` to import `B` to import `A`, so refusing a cycle would refuse
    // an ontology the specification allows.
    let mut queue: VecDeque<String> = direct.into_iter().collect();
    let mut requested: BTreeSet<String> = BTreeSet::new();
    let mut documents: Vec<ShapesDocument> = Vec::new();
    while let Some(iri) = queue.pop_front() {
        if !requested.insert(iri.clone()) {
            continue;
        }
        let Some(pair) = pairs.iter().find(|pair| pair.iri == iri) else {
            return Err(CliError::Runtime(format!(
                "the shapes graph imports <{iri}>, and no `--import <{iri}>=FILE` pair \
                 resolves it. PurRDF fetches nothing the operator did not name, so an \
                 unresolved import is refused rather than folded in as an empty graph"
            )));
        };
        // The imported document parses under the ONTOLOGY IRI as its base, which is the
        // per-document base an `owl:imports` names — not the root's base and not `--base`.
        let document = read_shapes_document(
            pair.path,
            pair.format,
            Some(pair.iri),
            &format!("--import {iri}"),
        )?;
        queue.extend(purrdf_entail::entails::imports::imported_iris(
            &document.dataset,
        ));
        documents.push(document);
    }

    // A pair the closure never reached, quoted back exactly as the operator wrote it. A
    // USAGE error (exit 2) that nevertheless needs the closure walked to detect: the fault
    // is in the command line, and only its DISCOVERY needed the documents.
    if let Some(pair) = pairs.iter().find(|pair| !requested.contains(pair.iri)) {
        return Err(CliError::Usage(format!(
            "--import {spec}: the shapes graph's import closure never reaches <{iri}>, so \
             this document would be read and never used. Remove the pair, or import the IRI \
             from the shapes graph",
            spec = pair.spec,
            iri = pair.iri
        )));
    }

    // `union` standardizes blank nodes apart per source document and dedupes, which is what
    // keeps two documents' independently-labelled property shapes from colliding.
    let merged = {
        let graphs: Vec<&RdfDataset> = std::iter::once(root.dataset.as_ref())
            .chain(documents.iter().map(|doc| doc.dataset.as_ref()))
            .collect();
        Arc::new(RdfDataset::union(&graphs))
    };
    // The root's prefixes come FIRST so its declarations win a collision: it is the document
    // the operator named, and `from_dataset_with_prefixes` takes the first match.
    let mut prefixes = root.prefixes;
    for document in documents {
        prefixes.extend(document.prefixes);
    }
    Ok(ShapesDocument {
        dataset: merged,
        prefixes,
    })
}

/// One `--import IRI=FILE` argument for the SHACL lane, fully DECIDED but not yet read.
struct ShapesImportPair<'a> {
    /// The pair exactly as the operator wrote it, so a diagnostic can quote it back.
    spec: &'a str,
    /// The ontology-IRI half, checked absolute.
    iri: &'a str,
    /// The document-path half.
    path: &'a str,
    /// The syntax that path's own extension classifies it as.
    format: SourceFormat,
}

/// Decide every `--import IRI=FILE` ARGUMENT, with no I/O at all.
///
/// Nothing here touches the filesystem, so the FIRST malformed pair is reported before the
/// FIRST file is opened — a malformed pair is a usage error naming the argument, never a
/// skipped import, because a shapes graph folded without a document the operator supplied is
/// a different shapes graph. This mirrors `shex`'s `resolve_import_pairs`, including why the
/// IRI half must be ABSOLUTE: it is matched against the shapes graph's `owl:imports` objects,
/// which are absolute by the time the parser is done with them, and it is the base the
/// imported document parses under.
fn resolve_shapes_import_pairs(specs: &[String]) -> Result<Vec<ShapesImportPair<'_>>, CliError> {
    let mut pairs: Vec<ShapesImportPair<'_>> = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some((iri, path)) = spec.split_once('=') else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the ontology IRI the shapes \
                 graph imports, then the local document that resolves it — and this one has \
                 no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the IRI names what \
                 the shapes graph imports, and the path names the document that is it"
            )));
        }
        if path == "-" {
            return Err(CliError::Usage(format!(
                "--import {spec}: an imported document's syntax is inferred from its own path \
                 extension, and `-` has none. Write the document to a file, or name it with a \
                 recognized RDF extension"
            )));
        }
        if let Err(error) = BaseIri::parse(iri) {
            return Err(CliError::Usage(format!(
                "--import {spec}: the IRI half `{iri}` is not an absolute IRI ({code}): it is \
                 matched against the shapes graph's owl:imports objects, which are absolute, \
                 and it is the base the imported document parses under. {error}",
                code = error.diagnostic_code()
            )));
        }
        if pairs.iter().any(|seen| seen.iri == iri) {
            return Err(CliError::Usage(format!(
                "--import {iri}=…: the IRI is named twice, and one IRI resolves to one \
                 document; the second pair would be read and never used"
            )));
        }
        pairs.push(ShapesImportPair {
            spec,
            iri,
            path,
            format: format::resolve(None, path)?,
        });
    }
    Ok(pairs)
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
