// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `query` subcommand: evaluate a SPARQL query over a data source.
//!
//! The data source is opened as a view (a pack is queried zero-copy) and the
//! prepared query is evaluated over it with [`NativeSparqlEngine`]. Both the input
//! parse and the query itself resolve relative IRIs against `--base`.
//!
//! ## `--entailment`: query the closure, not the raw view
//!
//! Without `--entailment` the query runs over the raw view (text `RdfDataset` or a
//! zero-copy `PackView`). With `--entailment REGIME` the pipeline enters the reasoner
//! over that SAME view — a pack is NOT rebuilt into an owned dataset to materialize —
//! and answers the query UNDER the regime through
//! [`purrdf::query_with_entailment_governed`] — the library's own entailment-aware query
//! entry point — using the same `EntailmentPlan` `reason` resolves, so `--rules` means the
//! same thing here. The run's reasoning report is surfaced under `--report`, so a solution
//! set drawn from a closure can be read beside the evidence of what closed it.
//!
//! Calling that entry point rather than open-coding "materialize the closure, then
//! evaluate over it" is load-bearing and not a refactor: the OWL-Direct lane is
//! QUERY-DIRECTED, and a pipeline that materializes first has no query to direct it
//! with. See [`entailed_query`] for the answer that behaviour cost.
//!
//! It is the GOVERNED entry point unconditionally, with `QueryGovernors::UNBOUNDED` when
//! the operator named no ceiling — so `--entailment` and a governor flag are one lane rather
//! than two, and there is no second code path for the combination to be forgotten on.
//!
//! ## The result-shape × format-kind dispatch
//!
//! `--results-format` is a superset [`QueryFormat`] of the four SPARQL-results
//! serializations and the nine RDF syntaxes; the result SHAPE selects which half is
//! legal:
//!
//! * SELECT [`Solutions`](SparqlResult::Solutions) / ASK
//!   [`Boolean`](SparqlResult::Boolean) + a SPARQL-results format → the W3C results
//!   serializer to stdout (a format that cannot carry the shape — CSV/TSV vs a
//!   boolean — surfaces the serializer's own error);
//! * a CONSTRUCT/DESCRIBE [`Graph`](SparqlResult::Graph) + an RDF syntax → the SAME
//!   [`sink::write_rdf`] the `convert`/`reason` lanes use, so a star-incapable target
//!   (e.g. RDF/XML) projects the RDF-1.2 statement layer and the loss ledger records
//!   the drop (the universal-sink invariant), surfaced under `--loss-ledger`;
//! * a shape/format-kind MISMATCH (solutions/boolean + an RDF syntax, or a graph +
//!   a SPARQL-results format) is a hard runtime error (exit 1);
//! * a graph result carrying a NAMED GRAPH + a single-graph syntax (turtle /
//!   ntriples / rdfxml) is a usage refusal (exit 2), naming the graphs and a
//!   quad-capable alternative — see [`refuse_uncarriable_named_graphs`] for why
//!   this one is refused rather than serialized-and-ledgered.
//!
//! ## The governed lane, and what a trip puts on which stream
//!
//! With any of the six governor flags the query runs through the engine's governed entry
//! point instead, over the same view, and comes back as a
//! [`GovernedOutcome`]: complete, or stopped by a governor and carrying the certificate of
//! what the rows in hand bound. Without a governor flag the ungoverned call is made
//! verbatim — the governed lane is a branch rather than a wrapper. (`--entailment` is the
//! exception, deliberately: that lane takes a `QueryGovernors` on every call, `UNBOUNDED`
//! when no flag was named, so its two-phase governing is written once.)
//!
//! A trip is not a failure ([`CliOutcome`], and `error`'s module documentation for why it
//! is exit 3), so it prints:
//!
//! * the governor report to **stderr** — which governor stopped the run, what the rows
//!   bound, and the whole consumption/ceiling vector — written FIRST, so that a trip is
//!   announced even if serializing the rows then fails;
//! * the certified rows to **stdout**, through the same [`emit_result`] a complete result
//!   goes through, in the requested `--results-format`.
//!
//! Nothing about the trip is interleaved into stdout. A caller piping SPARQL-Results JSON
//! or XML receives a WELL-FORMED document of the rows that were certified — the partial
//! status is carried by the exit code and the stderr report, because an in-band marker
//! would either corrupt that document or require inventing a non-W3C extension to four
//! serializations. The one case with nothing to print is
//! [`PartialAnswers::Unknown`](purrdf_sparql_eval::PartialAnswers::Unknown), where no bound
//! survived to the root: there are structurally no rows to hand out, so stdout stays empty
//! rather than carrying a serialized empty result, which would be an "there are no answers"
//! claim the run cannot make. The report's `barrier` line names the operator that withheld
//! them.
//!
//! ## `--explain`
//!
//! `--explain` prints [`QueryExplanation::render`] to stdout in place of the answers, the
//! way `EXPLAIN` replaces a result set everywhere else. The rendering is deterministic — a
//! profile header, the charge schedule, one line per algebra node with the planner's
//! estimate beside the count that materialized, the join orders, and the per-dimension
//! consumption — and it is produced by ONE evaluation, because
//! [`NativeSparqlEngine::explain_query_with_options_view`] evaluates the query itself under
//! the metering profile.
//!
//! That profile is why `--explain` refuses a governor flag rather than accepting it:
//! metering engages every counter at a ceiling nothing can reach, so a `--fuel 1000
//! --explain` would print the receipt of a run that was never bounded by 1000. It refuses
//! `--entailment` for the same shape of reason — the entailment-aware query lane is a
//! different entry point with no explanation to give — and it refuses
//! `--provenance-namespace`, `--results-format`, `--loss-ledger`, and `--jsonld-options`
//! for a third shape: each names something about the ANSWERS `--explain` never produces
//! (an extension to anchor, a serialization to pick, a lossy-transcode report to surface,
//! a JSON-LD/YAML-LD serializer to configure), so accepting any of them would be a flag
//! that is parsed, validated, and then never acted on. Every refusal names what to drop
//! (see [`refuse_unenforceable_combinations`]). `--entailment` beside a governor flag and
//! `--aggregate-namespace` beside either `--entailment` or `--explain` used to be
//! refusals too; both now run instead of being refused.
//!
//! The rendered `relations` block lists whatever `--path-relation` registered — that flag
//! is this binary's property-function registration surface (see [`crate::path_relation`]),
//! and [`ExplainOp::run`] sets
//! [`QueryOptions::property_functions`](purrdf_sparql_eval::QueryOptions::property_functions)
//! from the SAME registry every other lane evaluates under. Without the flag the block is
//! empty, which is the honest minimal form rather than a missing feature —
//! [`QueryExplanation::render`] always emits the block, empty or not, precisely so "no
//! relations were in scope" and "this build does not report relations" stay
//! distinguishable bytes. The `aggregates` block is the same case: with
//! `--aggregate-namespace`, [`ExplainOp::run`] sets
//! [`QueryOptions::aggregates`](purrdf_sparql_eval::QueryOptions::aggregates) on the SAME
//! [`NativeSparqlEngine::explain_query_with_options_view`] call — there is one explain code
//! path, not a registry-free entry and a registry-carrying one that could drift apart, which
//! is exactly the shape gap a per-registry explain entry (removed; see
//! [`NativeSparqlEngine::explain_query_with_options`]'s documentation) exposed for a query
//! using more than one registered extension at once — and the block lists the ten
//! registered statistical aggregate IRIs.

use std::sync::Arc;

use purrdf::GovernedEntailment;
use purrdf_core::named_graph::{distinct_graph_names, named_graph_refusal};
use purrdf_core::{DatasetView, LossLedger, SparqlRequest, SparqlResult};
use purrdf_entail::EntailError;
use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_rdf::{NativeRdfFormat, SourceFormat};
use purrdf_sparql_eval::{
    AggregateRegistry, GovernedOutcome, NativeSparqlEngine, PreparedQuery,
    PropertyFunctionRegistry, QueryExplanation, QueryGovernors, QueryOptions as EngineQueryOptions,
};
use purrdf_sparql_results::{
    ProvenanceNamespace, ResultProvenance, SparqlResultsFormat, serialize,
};
use sha2::{Digest, Sha256};

use crate::cli::{CliRegime, LedgerTarget, QueryFormat, ReportTarget};
use crate::error::{CliError, CliOutcome};
use crate::format;
use crate::governors::{self, GovernorFlags};
use crate::ledger;
use crate::path_relation::{self, PathRelationSpec};
use crate::reason;
use crate::report;
use crate::sink;
use crate::source::{self, ViewOp};

/// The generic query operation: evaluate the prepared query over whichever concrete
/// view the data source resolved to, returning the (fully owned) [`SparqlResult`].
///
/// A [`SparqlResult`] borrows nothing from the view — `Graph` is an `Arc<RdfDataset>`
/// and every solution cell is an owned `TermValue` — so the result outlives the
/// borrowed view and is emitted after `run_over_input` returns.
struct QueryOp<'a> {
    engine: &'a NativeSparqlEngine,
    prepared: &'a PreparedQuery,
    /// The registry `--aggregate-namespace` built, or `None` when the flag was not
    /// given. This MUST be the exact same registry instance `prepared` was parsed
    /// against (see [`AggregateRegistry`]'s instance-identity fingerprint) — never a
    /// freshly built one, even with identical content, or evaluation refuses the plan.
    aggregates: Option<&'a AggregateRegistry>,
    /// The `--path-relation` specs to snapshot over this view. See [`prepare_against`]
    /// for why the registry is born here rather than beside the flags.
    relations: RelationSpecs<'a>,
}

/// The `--path-relation` specs one lane will snapshot, plus the query text they force a
/// re-prepare of.
///
/// # Why the registry is built inside [`ViewOp::run`]
///
/// [`purrdf_sparql_eval::PathGraph::from_dataset`] reads the step's edges out of the
/// dataset being queried, so a registry cannot exist before the data source has been
/// opened. And a plan must be prepared against the registry it is evaluated under —
/// the registry is what decides which predicates became CALL nodes at all, and the engine
/// refuses a plan/registry disagreement rather than silently evaluating a relation's
/// predicate as an ordinary triple pattern. Both therefore happen inside `run`, where the
/// concrete view is in hand; the engine's plan cache makes the second parse of the same
/// text free.
///
/// The outer prepare in [`run`] is kept and reused verbatim whenever no `--path-relation`
/// was given, so a malformed query is still a parse error raised before the data source
/// is read, exactly as it was before this flag existed.
#[derive(Clone, Copy)]
struct RelationSpecs<'a> {
    /// The parsed specs, empty when `--path-relation` was not given.
    specs: &'a [PathRelationSpec],
    /// The query text, re-prepared against the registry when there is one.
    query: &'a str,
    /// `--base`, threaded into that re-prepare so both parses resolve identically.
    base: Option<&'a str>,
}

impl<'a> RelationSpecs<'a> {
    /// Snapshot the specs over `view` and prepare the plan the resulting registry
    /// implies, returning both plus the [`EngineQueryOptions`] they must be evaluated
    /// under.
    ///
    /// The returned plan is `None` when no `--path-relation` was given: the caller then
    /// uses the plan it prepared before the source was read, which is the pre-existing
    /// behaviour byte-for-byte.
    fn prepare_against<D: DatasetView>(
        self,
        engine: &NativeSparqlEngine,
        view: &D,
        aggregates: Option<&'a AggregateRegistry>,
    ) -> Result<(Option<PropertyFunctionRegistry>, Option<Arc<PreparedQuery>>), CliError> {
        let relations = path_relation::build_registry(view, self.specs)?;
        let Some(registry) = relations else {
            return Ok((None, None));
        };
        let prepared = engine.prepare_query_with_options(
            self.query,
            self.base,
            EngineQueryOptions {
                aggregates: aggregates.unwrap_or(&AggregateRegistry::EMPTY),
                property_functions: &registry,
                ..EngineQueryOptions::EMPTY
            },
        )?;
        Ok((Some(registry), Some(prepared)))
    }
}

/// The canonical empty aggregate registry, as a `static` rather than a temporary: the
/// options this module builds outlive the expression that builds them, and a
/// `HashMap`-backed registry's drop glue blocks Rust's rvalue static promotion for a
/// reference that must live that long. (`crate::update` states the same reason.)
static EMPTY_AGGREGATES: AggregateRegistry = AggregateRegistry::EMPTY;
/// The canonical empty property-function registry; see [`EMPTY_AGGREGATES`].
static EMPTY_RELATIONS: PropertyFunctionRegistry = PropertyFunctionRegistry::EMPTY;

/// The evaluation options a lane runs under, given what its two registries resolved to.
fn engine_options<'a>(
    aggregates: Option<&'a AggregateRegistry>,
    relations: Option<&'a PropertyFunctionRegistry>,
) -> EngineQueryOptions<'a> {
    EngineQueryOptions {
        aggregates: aggregates.unwrap_or(&EMPTY_AGGREGATES),
        property_functions: relations.unwrap_or(&EMPTY_RELATIONS),
        ..EngineQueryOptions::EMPTY
    }
}

impl ViewOp for QueryOp<'_> {
    type Output = SparqlResult;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<SparqlResult, CliError> {
        let (relations, prepared) =
            self.relations
                .prepare_against(self.engine, view, self.aggregates)?;
        let options = engine_options(self.aggregates, relations.as_ref());
        Ok(self.engine.query_prepared_view(
            view,
            prepared.as_deref().unwrap_or(self.prepared),
            &[],
            options,
        )?)
    }
}

/// Build the statistical-aggregate registry `--aggregate-namespace` requests, or
/// `None` when the flag is absent — the CLI's SOLE aggregate-registration surface.
///
/// `AggregateRegistry::register_statistical_aggregates` takes only an IRI namespace
/// string, so it crosses this command-line boundary the same way `--property-fn-namespaces`
/// would if this binary had a relations surface to declare one over: no callback, no
/// per-aggregate marshaling. The general custom-aggregate seam
/// (`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`, an arbitrary
/// `init`/`step`/`combine`/`finish` closure) is a Rust-host-only capability with no
/// string-shaped surface at all — it genuinely cannot reach a command-line flag — and this
/// binary does not attempt to expose it.
pub(crate) fn build_aggregate_registry(namespace: Option<&str>) -> Option<AggregateRegistry> {
    let namespace = namespace?;
    let mut registry = AggregateRegistry::new();
    registry.register_statistical_aggregates(namespace);
    Some(registry)
}

/// Parse `--provenance-namespace PREFIX=IRI` into its raw `(prefix, iri)` halves.
///
/// A bare split on the first `=` — [`ProvenanceNamespace::new`] does the real
/// validation (a well-formed XML NCName prefix that is neither `xml` nor `xmlns`, and an
/// absolute IRI) once [`run`] builds the namespace, so this clap `value_parser` only has
/// to find the separator and name a missing one.
pub(crate) fn parse_provenance_namespace(text: &str) -> Result<(String, String), String> {
    let (prefix, iri) = text.split_once('=').ok_or_else(|| {
        format!(
            "--provenance-namespace must be `PREFIX=IRI` (e.g. \
             `prov=https://example.org/ns/prov#`), got `{text}` with no `=`"
        )
    })?;
    Ok((prefix.to_owned(), iri.to_owned()))
}

/// Build the [`ResultProvenance`] a governed/ungoverned SPARQL-results emission carries:
/// empty when no `--provenance-namespace` was supplied (pure-W3C output, unchanged from
/// before this flag existed), or populated with a content hash of the query text plus this
/// engine's label when one was.
///
/// `query_hash` is `sha256:` followed by the lowercase hex digest of the UTF-8 query text —
/// an opaque, deterministic query identity a caller can compare across runs, computed from
/// data this host already has in hand rather than anything the evaluator would need to
/// track. `solutions` stays empty: per-solution source provenance is the evaluator/S11
/// derivation graph's progressive fill (see `purrdf_sparql_results::ResultProvenance`'s
/// module docs), not something this command-line host can populate on its own.
fn build_query_provenance(
    namespace: Option<&ProvenanceNamespace>,
    query: &str,
) -> ResultProvenance {
    if namespace.is_none() {
        return ResultProvenance::default();
    }
    let digest = Sha256::digest(query.as_bytes());
    ResultProvenance {
        query_hash: Some(format!("sha256:{digest:x}")),
        engine: Some("purrdf-sparql-eval".to_owned()),
        solutions: Vec::new(),
    }
}

/// The GOVERNED query operation: evaluate the prepared query over the concrete view under
/// the caller's ceilings, returning the outcome rather than a result.
///
/// A separate operation rather than a flag on [`QueryOp`] because the return type is
/// genuinely different: a governed run answers "complete, or stopped — and here is the
/// receipt", and collapsing that into a `SparqlResult` at this seam would throw away the
/// only thing that distinguishes a truncated answer from a whole one.
///
/// It carries the FLAGS rather than a built [`QueryGovernors`], and builds them inside
/// [`ViewOp::run`], because `run` is called with the view already open — so a `--deadline`
/// becomes a running clock at the last moment before evaluation rather than before the
/// data source has even been read. Handing this a pre-built configuration would silently
/// charge a large file's parse time to the caller's evaluation budget, which is not what
/// `--deadline`'s help promises and not a budget an operator could reason about.
struct GovernedQueryOp<'a> {
    engine: &'a NativeSparqlEngine,
    prepared: &'a PreparedQuery,
    flags: GovernorFlags,
    /// The SAME registry instance `prepared` was parsed against; see [`QueryOp::aggregates`].
    aggregates: Option<&'a AggregateRegistry>,
    /// The `--path-relation` specs to snapshot over this view; see [`RelationSpecs`].
    relations: RelationSpecs<'a>,
}

impl ViewOp for GovernedQueryOp<'_> {
    type Output = GovernedOutcome;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<GovernedOutcome, CliError> {
        let governors: QueryGovernors = self.flags.to_governors();
        let (relations, prepared) =
            self.relations
                .prepare_against(self.engine, view, self.aggregates)?;
        // `QueryOptions::EMPTY` for every axis but the two registries: the CLI wires no
        // SHACL-AF function table. Both registries here are the SAME instances the plan
        // being evaluated was parsed against — `aggregates` from `run` below, and
        // `property_functions` from the re-prepare `prepare_against` just did over this
        // view — which is what the engine's plan/registry identity check demands.
        let options = engine_options(self.aggregates, relations.as_ref());
        Ok(self.engine.query_prepared_governed_view(
            view,
            prepared.as_deref().unwrap_or(self.prepared),
            &[],
            options,
            &governors,
        )?)
    }
}

/// The `--explain` operation: explain the query against whichever concrete view the data
/// source resolved to.
///
/// It takes the query TEXT rather than a `PreparedQuery` because that is the shape of the
/// engine's explanation entry point — it surveys the plan against this view's cardinalities
/// and then evaluates it under the metering profile — and the plan cache makes the second
/// parse of the same text free.
struct ExplainOp<'a> {
    engine: &'a NativeSparqlEngine,
    query: &'a str,
    base: Option<&'a str>,
    /// The SAME registry `--aggregate-namespace` builds for every other lane; see
    /// [`QueryOp::aggregates`] for why identity (not merely content) matters.
    aggregates: Option<&'a AggregateRegistry>,
    /// The `--path-relation` specs to snapshot over this view; see [`RelationSpecs`].
    /// This lane needs no re-prepare of its own — the explain entry takes the query TEXT
    /// and parses it against the options it is handed — so only the registry is used.
    relations: RelationSpecs<'a>,
}

impl ViewOp for ExplainOp<'_> {
    type Output = QueryExplanation;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<QueryExplanation, CliError> {
        // Routed through the one options-carrying explain entry rather than a
        // narrower per-registry explain entry (none exist any more; see
        // `NativeSparqlEngine::explain_query_with_options`'s documentation): it is the
        // one CLI code path for "explain, optionally with an aggregate registry, a
        // property-function registry, or both", so the two cannot drift apart. The
        // rendered `relations` block therefore lists exactly what `--path-relation`
        // registered.
        let relations = path_relation::build_registry(view, self.relations.specs)?;
        let options = engine_options(self.aggregates, relations.as_ref());
        Ok(self
            .engine
            .explain_query_with_options_view(view, self.query, self.base, options)?)
    }
}

/// Emit a SPARQL result to stdout, dispatching on the result shape × format kind, and
/// surface the loss ledger the emission produced.
#[allow(
    clippy::too_many_arguments,
    reason = "each parameter is a distinct, independently-named input; bundling them into a \
              struct would not shrink the call sites, which already name every field"
)]
fn emit_result(
    result: &SparqlResult,
    results_format: QueryFormat,
    base: Option<&str>,
    jsonld_options: Option<&JsonLdSerializeOptions>,
    provenance_namespace: Option<&ProvenanceNamespace>,
    query: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    match result {
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            if jsonld_options.is_some() {
                return Err(CliError::Usage(
                    "--jsonld-options requires an RDF graph result serialized as JSON-LD or YAML-LD"
                        .to_owned(),
                ));
            }
            let Some(fmt) = results_format.to_results_format() else {
                let kind = match result {
                    SparqlResult::Boolean(_) => "an ASK boolean",
                    _ => "SELECT solutions",
                };
                return Err(CliError::Runtime(format!(
                    "{kind} result cannot be serialized to the RDF syntax `{}`: a SELECT/ASK \
                     result needs a SPARQL-results format (json/xml/csv/tsv)",
                    results_format.token()
                )));
            };
            // The results serializer itself rejects the shapes its format cannot carry
            // (CSV/TSV reject a boolean); its `Err` maps cleanly to a runtime failure.
            // `--provenance-namespace` + CSV/TSV is refused up front by
            // `refuse_unenforceable_combinations`, before this lane ever runs the query.
            let provenance = build_query_provenance(provenance_namespace, query);
            let outcome = serialize(result, fmt, &provenance, provenance_namespace)?;
            sink::write_out("-", &outcome.bytes)?;
            // A tabular/boolean result performs no lossy transcode; honor the flag
            // uniformly with an empty ledger.
            ledger::surface(ledger_target, &LossLedger::new())
        }
        SparqlResult::Graph(graph) => {
            if provenance_namespace.is_some() {
                return Err(CliError::Usage(
                    "--provenance-namespace anchors the additive extension on a SELECT/ASK \
                     result serialized to SPARQL-results JSON/XML: a CONSTRUCT/DESCRIBE graph \
                     is serialized as RDF and has no SPARQL-results provenance extension to \
                     carry it"
                        .to_owned(),
                ));
            }
            let Some(fmt) = results_format.to_rdf_format() else {
                return Err(CliError::Runtime(format!(
                    "a CONSTRUCT/DESCRIBE graph result cannot be serialized to the SPARQL-results \
                     format `{}`: a graph needs an RDF syntax \
                     (turtle/trig/ntriples/nquads/rdfxml/trix/hextuples/jsonld/yamlld)",
                    results_format.token()
                )));
            };
            // Refused BEFORE the serializer runs: a result the requested syntax would
            // silently empty out never reaches stdout. See
            // `refuse_uncarriable_named_graphs`.
            refuse_uncarriable_named_graphs(&**graph, fmt, results_format)?;
            // The universal sink: a star-incapable target (RDF/XML, TriX, HexTuples)
            // projects the RDF-1.2 statement layer and records the drop in the ledger.
            // The graph is freshly constructed, so there is no source codec to seed the
            // contract-loss half (`None`); only the realized dropped-row counts appear.
            let ledger = sink::write_rdf(
                &**graph,
                "-",
                SourceFormat::Native(fmt),
                base,
                None,
                jsonld_options,
            )?;
            ledger::surface(ledger_target, &ledger)
        }
    }
}

/// The closing imperative of every named-graph refusal on this lane: the quad-capable
/// `--results-format` tokens, in [`QueryFormat`] declaration order.
///
/// The rest of the sentence is `purrdf_core::named_graph::named_graph_refusal`, shared
/// verbatim with the Python and wasm hosts; only the remedy is per-host, because
/// "`--results-format`" is a spelling this binary has and they do not.
const QUAD_CAPABLE_REMEDY: &str =
    "Re-run with a quad-capable --results-format (trig/nquads/trix/hextuples/jsonld/yamlld)";

/// Refuse to serialize a graph result that carries named graphs to a single-graph
/// RDF syntax, naming the graphs, the format, and what to use instead.
///
/// # Why this REFUSES rather than serializing and ledgering the loss
///
/// Every other loss in this pipeline is a transcode the caller asked for implicitly —
/// they named a source document and a target syntax, and the sink reports what the
/// pair cannot carry. A named graph in a CONSTRUCT result is not that: the graph name
/// is in the QUERY the caller wrote, one token at a time (`CONSTRUCT GRAPH ex:out {…}`),
/// so it is the single most explicit thing in the request. Turtle, N-Triples and
/// RDF/XML have no named-graph construct, and the serializer's single-graph flattening
/// DROPS every graph-scoped row (it does not fold them into the default graph — see
/// `purrdf_core::loss`'s `named-graph-dropped` note). Serializing anyway would print a
/// well-formed document that is missing exactly the statements the caller asked for,
/// and exit 0 — the silent-wrong shape this binary refuses everywhere else.
///
/// A whole-template `CONSTRUCT GRAPH g {…}` against Turtle produced ZERO bytes, an
/// EMPTY loss ledger and exit 0 before this refusal existed. That is the case it
/// closes, and it closes the general one with it: a per-statement template may write
/// into many graphs at once, or mix default-graph triples with named-graph quads, and
/// a result that carries ANY non-default graph is refused, because the mixed case
/// would otherwise emit the default-graph half and drop the rest — a partial answer
/// reported as a complete one, which is worse than emitting nothing.
///
/// A result carrying ONLY default-graph triples is untouched: the plain
/// SPARQL 1.1 `CONSTRUCT`/`DESCRIBE` lane serializes to Turtle, N-Triples and RDF/XML
/// exactly as it always has.
fn refuse_uncarriable_named_graphs<D: DatasetView>(
    graph: &D,
    fmt: NativeRdfFormat,
    results_format: QueryFormat,
) -> Result<(), CliError> {
    if fmt.supports_datasets() {
        return Ok(());
    }
    let names = distinct_graph_names(graph);
    if names.is_empty() {
        return Ok(());
    }
    Err(CliError::Usage(named_graph_refusal(
        &names,
        results_format.token(),
        QUAD_CAPABLE_REMEDY,
    )))
}

/// Answer `query` over `dataset` under the resolved entailment plan AND the caller's
/// execution governors, surfacing the run's reasoning report to `--report` either way.
///
/// # Why this is not the ungoverned call with a ceiling bolted on
///
/// An entailment-regime query is two phases — materialize the regime's closure, then
/// evaluate SPARQL over that frozen closure — and
/// [`purrdf::query_with_entailment_governed`] governs both, differently and on purpose. The
/// EVALUATION is governed completely: every one of the six flags is in force over the
/// closure exactly as it is over a raw view. The CLOSURE honours the stop signal alone, so
/// `--deadline` bounds it and a numeric ceiling does not — a caller-settable charge schedule
/// on a reasoning run would mean two operators materializing the same regime over the same
/// data get different closures, and `--fuel 1000` is not worth that. `--entailment`'s own
/// help states the split, so the flag combination is accepted with its scope written down
/// rather than refused with an apology.
///
/// A closure the deadline stopped is [`purrdf::GovernedEntailment::ClosureStopped`]: nothing
/// ran, nothing is on stdout, and the exit is 3 like any other trip.
///
/// # Why this is not `materialize` + `query_prepared`
///
/// It used to be, and that cost the binary a whole capability. `purrdf_entail::materialize`
/// takes a `Materialization`, and `Materialization::OwlDirect` takes the QUERY's basic graph
/// pattern; a lane that has a query and hands over `&[]` anyway gets the query-independent
/// whole-vocabulary augmentation, which cannot answer a basic graph pattern carrying a
/// NON-DISTINGUISHED variable. `purrdf query --entailment owl-direct` therefore answered `[]`
/// for `SELECT ?x WHERE { ?x r ?y . ?y a B }` over a TBox stating `A ⊑ ∃r.B` and an ABox
/// stating `a : A` — a query whose certain answer is `ex:a`, and whose answer the library
/// computes through [`purrdf::query_with_entailment`]'s combined-approach lane. That entry
/// point collects the pattern from the parsed query, runs the restricted chase when the TBox
/// is in the fragment it certifies, and filters the chase's witnesses out of every observable
/// binding; it also returns the report, which is why the report is surfaced from here rather
/// than by `report::materialize_reported`.
///
/// An INCONSISTENT knowledge base is handled exactly as `reason` handles it: it has no closure
/// and it did have a run, so the report is written first and the refusal returned after.
///
/// `options` carries the SAME registry instances every other lane in this module uses (see
/// [`QueryOp::aggregates`]): `query_with_entailment_governed` takes the engine's
/// `QueryOptions` and threads it into both the closure query's PARSE and its evaluation,
/// so a statistical aggregate — or a `--path-relation` — reaches the entailment-aware lane
/// exactly as it reaches the ordinary one.
///
/// A path relation snapshotted for this lane is built from the PRE-closure view: it is
/// registered inside [`EntailedQueryOp::run`], where the source view is what the reasoner
/// has not yet closed over. That matches the Python surface's `relations_from_graph` on
/// the same lane, and it is stated rather than assumed because a relation answers about
/// the dataset it was built from and nothing at the property-function seam can check that.
#[allow(
    clippy::too_many_arguments,
    reason = "each parameter is a distinct, independently-named input (the engine, the \
              dataset, the query text/base, the resolved entailment plan, the governors, \
              the evaluation options, and the report target); bundling them into a struct \
              would not shrink the call sites, which already name every field"
)]
fn entailed_query<D: DatasetView>(
    engine: &NativeSparqlEngine,
    dataset: &D,
    query: &str,
    base: Option<&str>,
    plan: &reason::EntailmentPlan,
    governors: &QueryGovernors,
    options: EngineQueryOptions<'_>,
    report_target: &ReportTarget,
) -> Result<GovernedEntailment, CliError> {
    let request = SparqlRequest {
        query,
        base_iri: base,
        substitutions: &[],
    };
    match purrdf::query_with_entailment_governed(
        engine,
        dataset,
        request,
        plan.query_entailment(),
        options,
        governors,
    ) {
        Ok(answered) => {
            // The certificate is surfaced for the run that HAPPENED. A closure the stop
            // signal ended produced none, so there is nothing to write and nothing is
            // written — `--report` is refused without `--entailment` for the same reason a
            // report of a run that did not happen is not printed here.
            if let Some(report) = answered.report() {
                report::surface(report_target, report)?;
            }
            Ok(answered)
        }
        Err(purrdf::ReasoningError::Entailment(EntailError::Inconsistent(run))) => {
            report::surface(report_target, run.report())?;
            Err(CliError::Runtime(
                EntailError::Inconsistent(run).to_string(),
            ))
        }
        Err(purrdf::ReasoningError::Entailment(other)) => Err(other.into()),
        Err(purrdf::ReasoningError::Query(diagnostic)) => Err(diagnostic.into()),
        // `ReasoningError` is `#[non_exhaustive]`, so a variant added in its own crate
        // reaches the CLI without a compile error. It is reported rather than swallowed:
        // the arms above exist to surface a REPORT alongside the failure where one exists,
        // and a variant this match has never seen carries none it knows how to find — so
        // the message is what there is to say, and saying it is strictly better than
        // exiting on a diagnostic the user never sees.
        Err(other) => Err(CliError::Runtime(other.to_string())),
    }
}

/// The `--entailment` query as a [`ViewOp`], so a pack source answers under the regime
/// directly over a zero-copy `PackView` — no `dataset_from_view` rebuild to cross the
/// reasoner boundary. A text source parses to an `RdfDataset` and answers over that.
struct EntailedQueryOp<'a> {
    engine: &'a NativeSparqlEngine,
    query: &'a str,
    base: Option<&'a str>,
    plan: &'a reason::EntailmentPlan,
    governors: &'a QueryGovernors,
    aggregates: Option<&'a AggregateRegistry>,
    /// The `--path-relation` specs, snapshotted over the PRE-closure view; see
    /// [`entailed_query`] for why that scope is the one stated on the flag.
    relations: RelationSpecs<'a>,
    report_target: &'a ReportTarget,
}

impl ViewOp for EntailedQueryOp<'_> {
    type Output = GovernedEntailment;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError> {
        let relations = path_relation::build_registry(view, self.relations.specs)?;
        entailed_query(
            self.engine,
            view,
            self.query,
            self.base,
            self.plan,
            self.governors,
            engine_options(self.aggregates, relations.as_ref()),
            self.report_target,
        )
    }
}

/// The resolved `query` flags: the data source and base, the two entailment inputs, the
/// result serialization, the query text, the execution governors, and `--explain`.
///
/// Grouped for the reason [`ConvertOptions`](crate::convert::ConvertOptions) is: the
/// dispatcher hands over one borrow rather than a dozen positional arguments whose order
/// is the only thing keeping them apart.
pub(crate) struct QueryOptions<'a> {
    /// `--data`: the data-source path.
    pub(crate) data: &'a str,
    /// `--base`: the parse and query base IRI.
    pub(crate) base: Option<&'a str>,
    /// `--entailment`: the regime whose closure the query runs under.
    pub(crate) entailment: Option<CliRegime>,
    /// `--rules`: the RIF-in-XML rule document `--entailment rif` runs.
    pub(crate) rules: Option<&'a std::path::Path>,
    /// `--results-format`: the result serialization. `None` means the flag was
    /// not named at all (distinct from naming its default, `json`) — the
    /// distinction [`refuse_unenforceable_combinations`] needs to refuse
    /// `--results-format` beside `--explain` by name rather than silently
    /// letting a named-but-inapplicable serialization do nothing.
    pub(crate) results_format: Option<QueryFormat>,
    /// The SPARQL query text.
    pub(crate) query: &'a str,
    /// The six execution-governor flags.
    pub(crate) governors: GovernorFlags,
    /// `--explain`: print the plan and its charge ledger instead of the answers.
    pub(crate) explain: bool,
    /// `--jsonld-options`: the configured JSON-LD/YAML-LD serializer options.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
    /// `--aggregate-namespace`: registers purrdf's first-party statistical aggregate
    /// set under this IRI namespace. `None` (the default) leaves every one of the ten
    /// names an unregistered custom-aggregate IRI, exactly as before this flag existed.
    pub(crate) aggregate_namespace: Option<&'a str>,
    /// `--provenance-namespace`: the raw `(prefix, iri)` halves anchoring the additive
    /// `purrdf` provenance extension on a SPARQL-results JSON/XML emission. `None` (the
    /// default) emits pure-W3C output, exactly as before this flag existed.
    pub(crate) provenance_namespace: Option<(&'a str, &'a str)>,
    /// `--path-relation`, repeatable: the path-witness relations this call registers.
    /// Empty (the default) leaves `QueryOptions::property_functions` at its `EMPTY`
    /// value, so a query naming one of these IRIs is an ordinary triple pattern reading
    /// the data — exactly as before this flag existed.
    pub(crate) path_relations: &'a [PathRelationSpec],
}

/// Run the `query` subcommand.
pub(crate) fn run(
    options: &QueryOptions<'_>,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<CliOutcome, CliError> {
    let data_format = format::resolve(None, options.data)?;
    // A report of a run that will not happen is a request the flag cannot honor, and
    // honoring it with silence is what this pipeline refuses.
    if report_target.is_requested() && options.entailment.is_none() {
        return Err(report::requires_entailment("query"));
    }
    refuse_unenforceable_combinations(options, ledger_target)?;
    // Resolved ONCE, here: `None` (the flag not named) defaults to `json`, exactly as
    // clap's own `default_value_t` used to before `--results-format` became an
    // `Option` — the only thing that changed is that "not named" and "named `json`"
    // are now distinguishable, which is what lets `--explain` refuse the former.
    let results_format = options.results_format.unwrap_or(QueryFormat::Json);

    let engine = NativeSparqlEngine::new();

    // Built ONCE, before the parse, and reused verbatim by every lane below (including
    // `--explain`): a `Custom` aggregate IRI is admitted (registered, correct arity)
    // against this EXACT registry instance at prepare time, and a plan/registry
    // disagreement at evaluation is refused rather than silently mismatched (see
    // `AggregateRegistry`'s instance-identity fingerprint) — so preparing and evaluating
    // against two independently built registries, even with identical content, would
    // break every `--aggregate-namespace` query.
    let aggregates = build_aggregate_registry(options.aggregate_namespace);

    // Refused ONCE, before any lane opens the data source: `PropertyFunctionRegistry`
    // PANICS on a duplicate IRI, and a command line is a host misconfiguration rather than
    // an abort.
    path_relation::refuse_duplicate_iris(options.path_relations)?;
    let relations = RelationSpecs {
        specs: options.path_relations,
        query: options.query,
        base: options.base,
    };

    // Validated ONCE, before any lane runs, so a malformed `--provenance-namespace` is a
    // usage error before the query is even parsed — never discovered only after a
    // successful evaluation, when serializing the answer would otherwise be the first
    // place `ProvenanceNamespace::new` could fail.
    let provenance_namespace = options
        .provenance_namespace
        .map(|(prefix, iri)| ProvenanceNamespace::new(prefix, iri))
        .transpose()
        .map_err(|e| CliError::Usage(format!("--provenance-namespace: {e}")))?;

    if options.explain {
        // `explain_query_with_options_view` parses, surveys and evaluates the query
        // itself, so this lane prepares nothing of its own and the plan cache is
        // warmed once.
        let explanation = source::run_over_input(
            options.data,
            data_format,
            options.base,
            ExplainOp {
                engine: &engine,
                query: options.query,
                base: options.base,
                aggregates: aggregates.as_ref(),
                relations,
            },
        )?;
        sink::write_out("-", explanation.render().as_bytes())?;
        return Ok(CliOutcome::Complete);
    }

    // Prepared unconditionally, before the source is opened, so a malformed query is a
    // parse error raised before any file is read. It is the plan the lanes below evaluate
    // whenever no `--path-relation` was given; with one, `RelationSpecs::prepare_against`
    // re-prepares against the registry it snapshots (the registry decides which predicates
    // are call nodes, and it cannot exist until the view does), and the plan cache makes
    // that second parse free.
    let prepared = engine.prepare_query_with_options(
        options.query,
        options.base,
        EngineQueryOptions {
            aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
            ..EngineQueryOptions::EMPTY
        },
    )?;

    if let Some(regime) = options.entailment {
        // The `--entailment` lane: answer the query UNDER the regime through the library's
        // own entailment-aware query entry point — governed, so a ceiling named beside
        // `--entailment` is in force rather than refused. A pack source enters the reasoner
        // as a zero-copy `PackView` (no `dataset_from_view` rebuild); a text source parses
        // to an `RdfDataset`.
        let plan = reason::EntailmentPlan::resolve(regime, options.rules)?;
        // Built HERE, with the source already read and the closure not yet started, for the
        // reason `GovernedQueryOp` states: a `--deadline` is a budget for the work the flag
        // names, and reading a large file is not that work.
        let governors = options.governors.to_governors();
        // The rows go to stdout and the certificate to `--report`: a solution set that
        // depends on a closure is not readable without knowing what closed it.
        let answered = source::run_over_input(
            options.data,
            data_format,
            options.base,
            EntailedQueryOp {
                engine: &engine,
                query: options.query,
                base: options.base,
                plan: &plan,
                governors: &governors,
                aggregates: aggregates.as_ref(),
                relations,
                report_target,
            },
        )?;
        return emit_entailed(
            options,
            provenance_namespace.as_ref(),
            answered,
            ledger_target,
        );
    }

    if options.governors.is_engaged() {
        let outcome = source::run_over_input(
            options.data,
            data_format,
            options.base,
            GovernedQueryOp {
                engine: &engine,
                prepared: &prepared,
                flags: options.governors,
                aggregates: aggregates.as_ref(),
                relations,
            },
        )?;
        return emit_governed(
            options,
            provenance_namespace.as_ref(),
            &outcome,
            ledger_target,
        );
    }

    // The ungoverned lane, unchanged: the same call over the same zero-copy view the
    // binary made before governors existed.
    let result = source::run_over_input(
        options.data,
        data_format,
        options.base,
        QueryOp {
            engine: &engine,
            // `prepared` is an `Arc<PreparedQuery>`; reborrow it as `&PreparedQuery`.
            prepared: &prepared,
            aggregates: aggregates.as_ref(),
            relations,
        },
    )?;
    emit_result(
        &result,
        results_format,
        options.base,
        options.jsonld_options,
        provenance_namespace.as_ref(),
        options.query,
        ledger_target,
    )?;
    Ok(CliOutcome::Complete)
}

/// Refuse every flag combination this lane cannot honor, naming what to drop.
///
/// It exists because the alternative is a flag that silently does nothing — the shape this
/// pipeline refuses everywhere else, and the most dangerous shape a GOVERNOR can take: a
/// ceiling an operator believes is in force and that nothing enforces is worse than no
/// ceiling at all, because it is relied upon.
///
/// * `--explain` with a governor flag. `--explain` runs under the metering profile — every
///   counter engaged at a ceiling nothing can reach — so a ceiling handed to it would be
///   printed in the receipt and enforced nowhere.
/// * `--explain` with `--entailment`: that lane has no plan this prices, so an explanation
///   printed beside it would describe work the answer did not come from.
/// * `--explain` with `--provenance-namespace`: `--explain` prints the plan INSTEAD of the
///   answers a provenance extension would anchor onto.
/// * `--explain` with `--results-format`: `--explain`'s rendering is plain text, not a
///   result serialization there is a format for.
/// * `--explain` with `--loss-ledger`: `--explain` never runs the serializer a loss ledger
///   is a report ABOUT.
/// * `--explain` with `--jsonld-options`: `--explain` never reaches the JSON-LD/YAML-LD
///   serializer those options would configure.
/// * `--provenance-namespace` with a CSV/TSV `--results-format`: those two are pure-W3C
///   value-only formats with no extension point to anchor it on.
///
/// # `--entailment` with a governor flag is NOT refused
///
/// It used to be, and the refusal was honest about a real gap rather than about a real
/// impossibility: the library's entailment-aware query lane took no governors, so a ceiling
/// handed to it would have bounded nothing. It takes them now
/// ([`purrdf::query_with_entailment_governed`]), so the combination WORKS and the flags mean
/// what their help says over a closure just as they do over a raw view. The one asymmetry —
/// only `--deadline` reaches the closure's own computation, because a numeric ceiling on a
/// reasoning run would be a caller-settable charge schedule and therefore a caller-settable
/// closure — is documented on `--entailment` itself, where an operator reading the flag
/// meets it. It is a stated scope, not a silent no-op: a `--fuel` beside `--entailment` is
/// enforced over every step of the evaluation it names.
///
/// # `--aggregate-namespace` with `--entailment` or `--explain` is NOT refused
///
/// Both used to be, and both were honest about real gaps rather than real impossibilities.
/// The entailment-aware query lane took no `QueryOptions` seam at all, so a registry named
/// beside `--entailment` never reached the evaluation that runs the closure's query; it
/// takes the engine's `QueryOptions` now
/// ([`purrdf::query_with_entailment_governed`]), threaded into both the closure query's
/// parse and its evaluation, so `AGG(<{NS}MEDIAN>, ?x)` resolves over the entailed closure
/// exactly as it resolves over a raw view. The engine had no aggregate-registry-aware
/// explain entry at all; [`NativeSparqlEngine::explain_query_with_options_view`] is that
/// entry now — the one options-carrying explain call every registry (relations,
/// aggregates, or both together) reaches this lane through — so `--explain` with
/// `--aggregate-namespace` prints the plan with the registered aggregates named in the
/// receipt's `aggregates` block instead of refusing every `Custom` call as unregistered.
///
/// # `--results-format`, `--loss-ledger`, and `--jsonld-options` beside `--explain` ARE
/// refused now
///
/// All three used to be silently accepted and ignored: `--explain` returns before
/// [`emit_result`] is ever called, so a named `--results-format` never selected a
/// serializer, a named `--loss-ledger` never had a transcode to report on, and a
/// configured `--jsonld-options` document never reached a serializer either. That was
/// exactly the shape this module's own doctrine names as the thing to refuse —
/// `--results-format` merely documented the no-op in its help text rather than
/// refusing it, and `--loss-ledger`/`--jsonld-options` did not even document it. All
/// three now name what to drop, matching every other refusal in this function.
fn refuse_unenforceable_combinations(
    options: &QueryOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let named = options.governors.named();
    if !named.is_empty() && options.explain {
        let named = named.join(", ");
        return Err(CliError::Usage(format!(
            "--explain MEASURES a run rather than bounding one: it evaluates the query \
             with every counter engaged at a ceiling nothing can reach, so {named} would \
             be reported and never enforced. Drop --explain to run under the ceiling, or \
             drop {named} to explain the query"
        )));
    }
    if options.explain && options.entailment.is_some() {
        return Err(CliError::Usage(
            "--explain describes the plan the SPARQL evaluator runs, and --entailment answers \
             through the entailment-aware query lane instead, whose work is a closure rather \
             than a plan this can price: drop one of the two"
                .to_owned(),
        ));
    }
    // `--rules` names the RIF-in-XML rule document `--entailment rif` runs; `options.rules`
    // is read only inside the `--entailment` lane below, so a bare `--rules FILE` with no
    // `--entailment` would otherwise be accepted by clap and silently do nothing.
    if options.rules.is_some() && options.entailment.is_none() {
        return Err(CliError::Usage(
            "--rules names the rule document an entailment regime runs under; it has no \
             effect without --entailment"
                .to_owned(),
        ));
    }
    if options.explain && options.provenance_namespace.is_some() {
        return Err(CliError::Usage(
            "--explain prints the plan and its charge ledger INSTEAD of the query's answers, \
             so a --provenance-namespace anchoring an extension on those answers would be \
             accepted and never emitted: drop one of the two"
                .to_owned(),
        ));
    }
    // `--explain` never reaches `emit_result`, so a named `--results-format` would
    // select a serializer that never runs — the same silent-no-op shape every other
    // arm in this function refuses by name.
    if options.explain && options.results_format.is_some() {
        return Err(CliError::Usage(
            "--explain prints the plan as plain text INSTEAD of the query's answers, so \
             --results-format (which names how ANSWERS serialize) would be accepted and \
             never applied: drop one of the two"
                .to_owned(),
        ));
    }
    // `--explain` evaluates the query under the metering profile rather than through
    // `emit_result`'s serializer, so it produces no loss ledger for `--loss-ledger` to
    // surface — the flag would be accepted, and no file/stderr output would ever
    // appear, exactly the silent shape this pipeline refuses everywhere else.
    if options.explain && ledger_target.is_requested() {
        return Err(CliError::Usage(
            "--explain prints the plan and its charge ledger INSTEAD of evaluating a \
             serializer, so --loss-ledger (which reports a lossy transcode THAT \
             serializer produced) would be accepted and never written: drop one of the \
             two"
            .to_owned(),
        ));
    }
    // `--explain` returns before `emit_result` is ever reached, so a configured
    // JSON-LD/YAML-LD serializer never runs — the same silent-no-op shape every other
    // arm in this function refuses by name.
    if options.explain && options.jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--explain prints the plan as plain text INSTEAD of the query's answers, so \
             --jsonld-options (which configures the JSON-LD/YAML-LD serializer a \
             CONSTRUCT/DESCRIBE graph result would use) would be accepted and never \
             applied: drop one of the two"
                .to_owned(),
        ));
    }
    // CSV/TSV are pure-W3C value-only SPARQL-results formats with no extension point at
    // all — unlike JSON/XML, which anchor the additive `purrdf` provenance extension.
    // `purrdf_sparql_results::serialize` tolerates a namespace here (it trims the
    // extension silently and reports `SerializeOutcome::provenance_dropped` for a library
    // caller to inspect), but this CLI's own contract is to refuse a flag by name when it
    // cannot do anything, exactly as `--provenance-namespace` is refused above beside
    // `--explain` and below for a CONSTRUCT/DESCRIBE graph, rather than accept it and
    // silently ignore it.
    let results_format = options.results_format.unwrap_or(QueryFormat::Json);
    if options.provenance_namespace.is_some()
        && matches!(
            results_format.to_results_format(),
            Some(SparqlResultsFormat::Csv | SparqlResultsFormat::Tsv)
        )
    {
        return Err(CliError::Usage(format!(
            "--provenance-namespace anchors the additive extension on SPARQL-results \
             JSON/XML: --results-format {} is a pure-W3C value-only format with no \
             extension point and cannot carry it",
            results_format.token()
        )));
    }
    Ok(())
}

/// Emit a governed ENTAILMENT outcome, returning the exit classification it carries.
///
/// Two arms rather than [`emit_governed`]'s two, because an entailment-regime query has a
/// third thing that can happen to it: the stop signal can end the CLOSURE, before any query
/// was evaluated. That case prints the governor banner and the trip on stderr and **nothing**
/// on stdout — there are no rows, not even an empty result set, because an empty result set
/// on stdout is the claim "this query has no answers" and this run never asked it.
fn emit_entailed(
    options: &QueryOptions<'_>,
    provenance_namespace: Option<&ProvenanceNamespace>,
    answered: GovernedEntailment,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    match answered {
        GovernedEntailment::Answered { outcome, .. } => {
            emit_governed(options, provenance_namespace, &outcome, ledger_target)
        }
        GovernedEntailment::ClosureStopped { tripped } => {
            eprint!("{}", governors::render_closure_stop(tripped));
            Ok(CliOutcome::BudgetExhausted)
        }
        _ => Err(CliError::Runtime(
            "unsupported governed entailment outcome".to_owned(),
        )),
    }
}

/// Emit a governed outcome, returning the exit classification it carries.
///
/// The complete arm is the ungoverned arm exactly: the same rows, through the same
/// serializer, exiting 0. The tripped arm writes the governor report to stderr FIRST — so a
/// trip is announced even if the rows then fail to serialize — and then hands whatever the
/// certificate licensed to the same [`emit_result`], so a partial answer is a well-formed
/// document of the requested format rather than a special one.
fn emit_governed(
    options: &QueryOptions<'_>,
    provenance_namespace: Option<&ProvenanceNamespace>,
    outcome: &GovernedOutcome,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    let emit = |result: &SparqlResult| {
        emit_result(
            result,
            options.results_format.unwrap_or(QueryFormat::Json),
            options.base,
            options.jsonld_options,
            provenance_namespace,
            options.query,
            ledger_target,
        )
    };
    match outcome {
        GovernedOutcome::Complete { result, .. } => {
            emit(result)?;
            Ok(CliOutcome::Complete)
        }
        GovernedOutcome::BudgetExhausted(exhausted) => {
            eprint!("{}", governors::render_trip(exhausted));
            if let Some(partial) = exhausted.partial.result() {
                emit(partial.result())?;
            }
            Ok(CliOutcome::BudgetExhausted)
        }
    }
}
