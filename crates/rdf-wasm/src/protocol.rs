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

use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::protocol::{
    OperationKind, ProtocolError, ProtocolRequest, ResultKind, format_media_type, negotiate,
    offered_media_types,
};
use wasm_bindgen::prelude::*;

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

/// Flatten `(name, value)` pairs into `[name, value, name, value, …]`.
fn flatten_pairs(pairs: &[(String, String)]) -> Vec<String> {
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

    #[test]
    fn extra_parameters_flatten_in_request_order() {
        let with = request("GET", None, Some("query=ASK%20%7B%7D&b=2&a=1"), "");
        assert_eq!(flatten_pairs(with.extra_parameters()), ["b", "2", "a", "1"]);
    }
}
