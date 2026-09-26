// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **SPARQL 1.1 Protocol** request surface: reading an HTTP request into a query or
//! update operation, applying the protocol's dataset parameters to the operation's text,
//! and negotiating the response format.
//!
//! This module owns no I/O. A host hands it the request's method, `Content-Type`, query
//! string and body bytes, and gets back either a [`ProtocolRequest`] or a typed
//! [`ProtocolError`] naming the protocol rule the request broke — the host maps that to
//! its HTTP `400`. [`negotiate`] turns an `Accept` header into the result-format token
//! the engine's serializers take, or `None` for the host's `406`.
//!
//! # Request forms (SPARQL 1.1 Protocol §2.1, §2.2)
//!
//! | Operation | Method | `Content-Type` | Where the operation text is |
//! |---|---|---|---|
//! | query (§2.1.1) | `GET` | — | the `query` parameter of the query string |
//! | query (§2.1.2) | `POST` | `application/x-www-form-urlencoded` | the `query` parameter of the body |
//! | query (§2.1.3) | `POST` | `application/sparql-query` | the body |
//! | update (§2.2.1) | `POST` | `application/x-www-form-urlencoded` | the `update` parameter of the body |
//! | update (§2.2.2) | `POST` | `application/sparql-update` | the body |
//!
//! Dataset parameters: `default-graph-uri` / `named-graph-uri` for a query (§2.1.4),
//! `using-graph-uri` / `using-named-graph-uri` for an update (§2.2.3), each repeatable
//! and each an absolute IRI. Parameters are read from the query string for the direct
//! `POST` forms, and from the body — plus the query string, when a client also wrote
//! some there — for the URL-encoded form. A parameter name the protocol does not define
//! is kept, in order, in [`ProtocolRequest::extra_parameters`] rather than discarded.
//!
//! # Dataset parameters are applied as text
//!
//! [`ProtocolRequest::effective_text`] writes the parameters into the operation at the
//! positions the SPARQL parser reports: the query's own dataset clause is replaced (the
//! protocol's dataset overrides the query's, §2.1.4), and each update operation with a
//! `WHERE` gains `USING`/`USING NAMED` clauses (an operation that already has `USING`,
//! `USING NAMED` or `WITH` is the protocol's own error, §2.2.3). The caller's text is never
//! re-serialized: every byte outside the spliced clauses is the request's own.

use std::fmt;

use purrdf_sparql_algebra::lexer::{Token, tokenize};
use purrdf_sparql_algebra::{ParserOptions, SparqlParser, UpdateDatasetSlot};

/// The media type of a query sent directly in a `POST` body (Protocol §2.1.3).
const SPARQL_QUERY: &str = "application/sparql-query";
/// The media type of an update sent directly in a `POST` body (Protocol §2.2.2).
const SPARQL_UPDATE: &str = "application/sparql-update";
/// The media type of URL-encoded parameters in a `POST` body (Protocol §2.1.2, §2.2.1).
const FORM: &str = "application/x-www-form-urlencoded";

/// The query-operation dataset parameter naming a default-graph IRI (Protocol §2.1.4).
pub const DEFAULT_GRAPH_URI: &str = "default-graph-uri";
/// The query-operation dataset parameter naming a named-graph IRI (Protocol §2.1.4).
pub const NAMED_GRAPH_URI: &str = "named-graph-uri";
/// The update-operation dataset parameter naming a `USING` IRI (Protocol §2.2.3).
pub const USING_GRAPH_URI: &str = "using-graph-uri";
/// The update-operation dataset parameter naming a `USING NAMED` IRI (Protocol §2.2.3).
pub const USING_NAMED_GRAPH_URI: &str = "using-named-graph-uri";

/// Which protocol operation a request carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationKind {
    /// A query operation (Protocol §2.1).
    Query,
    /// An update operation (Protocol §2.2).
    Update,
}

impl OperationKind {
    /// `"query"` or `"update"` — the protocol's own parameter name for the operation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Update => "update",
        }
    }
}

/// The shapes a query result takes, which decide the formats it can be sent in
/// (Protocol §2.1.5 → the SPARQL 1.1 result formats and the RDF syntaxes).
///
/// [`ProtocolRequest::result_kind`] reads [`Self::Solutions`], [`Self::Boolean`] or
/// [`Self::Graph`] from the query form, before evaluation. [`Self::Dataset`] is known only once a graph result
/// exists: it is a `CONSTRUCT`/`DESCRIBE` result that carries a named graph, which only
/// the quad-capable syntaxes can hold. A host negotiates for [`Self::Graph`] before
/// evaluating (to answer `406` without spending the evaluation) and again for
/// [`Self::Dataset`] when the result turns out to carry named graphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResultKind {
    /// A `SELECT` solution sequence: a SPARQL results document.
    Solutions,
    /// An `ASK` boolean: a SPARQL results document in JSON or XML. The CSV and TSV result
    /// formats are defined for `SELECT` variable bindings only (SPARQL 1.1 Query Results
    /// CSV and TSV Formats §1), so they are never offered for a boolean.
    Boolean,
    /// A `CONSTRUCT` or `DESCRIBE` graph: an RDF document.
    Graph,
    /// A `CONSTRUCT` or `DESCRIBE` result carrying at least one named graph: an RDF
    /// document in a syntax that can carry named graphs.
    Dataset,
}

/// Why an HTTP request is not a SPARQL 1.1 Protocol operation, or why its dataset
/// parameters cannot be applied to it.
///
/// Every variant is a client error — the protocol's `400 Bad Request` (Protocol §2.1.1,
/// §2.2.1: "a malformed request") — except that a host may answer
/// [`ProtocolError::UnsupportedMethod`] with `405` and
/// [`ProtocolError::UnsupportedContentType`] with `415`, the more specific HTTP statuses.
/// [`ProtocolError::name`] is a stable identifier for each variant and
/// [`ProtocolError::parameter`] names the protocol parameter at fault, when one is.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtocolError {
    /// The method is neither `GET` nor `POST`, the only two the protocol binds operations
    /// to (Protocol §2.1, §2.2). Methods are case-sensitive (RFC 9110 §9.1).
    UnsupportedMethod {
        /// The method the request used.
        method: String,
    },
    /// A `POST` without a `Content-Type`: the three `POST` forms are told apart by it
    /// alone (Protocol §2.1.2, §2.1.3, §2.2.1, §2.2.2).
    MissingContentType,
    /// A `POST` whose media type is none of `application/x-www-form-urlencoded`,
    /// `application/sparql-query` and `application/sparql-update`.
    UnsupportedContentType {
        /// The media type the request declared, lower-cased, parameters removed.
        content_type: String,
    },
    /// A `charset` parameter other than UTF-8. The SPARQL media types are UTF-8 by
    /// definition, and URL-encoded parameters are percent-encoded UTF-8.
    UnsupportedCharset {
        /// The declared charset.
        charset: String,
    },
    /// A request body — a direct `POST` operation — that is not UTF-8.
    NonUtf8Body {
        /// The byte offset of the first invalid sequence.
        valid_up_to: usize,
    },
    /// A `GET` that carries a body. A query via `GET` is written entirely in the query
    /// string (Protocol §2.1.1); a body there has no protocol meaning, and ignoring it
    /// would silently drop what the client sent.
    BodyOnGet,
    /// URL-encoded parameters (a query string or a form body) that do not decode: a `%`
    /// not followed by two hex digits, or decoded bytes that are not UTF-8.
    MalformedForm {
        /// What was wrong, and where.
        reason: String,
    },
    /// No operation: no `query`/`update` parameter where the request form puts one
    /// (Protocol §2.1.1, §2.1.2, §2.2.1).
    MissingOperation,
    /// More than one operation: both a query and an update, or the same operation twice
    /// — the protocol conveys exactly one per request (Protocol §2.1, §2.2).
    AmbiguousOperation {
        /// Which operations the request carried.
        reason: String,
    },
    /// An `update` parameter in a `GET` query string: the update operation is bound to
    /// `POST` only (Protocol §2.2), so a `GET` never performs one.
    QueryViaGetIsNotUpdate,
    /// A query-dataset parameter (`default-graph-uri`, `named-graph-uri`) on an update
    /// request; an update's dataset is given by `using-graph-uri` /
    /// `using-named-graph-uri` (Protocol §2.2.3).
    DatasetParametersOnUpdate {
        /// The offending parameter.
        parameter: &'static str,
    },
    /// An update-dataset parameter (`using-graph-uri`, `using-named-graph-uri`) on a
    /// query request; a query's dataset is given by `default-graph-uri` /
    /// `named-graph-uri` (Protocol §2.1.4).
    UsingParametersOnQuery {
        /// The offending parameter.
        parameter: &'static str,
    },
    /// A dataset parameter whose value is not an absolute IRI.
    InvalidGraphIri {
        /// The parameter it was given in.
        parameter: &'static str,
        /// The value as sent.
        value: String,
        /// Why it is not an absolute IRI.
        reason: String,
    },
    /// `using-graph-uri`/`using-named-graph-uri` given for an update in which an
    /// operation already writes `USING`, `USING NAMED` or `WITH` (Protocol §2.2.3: the
    /// request is an error, not a merge and not an override).
    UsingConflictsWithClause {
        /// The dataset parameter the request gave (`using-graph-uri` when it gave both).
        parameter: &'static str,
        /// The zero-based index of the first conflicting operation in the request.
        operation: usize,
        /// The clause it writes: `"USING"` or `"WITH"`.
        clause: &'static str,
    },
    /// The operation text does not parse, so the dataset parameters have no position to
    /// be written at. Reported only when parameters must be applied; an operation
    /// without them is handed to the engine untouched, which reports its own error.
    MalformedOperation {
        /// The operation that failed.
        kind: OperationKind,
        /// The parser's diagnostic.
        message: String,
    },
}

impl ProtocolError {
    /// A stable identifier for the variant, e.g. `"MissingOperation"` — the name a host
    /// carries across a language boundary or into a problem document's `title`.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UnsupportedMethod { .. } => "UnsupportedMethod",
            Self::MissingContentType => "MissingContentType",
            Self::UnsupportedContentType { .. } => "UnsupportedContentType",
            Self::UnsupportedCharset { .. } => "UnsupportedCharset",
            Self::NonUtf8Body { .. } => "NonUtf8Body",
            Self::BodyOnGet => "BodyOnGet",
            Self::MalformedForm { .. } => "MalformedForm",
            Self::MissingOperation => "MissingOperation",
            Self::AmbiguousOperation { .. } => "AmbiguousOperation",
            Self::QueryViaGetIsNotUpdate => "QueryViaGetIsNotUpdate",
            Self::DatasetParametersOnUpdate { .. } => "DatasetParametersOnUpdate",
            Self::UsingParametersOnQuery { .. } => "UsingParametersOnQuery",
            Self::InvalidGraphIri { .. } => "InvalidGraphIri",
            Self::UsingConflictsWithClause { .. } => "UsingConflictsWithClause",
            Self::MalformedOperation { .. } => "MalformedOperation",
        }
    }

    /// The protocol parameter at fault, when the error is about one.
    pub const fn parameter(&self) -> Option<&'static str> {
        match self {
            Self::DatasetParametersOnUpdate { parameter }
            | Self::UsingParametersOnQuery { parameter }
            | Self::InvalidGraphIri { parameter, .. }
            | Self::UsingConflictsWithClause { parameter, .. } => Some(parameter),
            Self::QueryViaGetIsNotUpdate => Some("update"),
            _ => None,
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMethod { method } => write!(
                f,
                "the SPARQL protocol binds operations to GET and POST only; the request used \
                 `{method}`"
            ),
            Self::MissingContentType => f.write_str(
                "a POST request must declare a Content-Type: application/sparql-query, \
                 application/sparql-update or application/x-www-form-urlencoded",
            ),
            Self::UnsupportedContentType { content_type } => write!(
                f,
                "a POST request's Content-Type must be application/sparql-query, \
                 application/sparql-update or application/x-www-form-urlencoded; the request \
                 declared `{content_type}`"
            ),
            Self::UnsupportedCharset { charset } => write!(
                f,
                "SPARQL requests are UTF-8; the request declared charset `{charset}`"
            ),
            Self::NonUtf8Body { valid_up_to } => write!(
                f,
                "the request body is not UTF-8 (invalid byte sequence at offset {valid_up_to})"
            ),
            Self::BodyOnGet => f.write_str(
                "a GET request carries its query in the query string; this one also has a \
                 body, which the protocol gives no meaning",
            ),
            Self::MalformedForm { reason } => {
                write!(f, "the URL-encoded parameters do not decode: {reason}")
            }
            Self::MissingOperation => f.write_str(
                "the request carries no operation: expected a `query` or `update` parameter, \
                 or a query or update body",
            ),
            Self::AmbiguousOperation { reason } => write!(
                f,
                "a SPARQL protocol request carries exactly one operation; this one carries \
                 {reason}"
            ),
            Self::QueryViaGetIsNotUpdate => f.write_str(
                "an update cannot be sent via GET: the SPARQL protocol binds the update \
                 operation to POST only",
            ),
            Self::DatasetParametersOnUpdate { parameter } => write!(
                f,
                "`{parameter}` specifies a query's dataset; an update's is given by \
                 `using-graph-uri` and `using-named-graph-uri`"
            ),
            Self::UsingParametersOnQuery { parameter } => write!(
                f,
                "`{parameter}` specifies an update's dataset; a query's is given by \
                 `default-graph-uri` and `named-graph-uri`"
            ),
            Self::InvalidGraphIri {
                parameter,
                value,
                reason,
            } => write!(
                f,
                "`{parameter}` must be an absolute IRI; `{value}` is not ({reason})"
            ),
            Self::UsingConflictsWithClause {
                parameter,
                operation,
                clause,
            } => write!(
                f,
                "`{parameter}` cannot be combined with an update that writes its own dataset: \
                 operation {operation} has a {clause} clause"
            ),
            Self::MalformedOperation { kind, message } => write!(
                f,
                "the {} does not parse, so the dataset parameters cannot be applied to it: \
                 {message}",
                kind.as_str()
            ),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// One SPARQL 1.1 Protocol operation, read from an HTTP request by
/// [`ProtocolRequest::parse`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolRequest {
    kind: OperationKind,
    text: String,
    default_graph_uris: Vec<String>,
    named_graph_uris: Vec<String>,
    using_graph_uris: Vec<String>,
    using_named_graph_uris: Vec<String>,
    extra_parameters: Vec<(String, String)>,
}

/// The parameters a request carried, by protocol name, in the order they were written.
#[derive(Default)]
struct Parameters {
    query: Vec<String>,
    update: Vec<String>,
    default_graph_uris: Vec<String>,
    named_graph_uris: Vec<String>,
    using_graph_uris: Vec<String>,
    using_named_graph_uris: Vec<String>,
    extra: Vec<(String, String)>,
}

impl Parameters {
    fn absorb(&mut self, pairs: Vec<(String, String)>) {
        for (name, value) in pairs {
            match name.as_str() {
                "query" => self.query.push(value),
                "update" => self.update.push(value),
                DEFAULT_GRAPH_URI => self.default_graph_uris.push(value),
                NAMED_GRAPH_URI => self.named_graph_uris.push(value),
                USING_GRAPH_URI => self.using_graph_uris.push(value),
                USING_NAMED_GRAPH_URI => self.using_named_graph_uris.push(value),
                _ => self.extra.push((name, value)),
            }
        }
    }
}

impl ProtocolRequest {
    /// Read an HTTP request as a SPARQL 1.1 Protocol operation.
    ///
    /// `method` is the HTTP method as sent (case-sensitive, RFC 9110 §9.1);
    /// `content_type` the `Content-Type` header, if any (compared case-insensitively,
    /// parameters other than `charset` ignored); `query_string` the request URI's query
    /// component, with or without its leading `?`; `body` the request body.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] naming the protocol rule the request breaks.
    pub fn parse(
        method: &str,
        content_type: Option<&str>,
        query_string: Option<&str>,
        body: &[u8],
    ) -> Result<Self, ProtocolError> {
        let mut params = Parameters::default();
        params.absorb(decode_form(
            query_string.map_or("", |qs| qs.strip_prefix('?').unwrap_or(qs)),
        )?);
        let (kind, text) = match method {
            // §2.1.1 query via GET: everything is in the query string.
            "GET" => {
                if !body.is_empty() {
                    return Err(ProtocolError::BodyOnGet);
                }
                if !params.update.is_empty() {
                    return Err(ProtocolError::QueryViaGetIsNotUpdate);
                }
                (
                    OperationKind::Query,
                    single(params.query.drain(..), "query")?,
                )
            }
            "POST" => {
                let media =
                    MediaType::parse(content_type.ok_or(ProtocolError::MissingContentType)?);
                media.require_utf8()?;
                match media.essence.as_str() {
                    // §2.1.3 query via POST directly: the body is the query; the
                    // query string carries only the dataset parameters.
                    SPARQL_QUERY => {
                        if !params.query.is_empty() || !params.update.is_empty() {
                            return Err(ProtocolError::AmbiguousOperation {
                                reason: "a query body and an operation parameter in the query \
                                         string"
                                    .to_owned(),
                            });
                        }
                        (OperationKind::Query, utf8_body(body)?)
                    }
                    // §2.2.2 update via POST directly.
                    SPARQL_UPDATE => {
                        if !params.query.is_empty() || !params.update.is_empty() {
                            return Err(ProtocolError::AmbiguousOperation {
                                reason: "an update body and an operation parameter in the \
                                         query string"
                                    .to_owned(),
                            });
                        }
                        (OperationKind::Update, utf8_body(body)?)
                    }
                    // §2.1.2 / §2.2.1 via POST with URL-encoded parameters.
                    FORM => {
                        let body = std::str::from_utf8(body).map_err(|err| {
                            ProtocolError::MalformedForm {
                                reason: format!(
                                    "the form body is not UTF-8 (invalid byte sequence at \
                                     offset {})",
                                    err.valid_up_to()
                                ),
                            }
                        })?;
                        params.absorb(decode_form(body)?);
                        match (params.query.is_empty(), params.update.is_empty()) {
                            (false, false) => {
                                return Err(ProtocolError::AmbiguousOperation {
                                    reason: "both a `query` and an `update` parameter".to_owned(),
                                });
                            }
                            (false, true) => (
                                OperationKind::Query,
                                single(params.query.drain(..), "query")?,
                            ),
                            (true, false) => (
                                OperationKind::Update,
                                single(params.update.drain(..), "update")?,
                            ),
                            (true, true) => return Err(ProtocolError::MissingOperation),
                        }
                    }
                    other => {
                        return Err(ProtocolError::UnsupportedContentType {
                            content_type: other.to_owned(),
                        });
                    }
                }
            }
            other => {
                return Err(ProtocolError::UnsupportedMethod {
                    method: other.to_owned(),
                });
            }
        };
        match kind {
            OperationKind::Query => {
                if !params.using_graph_uris.is_empty() {
                    return Err(ProtocolError::UsingParametersOnQuery {
                        parameter: USING_GRAPH_URI,
                    });
                }
                if !params.using_named_graph_uris.is_empty() {
                    return Err(ProtocolError::UsingParametersOnQuery {
                        parameter: USING_NAMED_GRAPH_URI,
                    });
                }
            }
            OperationKind::Update => {
                if !params.default_graph_uris.is_empty() {
                    return Err(ProtocolError::DatasetParametersOnUpdate {
                        parameter: DEFAULT_GRAPH_URI,
                    });
                }
                if !params.named_graph_uris.is_empty() {
                    return Err(ProtocolError::DatasetParametersOnUpdate {
                        parameter: NAMED_GRAPH_URI,
                    });
                }
            }
        }
        for (parameter, values) in [
            (DEFAULT_GRAPH_URI, &params.default_graph_uris),
            (NAMED_GRAPH_URI, &params.named_graph_uris),
            (USING_GRAPH_URI, &params.using_graph_uris),
            (USING_NAMED_GRAPH_URI, &params.using_named_graph_uris),
        ] {
            for value in values {
                validate_graph_iri(parameter, value)?;
            }
        }
        Ok(Self {
            kind,
            text,
            default_graph_uris: params.default_graph_uris,
            named_graph_uris: params.named_graph_uris,
            using_graph_uris: params.using_graph_uris,
            using_named_graph_uris: params.using_named_graph_uris,
            extra_parameters: params.extra,
        })
    }

    /// Whether the request is a query or an update.
    pub const fn kind(&self) -> OperationKind {
        self.kind
    }

    /// The operation text exactly as the request carried it, before any dataset
    /// parameter is applied. See [`Self::effective_text`] for the text to execute.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The `default-graph-uri` values, in request order (Protocol §2.1.4).
    pub fn default_graph_uris(&self) -> &[String] {
        &self.default_graph_uris
    }

    /// The `named-graph-uri` values, in request order (Protocol §2.1.4).
    pub fn named_graph_uris(&self) -> &[String] {
        &self.named_graph_uris
    }

    /// The `using-graph-uri` values, in request order (Protocol §2.2.3).
    pub fn using_graph_uris(&self) -> &[String] {
        &self.using_graph_uris
    }

    /// The `using-named-graph-uri` values, in request order (Protocol §2.2.3).
    pub fn using_named_graph_uris(&self) -> &[String] {
        &self.using_named_graph_uris
    }

    /// Every parameter the protocol does not define, as `(name, value)` pairs in request
    /// order (query string first, then a form body), for a host that gives them meaning.
    pub fn extra_parameters(&self) -> &[(String, String)] {
        &self.extra_parameters
    }

    /// Whether the request carries dataset parameters, i.e. whether
    /// [`Self::effective_text`] differs from [`Self::text`].
    pub fn has_dataset_parameters(&self) -> bool {
        !(self.default_graph_uris.is_empty()
            && self.named_graph_uris.is_empty()
            && self.using_graph_uris.is_empty()
            && self.using_named_graph_uris.is_empty())
    }

    /// The shape of the query's result, read from its query form, or `None` for an
    /// update. Needed before evaluation, to [`negotiate`] the response format.
    ///
    /// Read from the token stream (the form keyword after the prologue), so it needs no
    /// base IRI or parser configuration and agrees with every configuration's parse.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::MalformedOperation`] when the text does not tokenize or no query
    /// form keyword follows the prologue.
    pub fn result_kind(&self) -> Result<Option<ResultKind>, ProtocolError> {
        if self.kind == OperationKind::Update {
            return Ok(None);
        }
        let malformed = |message: String| ProtocolError::MalformedOperation {
            kind: OperationKind::Query,
            message,
        };
        let tokens = tokenize(&self.text).map_err(|err| malformed(err.to_string()))?;
        for spanned in &tokens {
            if let Token::Word(word) = &spanned.token {
                if ["BASE", "PREFIX", "VERSION"]
                    .iter()
                    .any(|directive| word.eq_ignore_ascii_case(directive))
                {
                    continue;
                }
                if word.eq_ignore_ascii_case("SELECT") {
                    return Ok(Some(ResultKind::Solutions));
                }
                if word.eq_ignore_ascii_case("ASK") {
                    return Ok(Some(ResultKind::Boolean));
                }
                if word.eq_ignore_ascii_case("CONSTRUCT") || word.eq_ignore_ascii_case("DESCRIBE") {
                    return Ok(Some(ResultKind::Graph));
                }
                return Err(malformed(format!(
                    "expected SELECT, CONSTRUCT, ASK or DESCRIBE after the prologue, found \
                     `{word}`"
                )));
            }
        }
        Err(malformed(
            "no query form (SELECT, CONSTRUCT, ASK or DESCRIBE) follows the prologue".to_owned(),
        ))
    }

    /// The operation text with the request's dataset parameters applied, parsed under a
    /// default [`SparqlParser`] and [`ParserOptions::default`].
    ///
    /// See [`Self::effective_text_with`], which a host with a base IRI or parser options
    /// calls instead.
    ///
    /// # Errors
    ///
    /// As [`Self::effective_text_with`].
    pub fn effective_text(&self) -> Result<String, ProtocolError> {
        self.effective_text_with(&SparqlParser::new(), &ParserOptions::default())
    }

    /// The operation text with the request's dataset parameters applied.
    ///
    /// - A request without dataset parameters: [`Self::text`], unparsed and unchanged.
    /// - A query with `default-graph-uri`/`named-graph-uri` (Protocol §2.1.4): the
    ///   query's own `FROM`/`FROM NAMED` run is removed and `FROM <…>` /
    ///   `FROM NAMED <…>` clauses built from the parameters are written at its position
    ///   (or where one would be written, when the query has none) — the protocol's
    ///   dataset overrides the query's.
    /// - An update with `using-graph-uri`/`using-named-graph-uri` (Protocol §2.2.3):
    ///   `USING <…>` / `USING NAMED <…>` clauses are written into every operation with a
    ///   `WHERE`; a `DELETE WHERE` shorthand is written as the `DELETE … WHERE` long form
    ///   it is defined as (SPARQL 1.1 Update §3.1.3.3), the only form that can hold them.
    ///   Operations without a `WHERE` (`INSERT DATA`, `LOAD`, `CLEAR`, …) are unchanged.
    ///
    /// The positions come from `parser` under `options`, which should be the ones the
    /// engine will parse the result with, so the text is read the same way both times.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::MalformedOperation`] when the text does not parse;
    /// [`ProtocolError::UsingConflictsWithClause`] when an update operation already
    /// writes `USING`, `USING NAMED` or `WITH`.
    pub fn effective_text_with(
        &self,
        parser: &SparqlParser,
        options: &ParserOptions,
    ) -> Result<String, ProtocolError> {
        if !self.has_dataset_parameters() {
            return Ok(self.text.clone());
        }
        match self.kind {
            OperationKind::Query => self.splice_query_dataset(parser, options),
            OperationKind::Update => self.splice_update_dataset(parser, options),
        }
    }

    fn splice_query_dataset(
        &self,
        parser: &SparqlParser,
        options: &ParserOptions,
    ) -> Result<String, ProtocolError> {
        let slot = parser
            .parse_query_dataset_slot(&self.text, options)
            .map_err(|err| ProtocolError::MalformedOperation {
                kind: OperationKind::Query,
                message: err.to_string(),
            })?;
        let mut clause = String::from(" ");
        for iri in &self.default_graph_uris {
            push_clause(&mut clause, "FROM", iri);
        }
        for iri in &self.named_graph_uris {
            push_clause(&mut clause, "FROM NAMED", iri);
        }
        let at = slot.dataset_at;
        let mut out = String::with_capacity(self.text.len() + clause.len());
        out.push_str(&self.text[..at.start]);
        out.push_str(&clause);
        out.push_str(&self.text[at.end..]);
        Ok(out)
    }

    fn splice_update_dataset(
        &self,
        parser: &SparqlParser,
        options: &ParserOptions,
    ) -> Result<String, ProtocolError> {
        let split = parser
            .parse_update_split(&self.text, options)
            .map_err(|err| ProtocolError::MalformedOperation {
                kind: OperationKind::Update,
                message: err.to_string(),
            })?;
        let parameter = if self.using_graph_uris.is_empty() {
            USING_NAMED_GRAPH_URI
        } else {
            USING_GRAPH_URI
        };
        let mut using = String::new();
        for iri in &self.using_graph_uris {
            push_clause(&mut using, "USING", iri);
        }
        for iri in &self.using_named_graph_uris {
            push_clause(&mut using, "USING NAMED", iri);
        }
        // (insertion offset, text to insert), in ascending offset order: operations are
        // reported in request order, and each operation's slot follows the previous one's.
        let mut inserts: Vec<(usize, String)> = Vec::with_capacity(split.operations.len());
        for (operation, slot) in split.operations.iter().enumerate() {
            match slot {
                UpdateDatasetSlot::NoWhereClause => {}
                UpdateDatasetSlot::Modify { with_at, using_at } => {
                    if with_at.is_some() {
                        return Err(ProtocolError::UsingConflictsWithClause {
                            parameter,
                            operation,
                            clause: "WITH",
                        });
                    }
                    if !using_at.is_empty() {
                        return Err(ProtocolError::UsingConflictsWithClause {
                            parameter,
                            operation,
                            clause: "USING",
                        });
                    }
                    inserts.push((using_at.start, format!(" {using}")));
                }
                UpdateDatasetSlot::DeleteWhere {
                    where_at,
                    pattern_at,
                } => {
                    // `DELETE WHERE P` → `DELETE P USING … WHERE P`: the pattern's own
                    // text, then the clauses, written before the `WHERE` keyword. The
                    // pattern ends at its closing `}`, so nothing after it (a comment
                    // running to the end of the line) can swallow the clauses; the
                    // newline guarantees it regardless.
                    inserts.push((
                        *where_at,
                        format!("{}\n{using}", &self.text[pattern_at.clone()]),
                    ));
                }
            }
        }
        let mut out = String::with_capacity(
            self.text.len() + inserts.iter().map(|(_, text)| text.len()).sum::<usize>(),
        );
        let mut cursor = 0;
        for (at, text) in inserts {
            out.push_str(&self.text[cursor..at]);
            out.push_str(&text);
            cursor = at;
        }
        out.push_str(&self.text[cursor..]);
        Ok(out)
    }
}

/// Append `KEYWORD <iri> ` to `out`. The IRI was validated by [`validate_graph_iri`], so
/// it contains no `>` or other character that could end the `IRIREF` early.
fn push_clause(out: &mut String, keyword: &str, iri: &str) {
    out.push_str(keyword);
    out.push_str(" <");
    out.push_str(iri);
    out.push_str("> ");
}

/// Exactly one value of an operation parameter, or the protocol error for none or many.
fn single(
    mut values: impl ExactSizeIterator<Item = String>,
    name: &str,
) -> Result<String, ProtocolError> {
    match values.len() {
        0 => Err(ProtocolError::MissingOperation),
        1 => Ok(values.next().unwrap_or_default()),
        n => Err(ProtocolError::AmbiguousOperation {
            reason: format!("{n} `{name}` parameters"),
        }),
    }
}

fn utf8_body(body: &[u8]) -> Result<String, ProtocolError> {
    std::str::from_utf8(body)
        .map(str::to_owned)
        .map_err(|err| ProtocolError::NonUtf8Body {
            valid_up_to: err.valid_up_to(),
        })
}

/// Validate a dataset parameter's value as an absolute IRI (RFC 3987 with a scheme).
fn validate_graph_iri(parameter: &'static str, value: &str) -> Result<(), ProtocolError> {
    match purrdf_iri::is_absolute(value) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ProtocolError::InvalidGraphIri {
            parameter,
            value: value.to_owned(),
            reason: "it is a relative reference, with no scheme".to_owned(),
        }),
        Err(err) => Err(ProtocolError::InvalidGraphIri {
            parameter,
            value: value.to_owned(),
            reason: err.to_string(),
        }),
    }
}

/// A `Content-Type` header value: its essence (`type/subtype`, lower-cased) and its
/// `charset` parameter, if any.
struct MediaType {
    essence: String,
    charset: Option<String>,
}

impl MediaType {
    fn parse(header: &str) -> Self {
        let mut parts = header.split(';');
        let essence = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
        let charset = parts.find_map(|param| {
            let (name, value) = param.split_once('=')?;
            name.trim()
                .eq_ignore_ascii_case("charset")
                .then(|| unquote(value.trim()).to_owned())
        });
        Self { essence, charset }
    }

    fn require_utf8(&self) -> Result<(), ProtocolError> {
        match &self.charset {
            Some(charset) if !charset.eq_ignore_ascii_case("utf-8") => {
                Err(ProtocolError::UnsupportedCharset {
                    charset: charset.clone(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// A parameter value with its surrounding `"…"` removed, if it has them.
fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value)
}

/// Decode `application/x-www-form-urlencoded` text (a form body or a URL query string)
/// into `(name, value)` pairs, in order.
///
/// `&` separates pairs (empty pairs are skipped), the first `=` separates a name from
/// its value (a pair without one has an empty value), `+` is a space, and `%XX` is the
/// byte `0xXX`. The decoded bytes must be UTF-8.
fn decode_form(text: &str) -> Result<Vec<(String, String)>, ProtocolError> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            Ok((percent_decode(name)?, percent_decode(value)?))
        })
        .collect()
}

fn percent_decode(text: &str) -> Result<String, ProtocolError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' => {
                let hex = |offset: usize| bytes.get(index + offset).copied().and_then(hex_value);
                let (Some(high), Some(low)) = (hex(1), hex(2)) else {
                    let end = (index + 3).min(bytes.len());
                    return Err(ProtocolError::MalformedForm {
                        reason: format!(
                            "`%` must be followed by two hex digits, found `{}` in `{text}`",
                            String::from_utf8_lossy(&bytes[index..end])
                        ),
                    });
                };
                out.push((high << 4) | low);
                index += 3;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|err| ProtocolError::MalformedForm {
        reason: format!(
            "`{text}` decodes to bytes that are not UTF-8 (invalid sequence at decoded offset \
             {})",
            err.utf8_error().valid_up_to()
        ),
    })
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The formats a solutions result is offered in, in server preference order: the token
/// the engine's result serializer takes, and its media type. The first is the default
/// (SPARQL 1.1 Query Results JSON).
const SOLUTION_FORMATS: [(&str, &str); 4] = [
    ("json", "application/sparql-results+json"),
    ("xml", "application/sparql-results+xml"),
    ("csv", "text/csv"),
    ("tsv", "text/tab-separated-values"),
];

/// The formats a graph result is offered in, in server preference order. The first is the
/// default (Turtle).
const GRAPH_FORMATS: [(&str, &str); 5] = [
    ("turtle", "text/turtle"),
    ("trig", "application/trig"),
    ("ntriples", "application/n-triples"),
    ("nquads", "application/n-quads"),
    ("jsonld", "application/ld+json"),
];

/// The formats a graph result carrying named graphs is offered in, in server preference
/// order: the quad-capable subset of [`GRAPH_FORMATS`], in the same relative order. The
/// first is the default (TriG).
const DATASET_FORMATS: [(&str, &str); 3] = [
    ("trig", "application/trig"),
    ("nquads", "application/n-quads"),
    ("jsonld", "application/ld+json"),
];

/// The formats an `ASK` boolean is offered in: the SPARQL results formats that define a
/// boolean result, in the same relative order. The first is the default (JSON).
const BOOLEAN_FORMATS: [(&str, &str); 2] = [
    ("json", "application/sparql-results+json"),
    ("xml", "application/sparql-results+xml"),
];

const fn formats(kind: ResultKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        ResultKind::Solutions => &SOLUTION_FORMATS,
        ResultKind::Boolean => &BOOLEAN_FORMATS,
        ResultKind::Graph => &GRAPH_FORMATS,
        ResultKind::Dataset => &DATASET_FORMATS,
    }
}

/// The media types a result of `kind` is offered in, in server preference order — the
/// formats [`negotiate`] chooses among, for a `406` response that names them.
pub fn offered_media_types(kind: ResultKind) -> impl ExactSizeIterator<Item = &'static str> {
    formats(kind).iter().map(|&(_, media)| media)
}

/// The media type of a format token [`negotiate`] returns, for the response's
/// `Content-Type`; `None` for a token it never returns.
pub fn format_media_type(token: &str) -> Option<&'static str> {
    SOLUTION_FORMATS
        .iter()
        .chain(GRAPH_FORMATS.iter())
        .find(|(candidate, _)| *candidate == token)
        .map(|&(_, media)| media)
}

/// One parsed `Accept` media range.
struct MediaRange<'a> {
    kind: &'a str,
    subtype: &'a str,
    /// The number of media-type parameters (before `q`), for specificity.
    params: usize,
    /// The weight in thousandths (RFC 9110 §12.4.2: at most three decimals).
    q: u16,
}

impl MediaRange<'_> {
    /// How specifically the range names `media`, or `None` if it does not match it:
    /// an exact `type/subtype` outranks `type/*`, which outranks `*/*`, and among equals
    /// a range with more parameters is the more specific (RFC 9110 §12.5.1).
    fn specificity(&self, media: &str) -> Option<(u8, usize)> {
        let (kind, subtype) = media.split_once('/')?;
        if self.kind == "*" {
            return (self.subtype == "*").then_some((0, self.params));
        }
        if !self.kind.eq_ignore_ascii_case(kind) {
            return None;
        }
        if self.subtype == "*" {
            return Some((1, self.params));
        }
        self.subtype
            .eq_ignore_ascii_case(subtype)
            .then_some((2, self.params))
    }
}

/// Parse one `Accept` element; `None` for one that is not a well-formed media range with
/// a well-formed weight, which then takes no part in the negotiation.
fn parse_media_range(element: &str) -> Option<MediaRange<'_>> {
    let mut parts = element.split(';');
    let range = parts.next()?.trim();
    let (kind, subtype) = range.split_once('/')?;
    let (kind, subtype) = (kind.trim(), subtype.trim());
    if kind.is_empty() || subtype.is_empty() || (kind == "*" && subtype != "*") {
        return None;
    }
    let mut params = 0;
    let mut q = 1000;
    for param in parts {
        let (name, value) = param.split_once('=')?;
        if name.trim().eq_ignore_ascii_case("q") {
            q = parse_weight(value.trim())?;
            // Everything after the weight is an accept-extension, not a media-type
            // parameter (RFC 9110 §12.5.1).
            break;
        }
        params += 1;
    }
    Some(MediaRange {
        kind,
        subtype,
        params,
        q,
    })
}

/// RFC 9110 §12.4.2 `qvalue = ( "0" [ "." 0*3DIGIT ] ) / ( "1" [ "." 0*3("0") ] )`, in
/// thousandths.
fn parse_weight(text: &str) -> Option<u16> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut thousandths: u16 = 0;
    for (index, digit) in fraction.bytes().enumerate() {
        let place = [100, 10, 1][index];
        thousandths += u16::from(digit - b'0') * place;
    }
    match whole {
        "0" => Some(thousandths),
        "1" if thousandths == 0 => Some(1000),
        _ => None,
    }
}

/// Choose the response format for a result of `kind` from an `Accept` header
/// (SPARQL 1.1 Protocol §2.1.5, RFC 9110 §12.5.1), as the format token the engine's
/// serializers take.
///
/// - No `Accept` header (or an empty one): the default — `json` for solutions and a
///   boolean, `turtle` for a graph, `trig` for a graph carrying named graphs.
/// - Otherwise each offered format takes the weight of the most specific media range
///   that matches it (`type/subtype` over `type/*` over `*/*`); a weight of `0`
///   excludes it. The highest-weighted format wins, and a tie goes to the server's
///   preference order: `json`, `xml`, `csv`, `tsv` for solutions; `json`, `xml` for a
///   boolean; `turtle`, `trig`,
///   `ntriples`, `nquads`, `jsonld` for a graph; `trig`, `nquads`, `jsonld` for a graph
///   carrying named graphs.
/// - `None` when no offered format is acceptable — the host's `406 Not Acceptable`.
///
/// A malformed element of the header (no `/`, a weight outside `0`–`1`) is skipped
/// rather than failing the whole header.
pub fn negotiate(accept: Option<&str>, kind: ResultKind) -> Option<&'static str> {
    let offered = formats(kind);
    let Some(accept) = accept.filter(|header| !header.trim().is_empty()) else {
        return offered.first().map(|&(token, _)| token);
    };
    let ranges: Vec<MediaRange<'_>> = accept.split(',').filter_map(parse_media_range).collect();
    let mut best: Option<(&'static str, u16)> = None;
    for &(token, media) in offered {
        let weight = ranges
            .iter()
            .filter_map(|range| range.specificity(media).map(|spec| (spec, range.q)))
            .max_by(|(left_spec, left_q), (right_spec, right_q)| {
                left_spec.cmp(right_spec).then(left_q.cmp(right_q))
            })
            .map(|(_, q)| q);
        if let Some(q) = weight.filter(|&q| q > 0) {
            // Strictly greater: an equal weight keeps the earlier, preferred format.
            if best.is_none_or(|(_, best_q)| q > best_q) {
                best = Some((token, q));
            }
        }
    }
    best.map(|(token, _)| token)
}
