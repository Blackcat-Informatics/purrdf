// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One implementation of every SPARQL operation kind, run by both lanes.
//!
//! [`OperationInput::execute`] is the only place a query, a serialized query, a governed
//! query, a negotiated query, an entailment query, an EXPLAIN, an update and a governed
//! update are evaluated. The synchronous entry points in [`crate::query`] call it
//! offline — no `SERVICE` or `LOAD` source, and the caller's own stop signal only on a
//! governed entry — and an asynchronous job ([`crate::async_query`]) calls it with its
//! stop watch and its effect sources. The two lanes therefore cannot answer the same
//! request differently.
//!
//! # Errors keep their code
//!
//! A failure is a [`JobError`]: how the operation failed ([`JobErrorKind`]) and the
//! diagnostic that says why, whose `code` is the stable string a host switches on. A
//! host never reads a failure's class back out of its message: the JavaScript error a
//! failure is thrown as carries the code as its `code` property ([`coded_error`]), and
//! that code is the [`FailureCode`] an HTTP host answers it by
//! ([`FailureCode::from_diagnostic_code`]).

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use purrdf::{
    ClosureRelations, EntailmentClosure, GovernedEntailment, JsonLdSerializeOptions,
    QueryEntailmentPlan, RdfDataset, ReasoningError, query_with_entailment_closure_governed,
};
use purrdf_core::{RdfDiagnostic, SparqlResult};
use purrdf_sparql_eval::protocol::{FailureCode, negotiate};
use purrdf_sparql_eval::{
    GovernedOutcome, GovernedUpdateOutcome, GraphResolver, NativeSparqlEngine, QueryGovernors,
    QueryOptions, ServiceResolver, StopCause, StopSignal, TrippedGovernor,
};
use purrdf_sparql_results::ProvenanceNamespace;
use wasm_bindgen::prelude::*;

use crate::async_query::{AsyncCounters, AsyncOperationKind, add_ms, now_ms};
use crate::protocol::not_acceptable_message;
use crate::query::{
    GovernorArgs, NegotiatedValue, aggregate_env_message, negotiable_result_kind,
    serialize_configured_graph, serialize_query_result, sparql_request,
};
use crate::shacl::{
    ShaclChangeValidation, ShaclEntailment, ShaclNodeExprOutcome, ShaclRefusal, ShaclRulesInference,
};

// ---------------------------------------------------------------------------
// Codes of the failures that are not an engine diagnostic
// ---------------------------------------------------------------------------

/// A result the operation reached could not be serialized in the format asked for.
pub(crate) const SERIALIZE_CODE: &str = "purrdf-wasm-serialize";
/// The entailment regime or its closure failed.
pub(crate) const ENTAILMENT_CODE: &str = "purrdf-wasm-entailment";
/// The extension environment (the aggregate registry) could not be built.
pub(crate) const EXTENSION_CODE: &str = "purrdf-wasm-extension-environment";
/// A SHACL surface failed: a shapes or data graph that does not parse, or a validation
/// that failed.
pub(crate) const SHACL_CODE: &str = "purrdf-wasm-shacl";
/// A prepared SHACL product was refused.
pub(crate) const SHACL_REFUSAL_CODE: &str = "purrdf-wasm-shacl-product-refusal";
/// An option or argument of an operation was refused before it began.
pub(crate) const OPTIONS_CODE: &str = "purrdf-wasm-options";
/// The host asked a job for something out of turn: an outcome of another kind, before
/// the job finished, or twice.
pub(crate) const USAGE_CODE: &str = "purrdf-wasm-usage";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// How an operation failed, for the host to decide what to reject with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobErrorKind {
    /// A parse or evaluation failure, or a refused result shape.
    Error,
    /// An ungoverned operation's cancellation.
    Cancelled,
    /// An ungoverned operation's deadline.
    Deadline,
    /// A latched fault: the host broke the effect protocol.
    Fault,
    /// A negotiated query's result, whose shape (a graph carrying named graphs) no format
    /// the `Accept` header allows can carry.
    NotAcceptable,
}

impl JobErrorKind {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Cancelled => "cancelled",
            Self::Deadline => "deadline",
            Self::Fault => "fault",
            Self::NotAcceptable => "not-acceptable",
        }
    }
}

/// How a [`JobError`]'s message is rendered for JavaScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorText {
    /// The diagnostic's own rendering, `error <code>: <message>`: an engine diagnostic.
    Diagnostic,
    /// The diagnostic's message alone: a failure whose words never carried a code.
    Message,
}

/// Why an operation failed: its kind, and the diagnostic whose code says why.
///
/// A failure reads the same on both lanes. An asynchronous job runs on a stack region
/// exactly as large as the synchronous lane's shadow stack, so the evaluator's stack
/// refusal falls at the same depth on each and carries no lane-specific remedy.
#[derive(Debug, Clone)]
pub(crate) struct JobError {
    kind: JobErrorKind,
    diagnostic: RdfDiagnostic,
    text: ErrorText,
}

impl JobError {
    /// An engine diagnostic, rendered as the engine renders it.
    pub(crate) const fn diagnostic(diagnostic: RdfDiagnostic) -> Self {
        Self {
            kind: JobErrorKind::Error,
            diagnostic,
            text: ErrorText::Diagnostic,
        }
    }

    /// A failure reported under `code` in `message`'s own words.
    pub(crate) fn message(code: &str, message: impl Into<String>) -> Self {
        Self {
            kind: JobErrorKind::Error,
            diagnostic: RdfDiagnostic::error(code, message),
            text: ErrorText::Message,
        }
    }

    /// A negotiated query whose result no acceptable format can carry.
    pub(crate) fn not_acceptable(message: String) -> Self {
        Self {
            kind: JobErrorKind::NotAcceptable,
            diagnostic: RdfDiagnostic::error(FailureCode::NotAcceptable.code(), message),
            text: ErrorText::Message,
        }
    }

    /// A latched fault: the host broke the effect protocol.
    pub(crate) fn fault(message: impl Into<String>) -> Self {
        Self {
            kind: JobErrorKind::Fault,
            diagnostic: RdfDiagnostic::error(FailureCode::HostFault.code(), message),
            text: ErrorText::Message,
        }
    }

    /// An ungoverned operation stopped by its signal. It has no outcome to carry a
    /// truncation in, so the stop is its error.
    pub(crate) fn stopped(tripped: TrippedGovernor) -> Self {
        let (kind, failure, message) = match tripped {
            TrippedGovernor::Stopped {
                cause: StopCause::Cancelled,
            } => (
                JobErrorKind::Cancelled,
                FailureCode::Cancelled,
                "the asynchronous operation was cancelled".to_owned(),
            ),
            TrippedGovernor::Stopped {
                cause: StopCause::Deadline,
            } => (
                JobErrorKind::Deadline,
                FailureCode::Deadline,
                "the asynchronous operation's deadline expired".to_owned(),
            ),
            other => (
                JobErrorKind::Error,
                FailureCode::Evaluation,
                format!("the asynchronous operation stopped: {other}"),
            ),
        };
        Self {
            kind,
            diagnostic: RdfDiagnostic::error(failure.code(), message),
            text: ErrorText::Message,
        }
    }

    /// An entailment query's failure: a query diagnostic keeps its code, and the closure's
    /// own failure is reported under [`ENTAILMENT_CODE`]; either reads as the reasoning
    /// layer renders it.
    fn reasoning(error: &ReasoningError) -> Self {
        let code = match error {
            ReasoningError::Query(diagnostic) => diagnostic.code.as_str(),
            _ => ENTAILMENT_CODE,
        };
        Self::message(code, error.to_string())
    }

    pub(crate) const fn kind(&self) -> JobErrorKind {
        self.kind
    }

    /// The stable code the failure is reported under.
    pub(crate) fn code(&self) -> &str {
        &self.diagnostic.code
    }

    /// The message a JavaScript caller reads.
    pub(crate) fn rendered(&self) -> String {
        match self.text {
            ErrorText::Diagnostic => self.diagnostic.to_string(),
            ErrorText::Message => self.diagnostic.message.clone(),
        }
    }

    /// The JavaScript error this failure is thrown as.
    pub(crate) fn to_js(&self) -> JsValue {
        coded_error(&self.rendered(), self.code())
    }
}

#[wasm_bindgen]
extern "C" {
    /// The host's `Error` constructor: a failure is a real `Error` carrying its code.
    #[wasm_bindgen(js_name = Error)]
    type CodedError;

    #[wasm_bindgen(constructor, js_class = "Error")]
    fn new(message: &str) -> CodedError;

    #[wasm_bindgen(method, setter = code)]
    fn set_code(this: &CodedError, code: &str);

    /// The host's `TypeError` constructor, for an argument of the wrong type.
    #[wasm_bindgen(js_name = TypeError)]
    type CodedTypeError;

    #[wasm_bindgen(constructor, js_class = "TypeError")]
    fn new(message: &str) -> CodedTypeError;

    #[wasm_bindgen(method, setter = code)]
    fn set_code(this: &CodedTypeError, code: &str);
}

/// An `Error` whose message is `message` and whose `code` property is `code`. Built only
/// on wasm, where the host constructor exists.
pub(crate) fn coded_error(message: &str, code: &str) -> JsValue {
    let error = CodedError::new(message);
    error.set_code(code);
    error.into()
}

/// A `TypeError` whose message is `message` and whose `code` property is `code`.
pub(crate) fn coded_type_error(message: &str, code: &str) -> JsValue {
    let error = CodedTypeError::new(message);
    error.set_code(code);
    error.into()
}

/// An engine diagnostic thrown on the synchronous lane, as the error carrying its code.
pub(crate) fn diagnostic_to_js(diagnostic: RdfDiagnostic) -> JsValue {
    JobError::diagnostic(diagnostic).to_js()
}

// ---------------------------------------------------------------------------
// Outcomes and runs
// ---------------------------------------------------------------------------

/// What a finished operation left for its caller to take.
pub(crate) enum JobOutcome {
    Query(SparqlResult),
    Raw(String),
    Governed(Box<GovernedOutcome>),
    Entailment(Box<GovernedEntailment>),
    Updated(Arc<RdfDataset>),
    UpdateGoverned {
        outcome: GovernedUpdateOutcome,
        frozen: Option<Arc<RdfDataset>>,
    },
    Negotiated(Box<NegotiatedValue>),
    /// A change validation's log beside the scope it describes.
    ShaclChange(ShaclChangeValidation),
    /// A SHACL-AF entailment's materialized dataset beside its diagnostics.
    ShaclEntailment(ShaclEntailment),
    /// A rules run's inference graph, proof and diagnostics.
    ShaclRules(ShaclRulesInference),
    /// A node expression's output nodes beside the shapes graph's mandatory diagnostics.
    ShaclNodeExpr(ShaclNodeExprOutcome),
    /// A SHACL refusal — a prepared product's, or the shapes graph's import refusal: the
    /// job's error, carried as the class the synchronous twin rejects with rather than
    /// flattened into a message.
    Refused(ShaclRefusal),
    Failed(JobError),
}

impl fmt::Debug for JobOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Query(_) => "Query",
            Self::Raw(_) => "Raw",
            Self::Governed(_) => "Governed",
            Self::Entailment(_) => "Entailment",
            Self::Updated(_) => "Updated",
            Self::UpdateGoverned { .. } => "UpdateGoverned",
            Self::Negotiated(_) => "Negotiated",
            Self::ShaclChange(_) => "ShaclChange",
            Self::ShaclEntailment(_) => "ShaclEntailment",
            Self::ShaclRules(_) => "ShaclRules",
            Self::ShaclNodeExpr(_) => "ShaclNodeExpr",
            Self::Refused(_) => "Refused",
            Self::Failed(_) => "Failed",
        })
    }
}

/// What an operation runs with: a stop signal, effect sources, and the counters an
/// asynchronous job records its evidence in.
///
/// The sources are owned (`Arc`) rather than borrowed so an operation can hand them to
/// code that reads its sources off an ambient scope rather than off a request — SHACL
/// validation installs them with [`purrdf_shapes::sparql::enter_execution_scope`]. The
/// synchronous lane runs with no source at all.
pub(crate) struct JobRun<'r> {
    pub(crate) stop: Option<Arc<dyn StopSignal>>,
    pub(crate) remote: Option<Arc<dyn ServiceResolver + Send + Sync>>,
    pub(crate) load: Option<Arc<dyn GraphResolver + Send + Sync>>,
    pub(crate) counters: Option<&'r AsyncCounters>,
}

impl JobRun<'_> {
    /// The offline run of the synchronous lane: no source, and `stop` only when the
    /// caller supplied a stop source.
    pub(crate) const fn offline(stop: Option<Arc<dyn StopSignal>>) -> Self {
        Self {
            stop,
            remote: None,
            load: None,
            counters: None,
        }
    }

    /// How an ungoverned operation (a query, a serialized query, an update) runs.
    /// `None` when nothing polls the run — the synchronous lane: the operation takes
    /// the engine's ungoverned entry, which installs no governor state and charges
    /// nothing. The metered base with the run's signal attached when a signal must be
    /// polled — an asynchronous job, which yields, stops and is cancelled through it; the
    /// base bounds nothing.
    fn ungoverned_watch(&self) -> Option<QueryGovernors> {
        self.stop
            .as_ref()
            .map(|stop| QueryGovernors::METERED.with_stop_signal(Arc::clone(stop)))
    }

    /// `ceilings` with the run's stop signal attached, when it has one.
    pub(crate) fn governors(&self, ceilings: QueryGovernors) -> QueryGovernors {
        match &self.stop {
            Some(stop) => ceilings.with_stop_signal(Arc::clone(stop)),
            None => ceilings,
        }
    }

    /// Request options carrying the run's sources and `env`.
    pub(crate) fn options<'o>(
        &'o self,
        env: &'o purrdf_sparql_eval::ExtensionEnv,
    ) -> QueryOptions<'o> {
        QueryOptions::new()
            .with_env(env)
            .with_remote(
                self.remote
                    .as_deref()
                    .map(|remote| remote as &(dyn ServiceResolver + Sync)),
            )
            .with_load(
                self.load
                    .as_deref()
                    .map(|load| load as &(dyn GraphResolver + Sync)),
            )
    }

    /// The run's sources, for an ambient execution scope.
    pub(crate) fn sources(&self) -> purrdf_shapes::sparql::QuerySources {
        purrdf_shapes::sparql::QuerySources {
            remote: self.remote.clone(),
            load: self.load.clone(),
        }
    }

    fn timed<T>(&self, cell: impl Fn(&AsyncCounters) -> &AtomicU64, work: impl FnOnce() -> T) -> T {
        let Some(counters) = self.counters else {
            return work();
        };
        let started = now_ms();
        let result = work();
        add_ms(cell(counters), now_ms() - started);
        result
    }

    pub(crate) fn evaluate<T>(&self, work: impl FnOnce() -> T) -> T {
        self.timed(|counters| &counters.evaluate_ms, work)
    }

    fn serialize<T>(&self, work: impl FnOnce() -> T) -> T {
        self.timed(|counters| &counters.serialize_ms, work)
    }

    /// Keep the silenced invocations an evaluation's `evidence` recorded, for the job's
    /// evidence. The synchronous lane reports them on the governed outcome itself.
    pub(crate) fn record_silenced(&self, evidence: &purrdf_core::GovernorEvidence) {
        if let Some(counters) = self.counters {
            counters.record_silenced(evidence);
        }
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// What an entailment query's closure is materialized over and under: the dataset's
/// `owl:imports` table (two parallel arrays), the IRIs the dataset was read from, and the
/// closure's evaluation limits — `queryEntailmentGoverned`'s own arguments. Empty arrays
/// are the ordinary "imports nothing" case; `None` is this target's default limit.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClosureInputs {
    pub(crate) import_iris: Vec<String>,
    pub(crate) import_documents: Vec<String>,
    pub(crate) premise_iris: Vec<String>,
    pub(crate) max_stored_facts: Option<u64>,
    pub(crate) max_join_steps: Option<u64>,
}

/// Everything an operation is built from.
pub(crate) struct OperationInput<'a> {
    pub(crate) kind: AsyncOperationKind,
    pub(crate) engine: Rc<NativeSparqlEngine>,
    pub(crate) frozen: Arc<RdfDataset>,
    pub(crate) sparql: Cow<'a, str>,
    pub(crate) base: Option<Cow<'a, str>>,
    pub(crate) aggregate_namespace: Option<String>,
    pub(crate) ceilings: GovernorArgs,
    pub(crate) format: Option<String>,
    pub(crate) provenance: Option<ProvenanceNamespace>,
    pub(crate) jsonld: Option<JsonLdSerializeOptions>,
    pub(crate) regime: Option<String>,
    pub(crate) program: Option<String>,
    pub(crate) closure: ClosureInputs,
    pub(crate) accept: Option<String>,
}

impl<'a> OperationInput<'a> {
    /// An input of `kind` over `frozen` with every optional part unset.
    pub(crate) fn new(
        kind: AsyncOperationKind,
        engine: &Rc<NativeSparqlEngine>,
        frozen: Arc<RdfDataset>,
        sparql: impl Into<Cow<'a, str>>,
        base: Option<&'a str>,
    ) -> Self {
        Self {
            kind,
            engine: Rc::clone(engine),
            frozen,
            sparql: sparql.into(),
            base: base.map(Cow::Borrowed),
            aggregate_namespace: None,
            ceilings: GovernorArgs::default(),
            format: None,
            provenance: None,
            jsonld: None,
            regime: None,
            program: None,
            closure: ClosureInputs::default(),
            accept: None,
        }
    }
}

/// Run an ungoverned query: on the engine's ungoverned entry when nothing polls the run,
/// and under the metered base and the run's signal when something does.
fn ungoverned_query(
    run: &JobRun<'_>,
    engine: &NativeSparqlEngine,
    frozen: &Arc<RdfDataset>,
    sparql: &str,
    base: Option<&str>,
) -> Result<SparqlResult, JobError> {
    let options = run.options(QueryOptions::EMPTY.env);
    let Some(governors) = run.ungoverned_watch() else {
        // The engine's ungoverned entry, exactly as its `SparqlEngine::query` runs it:
        // the plan, then its evaluation with no governor state.
        return run
            .evaluate(|| {
                let prepared = engine.prepare_query_with_options(sparql, base, options)?;
                engine.query_prepared(frozen, &prepared, &[], options)
            })
            .map_err(JobError::diagnostic);
    };
    let outcome = run.evaluate(|| {
        engine.query_governed(frozen, sparql_request(sparql, base), options, &governors)
    });
    if let Ok(outcome) = &outcome {
        run.record_silenced(outcome.evidence());
    }
    match outcome {
        Err(diagnostic) => Err(JobError::diagnostic(diagnostic)),
        Ok(GovernedOutcome::Complete { result, .. }) => Ok(result),
        Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {
            Err(JobError::stopped(exhausted.tripped))
        }
    }
}

/// The aggregate environment a governed operation's `aggregateNamespace` requests.
fn governed_env(
    aggregate_namespace: Option<&str>,
) -> Result<purrdf_sparql_eval::ExtensionEnv, JobError> {
    // The ENTIRE wasm surface for the first-party statistical aggregate set: one
    // namespace string crosses the boundary, with no callback and no per-aggregate
    // marshaling; the general custom-aggregate seam is Rust-host-only.
    let aggregates = purrdf_validate::query::statistical_aggregates(aggregate_namespace);
    aggregate_env_message(aggregates.as_ref())
        .map_err(|message| JobError::message(EXTENSION_CODE, message))
}

impl OperationInput<'_> {
    /// Evaluate the operation under `run`.
    pub(crate) fn execute(self, run: &JobRun<'_>) -> Result<JobOutcome, JobError> {
        let Self {
            kind,
            engine,
            frozen,
            sparql,
            base,
            aggregate_namespace,
            ceilings,
            format,
            provenance,
            jsonld,
            regime,
            program,
            closure,
            accept,
        } = self;
        let base = base.as_deref();
        let request = sparql_request(&sparql, base);
        match kind {
            AsyncOperationKind::Query => Ok(JobOutcome::Query(ungoverned_query(
                run, &engine, &frozen, &sparql, base,
            )?)),
            AsyncOperationKind::Raw | AsyncOperationKind::RawWithContext => {
                let result = ungoverned_query(run, &engine, &frozen, &sparql, base)?;
                let text = run.serialize(|| match (&jsonld, format.as_deref()) {
                    (Some(options), Some(format)) => {
                        serialize_configured_graph(result, format, options)
                    }
                    _ => serialize_query_result(
                        &result,
                        format.as_deref(),
                        provenance.as_ref(),
                        &sparql,
                    ),
                });
                Ok(JobOutcome::Raw(text.map_err(|message| {
                    JobError::message(SERIALIZE_CODE, message)
                })?))
            }
            AsyncOperationKind::Governed => {
                let env = governed_env(aggregate_namespace.as_deref())?;
                let governors = run.governors(
                    ceilings
                        .ceilings()
                        .map_err(|message| JobError::message(OPTIONS_CODE, message))?,
                );
                let outcome = run
                    .evaluate(|| {
                        engine.query_governed(&frozen, request, run.options(&env), &governors)
                    })
                    .map_err(JobError::diagnostic)?;
                run.record_silenced(outcome.evidence());
                Ok(JobOutcome::Governed(Box::new(outcome)))
            }
            AsyncOperationKind::Negotiated => {
                let env = governed_env(aggregate_namespace.as_deref())?;
                let governors = run.governors(
                    ceilings
                        .ceilings()
                        .map_err(|message| JobError::message(OPTIONS_CODE, message))?,
                );
                let outcome = run
                    .evaluate(|| {
                        engine.query_governed(&frozen, request, run.options(&env), &governors)
                    })
                    .map_err(JobError::diagnostic)?;
                run.record_silenced(outcome.evidence());
                let value = match outcome {
                    GovernedOutcome::Complete {
                        result, evidence, ..
                    } => {
                        // Negotiated against the result's actual shape: a graph carrying
                        // named graphs is offered only in the syntaxes that can hold it,
                        // so no format ever silently drops a graph.
                        let shape = negotiable_result_kind(&result);
                        let format = negotiate(accept.as_deref(), shape).ok_or_else(|| {
                            JobError::not_acceptable(not_acceptable_message(shape))
                        })?;
                        let text = run
                            .serialize(|| {
                                serialize_query_result(&result, Some(format), None, &sparql)
                            })
                            .map_err(|message| JobError::message(SERIALIZE_CODE, message))?;
                        NegotiatedValue::Complete {
                            bytes: text.into_bytes(),
                            format,
                            evidence,
                        }
                    }
                    GovernedOutcome::BudgetExhausted(exhausted) => {
                        NegotiatedValue::Exhausted(exhausted)
                    }
                };
                Ok(JobOutcome::Negotiated(Box::new(value)))
            }
            AsyncOperationKind::EntailmentGoverned => {
                let regime = regime.unwrap_or_default();
                let plan = QueryEntailmentPlan::parse(&regime, program.as_deref().unwrap_or(""))
                    .map_err(|message| JobError::message(ENTAILMENT_CODE, message))?;
                let imports = crate::entail::entailment_import_map(
                    &closure.import_iris,
                    &closure.import_documents,
                    &closure.premise_iris,
                )
                .map_err(|message| JobError::message(ENTAILMENT_CODE, message))?;
                let limits =
                    crate::entail::wasm_limits(closure.max_stored_facts, closure.max_join_steps);
                let env = governed_env(aggregate_namespace.as_deref())?;
                let governors = run.governors(
                    ceilings
                        .ceilings()
                        .map_err(|message| JobError::message(OPTIONS_CODE, message))?,
                );
                let outcome = run
                    .evaluate(|| {
                        query_with_entailment_closure_governed(
                            &engine,
                            &frozen,
                            request,
                            &EntailmentClosure::new(plan.entailment(), &imports)
                                .with_limits(limits.eval_options()),
                            run.options(&env),
                            // This surface registers no relation, so there is none to
                            // re-derive over the closure.
                            &ClosureRelations::NONE,
                            &governors,
                        )
                    })
                    .map_err(|error| match error {
                        // Rendered by the shared boundary, so a passed evaluation limit
                        // names `queryEntailmentGoverned`'s argument rather than a Rust type
                        // a JavaScript caller cannot reach.
                        ReasoningError::Entailment(error) => JobError::message(
                            ENTAILMENT_CODE,
                            purrdf_validate::render_entail_error_in(
                                &regime,
                                &error,
                                purrdf_validate::RegimeHost::Wasm,
                                purrdf_validate::RegimeService::Query,
                            ),
                        ),
                        other => JobError::reasoning(&other),
                    })?;
                if let Some(answered) = outcome.outcome() {
                    run.record_silenced(answered.evidence());
                }
                Ok(JobOutcome::Entailment(Box::new(outcome)))
            }
            AsyncOperationKind::Explain => {
                // The measuring run — metered, never bounded — with the run's sources
                // installed and its signal, when it has one, polled at every charge point.
                let options = run.options(QueryOptions::EMPTY.env);
                let explanation = run
                    .evaluate(|| match &run.stop {
                        Some(stop) => engine.explain_query_with_stop_signal(
                            &frozen,
                            &sparql,
                            base,
                            options,
                            Arc::clone(stop),
                        ),
                        None => engine.explain_query_with_options(&frozen, &sparql, base, options),
                    })
                    .map_err(JobError::diagnostic)?;
                run.record_silenced(explanation.evidence());
                // A stop cut the measuring run short, so its ledger describes a truncated
                // run rather than the query: the stop is the operation's error, as it is
                // for every ungoverned operation. Any other trip is part of the
                // explanation.
                if let Some(tripped @ TrippedGovernor::Stopped { .. }) =
                    explanation.evidence().tripped
                {
                    return Err(JobError::stopped(tripped));
                }
                Ok(JobOutcome::Raw(explanation.render()))
            }
            AsyncOperationKind::Update => {
                let mut target = Arc::clone(&frozen);
                let Some(governors) = run.ungoverned_watch() else {
                    run.evaluate(|| {
                        engine.update_with_options(
                            &mut target,
                            request,
                            run.options(QueryOptions::EMPTY.env),
                        )
                    })
                    .map_err(JobError::diagnostic)?;
                    return Ok(JobOutcome::Updated(target));
                };
                let outcome = run
                    .evaluate(|| {
                        engine.update_governed(
                            &mut target,
                            request,
                            run.options(QueryOptions::EMPTY.env),
                            &governors,
                        )
                    })
                    .map_err(JobError::diagnostic)?;
                run.record_silenced(outcome.evidence());
                match outcome.tripped() {
                    None => Ok(JobOutcome::Updated(target)),
                    Some(tripped) => Err(JobError::stopped(tripped)),
                }
            }
            // A SHACL operation reads no dataset and no SPARQL text; it is built by
            // `AsyncJob.beginShacl`, never as an input.
            AsyncOperationKind::Shacl => Err(JobError::message(
                OPTIONS_CODE,
                crate::async_query::SHACL_STARTS_ELSEWHERE,
            )),
            AsyncOperationKind::UpdateGoverned => {
                let env = governed_env(aggregate_namespace.as_deref())?;
                let governors = run.governors(
                    ceilings
                        .update_ceilings()
                        .map_err(|message| JobError::message(OPTIONS_CODE, message))?,
                );
                let mut target = Arc::clone(&frozen);
                let outcome = run
                    .evaluate(|| {
                        engine.update_governed(&mut target, request, run.options(&env), &governors)
                    })
                    .map_err(JobError::diagnostic)?;
                run.record_silenced(outcome.evidence());
                // The engine publishes into `target` only on the applied path, so a
                // tripped request offers nothing to commit.
                let frozen = outcome.is_applied().then_some(target);
                Ok(JobOutcome::UpdateGoverned { outcome, frozen })
            }
        }
    }

    /// Run the operation on the synchronous lane: offline, with `stop` only when the
    /// caller supplied a stop source, and a failure thrown as the error carrying its code.
    pub(crate) fn run_offline(
        self,
        stop: Option<Arc<dyn StopSignal>>,
    ) -> Result<JobOutcome, JsValue> {
        self.execute(&JobRun::offline(stop))
            .map_err(|error| error.to_js())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every failure renders in its own words, the stack refusals included: the
    /// evaluator's shadow-stack refusal and the host-stack refusal read exactly as the
    /// engine renders them, with nothing appended, and so does every neighbour — a parse
    /// failure, an evaluation failure, a fault quoting the code.
    #[test]
    fn a_stack_refusal_renders_as_the_evaluator_gives_it() {
        use purrdf_sparql_eval::EvalError;

        let code = EvalError::STACK_EXHAUSTED_CODE;
        let error = JobError::diagnostic(RdfDiagnostic::error(code, "nested too deeply"));
        assert_eq!(error.rendered(), format!("error {code}: nested too deeply"));
        for error in [
            JobError::diagnostic(RdfDiagnostic::error(
                EvalError::HOST_STACK_EXHAUSTED_CODE,
                "the host stack is the same on both lanes",
            )),
            JobError::diagnostic(RdfDiagnostic::error(
                "native-sparql-query-parse",
                "SPARQL syntax error at byte 9: native-sparql-evaluation-stack-exhausted is not a keyword",
            )),
            JobError::diagnostic(RdfDiagnostic::error(
                "native-sparql-query-eval",
                "the example.org function failed",
            )),
            JobError::fault(format!("error {code}: a host fault quoting it")),
        ] {
            let plain = match error.text {
                ErrorText::Diagnostic => error.diagnostic.to_string(),
                ErrorText::Message => error.diagnostic.message.clone(),
            };
            assert_eq!(error.rendered(), plain);
        }
    }

    /// A stop signal that never fires: something polls the run, nothing stops it.
    #[derive(Debug)]
    struct Quiet;

    impl StopSignal for Quiet {
        fn poll(&self) -> Option<StopCause> {
            None
        }
    }

    /// The synchronous lane's ungoverned operations take the engine's ungoverned entry:
    /// no governor state is installed, so nothing is metered and no `SILENT` invocation
    /// is recorded. The neighbour — the same operation with a signal to poll, as an
    /// asynchronous job runs it — is metered, and records the invocation `SILENT`
    /// absorbed. Both answer the same rows.
    #[test]
    fn an_ungoverned_offline_operation_installs_no_governor_state() {
        let engine = Rc::new(NativeSparqlEngine::new());
        let frozen = crate::dataset::Dataset::parse(
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
            "ntriples",
            None,
        )
        .expect("parses")
        .view()
        .freeze()
        .expect("freezes");
        let silent =
            "SELECT ?s WHERE { ?s ?p ?o SERVICE SILENT <http://example.org/sparql> { ?o ?q ?x } }";
        let run_with = |stop: Option<Arc<dyn StopSignal>>| {
            let counters = AsyncCounters::default();
            let run = JobRun {
                stop,
                remote: None,
                load: None,
                counters: Some(&counters),
            };
            for kind in [AsyncOperationKind::Query, AsyncOperationKind::Raw] {
                let input = OperationInput::new(kind, &engine, Arc::clone(&frozen), silent, None);
                assert!(input.execute(&run).is_ok(), "{kind:?}");
            }
            let update = OperationInput::new(
                AsyncOperationKind::Update,
                &engine,
                Arc::clone(&frozen),
                "LOAD SILENT <http://example.org/doc>",
                None,
            );
            assert!(matches!(update.execute(&run), Ok(JobOutcome::Updated(_))));
            counters.snapshot(0).silenced().len()
        };
        assert_eq!(
            run_with(None),
            0,
            "an ungoverned run keeps no evidence at all"
        );
        assert_eq!(
            run_with(Some(Arc::new(Quiet))),
            3,
            "a watched run is metered and records every silenced invocation"
        );
        assert!(JobRun::offline(None).ungoverned_watch().is_none());
        assert!(
            JobRun::offline(Some(Arc::new(Quiet)))
                .ungoverned_watch()
                .is_some()
        );
    }

    /// Each kind of failure carries the code an HTTP host answers it by, read from its
    /// structure rather than its words.
    #[test]
    fn every_error_kind_names_its_failure() {
        let failure = |error: &JobError| FailureCode::from_diagnostic_code(error.code());
        let parse = JobError::diagnostic(RdfDiagnostic::error(
            "native-sparql-query-parse",
            "SPARQL syntax error at byte 0",
        ));
        assert_eq!(failure(&parse), FailureCode::QueryParse);
        let evaluation = JobError::diagnostic(RdfDiagnostic::error(
            "native-sparql-query-eval",
            "evaluation failed",
        ));
        assert_eq!(failure(&evaluation), FailureCode::Evaluation);
        let cancelled = JobError::stopped(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled,
        });
        assert_eq!(cancelled.kind(), JobErrorKind::Cancelled);
        assert_eq!(failure(&cancelled), FailureCode::Cancelled);
        assert_eq!(
            cancelled.rendered(),
            "the asynchronous operation was cancelled"
        );
        let deadline = JobError::stopped(TrippedGovernor::Stopped {
            cause: StopCause::Deadline,
        });
        assert_eq!(deadline.kind(), JobErrorKind::Deadline);
        assert_eq!(failure(&deadline), FailureCode::Deadline);
        let fault = JobError::fault("resolver returned nothing");
        assert_eq!(fault.kind(), JobErrorKind::Fault);
        assert_eq!(failure(&fault), FailureCode::HostFault);
        assert_eq!(fault.rendered(), "resolver returned nothing");
        let refused = JobError::not_acceptable("no format".to_owned());
        assert_eq!(failure(&refused), FailureCode::NotAcceptable);
        // A failure with its own words keeps its code apart from them.
        let serialize = JobError::message(SERIALIZE_CODE, "unsupported format");
        assert_eq!(serialize.rendered(), "unsupported format");
        assert_eq!(serialize.code(), SERIALIZE_CODE);
        assert_eq!(failure(&serialize), FailureCode::Evaluation);
    }
}
