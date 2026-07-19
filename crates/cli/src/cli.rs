// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The clap command tree: the `purrdf` binary's argument model.
//!
//! One pipeline, five subcommands ([`Command`]), and one global flag
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
use purrdf_rdf::{LiftProfile, NativeRdfFormat, ProjectionProfile};
use purrdf_sparql_results::SparqlResultsFormat;

use crate::format::CliFormat;

/// The `purrdf` command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "purrdf",
    version,
    about = "PurRDF: convert, query, reason, project, and lift RDF 1.2 data",
    propagate_version = true
)]
pub(crate) struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub(crate) cmd: Command,

    /// Surface the conversion/projection loss ledger: bare writes it to stderr,
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

    /// Versioned JSON options document for configured JSON-LD/YAML-LD output.
    /// The selected output must be JSON-LD or YAML-LD; otherwise the option is
    /// rejected instead of ignored.
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) jsonld_options: Option<PathBuf>,
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

/// The five pipeline subcommands.
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
        /// queried zero-copy (unless `--entailment` forces materialization).
        #[arg(long)]
        data: String,
        /// Base IRI for resolving relative IRIs while parsing the data AND in the
        /// query text.
        #[arg(long, value_name = "IRI")]
        base: Option<String>,
        /// Materialize an entailment regime's closure in memory before querying
        /// (the query then runs over the closure, not the raw view).
        #[arg(long, value_enum, value_name = "REGIME")]
        entailment: Option<CliRegime>,
        /// Result serialization: a SPARQL-results format (json/xml/csv/tsv) for
        /// SELECT/ASK, or an RDF syntax (turtle/trig/…) for CONSTRUCT/DESCRIBE.
        #[arg(long, value_enum, default_value_t = QueryFormat::Json)]
        results_format: QueryFormat,
        /// The SPARQL query text.
        query: String,
    },
    /// Materialize an entailment regime's closure over a source graph.
    Reason {
        /// The entailment regime to close under.
        #[arg(long, value_enum)]
        regime: CliRegime,
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
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Output path `OUT`, or `-` for stdout (which requires `--to`).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Project RDF into a deterministic graph, tabular, or research-object USTAR carrier.
    Project {
        /// Closed projection carrier profile.
        #[arg(long, value_enum)]
        profile: CliProjectionProfile,
        /// Profile-tagged mandatory JSON configuration path, or `-` for stdin.
        #[arg(long, value_name = "PATH")]
        config: String,
        /// Canonical payload-only USTAR path for attached RO-Crate output.
        #[arg(long, value_name = "PATH")]
        assets: Option<String>,
        /// Input RDF/pack format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Base IRI for resolving relative IRIs while parsing input RDF.
        #[arg(long, value_name = "IRI")]
        base: Option<String>,
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Canonical USTAR output path `OUT`, or `-` for stdout.
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Lift a strict bidirectional graph, tabular, or research-object carrier into RDF.
    Lift {
        /// Bidirectional carrier profile; OBO Graphs and SKOS are intentionally absent.
        #[arg(long, value_enum)]
        profile: CliLiftProfile,
        /// Profile-tagged mandatory JSON configuration path, or `-` for stdin.
        #[arg(long, value_name = "PATH")]
        config: String,
        /// Native RDF output syntax.
        #[arg(long, value_enum)]
        to: CliNativeRdfFormat,
        /// Base IRI threaded to the native RDF serializer.
        #[arg(long, value_name = "IRI")]
        base: Option<String>,
        /// Canonical USTAR input path `IN`, or `-` for stdin.
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// RDF output path `OUT`, or `-` for stdout.
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
}

/// Projection profiles accepted by `purrdf project`.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliProjectionProfile {
    /// Generic deterministic LPG CSV.
    LpgCsv,
    /// Neo4j Admin Import CSV.
    Neo4jCsv,
    /// Closed deterministic openCypher.
    OpenCypher,
    /// GraphML 1.0.
    Graphml,
    /// Exact lossless RDF 1.2 CSVW.
    CsvwExact,
    /// Caller-declared curated CSVW terms view.
    CsvwTerms,
    /// Caller-declared OKF v0.1 concept-bundle view.
    OkfTerms,
    /// OBO Graphs 0.3.2 JSON view.
    OboGraphs,
    /// SKOS Turtle concept-scheme view.
    Skos,
    /// Croissant 1.1 research-object package.
    #[value(name = "croissant-1.1")]
    Croissant11,
    /// RO-Crate 1.3 research-object package.
    #[value(name = "ro-crate-1.3")]
    RoCrate13,
    /// DataCite Metadata Schema 4.6 package.
    #[value(name = "datacite-4.6")]
    DataCite46,
    /// DCAT 3 research-object package.
    #[value(name = "dcat-3")]
    Dcat3,
    /// Frictionless Data Package v1.
    #[value(name = "frictionless-data-package-1")]
    FrictionlessDataPackage1,
}

impl CliProjectionProfile {
    /// Convert to the library's closed profile enum.
    pub(crate) const fn to_profile(self) -> ProjectionProfile {
        match self {
            Self::LpgCsv => ProjectionProfile::LpgCsv,
            Self::Neo4jCsv => ProjectionProfile::Neo4jCsv,
            Self::OpenCypher => ProjectionProfile::OpenCypher,
            Self::Graphml => ProjectionProfile::Graphml,
            Self::CsvwExact => ProjectionProfile::CsvwExact,
            Self::CsvwTerms => ProjectionProfile::CsvwTerms,
            Self::OkfTerms => ProjectionProfile::OkfTerms,
            Self::OboGraphs => ProjectionProfile::OboGraphs,
            Self::Skos => ProjectionProfile::Skos,
            Self::Croissant11 => ProjectionProfile::Croissant11,
            Self::RoCrate13 => ProjectionProfile::RoCrate13,
            Self::DataCite46 => ProjectionProfile::DataCite46,
            Self::Dcat3 => ProjectionProfile::Dcat3,
            Self::FrictionlessDataPackage1 => ProjectionProfile::FrictionlessDataPackage1,
        }
    }
}

/// Bidirectional profiles accepted by `purrdf lift`.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliLiftProfile {
    /// Generic deterministic LPG CSV.
    LpgCsv,
    /// Neo4j Admin Import CSV.
    Neo4jCsv,
    /// Closed deterministic openCypher.
    OpenCypher,
    /// GraphML 1.0.
    Graphml,
    /// Exact lossless RDF 1.2 CSVW.
    CsvwExact,
    /// Croissant 1.1 research-object package.
    #[value(name = "croissant-1.1")]
    Croissant11,
    /// RO-Crate 1.3 research-object package.
    #[value(name = "ro-crate-1.3")]
    RoCrate13,
    /// DataCite Metadata Schema 4.6 package.
    #[value(name = "datacite-4.6")]
    DataCite46,
    /// DCAT 3 research-object package.
    #[value(name = "dcat-3")]
    Dcat3,
    /// Frictionless Data Package v1.
    #[value(name = "frictionless-data-package-1")]
    FrictionlessDataPackage1,
}

impl CliLiftProfile {
    /// Convert to the library's write/read profile enum.
    pub(crate) const fn to_profile(self) -> LiftProfile {
        match self {
            Self::LpgCsv => LiftProfile::LpgCsv,
            Self::Neo4jCsv => LiftProfile::Neo4jCsv,
            Self::OpenCypher => LiftProfile::OpenCypher,
            Self::Graphml => LiftProfile::Graphml,
            Self::CsvwExact => LiftProfile::CsvwExact,
            Self::Croissant11 => LiftProfile::Croissant11,
            Self::RoCrate13 => LiftProfile::RoCrate13,
            Self::DataCite46 => LiftProfile::DataCite46,
            Self::Dcat3 => LiftProfile::Dcat3,
            Self::FrictionlessDataPackage1 => LiftProfile::FrictionlessDataPackage1,
        }
    }
}

/// Native RDF output syntaxes accepted by `purrdf lift`.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliNativeRdfFormat {
    /// Turtle.
    #[value(alias = "ttl")]
    Turtle,
    /// TriG.
    Trig,
    /// N-Triples.
    #[value(alias = "nt", alias = "n-triples")]
    Ntriples,
    /// N-Quads.
    #[value(alias = "nq", alias = "n-quads")]
    Nquads,
    /// RDF/XML.
    #[value(alias = "rdf", alias = "xml")]
    Rdfxml,
    /// TriX.
    Trix,
    /// HexTuples.
    #[value(alias = "hext")]
    Hextuples,
    /// JSON-LD.
    #[value(alias = "json-ld")]
    Jsonld,
    /// YAML-LD.
    #[value(alias = "yaml-ld")]
    Yamlld,
}

impl CliNativeRdfFormat {
    /// Convert to the native codec enum.
    pub(crate) const fn to_native(self) -> NativeRdfFormat {
        match self {
            Self::Turtle => NativeRdfFormat::Turtle,
            Self::Trig => NativeRdfFormat::TriG,
            Self::Ntriples => NativeRdfFormat::NTriples,
            Self::Nquads => NativeRdfFormat::NQuads,
            Self::Rdfxml => NativeRdfFormat::RdfXml,
            Self::Trix => NativeRdfFormat::TriX,
            Self::Hextuples => NativeRdfFormat::HexTuples,
            Self::Jsonld => NativeRdfFormat::JsonLd,
            Self::Yamlld => NativeRdfFormat::YamlLd,
        }
    }
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

/// The `--results-format` choices the `query` subcommand accepts: a SUPERSET of the
/// four W3C SPARQL-results serializations (for SELECT solutions / ASK booleans) and
/// the nine native RDF syntaxes (for CONSTRUCT / DESCRIBE graphs).
///
/// The result SHAPE selects which half is legal: a SELECT/ASK result serializes
/// through a SPARQL-results format, a CONSTRUCT/DESCRIBE graph through an RDF syntax.
/// A shape/format-kind mismatch (e.g. a graph with `csv`, or solutions with
/// `turtle`) is a hard error at emit time. [`Self::to_results_format`] and
/// [`Self::to_rdf_format`] project a choice into whichever half it names.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueryFormat {
    // --- SPARQL-results serializations (SELECT solutions / ASK boolean) ---
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
    // --- Native RDF syntaxes (CONSTRUCT / DESCRIBE graph) ---
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
    /// RDF/XML. (`xml` names the SPARQL-results format, so RDF/XML aliases `rdf`.)
    #[value(name = "rdfxml", alias = "rdf")]
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
}

impl QueryFormat {
    /// The [`SparqlResultsFormat`] this choice names, or `None` when it names an
    /// RDF syntax (a graph target).
    pub(crate) fn to_results_format(self) -> Option<SparqlResultsFormat> {
        match self {
            Self::Json => Some(SparqlResultsFormat::Json),
            Self::Xml => Some(SparqlResultsFormat::Xml),
            Self::Csv => Some(SparqlResultsFormat::Csv),
            Self::Tsv => Some(SparqlResultsFormat::Tsv),
            _ => None,
        }
    }

    /// The [`NativeRdfFormat`] this choice names, or `None` when it names a
    /// SPARQL-results format (a solutions/boolean target).
    pub(crate) fn to_rdf_format(self) -> Option<NativeRdfFormat> {
        use NativeRdfFormat as N;
        match self {
            Self::Turtle => Some(N::Turtle),
            Self::Trig => Some(N::TriG),
            Self::Ntriples => Some(N::NTriples),
            Self::Nquads => Some(N::NQuads),
            Self::Rdfxml => Some(N::RdfXml),
            Self::Trix => Some(N::TriX),
            Self::Hextuples => Some(N::HexTuples),
            Self::Jsonld => Some(N::JsonLd),
            Self::Yamlld => Some(N::YamlLd),
            _ => None,
        }
    }

    /// The canonical CLI token that names this choice (for diagnostics).
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Turtle => "turtle",
            Self::Trig => "trig",
            Self::Ntriples => "ntriples",
            Self::Nquads => "nquads",
            Self::Rdfxml => "rdfxml",
            Self::Trix => "trix",
            Self::Hextuples => "hextuples",
            Self::Jsonld => "jsonld",
            Self::Yamlld => "yamlld",
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
