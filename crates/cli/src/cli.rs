// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The clap command tree: the `purrdf` binary's argument model.
//!
//! One pipeline, seven subcommands ([`Command`]), and one global flag
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
//! is a usage error rather than an empty file.
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
    about = "PurRDF: convert, query, update, reason, decide entailment, decide consistency, \
             project, and lift RDF 1.2 data",
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

/// The eight pipeline subcommands.
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
        /// `AGG(<NAMESPACE><NAME>, args…)`, e.g. `AGG(<https://ex.example/agg#MEDIAN>,
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
        /// Base IRI for parsing the data and UPDATE request.
        #[arg(long, value_name = "IRI")]
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
        /// Base IRI for resolving relative IRIs while parsing those documents.
        #[arg(long, value_name = "IRI")]
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
        /// Base IRI for resolving relative IRIs while parsing the input.
        #[arg(long, value_name = "IRI")]
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
