// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The clap command tree: the `purrdf` binary's argument model.
//!
//! One pipeline, twelve subcommands ([`Command`]), and two global flags
//! (`--loss-ledger`, `--jsonld-options`). The format / regime / results-format
//! choices are modeled as
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
//!
//! ## `--report`, the same tri-state for the reasoning certificate
//!
//! Every entailment entry point in `purrdf-entail` hands back a
//! [`ReasoningReport`](purrdf_entail::ReasoningReport) with its closure: which rules fired
//! and how often, which constructs the run could not fully handle and why, what the run
//! cost, and the contract hash of the calculus that produced it. `--report` is the surface
//! that carries it to an operator, decoded exactly like `--loss-ledger` into a
//! [`ReportTarget`]. Without it a CLI closure is a document with no provenance: nothing
//! distinguishes "closed under every rule the regime defines" from "closed under a subset,
//! over the default graph, with the named graphs untouched".
//!
//! It is a per-subcommand flag rather than a global one, because exactly four subcommands
//! can reason (`reason` and `entails` always, `convert` and `query` under `--entailment`). A
//! global flag would be accepted by `project` and `lift`, which run no reasoner, and would
//! then have to do nothing — a silent no-op being precisely the shape this repository
//! refuses. For the same reason `convert --report` / `query --report` WITHOUT `--entailment`
//! is a usage error rather than an empty file. `validate`, `shex` and `describe` carry no
//! `--report` for exactly that reason: SHACL and ShEx conformance are decided WITHOUT
//! entailment (neither engine closes the data graph under any regime), and a Symmetric CBD
//! is a bounded walk over asserted quads — none of the three infers anything, so there is no
//! reasoning certificate for the flag to carry.
//!
//! ## The `query` governor flags
//!
//! Six flags — `--fuel`, `--deadline`, `--max-answers`, `--max-intermediate-cells`,
//! `--max-scratch-bytes`, `--max-remote-requests` — carry the engine's execution governors
//! to the command line, and they are `query`'s alone for the reason `--report` is not
//! global: they bound a SPARQL evaluation, and a subcommand that runs none would have
//! nothing to enforce them over. Each is `Option`, and `None` is the only thing that
//! becomes an unbounded dimension; [`GovernorFlags`](crate::governors::GovernorFlags) is
//! where they are decoded and where [`QueryGovernors`](purrdf_sparql_eval::QueryGovernors)'
//! deliberately-unnameable-by-default "no ceiling" state is named exactly once. A trip
//! exits **3** rather than failing — see [`CliOutcome`](crate::error::CliOutcome).
//!
//! `--explain` sits beside them and takes none of them: it measures a run rather than
//! bounding one, so accepting a ceiling it cannot enforce would be a governor that governs
//! nothing. The refusal is written where the reason can be given, in
//! [`query`](crate::query), rather than as a bare clap conflict.
//!
//! `--entailment` DOES take them. All six bound the SPARQL evaluation over the materialized
//! closure, exactly as they bound one over a raw view; `--deadline` alone additionally bounds
//! computing the closure, because a stop signal changes no answer while a caller-settable
//! numeric ceiling on a reasoning run would change the closure itself. That split is stated
//! on `--entailment`'s own help — an operator meets it at the flag rather than in a refusal
//! after the fact — and argued in [`query`](crate::query).
//!
//! ## `validate`'s five governors, and the sixth it does not take
//!
//! `validate` reaches `purrdf_shapes::engine::validate_dataset_with_governors`, whose budget
//! bounds every SPARQL path ONE validation decomposes into — `sh:SPARQLTarget` resolution,
//! each `sh:sparql` constraint, each SHACL-AF node expression — against a single
//! [`QueryGovernors`](purrdf_sparql_eval::QueryGovernors). So five of the six flags carry
//! over unchanged. `--max-answers` does not: it bounds the ANSWER SEQUENCE a caller asked
//! for, and a validation's answer is a conformance report rather than a row sequence. Every
//! solution a SHACL constraint query produces is an internal intermediate, which is what
//! `--max-intermediate-cells` already bounds — so `validate` omits `--max-answers` for the
//! same reason `update` does, rather than accepting it and quietly re-interpreting it as a
//! per-constraint row cap.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use purrdf_entail::Regime;
use purrdf_rdf::{
    LiftProfile, NativeRdfFormat, ProjectionProfile, SourceFormat, TransportEncoding,
};
use purrdf_sparql_results::SparqlResultsFormat;

use crate::source::TransportPolicy;

/// Validate a `--base` value at the ARGUMENT boundary.
///
/// A base IRI must be absolute (RFC-3986 §5.1), and the commonest mistake is pasting a
/// filesystem path — which is a relative reference, not an IRI. Checking here means that
/// is a clap usage error naming the fix, instead of a codec diagnostic surfacing much
/// later from somewhere that no longer knows the value came from `--base`.
fn parse_base_iri(raw: &str) -> Result<String, String> {
    match purrdf_iri::BaseIri::parse(raw) {
        Ok(base) => Ok(base.as_str().to_owned()),
        Err(error) => Err(format!(
            "a base IRI must be absolute (with a scheme): {error}.{}",
            path_shaped_hint(raw)
        )),
    }
}

/// The `did you mean …?` half of a rejected `--base`, for a value shaped like a filesystem
/// path.
///
/// The suggestion is DERIVED, never spliced. The previous version built it by trimming the
/// leading dots off the argument and prefixing `file://`, which named a directory the
/// operator did not write: `./vocab/` suggested `file:///vocab/`, and `../vocab/` suggested
/// the same thing, because `trim_start_matches` strips every leading dot. A diagnostic that
/// confidently names the wrong fix is worse than one that names none, so a dot-relative
/// value is RESOLVED through the same [`crate::source::retrieval_base_iri`] the pipeline
/// derives a retrieval IRI with, and a value that does not resolve gets the rule and no
/// path-specific suggestion at all.
fn path_shaped_hint(raw: &str) -> String {
    match path_shaped_base(raw) {
        Some(iri) => format!(" did you mean `{iri}`?"),
        // A relative path is not a base IRI and, unresolved, there is no honest absolute
        // spelling to offer for it — so state the rule and stop.
        None if is_relative_path(raw) => {
            " a relative filesystem path is not a base IRI, and this one does not resolve \
             against the working directory: pass an absolute `file://` IRI."
                .to_owned()
        }
        None => String::new(),
    }
}

/// The absolute `file://` IRI a path-shaped `--base` value denotes, or `None` when the value
/// is not path-shaped or cannot be resolved.
fn path_shaped_base(raw: &str) -> Option<String> {
    if is_relative_path(raw) {
        let mut iri = crate::source::retrieval_base_iri(raw).ok()?;
        // A DIRECTORY base ends in `/`. RFC-3986 §5.2.4 resolution replaces a base's last
        // segment, so `file:///x/vocab` and `file:///x/vocab/` are different bases and only
        // the second is the directory the operator named.
        if !iri.ends_with('/') && std::path::Path::new(raw).is_dir() {
            iri.push('/');
        }
        return Some(iri);
    }
    is_absolute_path(raw).then(|| crate::source::file_iri_for_absolute_path(raw))
}

/// Whether `raw` is a DOT-RELATIVE filesystem path — one whose meaning depends on the
/// working directory, so nothing but the filesystem can say which IRI it denotes.
fn is_relative_path(raw: &str) -> bool {
    raw.starts_with('.')
}

/// Whether `raw` is an ABSOLUTE filesystem path in this platform's spelling: a POSIX
/// `/path`, a Windows UNC `\\host\share`, or a Windows drive path `C:\dir` / `C:/dir`.
///
/// An absolute path needs no filesystem lookup to name its IRI, which is what lets the
/// suggestion stand for a path that does not exist yet.
fn is_absolute_path(raw: &str) -> bool {
    if raw.starts_with('/') || raw.starts_with(r"\\") {
        return true;
    }
    let mut chars = raw.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(drive), Some(':'), Some('\\' | '/')) if drive.is_ascii_alphabetic()
    )
}

/// The `purrdf` command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "purrdf",
    version,
    about = "PurRDF: convert, query, update, reason, decide entailment, decide consistency, \
             validate with SHACL or ShEx, describe a resource, project, and lift RDF 1.2 data",
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

impl LedgerTarget {
    /// Whether the operator asked for the loss ledger at all — the same
    /// tri-state-collapsing query [`ReportTarget::is_requested`] answers for
    /// `--report`.
    pub(crate) const fn is_requested(&self) -> bool {
        !matches!(self, Self::Silent)
    }
}

/// Where (if anywhere) the reasoning report should be surfaced — the decoded form of the
/// `--report` tri-state flag, with the same three states as [`LedgerTarget`].
#[derive(Debug, Clone)]
pub(crate) enum ReportTarget {
    /// The flag was absent: do not surface the report.
    Silent,
    /// Bare `--report`: render the report to stderr, leaving stdout for the data.
    Stderr,
    /// `--report=PATH`: write the report to PATH.
    File(PathBuf),
}

impl ReportTarget {
    /// Decode the raw `--report` tri-state.
    ///
    /// A free function over the flag rather than a method on [`Cli`], because `--report` is
    /// carried by the three subcommands that can reason rather than globally by the root
    /// command.
    #[allow(
        clippy::option_option,
        reason = "clap's encoding of an optional-value flag; this is the one place it is decoded"
    )]
    pub(crate) fn decode(flag: Option<&Option<PathBuf>>) -> Self {
        match flag {
            None => Self::Silent,
            Some(None) => Self::Stderr,
            Some(Some(path)) => Self::File(path.clone()),
        }
    }

    /// Whether the operator asked for a report at all.
    pub(crate) const fn is_requested(&self) -> bool {
        !matches!(self, Self::Silent)
    }
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

/// The twelve pipeline subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Convert RDF between syntaxes, and to/from the native pack container.
    Convert {
        /// Input format override; inferred from each input's extension when omitted.
        /// Applies to EVERY source in the list, including each `--input`.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// An ADDITIONAL input source, repeatable. The effective ordered source list
        /// is the positional `IN` followed by each `--input` in the order written;
        /// two or more sources are merged with the deterministic dataset union, under
        /// a separate blank-node scope per source.
        #[arg(long = "input", value_name = "PATH")]
        inputs: Vec<String>,
        /// How a gzip/zstd transport wrapper around each input is handled: `auto`
        /// sniffs the leading bytes then the filename suffix, `none` reads the bytes
        /// verbatim, and `gzip`/`zstd` decode under exactly that encoding. A
        /// truncated or corrupt stream is always a hard failure, never a short read.
        #[arg(long, value_enum, value_name = "ENCODING", default_value = "auto")]
        transport: CliTransport,
        /// Output format override; inferred from the output extension when omitted.
        #[arg(long, value_enum)]
        to: Option<CliRdfFormat>,
        /// Base IRI, on BOTH legs of the conversion. A relative IRI in the input
        /// resolves against it while parsing, and a target syntax that can write a
        /// base directive (turtle, trig, rdfxml, jsonld, yamlld) emits it as the
        /// output document's base and relativizes against it; a target that cannot
        /// (ntriples, nquads, trix, hextuples) writes absolute IRIs. When omitted, a
        /// filesystem input still parses under its own `file://` retrieval IRI —
        /// stdin has none, so a relative IRI there is an error — and no base is
        /// written on output.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Materialize an entailment regime's closure in memory before
        /// serializing (applied before `--canonical`).
        #[arg(long, value_enum, value_name = "REGIME")]
        entailment: Option<CliRegime>,
        /// RIF-in-XML rule document for `rif`; required by that regime and
        /// refused for every other (their rule table is the specification's).
        #[arg(long, value_name = "FILE")]
        rules: Option<PathBuf>,
        /// Surface the reasoning report for `--entailment`: bare writes it to
        /// stderr, `--report=PATH` writes it to PATH. Requires `--entailment`.
        #[allow(clippy::option_option)]
        #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
        report: Option<Option<PathBuf>>,
        /// Emit RDFC-1.0 canonical N-Quads instead of `--to`. Canonical output is
        /// always N-Quads, so `--to` may be omitted — and is REFUSED (not silently
        /// ignored) when named beside `--canonical`, since it would otherwise be
        /// accepted and never read. Combine with `--entailment` to canonicalize the
        /// closure.
        #[arg(long)]
        canonical: bool,
        /// First input path `IN`, or `-` for stdin (which requires `--from`). Every
        /// `--input` is appended AFTER this one.
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
        /// query text. A CONSTRUCT/DESCRIBE graph written through an RDF
        /// `--results-format` that can express a base (turtle, trig, rdfxml, jsonld,
        /// yamlld) is additionally serialized under it; a SPARQL-results
        /// serialization has no base surface to carry one.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Materialize an entailment regime's closure in memory before querying
        /// (the query then runs over the closure, not the raw view). Combines with
        /// every governor flag: the ceilings bound the QUERY over the closure, and
        /// `--deadline` additionally bounds computing the closure itself — a numeric
        /// ceiling cannot, because a caller-settable budget on a reasoning run would
        /// make the closure itself depend on the caller. A deadline that expires while
        /// the closure is still being computed prints the governor report, writes no
        /// rows, and exits 3.
        #[arg(long, value_enum, value_name = "REGIME")]
        entailment: Option<CliRegime>,
        /// RIF-in-XML rule document for `rif`; required by that regime and
        /// refused for every other (their rule table is the specification's).
        #[arg(long, value_name = "FILE")]
        rules: Option<PathBuf>,
        /// Surface the reasoning report for `--entailment`: bare writes it to
        /// stderr, `--report=PATH` writes it to PATH. Requires `--entailment`.
        #[allow(clippy::option_option)]
        #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
        report: Option<Option<PathBuf>>,
        /// Result serialization: a SPARQL-results format (json/xml/csv/tsv) for
        /// SELECT/ASK, or an RDF syntax (turtle/trig/…) for CONSTRUCT/DESCRIBE.
        /// Defaults to `json` when omitted. `None` here (the flag genuinely
        /// absent, not merely defaulted) is load-bearing: it is what lets
        /// `--explain` tell "the operator asked for a serialization" apart from
        /// "nothing was named" and refuse the former (see
        /// `crate::query::refuse_unenforceable_combinations`) rather than
        /// silently ignore it.
        #[arg(long, value_enum)]
        results_format: Option<QueryFormat>,
        /// Bound the query's abstract execution steps. The unit is the engine's own
        /// charge schedule, which `--explain` prints, so a fuel budget is comparable
        /// only against the same schedule. The ceiling is inclusive, and `0` is a valid
        /// one that trips at the first charge. A trip prints the answers it certified
        /// and exits 3.
        #[arg(long, value_name = "UNITS")]
        fuel: Option<u64>,
        /// Bound the query's wall-clock EVALUATION time: a run of count+unit components
        /// over `ms`, `s`, `m`, `h` (`750ms`, `30s`, `1m30s`, `2h`). The budget starts
        /// when evaluation starts — reading and parsing the data source happen before
        /// it — and the engine observes it when it enters an algebra node, so an
        /// evaluation overruns it by at most one operator rather than being killed
        /// mid-step. This is not a timeout on the process.
        #[arg(long, value_name = "DURATION", value_parser = crate::governors::parse_deadline)]
        deadline: Option<std::time::Duration>,
        /// Bound the ANSWER SEQUENCE: solution rows for SELECT, output statements for
        /// CONSTRUCT/DESCRIBE (an ASK boolean has no sequence to bound). This is an
        /// operational ceiling and never `LIMIT`: `LIMIT` is query semantics and applies
        /// before this is tested. Inclusive.
        #[arg(long, value_name = "ROWS")]
        max_answers: Option<u64>,
        /// Bound the largest INTERMEDIATE solution bag, in cells (rows × columns) — the
        /// ceiling that actually bounds allocation. Compared against the largest single
        /// bag rather than a running total, and a plan whose ESTIMATED peak already
        /// exceeds it is refused before evaluation starts.
        #[arg(long, value_name = "CELLS")]
        max_intermediate_cells: Option<u64>,
        /// Bound the bytes value-constructing operations mint into the per-query scratch
        /// arena, which grow independently of any row or cell count.
        #[arg(long, value_name = "BYTES")]
        max_scratch_bytes: Option<u64>,
        /// Bound the requests issued to a remote or federated endpoint by a `SERVICE`
        /// clause. The ceiling is enforced and reported like any other; this binary
        /// configures no federation source, so a `SERVICE` clause fails to evaluate
        /// before it can be charged.
        #[arg(long, value_name = "REQUESTS")]
        max_remote_requests: Option<u64>,
        /// Print what the engine does with the query and what it costs — the charge
        /// schedule it was priced under, the per-node ledger with the planner's estimate
        /// beside the cardinality that materialized, the cost-based join orders, and the
        /// per-dimension consumption — INSTEAD of the query's answers. The query is
        /// evaluated to produce it, under the metering profile: every counter engaged at
        /// a ceiling nothing can reach. The rendering is plain text, so it is refused
        /// beside every flag that names something about the ANSWERS: a governor flag,
        /// `--entailment`, `--results-format` (which serialization to use),
        /// `--loss-ledger` (which lossy transcode to report), and `--jsonld-options`
        /// (which JSON-LD/YAML-LD serializer to configure) — none of which this lane
        /// can honor.
        #[arg(long)]
        explain: bool,
        /// Register purrdf's first-party statistical aggregate set (`MEDIAN`,
        /// `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`, `FIRST`,
        /// `LAST`, `TOPK`) under this IRI namespace, so the query text can call
        /// `AGG(<{NAMESPACE}NAME>, args…)`, e.g. `AGG(<https://ex.example/agg#MEDIAN>,
        /// ?x)`. There is no default namespace (PurRDF mints no vocabulary IRIs of its
        /// own) — omit this flag and every one of the ten names is an ordinary
        /// unregistered custom-aggregate IRI, refused at parse time exactly as before.
        /// This is the CLOSED, namespace-only statistical set; it carries no surface for
        /// an arbitrary caller-defined aggregate, which is host Rust closures that cannot
        /// cross this command-line boundary as a string.
        #[arg(long, value_name = "IRI")]
        aggregate_namespace: Option<String>,
        /// Anchor the additive `purrdf` provenance extension under `PREFIX=IRI`
        /// (e.g. `prov=https://example.org/ns/prov#`) on a SPARQL-results JSON/XML
        /// `--results-format`. PurRDF mints no vocabulary IRIs of its own — there is no
        /// default namespace, and omitting this flag emits pure-W3C output exactly as
        /// before it existed. `PREFIX` must be a valid XML NCName (neither `xml` nor
        /// `xmlns`) and `IRI` must be an absolute IRI; CSV/TSV have no extension point,
        /// so this flag is REFUSED (not silently ignored) when combined with
        /// `--results-format csv`/`tsv`. Read the extension back with
        /// `purrdf_sparql_results::provenance_from_json`/`provenance_from_xml` under the
        /// SAME namespace.
        #[arg(long, value_name = "PREFIX=IRI", value_parser = crate::query::parse_provenance_namespace)]
        provenance_namespace: Option<(String, String)>,
        /// The SPARQL query text.
        query: String,
    },
    /// Apply a SPARQL UPDATE and serialize the resulting RDF dataset.
    Update {
        /// Input dataset path; format is inferred from its extension unless `--from` is set.
        #[arg(long)]
        data: String,
        /// Input format override, required when `--data -` reads stdin.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Output path, or `-` for stdout (the default).
        #[arg(long, default_value = "-")]
        output: String,
        /// Output format override, required when `--output -` writes stdout.
        #[arg(long, value_enum)]
        to: Option<CliRdfFormat>,
        /// Base IRI for parsing the data and the UPDATE request, and for the
        /// mutated dataset on the way out: a `--to` syntax that can express a base
        /// (turtle, trig, rdfxml, jsonld, yamlld) writes it and relativizes against
        /// it.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Bound abstract execution steps. Inclusive; zero trips on the first charge.
        #[arg(long, value_name = "UNITS")]
        fuel: Option<u64>,
        /// Wall-clock UPDATE budget (`750ms`, `30s`, `1m30s`, `2h`). A trip applies
        /// nothing, writes no dataset, prints its receipt, and exits 3.
        #[arg(long, value_name = "DURATION", value_parser = crate::governors::parse_deadline)]
        deadline: Option<std::time::Duration>,
        /// Bound the largest intermediate solution bag in cells (rows × columns).
        #[arg(long, value_name = "CELLS")]
        max_intermediate_cells: Option<u64>,
        /// Bound bytes minted into the per-request scratch arena.
        #[arg(long, value_name = "BYTES")]
        max_scratch_bytes: Option<u64>,
        /// Bound remote/federated requests issued while computing the mutation.
        #[arg(long, value_name = "REQUESTS")]
        max_remote_requests: Option<u64>,
        /// Register purrdf's first-party statistical aggregate set under this IRI
        /// namespace — identical to `query --aggregate-namespace`, reachable from a
        /// `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY`, which
        /// is the only place SPARQL UPDATE's grammar admits an aggregate. Omit it and
        /// every one of the ten names stays an unregistered custom-aggregate IRI.
        #[arg(long, value_name = "IRI")]
        aggregate_namespace: Option<String>,
        /// The SPARQL UPDATE text.
        update: String,
    },
    /// Materialize an entailment regime's closure over a source graph.
    Reason {
        /// The entailment regime to close under.
        #[arg(long, value_enum)]
        regime: CliRegime,
        /// RIF-in-XML rule document for `rif`; required by that regime and
        /// refused for every other (their rule table is the specification's).
        #[arg(long, value_name = "FILE")]
        rules: Option<PathBuf>,
        /// Surface the reasoning report: bare writes it to stderr,
        /// `--report=PATH` writes it to PATH.
        #[allow(clippy::option_option)]
        #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
        report: Option<Option<PathBuf>>,
        /// Input format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Output format override; inferred from the output extension when omitted.
        #[arg(long, value_enum)]
        to: Option<CliRdfFormat>,
        /// Base IRI, on BOTH legs. A relative IRI in the input resolves against it
        /// while parsing, and a target syntax that can write a base directive
        /// (turtle, trig, rdfxml, jsonld, yamlld) emits it as the closure document's
        /// base and relativizes against it. When omitted, a filesystem input still
        /// parses under its own `file://` retrieval IRI; stdin has none, so a
        /// relative IRI there is an error.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Output path `OUT`, or `-` for stdout (which requires `--to`).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Decide whether a premise entails a conclusion under an entailment regime.
    Entails {
        /// The entailment regime to decide under. Five of the seven are served;
        /// `owl-direct` and `rif` are each defined by an input this question does
        /// not carry, and the boundary refuses them by name.
        #[arg(long, value_enum)]
        regime: CliRegime,
        /// The premise document `FILE`, or `-` for stdin (which requires `--from`).
        #[arg(long, value_name = "FILE")]
        premise: String,
        /// The conclusion graph `FILE`, or `-` for stdin. The answer is a verdict:
        /// `entailed`, `not-entailed` (a proof), or `undecided`.
        #[arg(
            long,
            value_name = "FILE",
            conflicts_with = "pattern",
            required_unless_present = "pattern"
        )]
        conclusion: Option<String>,
        /// A basic graph pattern `FILE` (N-Triples with `?name` in any position,
        /// the predicate included), or `-` for stdin. The answer is its certain
        /// answers. A pattern is not an RDF document, so its bytes are handed to
        /// the boundary untranscoded and `--from` says nothing about it.
        #[arg(long, value_name = "FILE")]
        pattern: Option<String>,
        /// Re-decide the warrant of an `entailed` verdict WITHOUT running a
        /// reasoner, adding `warrant` and `verified` lines to the answer.
        #[arg(long, conflicts_with = "pattern")]
        verify: bool,
        /// An `owl:imports` the premise declares, resolved to a local document:
        /// repeatable, `IRI=FILE`. PurRDF fetches nothing, so an import no pair
        /// resolves is refused by name rather than treated as an empty document.
        #[arg(long = "import", value_name = "IRI=FILE")]
        imports: Vec<String>,
        /// Surface the reasoning certificate: bare writes it to stderr,
        /// `--report=PATH` writes it to PATH.
        #[allow(clippy::option_option)]
        #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
        report: Option<Option<PathBuf>>,
        /// Input format override for the premise, the conclusion and every
        /// `--import` document; inferred from each path's extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Base IRI for resolving relative IRIs while parsing those documents. A
        /// PARSE base only: the answer is a verdict rather than a document, and each
        /// input crosses the entailment boundary as N-Quads, whose grammar can
        /// express no base directive.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Answer path `OUT`, or `-` for stdout.
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Decide whether an OWL-Direct ontology has a model.
    ///
    /// The one DL service `reason`/`entails` cannot reach: both require a closure or a
    /// conclusion to exist, and an INCONSISTENT ontology has neither — it entails every
    /// triple, so `reason --regime owl-direct` refuses it and `entails` has no closure to
    /// decide a conclusion against. `consistency` asks the question directly, through
    /// [`purrdf_validate::regime::consistency_to_string`], the same string boundary the
    /// Python, WebAssembly and C-ABI hosts already reach.
    ///
    /// Prints two things to stdout, always: the one-line verdict (`consistency true |
    /// false | unknown`), then the full DL certificate — completeness, the reverse
    /// mapping's boundary list, and the search-cost counters (`steps`, `budget`, `work`,
    /// `work-budget`, `decisions`, `peak-nodes`, `disjunctions`, `peak-depth`). Unlike `--report` on the
    /// four materializing subcommands, the certificate here is not optional and not
    /// redirectable: this command answers exactly one question, and the certificate is
    /// the ONLY evidence of how completely the tableau answered it — hiding it behind a
    /// flag would restore the "the reasoner says no" ambiguity the certificate exists to
    /// remove, for the one caller — running this command by hand and reading the search's
    /// own completeness off it — who most needs it in hand by default.
    ///
    /// Exit codes: **0** for `true` OR `false` — both are DECIDED verdicts, and a decided
    /// `false` is not a failure of this command any more than a `false` ASK answer is a
    /// failure of `query`. **3** for `unknown`, exactly like a `query` a governor cut
    /// short: the certificate's `completeness budget-exhausted` line says a hypertableau
    /// run reached its round cap OR its work cap before saturating, so the run stopped
    /// incomplete rather than failed, and the exit code carries that distinction to a
    /// shell the same way it does for a tripped query. Which cap it was is read off the
    /// certificate: an exhausted run has `steps` at `budget` or `work` at `work-budget`.
    Consistency {
        /// Also record the run's PROOF TERM and print it after the certificate.
        ///
        /// Opt-in, and it costs what it records: the completion graph of every tableau run
        /// the decision made. Without it nothing is recorded and the run is exactly the one
        /// this command has always made — the verdict and the certificate are byte-identical
        /// either way, because recording is an observation the reasoner makes of itself
        /// rather than a lever it reads.
        ///
        /// The document is a `purrdf-dl-proof 1` block: a header derived from the term, then
        /// the term's own canonical bytes as lowercase hex. `--check-proof` verifies one.
        #[arg(long)]
        proof: bool,
        /// CHECK a `purrdf-dl-proof 1` document at PATH against THIS ontology, this question
        /// and this run's own answer, printing a `purrdf-dl-proof-check 1` report.
        ///
        /// Nothing about the check trusts the producer: the ontology is the one named on this
        /// command line, the question is re-derived here, and the claims are read out of this
        /// run's own answer. A proof for a different ontology, or a proof of a different
        /// answer, is refused. A document reading `availability not-recorded` is refused too,
        /// by name — an answer nobody asked to record is never presented as a verified one.
        #[arg(long, value_name = "PATH")]
        check_proof: Option<PathBuf>,
        /// Narrow the per-decision round cap the ontology's own size already derives;
        /// `0` (the default) applies no narrowing and runs under the derived cap alone.
        /// This can only TIGHTEN the cap, never loosen it — mirrors the `step_cap`
        /// parameter [`purrdf_validate::regime::consistency_to_string`] takes, so a
        /// caller narrowing here narrows the exact same knob the Python/WASM/C-ABI hosts
        /// do. A run this narrows into its cap answers `unknown` (exit 3) rather than
        /// `false`, never the reverse.
        #[arg(long, value_name = "N", default_value_t = 0)]
        step_cap: u32,
        /// Narrow the per-decision WORK cap the ontology's own size already derives; `0`
        /// (the default) applies no narrowing and runs under the derived cap alone. Like
        /// `--step-cap` this can only TIGHTEN, and it mirrors the `work_cap` parameter
        /// [`purrdf_validate::regime::consistency_to_string`] takes.
        ///
        /// It bounds what `--step-cap` structurally cannot. A round is a PASS over the
        /// completion graph rather than a unit of cost, so an ontology can make every
        /// round enormously more expensive without making the search take more rounds —
        /// one individual co-typed with several equivalence-defined classes does exactly
        /// that, and used to grind while the certificate reported a few percent of the
        /// round budget. This cap counts the matcher, scan, closure and clone work
        /// itself, and a run that reaches it answers `unknown` (exit 3) with `work` equal
        /// to `work-budget` in its certificate.
        #[arg(long, value_name = "N", default_value_t = 0)]
        work_cap: u32,
        /// Input format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Base IRI for resolving relative IRIs while parsing the input. A PARSE
        /// base only: this command answers with a verdict and a certificate rather
        /// than a document, so there is no serializer for one to reach.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
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
        /// Base IRI for resolving relative IRIs while parsing input RDF. A PARSE
        /// base only: the output is a carrier archive rather than an RDF document,
        /// so no serializer leg reads it.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
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
        /// Base IRI the RDF SERIALIZER writes as the output document's base and
        /// relativizes against, on a `--to` syntax that can express one (turtle,
        /// trig, rdfxml, jsonld, yamlld). `lift` reads a USTAR carrier archive
        /// rather than an RDF document, so there is no parse leg for a base to feed
        /// and no `file://` retrieval IRI is derived for the input.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Canonical USTAR input path `IN`, or `-` for stdin.
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// RDF output path `OUT`, or `-` for stdout.
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Validate an RDF data graph against a SHACL shapes graph.
    ///
    /// The answer is the W3C SHACL **validation report** — the artifact the SHACL
    /// specification defines the validation process to produce — serialized through
    /// `--format` into any of the nine native RDF syntaxes, or projected into SARIF 2.1.0
    /// for an editor or a code-scanning dashboard. Both come from the SAME
    /// `purrdf_shapes::engine` run and the SAME `purrdf_validate` writer the Python,
    /// WebAssembly and C-ABI hosts reach; there is no CLI-local validator and no CLI-local
    /// SARIF mapping.
    ///
    /// Exit codes: **0** whether the data CONFORMS or does not — both are decided verdicts,
    /// exactly as `consistency true|false` and a `false` ASK are, and the report on stdout is
    /// the answer either way. **1** for a malformed data or shapes document and for an
    /// unsupported SHACL construct (the engine hard-fails rather than skipping it). **2** for
    /// a usage error. **3** when a `--fuel`/`--deadline`/`--max-*` ceiling stopped the run:
    /// the engine returns no partial report by design (every SHACL constraint is a negative
    /// claim, so a truncated solution bag and a complete empty one read identically), so
    /// stdout carries NOTHING and stderr carries the governor report.
    ///
    /// The one-line verdict (`shacl conforms true|false`) and the result count are ALWAYS
    /// written to stderr, so a shell learns the answer without parsing the artifact on
    /// stdout — and stdout stays a well-formed RDF or SARIF document, which it could not if
    /// the verdict were interleaved into it.
    Validate {
        /// The SHACL shapes graph `FILE`, or `-` for stdin (which requires `--shapes-from`).
        #[arg(long, value_name = "FILE")]
        shapes: String,
        /// Shapes-graph format override; inferred from the shapes path's extension when
        /// omitted. Turtle is read through `purrdf_shapes::engine::parse_shapes`, the exact
        /// boundary every other host uses, which additionally recovers the shapes DOCUMENT's
        /// `@prefix`/`PREFIX` map as the fallback prefix environment for SHACL-AF `sh:select`
        /// queries. Every other syntax is parsed by the native codec into the same IR and
        /// carries no such fallback (it is a recovery from Turtle source text), so a SHACL-AF
        /// query in a non-Turtle shapes graph must declare its own `sh:prefixes`.
        #[arg(long = "shapes-from", value_enum)]
        shapes_from: Option<CliRdfFormat>,
        /// Expose the shapes graph to SHACL-SPARQL paths as a named graph under this IRI,
        /// overriding a `sh:shapesGraph` the shapes document declares. PurRDF mints no
        /// vocabulary IRIs, so there is no default: without this flag and without a
        /// `sh:shapesGraph` declaration the shapes graph is simply not exposed.
        #[arg(long = "shapes-graph", value_name = "IRI")]
        shapes_graph: Option<String>,
        /// Data-graph format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Base IRI for resolving relative IRIs while parsing the DATA graph. The shapes
        /// graph is a separate document and resolves against its OWN `file://` retrieval
        /// IRI (or its own `@base`), so this flag never silently retargets it. A PARSE
        /// base only: the validation report is a graph the engine mints with absolute
        /// terms, and it is serialized with no base rather than under this one.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// How to serialize the validation report: an RDF syntax for the SHACL results
        /// graph (the default, `ntriples`), or `sarif` for SARIF 2.1.0 JSON.
        #[arg(long, value_enum, default_value = "ntriples")]
        format: ValidateFormat,
        /// Bound the abstract execution steps every SHACL-SPARQL and SHACL-AF path in the
        /// validation charges, against ONE budget for the whole run. Inclusive; `0` trips at
        /// the first charge. Core constraint evaluation reads the IR directly and charges
        /// nothing, so a shapes graph with no SPARQL in it validates under any budget.
        #[arg(long, value_name = "UNITS")]
        fuel: Option<u64>,
        /// Wall-clock VALIDATION budget (`750ms`, `30s`, `1m30s`, `2h`). A trip writes no
        /// report, prints the governor receipt to stderr, and exits 3.
        #[arg(long, value_name = "DURATION", value_parser = crate::governors::parse_deadline)]
        deadline: Option<std::time::Duration>,
        /// Bound the largest intermediate solution bag, in cells (rows × columns), across
        /// every SPARQL path the validation runs.
        #[arg(long, value_name = "CELLS")]
        max_intermediate_cells: Option<u64>,
        /// Bound the bytes minted into the per-validation scratch arena.
        #[arg(long, value_name = "BYTES")]
        max_scratch_bytes: Option<u64>,
        /// Bound the requests a `SERVICE` clause in a SHACL-SPARQL constraint issues.
        #[arg(long, value_name = "REQUESTS")]
        max_remote_requests: Option<u64>,
        /// Data-graph path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Report path `OUT`, or `-` for stdout (the default).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Validate RDF nodes against a ShEx 2.1 schema through a query shape map.
    ///
    /// The answer is the ShapeMap specification's **result shape map**: a JSON array of
    /// `{"node","shape","status","reason"?}` objects, one per resolved association, in the
    /// engine's own deterministic order (query selectors de-duplicate and sort by term
    /// string). That is `purrdf_shex`'s single rendered form, so there is no `--format`
    /// choice to make and no second renderer to disagree with it.
    ///
    /// Exit codes: **0** whether every association CONFORMS or none does. **1** for a
    /// malformed schema, a schema that violates the spec §5.7 structural requirements, an
    /// unresolved `IMPORT`, an `EXTERNAL` shape with no semantics to decide against, a
    /// malformed shape map, or an unreadable data graph. **2** for a usage error. There is no
    /// **3**: the ShEx engine takes no execution governors.
    ///
    /// The one-line verdict (`shex conformant true|false`) and the entry counts are ALWAYS
    /// written to stderr, for the reason `validate`'s are.
    Shex {
        /// The ShEx schema `FILE`, or `-` for stdin (which requires `--schema-from`).
        #[arg(long, value_name = "FILE")]
        schema: String,
        /// Schema syntax override; inferred from the schema path's extension when omitted
        /// (`.shex`/`.shexc` → `shexc`, `.json`/`.shexj` → `shexj`).
        #[arg(long = "schema-from", value_enum)]
        schema_from: Option<CliShexFormat>,
        /// An `IMPORT` the schema declares, resolved to a local document: repeatable,
        /// `IRI=FILE`. PurRDF fetches nothing, so an import no pair resolves is refused by
        /// name rather than treated as an empty schema — and a pair the schema's import
        /// closure never reaches is refused too, rather than silently unused. Each imported
        /// document's syntax is inferred from its own extension.
        #[arg(long = "import", value_name = "IRI=FILE")]
        imports: Vec<String>,
        /// Data-graph path, or `-` for stdin (which requires `--from`).
        #[arg(long)]
        data: String,
        /// Data-graph format override; inferred from `--data`'s extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Base IRI for the relative IRIs of the DATA graph and the MAP — the two inputs
        /// with no base of their own (a shape map is command-line text, so `--base` is the
        /// only base it can ever have). NOT the schema's: `--schema` is an independent
        /// document and resolves its relative IRIs — in BOTH syntaxes, since ShExJ is a
        /// JSON-LD dialect whose IRI-valued members are document-relative exactly as ShExC's
        /// IRIREFs are — against its own `file://` retrieval IRI, or its `BASE` directive.
        /// Each `--import`ed document likewise resolves against the import IRI.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// The query shape map: `<node>@<shape>` associations separated by commas, where a
        /// node is `<iri>` / `_:label` / a Turtle literal / a triple-pattern selector
        /// (`{FOCUS <p> _}`, `{FOCUS a <C>}`, `{_ <p> FOCUS}`), and a shape is `START` or
        /// `<label>`.
        #[arg(value_name = "MAP")]
        map: String,
        /// Result-shape-map path `OUT`, or `-` for stdout (the default).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Extract the Symmetric Concise Bounded Description of one or more resources.
    ///
    /// The SCBD is what `purrdf_core::describe` computes and what SPARQL `DESCRIBE` returns
    /// in this engine — one authority, reached here through the same `Describer` rather than
    /// re-derived. It is symmetric (incoming links as well as outgoing), closes blank nodes
    /// transitively in both directions, and carries the RDF 1.2 statement layer: the reifiers
    /// whose reified triple touches the closure, and their annotations.
    ///
    /// A dedicated verb rather than sugar over `query "DESCRIBE <iri>"`, for three reasons an
    /// operator meets immediately. The SPARQL route's `--results-format` defaults to `json`,
    /// which is illegal for a graph result — so the obvious `purrdf query --data d.ttl
    /// 'DESCRIBE <x>'` HARD-FAILS, while `describe` resolves `--to`/the `OUT` extension
    /// exactly as `convert` and `reason` do. It takes IRIs as ARGUMENTS, so a script naming
    /// a resource does not have to build SPARQL text around it. And being an RDF-emitting
    /// verb, `--loss-ledger` and `--jsonld-options` apply to it exactly as they do to
    /// `convert` — a description whose statement layer cannot survive the target syntax
    /// records the drop instead of losing it silently.
    Describe {
        /// A resource to describe: repeatable, and at least one is required. Several are
        /// described as ONE union subgraph (the same union `DESCRIBE <a> <b>` returns), not
        /// as several documents.
        #[arg(long = "iri", value_name = "IRI", required = true)]
        iris: Vec<String>,
        /// Input format override; inferred from the input extension when omitted.
        #[arg(long, value_enum)]
        from: Option<CliRdfFormat>,
        /// Output format override; inferred from the output extension when omitted.
        #[arg(long, value_enum)]
        to: Option<CliRdfFormat>,
        /// Base IRI, on BOTH legs: relative IRIs in the input resolve against it while
        /// parsing, and a `--to` syntax that can write a base directive (turtle, trig,
        /// rdfxml, jsonld, yamlld) emits it as the description's base and relativizes
        /// against it. When omitted, a filesystem input is still parsed under its own
        /// `file://` retrieval IRI.
        #[arg(long, value_name = "IRI", value_parser = parse_base_iri)]
        base: Option<String>,
        /// Input path `IN`, or `-` for stdin (which requires `--from`).
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
        /// Output path `OUT`, or `-` for stdout (which requires `--to`).
        #[arg(value_name = "OUT", default_value = "-")]
        output: String,
    },
    /// Pack container utilities.
    Pack {
        /// The pack subcommand to run.
        #[command(subcommand)]
        command: PackCommand,
    },
}

/// The `--format` choices `validate` accepts: the nine native RDF syntaxes, which serialize
/// the W3C SHACL **validation report graph**, plus `sarif`, which projects the same report
/// into a SARIF 2.1.0 log.
///
/// # Why the results graph is the default and SARIF is the option
///
/// Both already ship, and both come from the same engine run, so the choice is about which
/// one is the ANSWER and which is a projection of it.
///
/// The SHACL specification defines the validation process to produce a **validation report**,
/// an RDF graph of `sh:ValidationResult` nodes hung off a `sh:ValidationReport`. That graph is
/// the answer in the language of the question: it names the focus node, the value node, the
/// result path, the source shape and the source constraint component as RDF terms, and it
/// composes with the rest of this binary — a report is a document `purrdf query` can query,
/// `purrdf convert` can transcode, and `purrdf validate` can itself validate. Making it the
/// default means the command's out-of-the-box answer is the artifact the specification names.
///
/// SARIF is a projection of that report into a different vocabulary for a different consumer:
/// a `level`, a `ruleId` and a `physicalLocation` an editor or a code-scanning dashboard can
/// render. It is genuinely lossy in the direction that matters here — several SHACL severities
/// collapse onto SARIF's three levels (the verbatim IRI survives only in a property bag), and
/// the RDF term structure becomes strings. That makes it exactly right for the CI consumer and
/// exactly wrong as the artifact everything else is derived from, so it is a named opt-in
/// rather than the default.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidateFormat {
    /// The SHACL results graph as N-Triples (the default).
    #[value(name = "ntriples", alias = "nt", alias = "n-triples")]
    Ntriples,
    /// The SHACL results graph as Turtle.
    #[value(name = "turtle", alias = "ttl")]
    Turtle,
    /// The SHACL results graph as TriG.
    #[value(name = "trig")]
    Trig,
    /// The SHACL results graph as N-Quads.
    #[value(name = "nquads", alias = "nq", alias = "n-quads")]
    Nquads,
    /// The SHACL results graph as RDF/XML.
    #[value(name = "rdfxml", alias = "rdf", alias = "xml")]
    Rdfxml,
    /// The SHACL results graph as TriX.
    #[value(name = "trix")]
    Trix,
    /// The SHACL results graph as HexTuples.
    #[value(name = "hextuples", alias = "hext")]
    Hextuples,
    /// The SHACL results graph as JSON-LD.
    #[value(name = "jsonld", alias = "json-ld")]
    Jsonld,
    /// The SHACL results graph as YAML-LD.
    #[value(name = "yamlld", alias = "yaml-ld")]
    Yamlld,
    /// SARIF 2.1.0 JSON, for an editor or a code-scanning dashboard.
    #[value(name = "sarif")]
    Sarif,
}

impl ValidateFormat {
    /// The [`NativeRdfFormat`] this choice serializes the results GRAPH through, or `None`
    /// for [`Self::Sarif`], which is not RDF at all.
    ///
    /// `None` is the single discriminator the `validate` lane branches on, so the SARIF
    /// arm is never reached by a fallible unwrap of a format that does not exist.
    pub(crate) const fn to_rdf_format(self) -> Option<NativeRdfFormat> {
        use NativeRdfFormat as N;
        match self {
            Self::Ntriples => Some(N::NTriples),
            Self::Turtle => Some(N::Turtle),
            Self::Trig => Some(N::TriG),
            Self::Nquads => Some(N::NQuads),
            Self::Rdfxml => Some(N::RdfXml),
            Self::Trix => Some(N::TriX),
            Self::Hextuples => Some(N::HexTuples),
            Self::Jsonld => Some(N::JsonLd),
            Self::Yamlld => Some(N::YamlLd),
            Self::Sarif => None,
        }
    }
}

/// The ShEx schema syntaxes `--schema-from` accepts.
///
/// Two, because the ShEx 2.1 specification defines two and `purrdf-shex` implements both:
/// the compact syntax (ShExC, §6) and the JSON wire format (ShExJ, Appendix A). They are the
/// same schema, and `purrdf shex` decides between them the way every other format in this
/// binary is decided — an explicit choice wins, otherwise the path's extension classifies it.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliShexFormat {
    /// ShExC, the compact syntax (`.shex`, `.shexc`).
    #[value(name = "shexc", alias = "shex", alias = "compact")]
    Shexc,
    /// ShExJ, the JSON wire format (`.shexj`, `.json`).
    #[value(name = "shexj", alias = "json")]
    Shexj,
}

/// The `pack` subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum PackCommand {
    /// Verify a pack container's full integrity — every section digest AND the
    /// RDFC-1.0 canonical-identity digest. Prints the verified 64-hex digest and
    /// exits 0; a corrupt or non-pack input exits non-zero with a message.
    ///
    /// The ordinary read/reason paths already verify a pack on every open (nothing
    /// enters the pipeline unverified); this is the explicit surface for confirming a
    /// pack in isolation, without running a conversion or query.
    Verify {
        /// Pack path `IN`, or `-` for stdin.
        #[arg(value_name = "IN", default_value = "-")]
        input: String,
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
    /// Native RDF DCAT description view.
    #[value(name = "dcat-rdf")]
    DcatRdf,
    /// VoID dataset-description and linkset view.
    Void,
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
            Self::DcatRdf => ProjectionProfile::DcatRdf,
            Self::Void => ProjectionProfile::Void,
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
    // No explicit `#[value(name = ...)]`: clap's default kebab-case rendering of this
    // variant IS the pack container's clap spelling, so it is not re-declared as a
    // literal here — the identifier lives once, in `purrdf_rdf::PACK_EXTENSIONS`.
    Pack,
    /// The GTS transport container. INPUT only: it is read through the authoritative
    /// event importer (per-segment blank-node scope preserved), and named as a `--to`
    /// target it is refused by name rather than silently written as something else —
    /// see `crate::format::refuse_gts_target`.
    // Same rule as `Pack`: clap's kebab-case rendering IS the spelling, so the literal
    // lives once, in `purrdf_rdf::GTS_EXTENSIONS`.
    Gts,
}

impl CliRdfFormat {
    /// Resolve this explicit choice to the pipeline's [`SourceFormat`].
    pub(crate) fn to_source_format(self) -> SourceFormat {
        use purrdf_rdf::NativeRdfFormat as N;
        use purrdf_rdf::SourceFormat as S;
        match self {
            Self::Turtle => S::Native(N::Turtle),
            Self::Trig => S::Native(N::TriG),
            Self::Ntriples => S::Native(N::NTriples),
            Self::Nquads => S::Native(N::NQuads),
            Self::Rdfxml => S::Native(N::RdfXml),
            Self::Trix => S::Native(N::TriX),
            Self::Hextuples => S::Native(N::HexTuples),
            Self::Jsonld => S::Native(N::JsonLd),
            Self::Yamlld => S::Native(N::YamlLd),
            Self::Pack => S::Pack,
            Self::Gts => S::Gts,
        }
    }
}

/// The `--transport` choices: how a gzip/zstd wrapper around an input is handled.
///
/// A transport encoding is not a format — `data.nt.gz` is an N-Triples document that
/// arrived gzipped — so it is decided by its own flag rather than by `--from`. `auto` is
/// the default and the only value most callers ever need; the three explicit values
/// exist so an operator can OVERRIDE the sniff rather than argue with it.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CliTransport {
    /// Sniff the leading bytes (gzip `1f 8b`, zstd `28 b5 2f fd`), then the filename
    /// suffix, and decode whatever is found.
    #[default]
    #[value(name = "auto")]
    Auto,
    /// Read the bytes verbatim; do not decode even a stream that sniffs as wrapped.
    #[value(name = "none")]
    None,
    /// Decode as gzip. A stream that is not gzip hard-fails.
    #[value(name = "gzip", alias = "gz")]
    Gzip,
    /// Decode as zstd. A stream that is not zstd hard-fails.
    #[value(name = "zstd", alias = "zst")]
    Zstd,
}

impl CliTransport {
    /// The pipeline policy this choice names.
    pub(crate) fn to_policy(self) -> TransportPolicy {
        match self {
            Self::Auto => TransportPolicy::Detect,
            Self::None => TransportPolicy::Verbatim,
            Self::Gzip => TransportPolicy::Forced(TransportEncoding::Gzip),
            Self::Zstd => TransportPolicy::Forced(TransportEncoding::Zstd),
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
    /// OWL Direct (DL) entailment via the tableau. A document pipeline has no
    /// query to direct it, so it runs the query-independent augmentation.
    #[value(name = "owl-direct")]
    OwlDirect,
    /// RIF-Core entailment under the rule set `--rules` names.
    #[value(name = "rif")]
    Rif,
    /// Datatype (D) entailment — Simple plus OWL 2 Profiles §4.3 Table 8.
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
