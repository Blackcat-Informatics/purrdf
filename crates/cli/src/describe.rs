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
//! # An absent subject is an empty description, not an error
//!
//! Describing an IRI the source does not mention yields an empty subgraph, which is the
//! library's own semantics: a term may legitimately carry no asserted or incoming triples, and
//! "nothing describes it" is a true answer rather than a failed run. The exit code stays 0 and
//! the sink writes an empty document in the requested syntax.
//!
//! # No `--report`
//!
//! Nothing here infers. An SCBD is a bounded walk over asserted quads and the statement layer;
//! there is no entailment regime, no closure and no reasoning certificate, so a `--report`
//! flag would have nothing to write. A caller who wants the description of a CLOSURE runs
//! `purrdf reason` and describes its output.

use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDataset, describe::Describer};
use purrdf_rdf::JsonLdSerializeOptions;

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
    /// The subjects whose union description is extracted.
    iris: &'a [String],
}

impl ViewOp for DescribeOp<'_> {
    type Output = Arc<RdfDataset>;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError> {
        Ok(Describer::new(view).describe_iris(self.iris.iter().map(String::as_str))?)
    }
}

/// Run the `describe` subcommand.
pub(crate) fn run(
    options: &DescribeOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    // Resolve BOTH formats before touching the source (mirroring `convert`/`reason`), so an
    // unresolvable OUT fails fast rather than after the description has been extracted.
    let source_format = format::resolve(options.from, options.input)?;
    let target_format = format::resolve(options.to, options.output)?;
    format::refuse_base_with_pack(source_format, options.base, "a pack --from source")?;
    format::refuse_base_with_pack(target_format, options.base, "a pack --to target")?;
    sink::validate_jsonld_options(target_format, options.jsonld_options)?;

    let description = source::run_over_input(
        options.input,
        source_format,
        options.base,
        DescribeOp { iris: options.iris },
    )?;

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
