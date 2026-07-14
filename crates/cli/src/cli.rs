// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The clap command tree: the `purrdf` binary's argument model.
//!
//! One pipeline, three subcommands ([`Command`]), and one global flag
//! (`--loss-ledger`). The format / regime / results-format choices are modeled as
//! [`clap::ValueEnum`] wrappers so `--help` enumerates the legal values and clap
//! validates them at parse time, and each wrapper carries a total conversion into
//! its library counterpart.
//!
//! ## The `--loss-ledger` tri-state
//!
//! `--loss-ledger` is an optional-value global flag whose three states are encoded
//! as `Option<Option<PathBuf>>`:
//!
//! * absent → `None` — do not surface the ledger.
//! * `--loss-ledger` (bare) → `Some(None)` — render the ledger to **stderr**.
//! * `--loss-ledger=PATH` → `Some(Some(PATH))` — write the ledger to **PATH**.
//!
//! `require_equals` forces the `=PATH` spelling so the optional value never
//! greedily swallows a following positional (e.g. a subcommand or a query string),
//! keeping the three states unambiguous.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use purrdf_entail::Regime;
use purrdf_sparql_results::SparqlResultsFormat;

use crate::format::CliFormat;

/// The `purrdf` command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "purrdf",
    version,
    about = "PurRDF: convert, query, and reason over RDF 1.2 data and native packs",
    propagate_version = true
)]
pub(crate) struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub(crate) cmd: Command,

    /// Surface the transcode loss ledger: bare writes it to stderr,
    /// `--loss-ledger=PATH` writes it to PATH.
    //
    // `Option<Option<PathBuf>>` is clap's idiom for an optional-value flag (the
    // three states are: absent / present-bare / present-with-value); it is the
    // only place this shape appears — `Cli::ledger_target` projects it into the
    // self-documenting `LedgerTarget` the pipeline actually threads.
    #[allow(clippy::option_option)]
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        num_args = 0..=1,
        require_equals = true
    )]
    pub(crate) loss_ledger: Option<Option<PathBuf>>,
}

/// Where (if anywhere) the loss ledger should be surfaced — the decoded form of
/// the `--loss-ledger` tri-state flag.
#[derive(Debug, Clone)]
pub(crate) enum LedgerTarget {
    /// The flag was absent: do not surface the ledger.
    Silent,
    /// Bare `--loss-ledger`: render the ledger to stderr.
    Stderr,
    /// `--loss-ledger=PATH`: write the ledger to PATH.
    File(PathBuf),
}

impl Cli {
    /// Decode the raw `--loss-ledger` tri-state into a [`LedgerTarget`].
    pub(crate) fn ledger_target(&self) -> LedgerTarget {
        match &self.loss_ledger {
            None => LedgerTarget::Silent,
            Some(None) => LedgerTarget::Stderr,
            Some(Some(path)) => LedgerTarget::File(path.clone()),
        }
    }
}

/// The three pipeline subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Convert RDF between syntaxes, and to/from the native pack container.
    Convert {
        /// Input format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Output format override; inferred from the output extension when omitted.
        #[arg(long, value_enum)]
        to: Option<CliRdfFormat>,
        /// Base IRI for resolving relative IRIs while parsing the input; also
        /// threaded into the serializer as its base.
        #[arg(long, value_name = "IRI")]
        base: Option<String>,
        /// Materialize an entailment regime's closure in memory before
        /// serializing (applied before `--canonical`).
        #[arg(long, value_enum, value_name = "REGIME")]
        entailment: Option<CliRegime>,
        /// Emit RDFC-1.0 canonical N-Quads instead of `--to`. This overrides the
        /// target format (canonical output is always N-Quads), so `--to` may be
        /// omitted; combine with `--entailment` to canonicalize the closure.
        #[arg(long)]
        canonical: bool,
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Output path `OUT`, or `-` for stdout (which requires `--to`).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Evaluate a SPARQL query over an RDF or pack data source.
    Query {
        /// Data-source path (format inferred from its extension). A pack file is
        /// queried zero-copy.
        #[arg(long)]
        data: String,
        /// SPARQL-results serialization for SELECT/ASK results.
        #[arg(long, value_enum, default_value_t = ResultsFormat::Json)]
        results_format: ResultsFormat,
        /// The SPARQL query text.
        query: String,
    },
    /// Materialize an entailment regime's closure over a source graph.
    Reason {
        /// The entailment regime to close under.
        #[arg(long, value_enum)]
        regime: CliRegime,
        /// Input path `IN`, or `-` for stdin (which requires a recognizable format).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Output path `OUT` (format inferred from its extension), or `-` for stdout.
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
}

/// The input/output format choices `--from`/`--to` accept: the nine native RDF
/// syntaxes plus the native `pack` container.
///
/// Each variant's canonical value is the one `--help` lists; the short
/// extension/id spellings the native codec [`classify`](purrdf_rdf::classify)
/// accepts (e.g. `ttl`, `nt`, `nq`) are registered as hidden aliases so the same
/// name works on the command line and in a filename.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliRdfFormat {
    /// Turtle.
    #[value(name = "turtle", alias = "ttl")]
    Turtle,
    /// TriG.
    #[value(name = "trig")]
    Trig,
    /// N-Triples.
    #[value(name = "ntriples", alias = "nt", alias = "n-triples")]
    Ntriples,
    /// N-Quads.
    #[value(name = "nquads", alias = "nq", alias = "n-quads")]
    Nquads,
    /// RDF/XML.
    #[value(name = "rdfxml", alias = "rdf", alias = "xml")]
    Rdfxml,
    /// TriX.
    #[value(name = "trix")]
    Trix,
    /// HexTuples.
    #[value(name = "hextuples", alias = "hext")]
    Hextuples,
    /// JSON-LD.
    #[value(name = "jsonld", alias = "json-ld")]
    Jsonld,
    /// YAML-LD.
    #[value(name = "yamlld", alias = "yaml-ld")]
    Yamlld,
    /// The native PurRDF pack container.
    #[value(name = "pack")]
    Pack,
}

impl CliRdfFormat {
    /// Resolve this explicit choice to the pipeline's [`CliFormat`].
    pub(crate) fn to_cli_format(self) -> CliFormat {
        use purrdf_rdf::NativeRdfFormat as N;
        match self {
            Self::Turtle => CliFormat::Rdf(N::Turtle),
            Self::Trig => CliFormat::Rdf(N::TriG),
            Self::Ntriples => CliFormat::Rdf(N::NTriples),
            Self::Nquads => CliFormat::Rdf(N::NQuads),
            Self::Rdfxml => CliFormat::Rdf(N::RdfXml),
            Self::Trix => CliFormat::Rdf(N::TriX),
            Self::Hextuples => CliFormat::Rdf(N::HexTuples),
            Self::Jsonld => CliFormat::Rdf(N::JsonLd),
            Self::Yamlld => CliFormat::Rdf(N::YamlLd),
            Self::Pack => CliFormat::Pack,
        }
    }
}

/// The SPARQL-results serialization choices `--results-format` accepts.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultsFormat {
    /// SPARQL Results JSON.
    #[value(name = "json")]
    Json,
    /// SPARQL Results XML.
    #[value(name = "xml")]
    Xml,
    /// SPARQL Results CSV.
    #[value(name = "csv")]
    Csv,
    /// SPARQL Results TSV.
    #[value(name = "tsv")]
    Tsv,
}

impl ResultsFormat {
    /// The library [`SparqlResultsFormat`] this choice maps to.
    pub(crate) fn to_native(self) -> SparqlResultsFormat {
        match self {
            Self::Json => SparqlResultsFormat::Json,
            Self::Xml => SparqlResultsFormat::Xml,
            Self::Csv => SparqlResultsFormat::Csv,
            Self::Tsv => SparqlResultsFormat::Tsv,
        }
    }
}

/// The entailment-regime choices `--regime` accepts.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliRegime {
    /// Simple entailment (a faithful copy of the source).
    #[value(name = "simple")]
    Simple,
    /// RDF entailment.
    #[value(name = "rdf")]
    Rdf,
    /// RDFS entailment.
    #[value(name = "rdfs")]
    Rdfs,
    /// OWL 2 RL entailment.
    #[value(name = "owl-rl")]
    OwlRl,
    /// OWL Direct (DL) entailment — not materializable without query class
    /// expressions.
    #[value(name = "owl-direct")]
    OwlDirect,
    /// RIF-Core entailment — not materializable without a rule set.
    #[value(name = "rif")]
    Rif,
    /// Datatype (D) entailment — a spec-inherent boundary for materialization.
    #[value(name = "d")]
    D,
}

impl CliRegime {
    /// The library [`Regime`] this choice maps to.
    pub(crate) fn to_native(self) -> Regime {
        match self {
            Self::Simple => Regime::Simple,
            Self::Rdf => Regime::Rdf,
            Self::Rdfs => Regime::Rdfs,
            Self::OwlRl => Regime::OwlRl,
            Self::OwlDirect => Regime::OwlDirect,
            Self::Rif => Regime::Rif,
            Self::D => Regime::D,
        }
    }
}
