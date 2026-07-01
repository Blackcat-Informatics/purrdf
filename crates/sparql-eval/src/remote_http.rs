// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HTTP-shaped transport adapter for SPARQL `SERVICE` federation.
//!
//! [`HttpRemoteQuerySource`] is a portable [`RemoteQuerySource`]: it builds the
//! SPARQL Protocol POST request, delegates the actual HTTP exchange to an injected
//! [`HttpTransport`], and decodes the `application/sparql-results+json` response
//! with the wasm-clean [`purrdf_sparql_results::from_json`] reader.

use std::time::Duration;

use purrdf_sparql_algebra::Variable;

use crate::remote::{RemoteError, RemoteQuerySource, ResolvedBindings};

/// The default per-request timeout for a federated `SERVICE` call.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Request data handed to an injected HTTP transport.
#[derive(Debug, Clone, Copy)]
pub struct HttpRequest<'a> {
    /// SPARQL Protocol endpoint URL.
    pub endpoint: &'a str,
    /// Complete forwarded SPARQL `SELECT` query text.
    pub query_text: &'a str,
    /// User-Agent value requested by the core adapter.
    pub user_agent: &'a str,
    /// Per-request timeout requested by the core adapter.
    pub timeout: Duration,
    /// Request content type, always `application/sparql-query`.
    pub content_type: &'a str,
    /// Accept header requested by the core adapter.
    pub accept: &'a str,
}

/// Host/runtime HTTP transport used by [`HttpRemoteQuerySource`].
///
/// Native binaries can implement this with `ureq`, `reqwest`, or platform code;
/// wasm hosts can implement it with `fetch`. The core evaluator depends only on
/// this trait, so it remains portable.
pub trait HttpTransport {
    /// POST `request.query_text` to `request.endpoint` and return the response body.
    fn post(&self, request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError>;
}

impl<F> HttpTransport for F
where
    F: for<'a> Fn(HttpRequest<'a>) -> Result<Vec<u8>, RemoteError>,
{
    fn post(&self, request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
        self(request)
    }
}

/// A [`RemoteQuerySource`] that forwards queries to a remote SPARQL endpoint over
/// an injected HTTP transport. Reusable across endpoints because the endpoint URL
/// is per-call.
#[derive(Debug, Clone)]
pub struct HttpRemoteQuerySource<T> {
    transport: T,
    timeout: Duration,
    user_agent: String,
}

impl<T> HttpRemoteQuerySource<T> {
    /// A source with the default 30s timeout.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            timeout: DEFAULT_TIMEOUT,
            user_agent: "purrdf-sparql-eval/0.1 (SERVICE federation)".to_owned(),
        }
    }

    /// Override the per-request timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl<T> RemoteQuerySource for HttpRemoteQuerySource<T>
where
    T: HttpTransport,
{
    fn query(&self, endpoint: &str, query_text: &str) -> Result<ResolvedBindings, RemoteError> {
        let body = self.transport.post(HttpRequest {
            endpoint,
            query_text,
            user_agent: &self.user_agent,
            timeout: self.timeout,
            content_type: "application/sparql-query",
            accept: "application/sparql-results+json",
        })?;

        let parsed = purrdf_sparql_results::from_json(&body).map_err(|e| {
            RemoteError::Decode(format!("SPARQL-results JSON from <{endpoint}>: {e}"))
        })?;

        Ok(ResolvedBindings {
            variables: parsed.variables.into_iter().map(Variable::new).collect(),
            rows: parsed.rows,
        })
    }
}
