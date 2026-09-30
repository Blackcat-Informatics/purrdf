// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SPARQL 1.1 Protocol request surface for JavaScript: [`SparqlProtocolRequest`], a
//! thin class over [`purrdf_sparql_eval::protocol`].
//!
//! A host (the package's `./cloudflare` adapter, or any other HTTP front end) hands it
//! the request's method, `Content-Type`, query string and body bytes and gets back the
//! operation, its effective text (the protocol's dataset parameters applied) and the
//! negotiated response format. Every protocol decision is made here, in Rust; the host
//! only moves bytes and maps the typed outcome onto HTTP.
//!
//! # Errors a host can map without reading prose
//!
//! A refused request is thrown as a JavaScript `Error` whose `name` is the stable
//! [`ProtocolError::name`] (e.g. `"MissingOperation"`), whose `status` is the HTTP status
//! the refusal maps to (`400`, or the more specific `405` for a method the protocol does
//! not bind and `415` for a `Content-Type` it does not define), and which carries
//! `parameter` — the protocol parameter at fault — when there is one. The message is the
//! error's own rendering. A host builds its problem document from those three fields.
//!
//! # A failed operation's problem
//!
//! Once a request has been read, every way its operation can fail reaches the host as an
//! error carrying a `code` ([`crate::operation`]). [`SparqlProtocolRequest::problem_for`]
//! answers it with the RFC 9457 problem [`problem_for`] maps its failure to: the status,
//! the problem's `code`, and a `detail` that is the failure's own words only where the
//! client is owed them. A failure that is the host's own is sanitized: its `detail` names
//! a correlation id the host logs the real error under, and nothing else.

use std::fmt::Write as _;

use purrdf_core::{ResourceDimension, StopCause, TrippedGovernor as TrippedValue};
use purrdf_lex::json_escape::{JsonEscapes, push_string};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::protocol::{
    FailureCode, OperationKind, ProblemDetail, ProtocolError, ProtocolRequest, ResultKind,
    format_media_type, negotiate, offered_media_types, problem_for,
};
use wasm_bindgen::prelude::*;

use crate::operation::{OPTIONS_CODE, USAGE_CODE};
use crate::query::aggregate_env_message;

#[wasm_bindgen]
extern "C" {
    /// The host's `Error` constructor: a protocol refusal is a real `Error` (with a stack
    /// trace, and `instanceof Error`), carrying its typed fields as properties.
    #[wasm_bindgen(js_name = Error)]
    type ProtocolJsError;

    #[wasm_bindgen(constructor, js_class = "Error")]
    fn new(message: &str) -> ProtocolJsError;

    #[wasm_bindgen(method, setter = name)]
    fn set_name(this: &ProtocolJsError, name: &str);

    #[wasm_bindgen(method, setter = status)]
    fn set_status(this: &ProtocolJsError, status: u16);

    #[wasm_bindgen(method, setter = parameter)]
    fn set_parameter(this: &ProtocolJsError, parameter: &str);

    /// A governed outcome's `tripped` record, read for the governor it describes.
    type TripRecord;

    #[wasm_bindgen(method, getter, structural)]
    fn kind(this: &TripRecord) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn label(this: &TripRecord) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn dimension(this: &TripRecord) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn limit(this: &TripRecord) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn consumed(this: &TripRecord) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn estimate(this: &TripRecord) -> JsValue;

    /// Whatever a failed operation rejected with, read for its `code` and `message`.
    type Failure;

    #[wasm_bindgen(method, getter, structural)]
    fn code(this: &Failure) -> JsValue;

    #[wasm_bindgen(method, getter, structural)]
    fn message(this: &Failure) -> JsValue;
}

/// A `u64` member of a trip record: a `bigint`, or a safe-integer `number`.
fn trip_count(value: &JsValue) -> Option<u64> {
    if value.is_bigint() {
        return u64::try_from(value.clone()).ok();
    }
    value
        .as_f64()
        .filter(|number| number.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(number))
        .map(|number| number as u64)
}

/// The governor a trip record describes: `kind` (`"budget"`, `"refused"`, `"stopped"`)
/// with the `label`, `dimension`, `limit`, `consumed` and `estimate` its kind carries.
///
/// # Errors
///
/// A record that describes no governor this build names.
fn trip_from_record(
    kind: Option<&str>,
    label: Option<&str>,
    dimension: Option<&str>,
    limit: Option<u64>,
    consumed: Option<u64>,
    estimate: Option<u64>,
) -> Result<TrippedValue, String> {
    let dimension = || {
        dimension
            .and_then(|name| {
                ResourceDimension::ALL
                    .into_iter()
                    .find(|candidate| candidate.label() == name)
            })
            .ok_or_else(|| format!("a {kind:?} trip names no governed dimension"))
    };
    let count = |value: Option<u64>, name: &str| {
        value.ok_or_else(|| format!("a {kind:?} trip carries no {name}"))
    };
    match kind {
        Some("budget") => Ok(TrippedValue::Budget {
            dimension: dimension()?,
            limit: count(limit, "limit")?,
            consumed: count(consumed, "consumed")?,
        }),
        Some("refused") => Ok(TrippedValue::Refused {
            dimension: dimension()?,
            limit: count(limit, "limit")?,
            estimate: count(estimate, "estimate")?,
        }),
        Some("stopped") => {
            let cause = match label {
                Some("cancelled") => StopCause::Cancelled,
                Some("deadline-exceeded") => StopCause::Deadline,
                other => {
                    return Err(format!(
                        "a stopped trip labelled {other:?} names no stop cause"
                    ));
                }
            };
            Ok(TrippedValue::Stopped { cause })
        }
        other => Err(format!(
            "a trip of kind {other:?} names no governor this build knows"
        )),
    }
}

/// The media type of every problem document.
const PROBLEM_JSON: &str = "application/problem+json";

/// The reason phrase of `status` (RFC 9110 §15), the `title` of an `about:blank` problem
/// (RFC 9457 §4.2.1); `None` for a status no problem here is answered with.
const fn status_title(status: u16) -> Option<&'static str> {
    Some(match status {
        400 => "Bad Request",
        403 => "Forbidden",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        409 => "Conflict",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => return None,
    })
}

/// A JSON string literal for `text`.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    push_string(&mut out, text, JsonEscapes::ShortForms);
    out
}

/// The HTTP problem a failed operation is answered with: build it with
/// [`SparqlProtocolRequest::problem_for`], read its `status` and whether it is
/// `internal`, and render its `body`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct FailureProblem {
    failure: FailureCode,
    /// The failure's own code, the problem's `code` for an evaluation failure.
    failure_code: String,
    message: String,
    /// The governor that stopped the operation, when the problem answers a trip: its
    /// label is the problem's `code`, and its dimension, ceiling and measurements are
    /// the problem's members.
    trip: Option<TrippedValue>,
}

impl FailureProblem {
    /// The problem for a failure reported under `code` (none: an exception nothing
    /// classified) in `message`'s words; `cancelled` when the request's own signal
    /// stopped the operation, whatever it rejected with.
    fn new(code: Option<&str>, message: &str, cancelled: bool) -> Self {
        let failure = match code {
            _ if cancelled => FailureCode::Cancelled,
            // An operation the host began with options it refuses, or a job the host drove
            // out of turn, is the host's own fault: nothing the client sent caused it.
            Some(OPTIONS_CODE | USAGE_CODE) | None => FailureCode::HostFault,
            Some(code) => FailureCode::from_diagnostic_code(code),
        };
        Self {
            failure,
            failure_code: code
                .unwrap_or_else(|| FailureCode::HostFault.code())
                .to_owned(),
            message: message.to_owned(),
            trip: None,
        }
    }

    /// The problem for an operation a governor stopped: a ceiling's `422`, or the `503`
    /// of a cancellation or a deadline.
    fn for_trip(tripped: TrippedValue) -> Self {
        let failure = FailureCode::from(&tripped);
        Self {
            failure,
            failure_code: tripped.label().to_owned(),
            message: tripped.to_string(),
            trip: Some(tripped),
        }
    }

    /// The problem document, with `correlation_id` for a failure that is the host's own.
    fn render(&self, correlation_id: Option<&str>) -> Result<String, String> {
        let problem = problem_for(self.failure);
        let code = match (self.failure, self.trip) {
            (_, Some(_)) | (FailureCode::Evaluation, None) => self.failure_code.as_str(),
            (_, None) => problem.code,
        };
        let (detail, correlation) = match (problem.detail, correlation_id) {
            (ProblemDetail::Message, _) => (self.message.clone(), None),
            (ProblemDetail::Fixed(detail), _) => (detail.to_owned(), None),
            (ProblemDetail::Internal { .. }, None) => {
                return Err(format!(
                    "a {} problem is the host's own fault and needs the correlation id its \
                     error was logged under",
                    problem.status
                ));
            }
            (
                ProblemDetail::Internal {
                    detail,
                    names_correlation,
                },
                Some(id),
            ) => (
                if names_correlation {
                    format!("{detail} {id}")
                } else {
                    detail.to_owned()
                },
                Some(id),
            ),
        };
        let mut body = format!(
            "{{\"type\":\"about:blank\",\"title\":{},\"status\":{},\"detail\":{},\"code\":{}",
            json_string(status_title(problem.status).unwrap_or("Error")),
            problem.status,
            json_string(&detail),
            json_string(code)
        );
        if let Some(id) = correlation {
            // Writing to a `String` cannot fail.
            let _ = write!(body, ",\"correlationId\":{}", json_string(id));
        }
        match self.trip {
            Some(TrippedValue::Stopped { .. }) => {
                let _ = write!(body, ",\"cause\":{}", json_string(code));
            }
            Some(TrippedValue::Budget {
                dimension,
                limit,
                consumed,
            }) => {
                let _ = write!(
                    body,
                    ",\"dimension\":{},\"limit\":{limit},\"consumed\":{consumed}",
                    json_string(dimension.label())
                );
            }
            Some(TrippedValue::Refused {
                dimension,
                limit,
                estimate,
            }) => {
                let _ = write!(
                    body,
                    ",\"dimension\":{},\"limit\":{limit},\"estimate\":{estimate}",
                    json_string(dimension.label())
                );
            }
            _ => {}
        }
        if self.failure == FailureCode::NotAcceptable {
            // A negotiated operation refuses only the one shape no format can carry
            // unasked: a graph with named graphs.
            let offered: Vec<String> = offered_media_types(ResultKind::Dataset)
                .map(json_string)
                .collect();
            let _ = write!(body, ",\"offered\":[{}]", offered.join(","));
        }
        body.push('}');
        Ok(body)
    }
}

#[wasm_bindgen]
impl FailureProblem {
    /// The response status.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn status(&self) -> u16 {
        problem_for(self.failure).status
    }

    /// The response's `Content-Type`: `application/problem+json`.
    #[wasm_bindgen(getter, js_name = contentType)]
    #[must_use]
    pub fn content_type(&self) -> String {
        PROBLEM_JSON.to_owned()
    }

    /// Whether the failure is the host's own: its real error belongs in the host's log
    /// under a correlation id, which `body` must be given and the response carries.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn internal(&self) -> bool {
        matches!(
            problem_for(self.failure).detail,
            ProblemDetail::Internal { .. }
        )
    }

    /// The RFC 9457 problem document: `type`, `title`, `status`, `detail` and `code`,
    /// plus `correlationId` for an internal failure and `offered` for a `406`.
    ///
    /// # Errors
    ///
    /// An internal failure without a `correlationId`.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn body(&self, correlation_id: Option<String>) -> Result<String, JsError> {
        self.render(correlation_id.as_deref())
            .map_err(|message| JsError::new(&message))
    }
}

/// The HTTP status a protocol refusal maps to: `405 Method Not Allowed` for a method the
/// protocol does not bind, `415 Unsupported Media Type` for a `POST` body type it does not
/// define, and `400 Bad Request` for every other malformed request (Protocol §2.1.1,
/// §2.2.1).
const fn refusal_status(error: &ProtocolError) -> u16 {
    match error {
        ProtocolError::UnsupportedMethod { .. } => 405,
        ProtocolError::UnsupportedContentType { .. } => 415,
        _ => 400,
    }
}

/// Throwable form of a [`ProtocolError`]: an `Error` with `name`, `status` and, when the
/// error names one, `parameter`. Built only on wasm, where the host constructor exists.
fn protocol_error(error: &ProtocolError) -> JsValue {
    let thrown = ProtocolJsError::new(&error.to_string());
    thrown.set_name(error.name());
    thrown.set_status(refusal_status(error));
    if let Some(parameter) = error.parameter() {
        thrown.set_parameter(parameter);
    }
    thrown.into()
}

/// The protocol's name for a result kind, as the JavaScript surface spells it.
const fn result_kind_name(kind: ResultKind) -> &'static str {
    match kind {
        ResultKind::Solutions => "solutions",
        ResultKind::Boolean => "boolean",
        ResultKind::Graph => "graph",
        ResultKind::Dataset => "dataset",
    }
}

/// [`result_kind_name`]'s inverse, refusing any other spelling.
fn parse_result_kind(name: &str) -> Result<ResultKind, String> {
    match name {
        "solutions" => Ok(ResultKind::Solutions),
        "boolean" => Ok(ResultKind::Boolean),
        "graph" => Ok(ResultKind::Graph),
        "dataset" => Ok(ResultKind::Dataset),
        other => Err(format!(
            "unknown result kind {other:?} (expected solutions, boolean, graph or dataset)"
        )),
    }
}

/// Why a result of `kind` cannot be sent: no format the client accepts can carry it. The
/// one wording both the pre-evaluation check and a negotiated job's refusal use.
pub(crate) fn not_acceptable_message(kind: ResultKind) -> String {
    let what = match kind {
        ResultKind::Solutions => "a SPARQL results document",
        ResultKind::Boolean => "a boolean SPARQL results document",
        ResultKind::Graph => "an RDF graph",
        ResultKind::Dataset => "an RDF graph carrying named graphs",
    };
    let offered: Vec<&str> = offered_media_types(kind).collect();
    format!(
        "the result is {what}, and the request's Accept header allows none of the formats \
         that can carry it: {}",
        offered.join(", ")
    )
}

/// The effective operation text, parsed once under the engine's own reading.
///
/// With dataset parameters, the splice itself parses the text. Without them the text is
/// parsed here as well, so a malformed operation is the protocol's `400` rather than an
/// evaluation failure: the engine reads the same text under the same parser options
/// (an environment with no registered aggregates, no base IRI) and would refuse it
/// identically.
fn effective_text(request: &ProtocolRequest) -> Result<String, ProtocolError> {
    let env = aggregate_env_message(None).map_err(|message| ProtocolError::MalformedOperation {
        kind: request.kind(),
        message,
    })?;
    let options = env.parser_options();
    let parser = SparqlParser::new();
    let text = request.effective_text_with(&parser, options)?;
    if !request.has_dataset_parameters() {
        let parsed = match request.kind() {
            OperationKind::Query => parser.parse_query_with(&text, options).map(drop),
            OperationKind::Update => parser.parse_update_with(&text, options).map(drop),
        };
        parsed.map_err(|err| ProtocolError::MalformedOperation {
            kind: request.kind(),
            message: err.to_string(),
        })?;
    }
    Ok(text)
}

/// Flatten `(name, value)` pairs into `[name, value, name, value, …]`, the shape a
/// JavaScript host reads a header or parameter list in. Every pair list the module
/// hands to JavaScript (protocol parameters, effect headers) is flattened here.
pub(crate) fn flatten_pairs(pairs: &[(String, String)]) -> Vec<String> {
    pairs
        .iter()
        .flat_map(|(name, value)| [name.clone(), value.clone()])
        .collect()
}

/// One SPARQL 1.1 Protocol operation read from an HTTP request.
///
/// `SparqlProtocolRequest.parse(method, contentType, queryString, body)` reads `GET
/// ?query=`, `POST application/sparql-query`, `POST application/sparql-update` and `POST
/// application/x-www-form-urlencoded` (`query=` / `update=`), with the dataset parameters
/// `default-graph-uri` / `named-graph-uri` (queries) and `using-graph-uri` /
/// `using-named-graph-uri` (updates). A malformed request throws the typed `Error` the
/// module documentation describes.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct SparqlProtocolRequest {
    inner: ProtocolRequest,
}

#[wasm_bindgen]
impl SparqlProtocolRequest {
    /// Read one protocol operation from an HTTP request's method, `Content-Type` header,
    /// query string (without the leading `?`) and body bytes.
    ///
    /// # Errors
    ///
    /// The typed protocol refusal: `name` is the `ProtocolError` variant, `status` the
    /// HTTP status, `parameter` the parameter at fault when there is one.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn parse(
        method: &str,
        content_type: Option<String>,
        query_string: Option<String>,
        body: &[u8],
    ) -> Result<Self, JsValue> {
        ProtocolRequest::parse(
            method,
            content_type.as_deref(),
            query_string.as_deref(),
            body,
        )
        .map(|inner| Self { inner })
        .map_err(|error| protocol_error(&error))
    }

    /// `"query"` or `"update"`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn kind(&self) -> String {
        self.inner.kind().as_str().to_owned()
    }

    /// The operation text as the request carried it, before any dataset parameter is
    /// applied.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn text(&self) -> String {
        self.inner.text().to_owned()
    }

    /// The operation text to evaluate: the request's dataset parameters applied (the
    /// query's own `FROM`/`FROM NAMED` replaced; `USING`/`USING NAMED` written into each
    /// update operation with a `WHERE`), and the whole text parsed under the engine's own
    /// reading, so a malformed operation is refused here as the client error it is.
    ///
    /// # Errors
    ///
    /// The typed protocol refusal (`MalformedOperation`, `UsingConflictsWithClause`).
    #[wasm_bindgen(js_name = effectiveText)]
    pub fn effective_text(&self) -> Result<String, JsValue> {
        effective_text(&self.inner).map_err(|error| protocol_error(&error))
    }

    /// The query's result kind — `"solutions"` (`SELECT`), `"boolean"` (`ASK`) or `"graph"`
    /// (`CONSTRUCT`/`DESCRIBE`) — read from its query form, or `undefined` for an update.
    ///
    /// # Errors
    ///
    /// The typed protocol refusal `MalformedOperation` when no query form follows the
    /// prologue.
    #[wasm_bindgen(getter, js_name = resultKind)]
    pub fn result_kind(&self) -> Result<Option<String>, JsValue> {
        self.inner
            .result_kind()
            .map(|kind| kind.map(|kind| result_kind_name(kind).to_owned()))
            .map_err(|error| protocol_error(&error))
    }

    /// Whether the request carries dataset parameters.
    #[wasm_bindgen(getter, js_name = hasDatasetParameters)]
    #[must_use]
    pub fn has_dataset_parameters(&self) -> bool {
        self.inner.has_dataset_parameters()
    }

    /// Every `default-graph-uri`, in request order.
    #[wasm_bindgen(getter, js_name = defaultGraphUris)]
    #[must_use]
    pub fn default_graph_uris(&self) -> Vec<String> {
        self.inner.default_graph_uris().to_vec()
    }

    /// Every `named-graph-uri`, in request order.
    #[wasm_bindgen(getter, js_name = namedGraphUris)]
    #[must_use]
    pub fn named_graph_uris(&self) -> Vec<String> {
        self.inner.named_graph_uris().to_vec()
    }

    /// Every `using-graph-uri`, in request order.
    #[wasm_bindgen(getter, js_name = usingGraphUris)]
    #[must_use]
    pub fn using_graph_uris(&self) -> Vec<String> {
        self.inner.using_graph_uris().to_vec()
    }

    /// Every `using-named-graph-uri`, in request order.
    #[wasm_bindgen(getter, js_name = usingNamedGraphUris)]
    #[must_use]
    pub fn using_named_graph_uris(&self) -> Vec<String> {
        self.inner.using_named_graph_uris().to_vec()
    }

    /// Every parameter the protocol does not define, as flattened `[name, value, name,
    /// value, …]` pairs in request order (query string first, then a form body).
    #[wasm_bindgen(getter, js_name = extraParameters)]
    #[must_use]
    pub fn extra_parameters(&self) -> Vec<String> {
        flatten_pairs(self.inner.extra_parameters())
    }

    /// The format token to answer this query in, negotiated from an `Accept` header
    /// against the query's result kind (RFC 9110 §12.5.1; absent or empty means the
    /// protocol's default), or `undefined` when no offered format is acceptable — the
    /// host's `406`.
    ///
    /// # Errors
    ///
    /// `MalformedOperation` as [`Self::result_kind`]; and an update, which has no result
    /// to negotiate.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn negotiate(&self, accept: Option<String>) -> Result<Option<String>, JsValue> {
        match self.inner.result_kind() {
            Err(error) => Err(protocol_error(&error)),
            Ok(None) => Err(JsError::new("an update has no result format to negotiate").into()),
            Ok(Some(kind)) => Ok(negotiate(accept.as_deref(), kind).map(str::to_owned)),
        }
    }

    /// The media type of a format token `negotiate` returns, or `undefined` for any other
    /// string.
    #[wasm_bindgen(js_name = formatMediaType)]
    #[must_use]
    pub fn format_media_type(token: &str) -> Option<String> {
        format_media_type(token).map(str::to_owned)
    }

    /// The media types a result of `kind` (`"solutions"`, `"boolean"`, `"graph"`, or
    /// `"dataset"` — a graph carrying named graphs) is offered in, in server preference
    /// order.
    ///
    /// # Errors
    ///
    /// Any other `kind`.
    #[wasm_bindgen(js_name = offeredMediaTypes)]
    pub fn offered_media_types(kind: &str) -> Result<Vec<String>, JsError> {
        let kind = parse_result_kind(kind).map_err(|message| JsError::new(&message))?;
        Ok(offered_media_types(kind).map(str::to_owned).collect())
    }

    /// The HTTP problem a failed operation is answered with: `error` is what it rejected
    /// with, read for its `code` and `message`; `cancelled` says the request's own signal
    /// stopped it, whatever it rejected with. An error without a string `code` is an
    /// exception nothing classified — the host's own fault.
    #[wasm_bindgen(js_name = problemFor)]
    #[must_use]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn problem_for(error: JsValue, cancelled: bool) -> FailureProblem {
        let (code, message) = if error.is_object() {
            let failure = error.unchecked_ref::<Failure>();
            (failure.code().as_string(), failure.message().as_string())
        } else {
            (None, error.as_string())
        };
        FailureProblem::new(code.as_deref(), &message.unwrap_or_default(), cancelled)
    }

    /// The HTTP problem a governed operation a governor stopped is answered with:
    /// `tripped` is its outcome's `tripped` record. A ceiling reached or refused at
    /// admission is a `422` carrying its `dimension`, `limit` and `consumed` or
    /// `estimate`; a stop signal is the `503` of its `cause`. The problem's `code` is the
    /// governor's label.
    ///
    /// # Errors
    ///
    /// A record that describes no governor this build names.
    #[wasm_bindgen(js_name = problemForTrip)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn problem_for_trip(tripped: JsValue) -> Result<FailureProblem, JsError> {
        if !tripped.is_object() {
            return Err(JsError::new(
                "problemForTrip expects a tripped-governor record",
            ));
        }
        let record = tripped.unchecked_ref::<TripRecord>();
        trip_from_record(
            record.kind().as_string().as_deref(),
            record.label().as_string().as_deref(),
            record.dimension().as_string().as_deref(),
            trip_count(&record.limit()),
            trip_count(&record.consumed()),
            trip_count(&record.estimate()),
        )
        .map(FailureProblem::for_trip)
        .map_err(|message| JsError::new(&message))
    }

    /// The reason phrase of an HTTP `status` (RFC 9110 §15) — an `about:blank`
    /// problem's `title` — or `undefined` for a status no problem is answered with here.
    #[wasm_bindgen(js_name = statusTitle)]
    #[must_use]
    pub fn status_title(status: u16) -> Option<String> {
        status_title(status).map(str::to_owned)
    }

    /// Why a result of `kind` cannot be sent to a client whose `Accept` header allows none
    /// of the formats that carry it — the `detail` of a `406`, naming those formats.
    ///
    /// # Errors
    ///
    /// Any `kind` other than `"solutions"`, `"boolean"`, `"graph"` and `"dataset"`.
    #[wasm_bindgen(js_name = notAcceptableDetail)]
    pub fn not_acceptable_detail(kind: &str) -> Result<String, JsError> {
        let kind = parse_result_kind(kind).map_err(|message| JsError::new(&message))?;
        Ok(not_acceptable_message(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::json;

    fn request(
        method: &str,
        content_type: Option<&str>,
        query: Option<&str>,
        body: &str,
    ) -> ProtocolRequest {
        ProtocolRequest::parse(method, content_type, query, body.as_bytes())
            .expect("a protocol request")
    }

    #[test]
    fn refusal_statuses_are_specific_where_http_has_a_status_and_400_elsewhere() {
        let method = ProtocolRequest::parse("PUT", None, None, b"").unwrap_err();
        assert_eq!(refusal_status(&method), 405);
        let media = ProtocolRequest::parse("POST", Some("text/plain"), None, b"x").unwrap_err();
        assert_eq!(refusal_status(&media), 415);
        let missing = ProtocolRequest::parse("GET", None, None, b"").unwrap_err();
        assert_eq!(missing.name(), "MissingOperation");
        assert_eq!(refusal_status(&missing), 400);
        // The valid neighbours of the refused method and media type parse.
        assert!(
            ProtocolRequest::parse("POST", Some("application/sparql-query"), None, b"ASK {}")
                .is_ok()
        );
        assert!(ProtocolRequest::parse("GET", None, Some("query=ASK%20%7B%7D"), b"").is_ok());
    }

    #[test]
    fn effective_text_refuses_a_malformed_query_and_passes_a_well_formed_one_untouched() {
        let bad = request(
            "POST",
            Some("application/sparql-query"),
            None,
            "SELECT WHERE {",
        );
        let error = effective_text(&bad).unwrap_err();
        assert_eq!(error.name(), "MalformedOperation");
        assert_eq!(refusal_status(&error), 400);
        let good = request(
            "POST",
            Some("application/sparql-query"),
            None,
            "SELECT * WHERE { ?s ?p ?o }",
        );
        assert_eq!(
            effective_text(&good).unwrap(),
            "SELECT * WHERE { ?s ?p ?o }"
        );
    }

    #[test]
    fn effective_text_refuses_a_malformed_update_and_passes_a_well_formed_one() {
        let bad = request(
            "POST",
            Some("application/sparql-update"),
            None,
            "INSERT DATA {",
        );
        assert_eq!(
            effective_text(&bad).unwrap_err().name(),
            "MalformedOperation"
        );
        let good = request(
            "POST",
            Some("application/sparql-update"),
            None,
            "INSERT DATA { <http://example.org/s> <http://example.org/p> 1 }",
        );
        assert!(effective_text(&good).is_ok());
    }

    #[test]
    fn effective_text_applies_dataset_parameters() {
        let with = request(
            "GET",
            None,
            Some(
                "query=SELECT%20*%20WHERE%20%7B%20%3Fs%20%3Fp%20%3Fo%20%7D&default-graph-uri=http%3A%2F%2Fexample.org%2Fg",
            ),
            "",
        );
        let text = effective_text(&with).unwrap();
        assert!(text.contains("FROM <http://example.org/g>"), "{text}");
    }

    #[test]
    fn result_kind_names_round_trip_and_unknown_names_are_refused() {
        for kind in [
            ResultKind::Solutions,
            ResultKind::Boolean,
            ResultKind::Graph,
            ResultKind::Dataset,
        ] {
            assert_eq!(parse_result_kind(result_kind_name(kind)).unwrap(), kind);
        }
        assert!(parse_result_kind("graphs").is_err());
    }

    #[test]
    fn a_graph_carrying_named_graphs_negotiates_only_among_quad_syntaxes() {
        // Accept absent: the dataset default is TriG, where a plain graph's is Turtle.
        assert_eq!(negotiate(None, ResultKind::Dataset), Some("trig"));
        assert_eq!(negotiate(None, ResultKind::Graph), Some("turtle"));
        // An explicit Turtle-only client cannot be sent a dataset; the plain graph can.
        assert_eq!(negotiate(Some("text/turtle"), ResultKind::Dataset), None);
        assert_eq!(
            negotiate(Some("text/turtle"), ResultKind::Graph),
            Some("turtle")
        );
        // A client preferring Turtle but accepting N-Quads gets N-Quads for a dataset.
        assert_eq!(
            negotiate(
                Some("text/turtle, application/n-quads;q=0.5"),
                ResultKind::Dataset
            ),
            Some("nquads")
        );
        assert_eq!(negotiate(Some("*/*"), ResultKind::Dataset), Some("trig"));
        assert_eq!(
            offered_media_types(ResultKind::Dataset).collect::<Vec<_>>(),
            [
                "application/trig",
                "application/n-quads",
                "application/ld+json"
            ]
        );
    }

    #[test]
    fn an_ask_is_never_offered_csv_or_tsv_while_a_select_is() {
        let ask = request("GET", None, Some("query=ASK%20%7B%7D"), "");
        assert_eq!(ask.result_kind().unwrap(), Some(ResultKind::Boolean));
        assert_eq!(negotiate(Some("text/csv"), ResultKind::Boolean), None);
        assert_eq!(
            negotiate(Some("text/tab-separated-values"), ResultKind::Boolean),
            None
        );
        assert_eq!(
            negotiate(Some("text/csv"), ResultKind::Solutions),
            Some("csv")
        );
        assert_eq!(
            negotiate(
                Some("text/csv, application/sparql-results+xml;q=0.1"),
                ResultKind::Boolean
            ),
            Some("xml")
        );
        assert_eq!(negotiate(None, ResultKind::Boolean), Some("json"));
    }

    #[test]
    fn the_not_acceptable_detail_names_the_carriable_formats() {
        let message = not_acceptable_message(ResultKind::Dataset);
        assert!(message.contains("named graphs"), "{message}");
        assert!(message.contains("application/trig, application/n-quads, application/ld+json"));
        assert!(!message.contains("text/turtle"));
    }

    fn body(problem: &FailureProblem, id: Option<&str>) -> json::Value {
        json::read(&problem.render(id).expect("renders")).expect("valid JSON")
    }

    /// A parse refusal is the client's `400` in its own words; an evaluation failure is a
    /// `500` in its own words under its own code; the two differ only by their codes.
    #[test]
    fn a_parse_refusal_and_an_evaluation_failure_are_told_apart_by_code() {
        let parse = FailureProblem::new(
            Some("native-sparql-query-parse"),
            "error native-sparql-query-parse: SPARQL syntax error at byte 7",
            false,
        );
        assert_eq!(parse.status(), 400);
        assert!(!parse.internal());
        let document = body(&parse, None);
        assert_eq!(document["title"], "Bad Request");
        assert_eq!(document["code"], "native-sparql-query-parse");
        assert_eq!(
            document["detail"],
            "error native-sparql-query-parse: SPARQL syntax error at byte 7"
        );
        let evaluation = FailureProblem::new(
            Some("native-sparql-query-eval"),
            "error native-sparql-query-eval: a cyclic list",
            false,
        );
        assert_eq!(evaluation.status(), 500);
        assert!(!evaluation.internal());
        assert_eq!(body(&evaluation, None)["code"], "native-sparql-query-eval");
    }

    /// A host fault — an exception with no code, or a job's latched fault — never shows
    /// its words, and cannot be rendered without the correlation id it was logged under.
    /// A catalog denial is a `403` whose detail is fixed, never the policy's words.
    #[test]
    fn host_faults_and_denials_never_disclose_their_words() {
        for fault in [
            FailureProblem::new(None, "secret-token-abc", false),
            FailureProblem::new(Some("native-sparql-host-fault"), "secret-token-abc", false),
        ] {
            assert_eq!(fault.status(), 500);
            assert!(fault.internal());
            assert!(
                fault.render(None).is_err(),
                "an internal problem needs its id"
            );
            let document = body(&fault, Some("id-1"));
            assert_eq!(document["code"], "InternalError");
            assert_eq!(
                document["detail"],
                "internal error; see the Worker log for correlation id id-1"
            );
            assert_eq!(document["correlationId"], "id-1");
            assert!(!document.to_string().contains("secret-token-abc"));
        }
        let denied = FailureProblem::new(
            Some("native-sparql-load-denied"),
            "LOAD <http://example.org/doc>: denied: secret policy",
            false,
        );
        assert_eq!(denied.status(), 403);
        let document = body(&denied, None);
        assert!(!document.to_string().contains("secret policy"));
        let host_denied = FailureProblem::new(
            Some("native-sparql-load-host-denied"),
            "LOAD <http://example.org/doc>: the host denied the request: secret",
            false,
        );
        assert_eq!(host_denied.status(), 403);
        assert_ne!(
            body(&host_denied, None)["detail"],
            document["detail"],
            "the catalog's refusal and the host's read differently"
        );
    }

    /// A stopped request is a `503` whatever it rejected with; a conflicting update a
    /// `409`; a result no acceptable format carries a `406` naming the formats offered.
    #[test]
    fn stops_conflicts_and_unacceptable_results_have_their_statuses() {
        let aborted = FailureProblem::new(None, "AbortError: the client went away", true);
        assert_eq!(aborted.status(), 503);
        assert!(!aborted.internal());
        assert_eq!(body(&aborted, None)["code"], "cancelled");
        // The neighbour: the same exception with the request still live is a host fault.
        let unexplained = FailureProblem::new(None, "AbortError: the client went away", false);
        assert_eq!(unexplained.status(), 500);
        // Options the host began an operation with and the operation refused are the
        // host's fault too; a serialization the operation could not write is its own.
        let misconfigured =
            FailureProblem::new(Some(OPTIONS_CODE), "a catalog governs nothing", false);
        assert!(misconfigured.internal());
        let unserializable = FailureProblem::new(
            Some(crate::operation::SERIALIZE_CODE),
            "unsupported format",
            false,
        );
        assert!(!unserializable.internal());
        assert_eq!(
            body(&unserializable, None)["code"],
            crate::operation::SERIALIZE_CODE
        );
        let conflict = FailureProblem::new(
            Some("native-sparql-update-in-flight"),
            "an asynchronous update of dataset 3 is already in flight",
            false,
        );
        assert_eq!(conflict.status(), 409);
        assert_eq!(body(&conflict, None)["title"], "Conflict");
        let unacceptable = FailureProblem::new(
            Some("native-sparql-not-acceptable"),
            &not_acceptable_message(ResultKind::Dataset),
            false,
        );
        assert_eq!(unacceptable.status(), 406);
        let document = body(&unacceptable, None);
        assert_eq!(document["code"], "NotAcceptable");
        assert_eq!(
            document["offered"],
            json::Value::array([
                "application/trig",
                "application/n-quads",
                "application/ld+json"
            ])
        );
    }

    /// A ceiling a governor reached is a `422` whose code is the governor's label and
    /// whose members are its dimension, ceiling and consumption, as exact integers; a
    /// refusal at admission carries the estimate instead; a stop signal is a `503` whose
    /// code and cause are its label.
    #[test]
    fn a_trip_is_answered_by_its_governor() {
        let reached = trip_from_record(
            Some("budget"),
            Some("answer-cap-exhausted"),
            Some("answer-rows"),
            Some(u64::MAX - 1),
            Some(3),
            None,
        )
        .expect("a budget trip");
        let problem = FailureProblem::for_trip(reached);
        assert_eq!(problem.status(), 422);
        assert!(!problem.internal());
        let text = problem.render(None).expect("renders");
        assert!(
            text.contains("\"limit\":18446744073709551614"),
            "exact: {text}"
        );
        let document = body(&problem, None);
        assert_eq!(document["title"], "Unprocessable Content");
        assert_eq!(document["code"], "answer-cap-exhausted");
        assert_eq!(document["dimension"], "answer-rows");
        assert_eq!(document["consumed"], 3);
        assert_eq!(document["estimate"], json::Value::Null);

        let refused = trip_from_record(
            Some("refused"),
            Some("cardinality-exhausted"),
            Some("intermediate-cells"),
            Some(10),
            None,
            Some(20),
        )
        .expect("a refusal");
        let document = body(&FailureProblem::for_trip(refused), None);
        assert_eq!(document["status"], 422);
        assert_eq!(document["estimate"], 20);
        assert_eq!(document["consumed"], json::Value::Null);

        for label in ["deadline-exceeded", "cancelled"] {
            let stopped = trip_from_record(Some("stopped"), Some(label), None, None, None, None)
                .expect("a stop");
            let problem = FailureProblem::for_trip(stopped);
            assert_eq!(problem.status(), 503);
            let document = body(&problem, None);
            assert_eq!(document["code"], label);
            assert_eq!(document["cause"], label);
            assert_eq!(document["dimension"], json::Value::Null);
        }
        // A record that names no governor is refused, not guessed at.
        assert!(trip_from_record(Some("unknown"), Some("x"), None, None, None, None).is_err());
        assert!(
            trip_from_record(
                Some("budget"),
                None,
                Some("no-such"),
                Some(1),
                Some(1),
                None
            )
            .is_err()
        );
    }

    #[test]
    fn extra_parameters_flatten_in_request_order() {
        let with = request("GET", None, Some("query=ASK%20%7B%7D&b=2&a=1"), "");
        assert_eq!(flatten_pairs(with.extra_parameters()), ["b", "2", "a", "1"]);
    }
}
