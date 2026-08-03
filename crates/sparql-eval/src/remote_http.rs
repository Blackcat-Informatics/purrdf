// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HTTP-shaped transport adapter for SPARQL `SERVICE` federation.
//!
//! [`HttpRemoteQuerySource`] is a portable [`RemoteQuerySource`]: it builds the
//! SPARQL Protocol POST request, delegates the actual HTTP exchange to an injected
//! [`HttpTransport`], and decodes the `application/sparql-results+json` response
//! with the wasm-clean [`purrdf_sparql_results::from_json`] reader.
//!
//! # This module never reads a clock
//!
//! Both bounds it carries are *data handed to the host*: the per-request [`Duration`]
//! timeout and the executing query's [`StopSignal`]. Neither is compared against a time
//! source here — the timeout is a number the transport enforces, and the signal is polled
//! for a decision the caller's own governor already made. The single clock read on the
//! governor path lives in [`crate::governor::WallDeadline`] and nowhere else, which is
//! what keeps this adapter deterministic and portable.

use std::sync::Arc;
use std::time::Duration;

use purrdf_core::TrippedGovernor;
use purrdf_sparql_algebra::Variable;

use crate::governor::StopSignal;
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
    /// The executing query's stop signal, or `None` when the caller set neither a deadline
    /// nor a cancellation.
    ///
    /// A transport that can abandon an in-flight exchange should poll this wherever it
    /// would otherwise wait, and return [`RemoteError::Governed`] once it fires. It is the
    /// only governor that can act during the exchange: the evaluator is blocked inside
    /// [`HttpTransport::post`] for its whole duration, so a deadline that only the
    /// evaluator polls cannot fire until exactly the wait it was set to bound is over.
    ///
    /// Polling it is *observing a decision*, not reading a clock — the deadline arithmetic
    /// happened when the caller built the signal.
    pub stop: Option<&'a dyn StopSignal>,
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
    /// # The signal is polled before the transport is touched
    ///
    /// An already-expired deadline **prevents** the request: the poll below happens before
    /// [`HttpTransport::post`] is called, so a caller whose budget is spent never issues a
    /// request it cannot wait for. The signal is then handed to the transport in
    /// [`HttpRequest::stop`], which is the only way it can act during the exchange itself.
    fn query(
        &self,
        endpoint: &str,
        query_text: &str,
        stop: Option<&Arc<dyn StopSignal>>,
    ) -> Result<ResolvedBindings, RemoteError> {
        if let Some(cause) = stop.and_then(|signal| signal.poll()) {
            return Err(RemoteError::Governed(TrippedGovernor::Stopped { cause }));
        }
        let body = self.transport.post(HttpRequest {
            endpoint,
            query_text,
            user_agent: &self.user_agent,
            timeout: self.timeout,
            content_type: "application/sparql-query",
            accept: "application/sparql-results+json",
            stop: stop.map(|signal| &**signal),
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
