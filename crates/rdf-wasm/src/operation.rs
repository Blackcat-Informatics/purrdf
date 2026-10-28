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

use purrdf::ir::GraphExistenceMode;
use purrdf::xsd::exact::DivisionPolicy;
use purrdf::{
    ClosureRelations, EntailmentClosure, GovernedEntailment, JsonLdSerializeOptions,
    QueryEntailmentPlan, RdfDataset, ReasoningError, query_with_entailment_closure_governed,
};
use purrdf_core::xsd_regex::xpath::Profile;
use purrdf_core::{DiagnosticPresentation, RdfDiagnostic, SparqlResult};
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
    /// A host context's existing rendering beside the untouched native diagnostic.
    ReasoningQuery,
}

/// Why an operation failed: its kind, and the diagnostic whose code says why.
///
/// A failure reads the same on both lanes. An asynchronous job runs on a stack region
/// exactly as large as the synchronous lane's shadow stack, so the evaluator's stack
/// refusal falls at the same depth on each and carries no lane-specific remedy.
#[derive(Debug, Clone)]
pub(crate) struct JobError {
    execution_receipt: Option<Box<str>>,
    kind: JobErrorKind,
    diagnostic: Box<RdfDiagnostic>,
    text: ErrorText,
    /// The engine diagnostic's typed presentation, when the failure carries one: a
    /// SPARQL parse failure's `sparql-parse-*` condition, thrown as `presentation`.
    presentation: Option<Box<DiagnosticPresentation>>,
}

impl JobError {
    /// An engine diagnostic, rendered as the engine renders it.
    pub(crate) fn diagnostic(diagnostic: RdfDiagnostic) -> Self {
        Self {
            execution_receipt: None,
            kind: JobErrorKind::Error,
            presentation: diagnostic.presentation().cloned().map(Box::new),
            diagnostic: Box::new(diagnostic),
            text: ErrorText::Diagnostic,
        }
    }

    /// A failure reported under `code` in `message`'s own words.
    pub(crate) fn message(code: &str, message: impl Into<String>) -> Self {
        Self {
            execution_receipt: None,
            kind: JobErrorKind::Error,
            diagnostic: Box::new(RdfDiagnostic::error(code, message)),
            text: ErrorText::Message,
            presentation: None,
        }
    }

    /// A negotiated query whose result no acceptable format can carry.
    pub(crate) fn not_acceptable(message: String) -> Self {
        Self {
            execution_receipt: None,
            kind: JobErrorKind::NotAcceptable,
            diagnostic: Box::new(RdfDiagnostic::error(
                FailureCode::NotAcceptable.code(),
                message,
            )),
            text: ErrorText::Message,
            presentation: None,
        }
    }

    /// A latched fault: the host broke the effect protocol.
    pub(crate) fn fault(message: impl Into<String>) -> Self {
        Self {
            execution_receipt: None,
            kind: JobErrorKind::Fault,
            diagnostic: Box::new(RdfDiagnostic::error(FailureCode::HostFault.code(), message)),
            text: ErrorText::Message,
            presentation: None,
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
            execution_receipt: None,
            kind,
            diagnostic: Box::new(RdfDiagnostic::error(failure.code(), message)),
            text: ErrorText::Message,
            presentation: None,
        }
    }

    /// An entailment query's failure: a query diagnostic keeps its code, and the closure's
    /// own failure is reported under [`ENTAILMENT_CODE`]; either reads as the reasoning
    /// layer renders it.
    fn reasoning(error: &ReasoningError) -> Self {
        match error {
            ReasoningError::Query(diagnostic) => Self {
                text: ErrorText::ReasoningQuery,
                ..Self::diagnostic(diagnostic.clone())
            },
            _ => Self::message(ENTAILMENT_CODE, error.to_string()),
        }
    }

    pub(crate) const fn kind(&self) -> JobErrorKind {
        self.kind
    }

    pub(crate) fn with_execution_receipt(mut self, receipt: String) -> Self {
        self.execution_receipt = Some(receipt.into_boxed_str());
        self
    }

    pub(crate) fn execution_receipt_json(&self) -> Option<String> {
        self.execution_receipt.as_deref().map(str::to_owned)
    }

    /// The stable code the failure is reported under.
    pub(crate) fn code(&self) -> &str {
        &self.diagnostic.code
    }

    /// The message a JavaScript caller reads.
    pub(crate) fn rendered(&self) -> String {
        if cfg!(target_arch = "wasm32")
            && self.text == ErrorText::Diagnostic
            && self.code() == purrdf_sparql_eval::EvalError::STACK_EXHAUSTED_CODE
        {
            // The evaluator retains the exact frame in its native diagnostic, but
            // two equivalent wasm runs can discover the low stack in adjacent frames.
            // The package reports one position-independent statement on both lanes.
            return RdfDiagnostic::error(
                self.code(),
                purrdf_sparql_eval::EvalError::StackExhausted {
                    construct: "query evaluation",
                }
                .to_string(),
            )
            .to_string();
        }
        match &self.text {
            ErrorText::Diagnostic => self.diagnostic.to_string(),
            ErrorText::Message => self.diagnostic.message.clone(),
            ErrorText::ReasoningQuery => {
                let error: ReasoningError = ReasoningError::Query((*self.diagnostic).clone());
                error.to_string()
            }
        }
    }

    /// The failure's typed presentation as its JSON record (`messageId`, typed
    /// `parameters` with exact integers as decimal strings, nested `detail`), or `None`
    /// when it carries none.
    pub(crate) fn presentation_json(&self) -> Option<String> {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.to_json().to_string())
    }

    /// A storage refusal's original diagnostic record. Capacity counts remain decimal
    /// strings, including values wider than either a JavaScript number or a native ID.
    pub(crate) fn diagnostic_json(&self) -> Option<String> {
        self.diagnostic
            .storage_error()
            .map(|_| self.diagnostic.to_json().to_string())
    }

    /// The JavaScript error this failure is thrown as: its message, its `code`, and,
    /// when it carries one, its typed `presentation`.
    pub(crate) fn to_js(&self) -> JsValue {
        let error = CodedError::new(&self.rendered());
        if let Some(receipt) = &self.execution_receipt {
            error.set_execution_receipt_json(receipt);
        }
        error.set_code(self.code());
        if let Some(record) = self.diagnostic_json() {
            match json_parse(&record) {
                Ok(diagnostic) => error.set_diagnostic(&diagnostic),
                // Match the asynchronous adapter: failure to construct the required
                // record is thrown, never a successful but incomplete storage error.
                Err(refusal) => return refusal,
            }
        }
        // The record is this crate's own deterministic JSON, so `JSON.parse` accepts it;
        // were it ever refused, the error is still thrown with its message and code.
        if let Some(record) = self.presentation_json()
            && let Ok(presentation) = json_parse(&record)
        {
            error.set_presentation(&presentation);
        }
        error.into()
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

    #[wasm_bindgen(method, setter = executionReceiptJson)]
    fn set_execution_receipt_json(this: &CodedError, receipt: &str);

    #[wasm_bindgen(method, setter = diagnostic)]
    fn set_diagnostic(this: &CodedError, diagnostic: &JsValue);

    #[wasm_bindgen(method, setter = presentation)]
    fn set_presentation(this: &CodedError, presentation: &JsValue);

    /// The host's `JSON.parse`, which builds a presentation record as a plain object.
    #[wasm_bindgen(catch, js_namespace = JSON, js_name = parse)]
    fn json_parse(text: &str) -> Result<JsValue, JsValue>;

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

/// Attach a receipt without changing the original JavaScript rejection class.
pub(crate) fn attach_execution_receipt(error: JsValue, receipt: &str) -> JsValue {
    use wasm_bindgen::JsCast;
    error
        .unchecked_ref::<CodedError>()
        .set_execution_receipt_json(receipt);
    error
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
#[expect(
    clippy::large_enum_variant,
    reason = "The stopped-update payload moves inline so outcome transfer does not allocate after workspace refusal."
)]
pub(crate) enum JobOutcome {
    Query(SparqlResult, purrdf_sparql_eval::QueryCoordinates),
    QueryRaw {
        body: String,
        coordinates: purrdf_sparql_eval::QueryCoordinates,
    },
    Raw(String),
    Governed(Box<GovernedOutcome>),
    Entailment(Box<GovernedEntailment>),
    Updated(Arc<RdfDataset>, purrdf_sparql_eval::QueryCoordinates),
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
            Self::Query(..) => "Query",
            Self::QueryRaw { .. } => "QueryRaw",
            Self::Raw(_) => "Raw",
            Self::Governed(_) => "Governed",
            Self::Entailment(_) => "Entailment",
            Self::Updated(..) => "Updated",
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
        env: &'o purrdf_sparql_eval::ExtensionEnv<'o>,
    ) -> QueryOptions<'o, 'o> {
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

    /// [`Self::options`] with every `xsd:integer`/`xsd:decimal` quotient formed under
    /// `division`: the request options every SPARQL operation evaluates with.
    pub(crate) fn options_under<'o>(
        &'o self,
        env: &'o purrdf_sparql_eval::ExtensionEnv<'o>,
        division: DivisionPolicy,
    ) -> QueryOptions<'o, 'o> {
        self.options(env).with_division(division)
    }

    /// [`Self::options_under`] evaluating `REGEX`/`REPLACE` under `xpath_regex`, when a
    /// dated law was selected.
    fn selected_options<'o>(
        &'o self,
        env: &'o purrdf_sparql_eval::ExtensionEnv<'o>,
        division: DivisionPolicy,
        xpath_regex: Option<Profile>,
        geo: &'o purrdf_validate::geo::GeoProfile,
        coordinates: &'o purrdf_validate::query::CoordinateInputs,
    ) -> QueryOptions<'o, 'o> {
        coordinates.apply(crate::xpath_regex::select(
            self.options_under(env, division).with_geo(geo),
            xpath_regex,
        ))
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

    /// Keep the silenced invocations and the absorbed expression errors an evaluation's
    /// `evidence` recorded, for the job's evidence. The synchronous lane reports them on
    /// the governed outcome itself.
    pub(crate) fn record_evidence(&self, evidence: &purrdf_core::GovernorEvidence) {
        if let Some(counters) = self.counters {
            counters.record_evidence(evidence);
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
    pub(crate) max_term_arena_bytes: Option<u64>,
}

/// Everything an operation is built from.
pub(crate) struct OperationInput<'a> {
    /// Exact caller inputs owned for the complete operation, including suspension.
    pub(crate) coordinates: purrdf_validate::query::CoordinateInputs,
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
    /// The dated native XPath law `REGEX`/`REPLACE` evaluate under (`xpathRegex`);
    /// `None` keeps the compatibility regex.
    pub(crate) xpath_regex: Option<Profile>,
    /// The precision of every `xsd:integer`/`xsd:decimal` quotient the operation forms.
    pub(crate) division: DivisionPolicy,
    /// The dataset policy captured before freezing, retained through UPDATE execution.
    pub(crate) graph_existence: GraphExistenceMode,
    /// The immutable profile captured when this operation starts.
    pub(crate) geo: purrdf_validate::geo::GeoProfileSnapshot,
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
            coordinates: purrdf_validate::query::CoordinateInputs::default(),
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
            xpath_regex: None,
            division: DivisionPolicy::xsd_default(),
            graph_existence: GraphExistenceMode::default(),
            geo: purrdf_validate::geo::GeoSession::default().profile_snapshot(),
        }
    }
}

/// Run an ungoverned query: on the engine's ungoverned entry when nothing polls the run,
/// and under the metered base and the run's signal when something does.
#[expect(
    clippy::too_many_arguments,
    reason = "The run keeps explicit dataset, source, arithmetic, regex and geographic policies as independently borrowed query inputs"
)]
fn ungoverned_query(
    run: &JobRun<'_>,
    engine: &NativeSparqlEngine,
    frozen: &Arc<RdfDataset>,
    sparql: &str,
    base: Option<&str>,
    division: DivisionPolicy,
    xpath_regex: Option<Profile>,
    geo: &purrdf_validate::geo::GeoProfile,
    coordinates: &purrdf_validate::query::CoordinateInputs,
) -> Result<(SparqlResult, purrdf_sparql_eval::QueryCoordinates), JobError> {
    let capture = purrdf_validate::query::ExecutionReceiptCapture::default();
    let options = run
        .selected_options(
            QueryOptions::EMPTY.env,
            division,
            xpath_regex,
            geo,
            coordinates,
        )
        .with_coordinate_observer(&capture);
    let Some(governors) = run.ungoverned_watch() else {
        // The engine's ungoverned entry, exactly as its `SparqlEngine::query` runs it:
        // the plan, then its evaluation with no governor state.
        let result = run
            .evaluate(|| {
                let prepared = engine.prepare_query_with_options(sparql, base, options)?;
                engine.query_prepared(frozen, &prepared, &[], options)
            })
            .map_err(JobError::diagnostic);
        let receipt = capture
            .into_coordinates()
            .map_err(|error| JobError::fault(error.to_string()))?;
        return result
            .map(|result| (result, receipt.clone()))
            .map_err(|error| {
                error.with_execution_receipt(purrdf_validate::query::execution_receipt_json(
                    &receipt,
                ))
            });
    };
    let outcome = run.evaluate(|| {
        engine.query_governed(frozen, sparql_request(sparql, base), options, &governors)
    });
    if let Ok(outcome) = &outcome {
        run.record_evidence(outcome.evidence());
    }
    match outcome {
        Err(diagnostic) => {
            let receipt = capture
                .into_coordinates()
                .map_err(|error| JobError::fault(error.to_string()))?;
            Err(JobError::diagnostic(diagnostic)
                .with_execution_receipt(purrdf_validate::query::execution_receipt_json(&receipt)))
        }
        Ok(GovernedOutcome::Complete {
            result,
            coordinates,
            ..
        }) => Ok((result, coordinates)),
        Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {
            Err(JobError::stopped(exhausted.tripped).with_execution_receipt(
                purrdf_validate::query::execution_receipt_json(&exhausted.coordinates),
            ))
        }
    }
}

/// The aggregate environment a governed operation's `aggregateNamespace` requests.
fn governed_env(
    aggregate_namespace: Option<&str>,
) -> Result<purrdf_sparql_eval::ExtensionEnv<'static>, JobError> {
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
            coordinates,
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
            xpath_regex,
            division,
            graph_existence,
            geo,
        } = self;
        let base = base.as_deref();
        let request = sparql_request(&sparql, base);
        let capture = purrdf_validate::query::ExecutionReceiptCapture::default();
        let result = (|| {
            match kind {
                AsyncOperationKind::Query => {
                    let (result, receipt) = ungoverned_query(
                        run,
                        &engine,
                        &frozen,
                        &sparql,
                        base,
                        division,
                        xpath_regex,
                        &geo,
                        &coordinates,
                    )?;
                    Ok(JobOutcome::Query(result, receipt))
                }
                AsyncOperationKind::Raw | AsyncOperationKind::RawWithContext => {
                    let (result, receipt) = ungoverned_query(
                        run,
                        &engine,
                        &frozen,
                        &sparql,
                        base,
                        division,
                        xpath_regex,
                        &geo,
                        &coordinates,
                    )?;
                    let text = run.serialize(|| match (&jsonld, format.as_deref()) {
                        (Some(options), Some(format)) => {
                            serialize_configured_graph(result, format, options)
                        }
                        _ => serialize_query_result(
                            &result,
                            format.as_deref(),
                            provenance.as_ref(),
                            &sparql,
                        )
                        .map_err(|message| JobError::message(SERIALIZE_CODE, message)),
                    });
                    Ok(JobOutcome::QueryRaw {
                        body: text.map_err(|error| {
                            error.with_execution_receipt(
                                purrdf_validate::query::execution_receipt_json(&receipt),
                            )
                        })?,
                        coordinates: receipt,
                    })
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
                            engine.query_governed(
                                &frozen,
                                request,
                                run.selected_options(
                                    &env,
                                    division,
                                    xpath_regex,
                                    &geo,
                                    &coordinates,
                                )
                                .with_coordinate_observer(&capture),
                                &governors,
                            )
                        })
                        .map_err(JobError::diagnostic)?;
                    run.record_evidence(outcome.evidence());
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
                            engine.query_governed(
                                &frozen,
                                request,
                                run.selected_options(
                                    &env,
                                    division,
                                    xpath_regex,
                                    &geo,
                                    &coordinates,
                                )
                                .with_coordinate_observer(&capture),
                                &governors,
                            )
                        })
                        .map_err(JobError::diagnostic)?;
                    run.record_evidence(outcome.evidence());
                    let value = match outcome {
                        GovernedOutcome::Complete {
                            result,
                            evidence,
                            coordinates,
                            ..
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
                                coordinates,
                                bytes: text.into_bytes(),
                                format,
                                evidence,
                            }
                        }
                        GovernedOutcome::BudgetExhausted(exhausted) => NegotiatedValue::Exhausted {
                            coordinates: exhausted.coordinates,
                            tripped: exhausted.tripped,
                            evidence: exhausted.evidence,
                            partial: exhausted.partial,
                        },
                    };
                    Ok(JobOutcome::Negotiated(Box::new(value)))
                }
                AsyncOperationKind::EntailmentGoverned => {
                    let regime = regime.unwrap_or_default();
                    let plan =
                        QueryEntailmentPlan::parse(&regime, program.as_deref().unwrap_or(""))
                            .map_err(|message| JobError::message(ENTAILMENT_CODE, message))?;
                    let imports = crate::entail::entailment_import_map(
                        &closure.import_iris,
                        &closure.import_documents,
                        &closure.premise_iris,
                    )
                    .map_err(|message| JobError::message(ENTAILMENT_CODE, message))?;
                    let limits = crate::entail::wasm_limits(
                        closure.max_stored_facts,
                        closure.max_join_steps,
                        closure.max_term_arena_bytes,
                    );
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
                                run.selected_options(
                                    &env,
                                    division,
                                    xpath_regex,
                                    &geo,
                                    &coordinates,
                                )
                                .with_coordinate_observer(&capture),
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
                        run.record_evidence(answered.evidence());
                    }
                    Ok(JobOutcome::Entailment(Box::new(outcome)))
                }
                AsyncOperationKind::Explain => {
                    // The measuring run — metered, never bounded — with the run's sources
                    // installed and its signal, when it has one, polled at every charge point.
                    let options = run
                        .selected_options(
                            QueryOptions::EMPTY.env,
                            division,
                            xpath_regex,
                            &geo,
                            &coordinates,
                        )
                        .with_coordinate_observer(&capture);
                    let explanation = run
                        .evaluate(|| match &run.stop {
                            Some(stop) => engine.explain_query_with_stop_signal(
                                &frozen,
                                &sparql,
                                base,
                                options,
                                Arc::clone(stop),
                            ),
                            None => {
                                engine.explain_query_with_options(&frozen, &sparql, base, options)
                            }
                        })
                        .map_err(JobError::diagnostic)?;
                    run.record_evidence(explanation.evidence());
                    // A stop cut the measuring run short, so its ledger describes a truncated
                    // run rather than the query: the stop is the operation's error, as it is
                    // for every ungoverned operation. Any other trip is part of the
                    // explanation.
                    if let Some(tripped @ TrippedGovernor::Stopped { .. }) =
                        explanation.evidence().tripped
                    {
                        return Err(JobError::stopped(tripped));
                    }
                    Ok(JobOutcome::QueryRaw {
                        body: explanation.render(),
                        coordinates: capture
                            .coordinates()
                            .map_err(|error| JobError::fault(error.to_string()))?,
                    })
                }
                AsyncOperationKind::Update => {
                    let mut target = Arc::clone(&frozen);
                    let capture = purrdf_validate::query::ExecutionReceiptCapture::default();
                    let options = run
                        .selected_options(
                            QueryOptions::EMPTY.env,
                            division,
                            xpath_regex,
                            &geo,
                            &coordinates,
                        )
                        .with_graph_existence(graph_existence)
                        .with_coordinate_observer(&capture);
                    let result = match run.ungoverned_watch() {
                        None => run
                            .evaluate(|| engine.update_with_options(&mut target, request, options))
                            .map_err(JobError::diagnostic),
                        Some(governors) => run
                            .evaluate(|| {
                                engine.update_governed(&mut target, request, options, &governors)
                            })
                            .map_err(JobError::diagnostic)
                            .and_then(|outcome| {
                                run.record_evidence(outcome.evidence());
                                match outcome.tripped() {
                                    None => Ok(()),
                                    Some(tripped) => Err(JobError::stopped(tripped)),
                                }
                            }),
                    };
                    let receipt = capture
                        .into_coordinates()
                        .map_err(|error| JobError::fault(error.to_string()))?;
                    result.map_err(|error| {
                        error.with_execution_receipt(
                            purrdf_validate::query::execution_receipt_json(&receipt),
                        )
                    })?;
                    Ok(JobOutcome::Updated(target, receipt))
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
                            engine.update_governed(
                                &mut target,
                                request,
                                run.selected_options(
                                    &env,
                                    division,
                                    xpath_regex,
                                    &geo,
                                    &coordinates,
                                )
                                .with_coordinate_observer(&capture)
                                .with_graph_existence(graph_existence),
                                &governors,
                            )
                        })
                        .map_err(JobError::diagnostic)?;
                    run.record_evidence(outcome.evidence());
                    // The engine publishes into `target` only on the applied path, so a
                    // tripped request offers nothing to commit.
                    let frozen = outcome.is_applied().then_some(target);
                    Ok(JobOutcome::UpdateGoverned { outcome, frozen })
                }
            }
        })();
        result.map_err(|error: JobError| {
            if error.execution_receipt.is_some() {
                return error;
            }
            match capture.coordinates() {
                Ok(receipt) => error.with_execution_receipt(
                    purrdf_validate::query::execution_receipt_json(&receipt),
                ),
                Err(cause) => JobError::fault(cause.to_string()),
            }
        })
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

    #[test]
    fn reasoning_storage_failure_keeps_original_cause_and_context() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

        let cause = StorageError::Capacity(CapacityError::new(
            CapacitySpace::GlobalTermBytes,
            u128::MAX,
            u128::from(u64::MAX) + 1,
        ));
        let original = RdfDiagnostic::from(cause);
        let expected = original.to_json().to_string();
        let reasoning = ReasoningError::Query(original.clone());
        for error in [
            JobError::diagnostic(original.clone()),
            JobError::reasoning(&reasoning),
        ] {
            assert_eq!(error.diagnostic.storage_error(), Some(cause));
            assert_eq!(error.code(), original.code);
            assert_eq!(error.diagnostic_json().as_deref(), Some(expected.as_str()));
            assert!(error.presentation_json().is_none());
        }
        assert_eq!(
            JobError::reasoning(&reasoning).rendered(),
            reasoning.to_string()
        );
    }

    /// A parse failure keeps the parser's typed presentation on both the direct and the
    /// entailment lane, with its message unchanged; a failure without one carries none.
    #[test]
    fn parse_failures_keep_their_typed_presentation() {
        let engine = NativeSparqlEngine::new();
        for (query, identity) in [
            ("ASK {", "sparql-parse-syntax"),
            ("ASK { ?s ?p \"unterminated", "sparql-parse-lex"),
            ("SELECT * WHERE { <relative> ?p ?o }", "sparql-parse-iri"),
        ] {
            let original = engine
                .prepare_query(query, None)
                .expect_err("refused query");
            let expected = original
                .presentation()
                .expect("parse presentation")
                .to_json()
                .to_string();
            assert!(expected.contains(identity), "{expected}");
            let direct = JobError::diagnostic(original.clone());
            assert_eq!(
                direct.presentation_json().as_deref(),
                Some(expected.as_str())
            );
            assert_eq!(direct.rendered(), original.to_string());
            let reasoning = JobError::reasoning(&ReasoningError::Query(original.clone()));
            assert_eq!(
                reasoning.presentation_json().as_deref(),
                Some(expected.as_str())
            );
            let expected: ReasoningError = ReasoningError::Query(original);
            assert_eq!(reasoning.rendered(), expected.to_string());
        }
        assert!(
            JobError::diagnostic(RdfDiagnostic::error("native-sparql-eval", "x"))
                .presentation_json()
                .is_none()
        );
        assert!(
            JobError::message("purrdf-wasm-usage", "x")
                .presentation_json()
                .is_none()
        );
    }

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
                ErrorText::ReasoningQuery => {
                    panic!("the stack fixtures do not construct a reasoning failure")
                }
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
            assert!(matches!(update.execute(&run), Ok(JobOutcome::Updated(..))));
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

    /// The SELECT, INSERT and kind-specific settings an operation of `kind` dividing
    /// `numerator` by `denominator` is built from.
    fn dividing<'a>(
        kind: AsyncOperationKind,
        engine: &Rc<NativeSparqlEngine>,
        frozen: &Arc<RdfDataset>,
        sparql: &'a str,
        division: DivisionPolicy,
    ) -> OperationInput<'a> {
        let mut input = OperationInput::new(kind, engine, Arc::clone(frozen), sparql, None);
        input.division = division;
        match kind {
            AsyncOperationKind::Negotiated => {
                input.accept = Some("application/sparql-results+json".to_owned());
            }
            AsyncOperationKind::EntailmentGoverned => input.regime = Some("rdfs".to_owned()),
            _ => {}
        }
        input
    }

    /// The rounded third, `1/3` at the default eighteen digits.
    const THIRD: &str = "0.333333333333333333";

    /// Whether a finished job's answer carries [`THIRD`]: a bound row, a serialized
    /// body, or — for an UPDATE — a dataset holding it.
    fn carries_third(engine: &NativeSparqlEngine, outcome: &JobOutcome) -> bool {
        let in_result = |result: &SparqlResult| {
            match result {
            SparqlResult::Solutions { rows, .. } => rows.iter().flatten().flatten().any(|term| {
                matches!(term, purrdf_core::TermValue::Literal { lexical_form, .. } if lexical_form == THIRD)
            }),
            _ => false,
        }
        };
        let in_outcome = |outcome: &GovernedOutcome| matches!(outcome, GovernedOutcome::Complete { result, .. } if in_result(result));
        let in_dataset = |dataset: &Arc<RdfDataset>| {
            let ask = format!("ASK {{ ?s ?p {THIRD} }}");
            matches!(
                engine.query_with_options_view(
                    &**dataset,
                    purrdf_core::SparqlRequest {
                        query: &ask,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                ),
                Ok(SparqlResult::Boolean(true))
            )
        };
        match outcome {
            JobOutcome::Query(result, _) => in_result(result),
            JobOutcome::Raw(text) | JobOutcome::QueryRaw { body: text, .. } => text.contains(THIRD),
            JobOutcome::Governed(governed) => in_outcome(governed),
            JobOutcome::Entailment(entailment) => matches!(
                entailment.as_ref(),
                GovernedEntailment::Answered { outcome, .. } if in_outcome(outcome)
            ),
            JobOutcome::Negotiated(value) => matches!(
                value.as_ref(),
                NegotiatedValue::Complete { bytes, .. } if String::from_utf8_lossy(bytes).contains(THIRD)
            ),
            JobOutcome::Updated(dataset, _)
            | JobOutcome::UpdateGoverned {
                frozen: Some(dataset),
                ..
            } => in_dataset(dataset),
            _ => false,
        }
    }

    /// The division policy reaches every SPARQL operation kind. Under `exact` the
    /// non-terminating `1/3` is an expression error on each one — the job succeeds, and
    /// its answer leaves `?x` unbound (an UPDATE inserts nothing for it) — while the
    /// terminating neighbour `1/8` answers on each; under the default the same `1/3`
    /// answers its rounded eighteen digits on each, so the difference is the policy's
    /// and not the operation's.
    #[test]
    fn the_division_policy_reaches_every_operation_kind() {
        let engine = Rc::new(NativeSparqlEngine::new());
        let frozen = crate::dataset::Dataset::parse(
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
            "ntriples",
            None,
            None,
        )
        .expect("parses")
        .view()
        .freeze()
        .expect("freezes");
        let queries = [
            AsyncOperationKind::Query,
            AsyncOperationKind::Raw,
            AsyncOperationKind::Governed,
            AsyncOperationKind::Negotiated,
            AsyncOperationKind::EntailmentGoverned,
        ];
        let updates = [
            AsyncOperationKind::Update,
            AsyncOperationKind::UpdateGoverned,
        ];
        let text = |kind: AsyncOperationKind, quotient: &str| {
            if updates.contains(&kind) {
                format!(
                    "INSERT {{ <http://example.org/s> <http://example.org/q> ?x }} \
                     WHERE {{ BIND({quotient} AS ?x) }}"
                )
            } else {
                format!("SELECT ?x WHERE {{ BIND({quotient} AS ?x) }}")
            }
        };
        let run = JobRun::offline(None);
        for kind in queries.into_iter().chain(updates) {
            let third = text(kind, "1/3");
            let unbound = dividing(kind, &engine, &frozen, &third, DivisionPolicy::Exact)
                .execute(&run)
                .unwrap_or_else(|error| {
                    panic!("{kind:?}: exact leaves 1/3 unbound: {}", error.rendered())
                });
            assert!(
                !carries_third(&engine, &unbound),
                "{kind:?}: exact binds no 1/3"
            );
            let terminating = text(kind, "1/8");
            assert!(
                dividing(kind, &engine, &frozen, &terminating, DivisionPolicy::Exact)
                    .execute(&run)
                    .is_ok(),
                "{kind:?}: exact answers 1/8"
            );
            let rounded = dividing(
                kind,
                &engine,
                &frozen,
                &third,
                DivisionPolicy::xsd_default(),
            )
            .execute(&run)
            .unwrap_or_else(|error| panic!("{kind:?}: {}", error.rendered()));
            assert!(
                carries_third(&engine, &rounded),
                "{kind:?}: the default rounds 1/3 to {THIRD}"
            );
        }
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
