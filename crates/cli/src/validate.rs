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
//! # `--shapes-product` is the SAME shapes input, restored rather than parsed
//!
//! The shapes side has two spellings and one meaning. `--shapes` reads a document and
//! parses it; `--shapes-product` restores a preparation `purrdf shacl pack` already made of
//! one. Everything downstream is identical — [`ShapesPlan`] resolves to a `&Shapes` either
//! way, and the engine call below it does not branch — which is the only arrangement under
//! which a cached preparation is allowed to exist: a product that reached a different
//! verdict than its own source document would be a cache that silently changes answers.
//!
//! Those bytes are UNTRUSTED. Nothing in a file on disk is evidence of its own provenance,
//! so restoring one is an ADMISSION: the container framing, every section digest and the
//! whole-container digest, then the product's stage id, profile and complete input binding,
//! all checked before a focus node is resolved. A refusal names its DIMENSION on stderr as
//! a `shacl dimension <label>` line, because `stage-id` (re-pack), `container-digest` (the
//! file is corrupt) and `function-registry` (your configuration, not the product) are three
//! different actions and one exit code cannot carry the difference.
//!
//! Every one of those checks asks about THIS PROCESS — is this the build that wrote the
//! memo, are these the registries the product was prepared against. None of them asks
//! whether this is the product the operator wanted, because the product cannot know:
//! nothing in a file states which file was meant. `--expect-identity HEX` is how the
//! operator states it, and it is checked first and costs 32 bytes of comparison. Without
//! it, naming the wrong product on the command line produces a decided, well-formed
//! report about a shapes graph nobody asked about, with a `0` exit status.
//!
//! `--shapes-from`, `--shapes-graph` and `--import` all configure a PARSE, and this route
//! performs none: the product carries its own base, prefix map and `sh:shapesGraph` IRI,
//! bound by its identity. They are refused by name rather than accepted and ignored — see
//! [`refuse_parse_flags_against_a_product`].
//!
//! # Reading the two graphs
//!
//! The DATA graph is read through the pipeline's own format resolution into the frozen IR —
//! any of the nine native syntaxes or a verified pack — and handed to the engine as an
//! `RdfDataset`. Nothing is transcoded on the way in, so unlike `entails`/`consistency` there
//! is no lossless-or-refused crossing to make: the engine reads the same IR the parser built.
//!
//! The SHAPES graph is read through [`load_shapes`], in two steps that
//! [`purrdf_shapes::engine::parse_shapes`] performs as one:
//! [`crate::shapes_source::read_shapes_document`] freezes the document into a graph, and
//! only then is that graph asked to be `Shapes`. That read-then-fold seam lives in
//! [`crate::shapes_source`] rather than here, because `shacl pack` needs the identical
//! sequence — a product packed with an `--import` table must fold the same closure
//! `validate --shapes --import` does, or the two commands would disagree about what the
//! shapes graph even is. Turtle
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
//! anything is asked to be a shape. [`crate::shapes_source::fold_shapes_imports`] walks that
//! closure against the `--import IRI=FILE` table — PurRDF fetches nothing — and a shapes
//! graph with no imports composes to exactly the `Shapes` `parse_shapes` produced before the
//! seam existed.
//!
//! # `--shapes-graph` is command-line text, and its refusal says so
//!
//! `--shapes-graph` names the graph the shapes document is exposed under, overriding a
//! `sh:shapesGraph` that document declares. That declaration is an IRI *inside* the shapes
//! document, so it resolves against the shapes document's base — and the flag that overrides
//! it resolves against the SAME base, through
//! [`resolve_shapes_graph`](crate::shapes_source::resolve_shapes_graph). `--shapes-graph
//! sg` therefore names what `sh:shapesGraph <sg>` written in that document names, and an
//! absolute value is carried lexical-verbatim (`BaseScope::resolve`'s own contract), so
//! nothing about an already-absolute invocation changes. `shacl pack` reads its own
//! `--shapes-graph IRI` through the same function against the same derivation, which is
//! what lets a product packed with the flag and a document validated with the flag agree
//! on what the graph is named.
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
    /// `--shapes`: the SHACL shapes graph, or `-`. `None` exactly when
    /// [`Self::shapes_product`] is `Some` — clap makes the two mutually required.
    pub(crate) shapes: Option<&'a str>,
    /// `--shapes-product`: a prepared product to RESTORE instead of parsing a shapes
    /// document. See [`load_prepared`].
    pub(crate) shapes_product: Option<&'a str>,
    /// `--expect-identity`: the input binding `--shapes-product` must carry, as the
    /// operator wrote it. [`ShapesPlan::decide`] turns it into the 32-byte selector the
    /// admission boundary compares, so a mis-typed digest is a usage error before any
    /// file is opened.
    pub(crate) expect_identity: Option<&'a str>,
    /// `--shapes-from`: the shapes-graph format override.
    pub(crate) shapes_from: Option<CliRdfFormat>,
    /// `--shapes-graph`: the IRI the shapes graph is exposed under to SHACL-SPARQL paths,
    /// as the operator wrote it.
    /// [`resolve_shapes_graph`](crate::shapes_source::resolve_shapes_graph) turns it into
    /// the absolute IRI the engine is handed.
    pub(crate) shapes_graph: Option<&'a str>,
    /// `--import IRI=FILE`, repeatable: the local documents that resolve the shapes graph's
    /// `owl:imports`. Empty means the operator named none, which is the pre-flag behaviour
    /// plus a diagnostic — see [`crate::shapes_source::fold_shapes_imports`].
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

/// The parsed shapes this run validates against, and the owner keeping them alive.
///
/// Two arms because there are two spellings of one input — a shapes DOCUMENT to parse or a
/// prepared PRODUCT to restore — and exactly one thing the engine wants from either: a
/// `&Shapes`. Making that the enum's only accessor is what keeps the rest of this lane from
/// branching: everything after [`load_shapes_source`] reads the same borrow whichever route
/// produced it, so the two routes cannot diverge on anything downstream of the shapes.
enum ShapesSource {
    /// A shapes document parsed on this run (`--shapes`).
    ///
    /// Boxed because a `Shapes` is two orders of magnitude larger than a
    /// `PreparedShapes` (which is a pair of `Arc`s), and an enum sized to the larger
    /// arm would be moved around at that size on both routes.
    Parsed(Box<Shapes>),
    /// A preparation restored from a prepared product (`--shapes-product`). The
    /// preparation owns its shapes; the product bytes it came from are already released,
    /// because `admit` hands back owned values rather than borrows of the container.
    Restored(engine::PreparedShapes),
}

impl ShapesSource {
    /// The shapes to validate against.
    fn shapes(&self) -> &Shapes {
        match self {
            Self::Parsed(shapes) => shapes,
            Self::Restored(prepared) => prepared.shapes(),
        }
    }
}

/// Run the `validate` subcommand.
pub(crate) fn run(
    options: &ValidateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    refuse_two_stdins(options)?;
    refuse_inapplicable_flags(options, ledger_target)?;
    refuse_parse_flags_against_a_product(options)?;
    refuse_an_expectation_with_no_product(options)?;

    let data_format = format::resolve(options.from, options.input)?;
    // The DATA parse is the only leg `--base` has here: the shapes graph resolves against
    // its own retrieval IRI, and the validation report is a fresh graph `emit` serializes
    // with no base at all (it passes `None`). So a data syntax that admits no relative IRI
    // leaves the flag with nowhere to go.
    format::refuse_unconsumable_base(
        options.base,
        &[format::BaseUse::parse(data_format, "the --from data graph")],
    )?;

    // Every DECISION about the shapes side is made here, before a byte of either document
    // is read: a `--shapes-graph` that names no graph is a malformed request and should
    // fail against the command line rather than after the data has been parsed and
    // validated.
    let plan = ShapesPlan::decide(options)?;

    let data = source::load_dataset(options.input, data_format, options.base)?;
    let source = plan.load(options)?;
    let shapes = source.shapes();

    let Some(report) = validate(&data, shapes, options, plan.shapes_graph())? else {
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
/// on `--shapes-graph` as well (see
/// [`resolve_shapes_graph`](crate::shapes_source::resolve_shapes_graph)): one derivation is
/// what keeps the flag and the document agreeing about what a relative IRI denotes.
fn load_shapes(
    options: &ValidateOptions<'_>,
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<Shapes, CliError> {
    let root = crate::shapes_source::read_shapes_document(path, format, base, "--shapes")?;
    let folded = crate::shapes_source::fold_shapes_imports(root, options.imports)?;
    purrdf::shapes::shapes::from_dataset_with_prefixes(&folded.dataset, &folded.prefixes)
        .map_err(|error| CliError::Runtime(format!("--shapes {path}: {error}")))
}

/// Everything DECIDED about the shapes side of this run, before either document is read.
///
/// The two arms are the two spellings of one input, and they carry different decisions
/// because they describe different work. A document has a syntax to resolve, a base to
/// derive and a `--shapes-graph` to resolve against that base; a product has none of those
/// — it carries the base, the prefix map and the shapes-graph IRI it was PREPARED under,
/// which is exactly why [`refuse_parse_flags_against_a_product`] refuses the three flags
/// that would otherwise look like they applied.
enum ShapesPlan<'a> {
    /// `--shapes`: a document to read and parse.
    Document {
        /// The document path, or `-`.
        path: &'a str,
        /// The syntax it is read as.
        format: SourceFormat,
        /// The base it parses under, and that `--shapes-graph` resolved against.
        base: Option<String>,
        /// `--shapes-graph`, resolved to an absolute IRI.
        shapes_graph: Option<String>,
    },
    /// `--shapes-product`: a prepared product to restore.
    Product {
        /// The product path, or `-`.
        path: &'a str,
        /// `--expect-identity`, decoded: the input binding the product must carry for
        /// this restore to proceed. `None` leaves the restore unbound — the product is
        /// still admitted in full, but nothing states WHICH product was wanted.
        expect_identity: Option<[u8; 32]>,
    },
}

impl<'a> ShapesPlan<'a> {
    /// Decide the shapes side from the command line, reading no document.
    ///
    /// # Errors
    ///
    /// Any usage error in `--shapes-from`, `--shapes-graph` or the shapes path.
    fn decide(options: &ValidateOptions<'a>) -> Result<Self, CliError> {
        if let Some(path) = options.shapes_product {
            // Decoded HERE, before a byte of the product is read: a mis-typed selector is
            // the operator's command line, not the artifact's fault, so it exits as a
            // usage error rather than as a refusal that would name a dimension of a
            // product nothing ever inspected.
            let expect_identity = options
                .expect_identity
                .map(purrdf_validate::parse_identity_digest)
                .transpose()
                .map_err(|why| CliError::Usage(format!("--expect-identity {why}")))?;
            return Ok(Self::Product {
                path,
                expect_identity,
            });
        }
        // clap makes exactly one of the two required, so the `else` is unreachable from a
        // command line; it is reported rather than unwrapped because an unreachable panic
        // in a CLI is a crash report.
        let Some(path) = options.shapes else {
            return Err(CliError::Usage(
                "one of --shapes (a SHACL shapes document) or --shapes-product (a prepared \
                 product written by `purrdf shacl pack`) is required"
                    .to_owned(),
            ));
        };
        let format = format::resolve(options.shapes_from, path)?;
        // The shapes document's base is derived ONCE and spent twice: the document parses
        // under it, and `--shapes-graph` resolves against it. Deriving it separately per
        // consumer is how the flag would come to name a graph the shapes document cannot.
        let base = shapes_document_base(path, format)?;
        let shapes_graph =
            crate::shapes_source::resolve_shapes_graph(options.shapes_graph, base.as_deref())?;
        Ok(Self::Document {
            path,
            format,
            base,
            shapes_graph,
        })
    }

    /// The shapes-graph IRI to expose to SHACL-SPARQL paths, or `None`.
    ///
    /// `None` for a product, and that is not a dropped flag: `--shapes-graph` is refused
    /// against `--shapes-product`, and the engine still honours the `sh:shapesGraph` the
    /// product itself recorded — which is what the restored `Shapes` carries.
    fn shapes_graph(&self) -> Option<&str> {
        match self {
            Self::Document { shapes_graph, .. } => shapes_graph.as_deref(),
            Self::Product { .. } => None,
        }
    }

    /// Read whichever input this plan names, into the shapes the engine validates with.
    ///
    /// # The product arm is an ADMISSION, not a load
    ///
    /// The bytes are acquired through the sealed immutable-input authority and handed to
    /// [`purrdf_validate::admit_shapes_product`], which verifies the container's framing
    /// and every section digest, then checks the product's stage id, profile and full
    /// input binding BEFORE any of it reaches a validator. A refusal names its
    /// dimension — see [`crate::shacl::admission_error`] — because "these bytes are
    /// corrupt", "this product is from another build" and "your configuration differs
    /// from the one it was prepared against" are three different actions, and the flag
    /// that flattened them into one message would have made the typed boundary
    /// pointless at exactly the process edge an operator meets it.
    ///
    /// # Errors
    ///
    /// Any read, parse, import-resolution or admission failure.
    fn load(&self, options: &ValidateOptions<'_>) -> Result<ShapesSource, CliError> {
        match *self {
            Self::Document {
                path,
                format,
                ref base,
                ..
            } => load_shapes(options, path, format, base.as_deref())
                .map(|shapes| ShapesSource::Parsed(Box::new(shapes))),
            Self::Product {
                path,
                expect_identity,
            } => {
                let owner = source::acquire_product_input(path)?;
                // Two entry points at one boundary, chosen by whether the operator
                // stated which product they wanted. `--expect-identity` adds a 32-byte
                // comparison ahead of every other check, so the wrong file is named as
                // the wrong file rather than restored and validated against.
                let admitted = match expect_identity {
                    None => purrdf_validate::admit_shapes_product(owner.as_bytes()),
                    Some(ref expected) => {
                        purrdf_validate::admit_shapes_product_expecting(owner.as_bytes(), expected)
                    }
                };
                admitted.map(ShapesSource::Restored).map_err(|error| {
                    crate::shacl::admission_error(&format!("--shapes-product {path}"), &error)
                })
            }
        }
    }
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

/// Refuse a command line that reads standard input twice.
///
/// The data graph and the shapes input — whichever spelling of it — may each be `-`, and at
/// most ONE of them may be: a
/// process has one standard input, so two documents naming it would each get part of one
/// stream. Refused naming both, exactly as `entails` refuses it, rather than mis-read.
fn refuse_two_stdins(options: &ValidateOptions<'_>) -> Result<(), CliError> {
    if options.input != "-" {
        return Ok(());
    }
    if options.shapes == Some("-") {
        return Err(CliError::Usage(
            "IN and --shapes both read standard input, and there is only one: a process has a \
             single stdin stream, so the data graph and the shapes graph would each get part \
             of one document. Give one of them a path"
                .to_owned(),
        ));
    }
    if options.shapes_product == Some("-") {
        return Err(CliError::Usage(
            "IN and --shapes-product both read standard input, and there is only one: a \
             process has a single stdin stream, so the data graph and the prepared product \
             would each get part of one byte stream. Give one of them a path"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Refuse `--expect-identity` when there is no product for it to bind.
///
/// The flag states which PREPARED PRODUCT the operator meant, and a shapes document has
/// no prepared binding to state: it is parsed on this run, from the path on this command
/// line, so the question "is this the artifact I asked for" has already been answered by
/// the path itself.
///
/// Accepting it against `--shapes` and ignoring it would be worse than the usual silent
/// no-op. This is the one flag an operator reaches for precisely because they do not
/// trust that the right bytes arrived; a spelling of it that quietly checks nothing
/// hands back exactly the reassurance it was asked to withhold.
fn refuse_an_expectation_with_no_product(options: &ValidateOptions<'_>) -> Result<(), CliError> {
    if options.expect_identity.is_none() || options.shapes_product.is_some() {
        return Ok(());
    }
    Err(CliError::Usage(
        "--expect-identity names the input binding a PREPARED PRODUCT must carry, and this run \
         parses a shapes document instead: `--shapes` has no prepared binding to require, and \
         the document it names is read on this run rather than restored from a cache. Pass \
         `--shapes-product FILE` to bind a product, or drop the flag"
            .to_owned(),
    ))
}

/// Refuse the shapes-PARSE flags against `--shapes-product`.
///
/// `--shapes-from`, `--shapes-graph` and `--import` all configure a parse of a shapes
/// DOCUMENT, and `--shapes-product` performs none: the product carries the compiled model
/// and, in its own authenticated parse provenance, the base, the prefix map and the
/// `sh:shapesGraph` IRI it was prepared under. Accepting any of the three and quietly
/// ignoring it is the silent no-op this pipeline refuses everywhere else, and here it would
/// be worse than usual — an operator who passed `--shapes-graph` and got a validation back
/// would reasonably believe the shapes graph was exposed under the IRI they named.
///
/// `--import` is refused for the additional reason that folding an import closure changes
/// WHICH shapes graph is validated against. The product's identity binds the graph it was
/// compiled from; there is no honest way to add documents to it after the fact, so the
/// remedy is to fold the closure at `shacl pack` time (or to validate from `--shapes`).
fn refuse_parse_flags_against_a_product(options: &ValidateOptions<'_>) -> Result<(), CliError> {
    let Some(product) = options.shapes_product else {
        return Ok(());
    };
    let refused = if options.shapes_from.is_some() {
        Some((
            "--shapes-from",
            "names the SYNTAX a shapes document is read as, and a product is not a document: \
             it carries an already-compiled model in a digest-chained container of its own",
        ))
    } else if options.shapes_graph.is_some() {
        Some((
            "--shapes-graph",
            "overrides the `sh:shapesGraph` a shapes DOCUMENT declares, and this product \
             already recorded the IRI it was prepared under — which its identity binds, so \
             it cannot be changed without re-preparing. Pass `--shapes-graph` to `purrdf \
             shacl pack` instead, and re-pack",
        ))
    } else if !options.imports.is_empty() {
        Some((
            "--import",
            "folds an `owl:imports` closure into a shapes DOCUMENT before it is parsed, and \
             this product's identity binds the shapes graph it was actually compiled from. \
             Fold the closure with `purrdf shacl pack`'s own `--import IRI=FILE`",
        ))
    } else {
        None
    };
    match refused {
        None => Ok(()),
        Some((flag, why)) => Err(CliError::Usage(format!(
            "{flag} {why}. Drop it, or validate from `--shapes` instead of \
             `--shapes-product {product}`"
        ))),
    }
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
