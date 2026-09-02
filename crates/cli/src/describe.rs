// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `describe` subcommand: `Source → extract SCBD → Sink`.
//!
//! The Symmetric Concise Bounded Description of one or more resources, written through the
//! shared [`sink`] like any other RDF this binary emits.
//!
//! # One authority, reached rather than re-derived
//!
//! [`purrdf_core::describe::Describer`] is the repo's single definition of what "describe"
//! means: outgoing triples AND incoming ones, the transitive blank-node closure in both
//! directions, and the RDF 1.2 statement layer — the reifiers whose reified triple touches the
//! closure, together with their annotations. It is what SPARQL `DESCRIBE` evaluates to in this
//! engine (`purrdf_sparql_eval::describe_query`), and what the documentation-site export uses.
//! This subcommand calls the SAME `Describer`. There is no second walk here, and adding one
//! would be the divergence the extractor's own module documentation exists to prevent.
//!
//! # Why it is a verb and not a documented `query` incantation
//!
//! `purrdf query --data d.ttl 'DESCRIBE <x>'` already reaches the same extractor, so the
//! question is whether a verb earns its place beside it. Three things say yes, and none of
//! them is a shorter spelling.
//!
//! * **The obvious `query` invocation hard-fails.** `--results-format` defaults to `json`,
//!   which is a SPARQL-results serialization; a `DESCRIBE` produces a GRAPH, and a
//!   shape/format-kind mismatch is a hard error at emit time. So the natural first attempt
//!   fails and the caller has to learn that a graph needs an RDF syntax named explicitly.
//!   `describe` resolves `--to` (or the `OUT` extension) exactly as `convert` and `reason` do,
//!   so `purrdf describe --iri … data.ttl out.ttl` works with nothing else supplied.
//! * **A resource is an argument, not a sentence.** Reaching a description through `query`
//!   means building SPARQL TEXT around an IRI — a second language, and in a script, string
//!   concatenation into a query. `--iri` takes the resource as the value it is.
//! * **It is an RDF-emitting verb, so the RDF-emitting flags apply.** `--loss-ledger` and
//!   `--jsonld-options` work here exactly as they do for `convert`, which matters precisely
//!   because an SCBD is the part of this toolkit most likely to CARRY a statement layer: a
//!   description serialized into a star-incapable syntax records the reifier rows it dropped
//!   instead of losing them silently.
//!
//! # A pack source is described zero-copy
//!
//! [`Describer`](purrdf_core::describe::Describer) is generic over [`DatasetView`], so the
//! extraction runs over whichever concrete view the input resolved to — a parsed `RdfDataset`
//! for a text source, or a verified `PackView` for a pack, with no `dataset_from_view` rebuild
//! in between. The extracted subgraph is always a fresh, frozen `RdfDataset`.
//!
//! # `--iri` resolves against the base, and a relative one is refused rather than dropped
//!
//! `--iri` is command-line text with no retrieval IRI of its own — the same shape as a ShEx
//! shape map — and it is compared against the graph's terms, which are ABSOLUTE by the time
//! the parser is done with them. So a relative `--iri` matched nothing, and
//! [`Describer::describe_iris`] drops a term the dataset does not contain: `purrdf describe
//! --iri alice --base http://example.org/dir/ rel.ttl` exited 0 with an empty document and
//! said nothing, while `--iri http://example.org/dir/alice` described it. A required
//! argument silently denoting nothing is exactly the shape this pipeline refuses.
//!
//! [`resolve_subjects`] therefore resolves every `--iri` through the base in force —
//! [`source::effective_base`], the same answer the DATA GRAPH parses under, so `--iri alice`
//! denotes what `<alice>` written inside the document would denote. An ABSOLUTE `--iri` is
//! carried lexical-verbatim (`BaseScope::resolve`'s own contract), so nothing about an
//! already-absolute invocation changes.
//!
//! A relative `--iri` with NO base in scope is a hard failure carrying the shared
//! `iri-relative-no-base` code. It is a usage error (exit 2) rather than a runtime one:
//! nothing is wrong with the data, the request itself does not denote a resource, and it is
//! decided before a single byte of the source is read.
//!
//! # An ABSENT subject is still an empty description, and that is not the same thing
//!
//! Describing an IRI the source does not mention yields an empty subgraph, which is the
//! library's own semantics: a term may legitimately carry no asserted or incoming triples, and
//! "nothing describes it" is a true answer rather than a failed run. The exit code stays 0 and
//! the sink writes an empty document in the requested syntax.
//!
//! The two cases are deliberately NOT conflated. An absolute `--iri` absent from the graph is
//! a well-formed question with an empty answer — the SCBD of a resource with no asserted or
//! incoming triples IS empty, and refusing it would break the composable "describe each of
//! these IRIs" use and contradict `DESCRIBE`'s own semantics. A RELATIVE `--iri` is not a
//! question at all until it is resolved: it denotes no resource, so there is nothing for an
//! empty answer to be about.
//!
//! # A description carries graphs, so a single-graph target is refused
//!
//! An SCBD is graph-faithful at every layer: a base quad, a reifier declaration and an
//! annotation each come back in the graph the source asserted them in. Turtle, N-Triples
//! and RDF/XML have no named-graph construct and the single-graph serializers DROP every
//! graph-scoped row rather than folding it into the default graph, so describing a
//! resource whose description lives in named graphs into one of those syntaxes wrote a
//! well-formed document missing exactly what was asked for — in the whole-named-graph
//! case, ZERO statements and exit 0. This verb therefore refuses (exit 2) exactly as the
//! `query` lane does for the same `DESCRIBE`, through the same shared refusal sentence:
//! two spellings of one operation must not give two answers. A description carrying only
//! default-graph statements serializes exactly as before.
//!
//! # No `--report`
//!
//! Nothing here infers. An SCBD is a bounded walk over asserted quads and the statement layer;
//! there is no entailment regime, no closure and no reasoning certificate, so a `--report`
//! flag would have nothing to write. A caller who wants the description of a CLOSURE runs
//! `purrdf reason` and describes its output.

use std::sync::Arc;

use purrdf_core::named_graph::{distinct_graph_names, named_graph_refusal};
use purrdf_core::{DatasetView, RdfDataset, describe::Describer};
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
use purrdf_rdf::{JsonLdSerializeOptions, SourceFormat};

use crate::cli::{CliRdfFormat, LedgerTarget};
use crate::error::CliError;
use crate::source::ViewOp;
use crate::{format, ledger, sink, source};

/// The resolved `describe` flags.
pub(crate) struct DescribeOptions<'a> {
    /// `--iri`, in the order the operator wrote them; at least one (clap enforces it).
    pub(crate) iris: &'a [String],
    /// `--from`: the input-format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--to`: the output-format override.
    pub(crate) to: Option<CliRdfFormat>,
    /// `--base`: the base IRI for parsing the input and for the serializer.
    pub(crate) base: Option<&'a str>,
    /// The input path `IN`, or `-` for stdin.
    pub(crate) input: &'a str,
    /// The output path `OUT`, or `-` for stdout.
    pub(crate) output: &'a str,
    /// Explicit JSON-LD/YAML-LD serialization configuration.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// The SCBD extraction, as the generic operation [`source::run_over_input`] monomorphizes per
/// concrete view.
///
/// It is a [`ViewOp`] rather than a closure for the reason every other one is: `DatasetView`
/// is not object-safe, so the operation has to be generic over the view type to run over both
/// a parsed `RdfDataset` and a zero-copy `PackView`.
struct DescribeOp<'a> {
    /// The subjects whose union description is extracted, already RESOLVED against the base
    /// in force — a relative selector never reaches the extractor, where it could only fail
    /// to match.
    iris: &'a [String],
}

impl ViewOp for DescribeOp<'_> {
    type Output = Arc<RdfDataset>;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError> {
        Ok(Describer::new(view).describe_iris(self.iris.iter().map(String::as_str))?)
    }
}

/// The closing imperative of this verb's named-graph refusal: the quad-capable `--to`
/// targets, in [`CliRdfFormat`](crate::cli::CliRdfFormat) declaration order.
///
/// The rest of the sentence is `purrdf_core::named_graph::named_graph_refusal`, shared
/// verbatim with the `query` lane and with the Python and wasm hosts; only the remedy is
/// per-surface, because "`--to`" is a spelling this verb has and they do not. The pack
/// container is listed too: it is a lossless RDF-1.2 dataset carrier, so it is a real
/// answer to "where can this description go", which `--results-format` has no member for.
const QUAD_CAPABLE_REMEDY: &str =
    "Re-run with a quad-capable --to target (trig/nquads/trix/hextuples/jsonld/yamlld/pack)";

/// Refuse to serialize a description that carries named graphs to a single-graph RDF
/// syntax, naming the graphs, the format, and what to use instead.
///
/// This is the `query` lane's refusal reached from the other spelling of the same
/// operation. `purrdf query --results-format turtle 'DESCRIBE <x>'` refuses; before this,
/// `purrdf describe --iri x --to turtle` serialized the default-graph half and exited 0 —
/// and when the whole description was graph-scoped, wrote nothing at all and still exited
/// 0. One `Describer`, one authority, so one answer.
///
/// It is a REFUSAL rather than a ledgered loss for the reason the `query` lane gives:
/// which resource to describe is the most explicit thing in the request, and a partial
/// description reported as a complete one is worse than no description. That is not in
/// tension with `convert`, which ledgers instead: there the caller named a source document
/// and a target syntax and asked for exactly that transcode, so the pair contract is the
/// answer. Here they asked for a resource.
///
/// [`SourceFormat::Pack`] carries a full RDF-1.2 dataset, and [`SourceFormat::Gts`] is
/// refused by name at format-resolution time, so only the native syntaxes are tested.
fn refuse_uncarriable_named_graphs<D: DatasetView>(
    description: &D,
    target: SourceFormat,
) -> Result<(), CliError> {
    let SourceFormat::Native(format) = target else {
        return Ok(());
    };
    if format.supports_datasets() {
        return Ok(());
    }
    let names = distinct_graph_names(description);
    if names.is_empty() {
        return Ok(());
    }
    Err(CliError::Usage(named_graph_refusal(
        &names,
        format.id(),
        QUAD_CAPABLE_REMEDY,
    )))
}

/// Run the `describe` subcommand.
pub(crate) fn run(
    options: &DescribeOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    // Resolve BOTH formats before touching the source (mirroring `convert`/`reason`), so an
    // unresolvable OUT fails fast rather than after the description has been extracted.
    let source_format = format::resolve(options.from, options.input)?;
    let target_format = format::resolve_target(options.to, options.output, "the --to target")?;
    // No `--base` refusal here, unlike `convert`/`reason`. This command has a leg no format
    // row can describe: `--iri` is command-line text and resolves against the base, exactly
    // as a ShEx shape map does. So a base handed to `describe` always has a live consumer —
    // `--iri` is REQUIRED, and every value passes through `resolve_subjects` — whatever the
    // `--from`/`--to` rows say about their own two legs. Refusing on those rows alone would
    // reject `--from ntriples --base http://example.org/ --iri alice`, whose base is doing
    // the one job that makes the selector denote anything.
    sink::validate_jsonld_options(target_format, options.jsonld_options)?;

    // Resolve the subjects BEFORE opening the source: a relative selector with no base is a
    // malformed request, and it should fail against the command line rather than after the
    // document has been read.
    let iris = resolve_subjects(options, source_format)?;

    let description = source::run_over_input(
        options.input,
        source_format,
        options.base,
        DescribeOp { iris: &iris },
    )?;
    refuse_uncarriable_named_graphs(&*description, target_format)?;

    let ledger = sink::write_rdf(
        &*description,
        options.output,
        target_format,
        options.base,
        source_format.loss_codec_name(),
        options.jsonld_options,
    )?;
    ledger::surface(ledger_target, &ledger)
}

/// Resolve every `--iri` against the base in force.
///
/// An ABSOLUTE selector is carried lexical-verbatim — [`BaseScope::resolve`]'s own contract,
/// and the reason an already-absolute invocation is byte-for-byte what it always was. A
/// RELATIVE one resolves, so `--iri alice` denotes what `<alice>` written inside the data
/// graph denotes. A relative one with nothing in scope is refused.
fn resolve_subjects(
    options: &DescribeOptions<'_>,
    source_format: SourceFormat,
) -> Result<Vec<String>, CliError> {
    let base = subject_base(options, source_format)?;
    let scope = match base {
        Some(base) => BaseScope::rooted(
            // `--base` already passed `cli::parse_base_iri` at the argument boundary and a
            // derived retrieval IRI is parsed by its own derivation, so a failure here is
            // not reachable from the command line; it is still reported rather than
            // unwrapped, because an unreachable panic in a CLI is a crash report.
            BaseIri::parse(&base).map_err(|error| {
                CliError::Usage(format!(
                    "the base `{base}` is not a usable base IRI: {error}"
                ))
            })?,
            BaseOrigin::Caller,
        ),
        None => BaseScope::empty(),
    };

    options
        .iris
        .iter()
        .map(|raw| {
            scope
                .resolve(raw)
                .map(|iri| iri.as_str().to_owned())
                .map_err(|error| subject_refusal(raw, &error))
        })
        .collect()
}

/// The base `--iri` resolves against: the one the DATA GRAPH itself parses under.
///
/// Sharing [`source::effective_base`] is the point — a selector that resolved against a
/// different base from the document would name a resource the document cannot contain, which
/// is the silent no-match this whole path exists to delete. A CONTAINER source has no
/// retrieval IRI to derive and stores fully-resolved terms, so only an explicit `--base` can
/// resolve a relative selector against one.
fn subject_base(
    options: &DescribeOptions<'_>,
    source_format: SourceFormat,
) -> Result<Option<String>, CliError> {
    match source_format {
        SourceFormat::Native(native) => source::effective_base(options.input, native, options.base),
        SourceFormat::Pack | SourceFormat::Gts => Ok(options.base.map(ToOwned::to_owned)),
    }
}

/// The refusal for an `--iri` that does not denote a resource.
///
/// It carries the shared [`purrdf_iri::IriError::diagnostic_code`] so it groups with every
/// other IRI failure in this toolkit, but it does NOT carry the library's remedy: that one
/// names `@base` and `xml:base`, which are document directives, and `--iri` is not in a
/// document. Naming a fix the operator cannot apply is worse than naming none.
fn subject_refusal(raw: &str, error: &purrdf_iri::IriError) -> CliError {
    let code = error.diagnostic_code();
    if code == "iri-relative-no-base" {
        return CliError::Usage(format!(
            "--iri `{raw}`: {code}: a relative IRI reference has no base in scope, so it \
             denotes no resource to describe. Pass --base <IRI> — `--iri` resolves against it \
             exactly as the data graph does — or write the resource in absolute form"
        ));
    }
    CliError::Usage(format!("--iri `{raw}`: {code}: {error}"))
}
