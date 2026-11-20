// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The caller-supplied HTTP exchange behind SPARQL `SERVICE` federation and `LOAD`.
//!
//! One host seam, [`HttpExchange`], carries every request either operation sends: the
//! method, the target, the ordered headers and the body go in, and one of three things
//! comes out:
//!
//! - an [`HttpOutcome`]: a [`HttpResponse`] (status, response headers, body and the URL
//!   the host finally fetched), an explicit [`HttpOutcome::OpaqueRedirect`] for a
//!   redirect whose status and `Location` the host cannot see, or a
//!   [`HttpOutcome::Dataset`] the host already holds;
//! - or a typed [`HttpFailure`]: the transport failed, the host denied the request, the
//!   host's own handler faulted, or this execution's governor stopped the exchange. A
//!   host failure is never encoded as an HTTP status.
//!
//! Two adapters consume it:
//!
//! - [`HttpRemoteQuerySource`] is a portable [`ServiceResolver`]: it builds the SPARQL
//!   Protocol POST, delegates the exchange, and decodes the
//!   `application/sparql-results+json` response with the wasm-clean
//!   [`purrdf_sparql_results::from_json`] reader.
//! - [`HttpGraphResolver`] is a portable [`GraphResolver`]: it builds the `LOAD` GET,
//!   follows redirects itself (each hop re-authorized), and hands the body to a
//!   caller-supplied [`DocumentDecoder`], since this crate carries no RDF parser.
//!
//! # An opaque redirect is refused, never guessed at
//!
//! A browser asked not to follow redirects reports one as status 0 with no headers. That
//! is not a response, so it is not a [`HttpResponse`]: the host reports
//! [`HttpOutcome::OpaqueRedirect`], and `SERVICE` refuses it as
//! [`RemoteError::OpaqueRedirect`] while `LOAD` refuses it as [`LoadError::OpaqueRedirect`].
//! Both are failures of the invocation, so the `SILENT` forms absorb them.
//!
//! # This module never reads a clock
//!
//! Both bounds it carries are *data handed to the host*: the per-request [`Duration`]
//! timeout and the executing query's [`StopSignal`]. Neither is compared against a time
//! source here — the timeout is a number the host enforces, and the signal is polled
//! for a decision the caller's own governor already made. The single clock read on the
//! governor path lives in [`crate::governor::WallDeadline`] and nowhere else, which is
//! what keeps these adapters deterministic and portable.

use std::sync::Arc;
use std::time::Duration;

use purrdf_core::{RdfDataset, TrippedGovernor};
use purrdf_sparql_algebra::Variable;

use crate::governor::StopSignal;
use crate::remote::{RemoteError, ResolvedBindings, ServiceRequest, ServiceResolver};
use crate::service::{ServiceCapabilities, ServiceCapability, ServiceCatalog, ServiceProfile};
use crate::update::{FetchAdmission, GraphResolveRequest, GraphResolver, LoadError};

/// The default per-request timeout for a federated `SERVICE` call, and for a `LOAD`
/// fetch whose catalog profile names none.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The most redirect hops one `LOAD` follows: generous enough for a real document
/// mirror, small enough that a redirect loop fails fast. A redirect past it fails the
/// `LOAD` as a transport failure.
pub const MAX_LOAD_REDIRECTS: u32 = 5;

/// The default User-Agent a federated `SERVICE` request identifies itself with.
///
/// Built from the crate's own version at compile time, so it names the release that
/// actually sent the request rather than a literal that went stale the first time the
/// version moved.
pub const DEFAULT_USER_AGENT: &str = concat!(
    "purrdf-sparql-eval/",
    env!("CARGO_PKG_VERSION"),
    " (SERVICE federation)"
);

/// The method of an [`HttpExchangeRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HttpMethod {
    /// `GET`, the method of a `LOAD` fetch.
    Get,
    /// `POST`, the method of a SPARQL Protocol `SERVICE` request.
    Post,
}

impl HttpMethod {
    /// The method's token as it appears on the request line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// One request handed to an [`HttpExchange`].
///
/// The headers come in two runs, in send order: [`Self::fixed_headers`], which the
/// adapter sets for the operation (lower-case names), then [`Self::profile_headers`],
/// which a service catalog's profile adds. [`Self::headers`] yields both runs in order.
#[derive(Clone, Copy)]
pub struct HttpExchangeRequest<'a> {
    /// The request method.
    pub method: HttpMethod,
    /// The absolute URL the request is sent to: the `SERVICE` endpoint, or the IRI of the
    /// current `LOAD` hop.
    pub target: &'a str,
    /// The headers the adapter sets for the operation, in send order, with lower-case
    /// names. `SERVICE` sends `content-type: application/sparql-query`,
    /// `accept: application/sparql-results+json` and `user-agent`. `LOAD` sends `accept`
    /// with the RDF media types its decoder reads, then `user-agent` when one is
    /// configured.
    pub fixed_headers: &'a [(&'static str, &'a str)],
    /// Per-service headers the host must send **in addition to** the fixed headers, in
    /// this exact order: the service profile's own headers followed by its credential
    /// header, when the profile has one and grants [`ServiceCapability::Credentials`].
    ///
    /// Empty without a [`ServiceCatalog`] and for a catalogued service whose profile adds
    /// none — which is why configuring a catalog changes no byte of a request that did
    /// not ask for one.
    ///
    /// Repeated names are legal HTTP and are preserved rather than merged; a host should
    /// append each pair rather than set-by-name, or a multi-valued header will lose all
    /// but its last value.
    ///
    /// # A host that ignores this field issues a different request than the one
    /// configured
    ///
    /// This is the field a service's credential arrives in. A host that sends the fixed
    /// headers but drops these would send an *unauthenticated* request for a service the
    /// catalog says is credentialed — and the endpoint's rejection is an ordinary
    /// transport failure, which the `SILENT` forms are entitled to swallow. The result
    /// is an answer that looks complete and is wrong, with nothing anywhere naming the
    /// cause. Send every pair.
    pub profile_headers: &'a [(String, String)],
    /// The request body: the SPARQL query text for `SERVICE`, empty for `LOAD`.
    pub body: &'a [u8],
    /// The per-request timeout the host enforces.
    pub timeout: Duration,
    /// The executing request's stop signal, or `None` when the caller set neither a
    /// deadline nor a cancellation.
    ///
    /// A host that can abandon an in-flight exchange should poll this wherever it would
    /// otherwise wait, and fail with [`HttpFailure::Governed`] once it fires. It is the
    /// only governor that can act during the exchange: the evaluator is blocked inside
    /// [`HttpExchange::exchange`] for its whole duration, so a deadline that only the
    /// evaluator polls cannot fire until exactly the wait it was set to bound is over.
    ///
    /// Polling it is *observing a decision*, not reading a clock — the deadline arithmetic
    /// happened when the caller built the signal.
    ///
    /// # Honouring it is optional, and the corpus pins exactly what it is worth
    ///
    /// Not every HTTP client can cancel a request it has already issued, so ignoring this
    /// field is a supported way to implement [`HttpExchange`] and not a bug. A deaf host
    /// is still bounded at **per-request granularity** — the evaluator polls the signal
    /// and charges the request before dispatch, and inspects the outcome the moment
    /// [`HttpExchange::exchange`] returns — so it reaches the same governor outcome and
    /// performs the same number of exchanges as an honouring one.
    ///
    /// The one thing that differs for `SERVICE` is the certificate the truncation
    /// carries, and it is pinned rather than smoothed over (`vectors/sparql-governors/`,
    /// the `service-*-transport-cancel-mid-exchange` pair). An honouring host *abandons*
    /// the exchange, so the rows in hand remain the true output's first rows in order and
    /// the partial answer keeps `is_positional_prefix == true`, which is the caller's
    /// licence to resume by raising the ceiling. A deaf host *completes* the exchange and
    /// has its response discarded, so the rows it would have contributed are missing from
    /// the middle of the answer rather than from its end: the positional claim is
    /// withdrawn (`is_positional_prefix == false`) and resumption is no longer licensed.
    /// The multiset bound survives in both cases — every row returned was genuinely
    /// established — so the loss is resumability, never soundness.
    ///
    /// See [`crate::remote::ServiceResolver::resolve`] for the same contract stated over
    /// the `SERVICE` seam.
    pub stop: Option<&'a dyn StopSignal>,
}

impl<'a> HttpExchangeRequest<'a> {
    /// Every header in send order: the fixed run, then the profile run.
    pub fn headers(&self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        let fixed = self.fixed_headers;
        let profile = self.profile_headers;
        fixed.iter().map(|&(name, value)| (name, value)).chain(
            profile
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str())),
        )
    }

    /// The value of the fixed header `name` (matched without regard to ASCII case), or
    /// `None` when the adapter did not set it.
    #[must_use]
    pub fn fixed_header(&self, name: &str) -> Option<&'a str> {
        self.fixed_headers
            .iter()
            .find(|(fixed, _)| fixed.eq_ignore_ascii_case(name))
            .map(|&(_, value)| value)
    }
}

// Hand-written so a header value — possibly a credential — never reaches a log.
impl core::fmt::Debug for HttpExchangeRequest<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let header_names: Vec<&str> = self.headers().map(|(name, _)| name).collect();
        f.debug_struct("HttpExchangeRequest")
            .field("method", &self.method)
            .field("target", &self.target)
            .field("headers", &header_names)
            .field("body_len", &self.body.len())
            .field("timeout", &self.timeout)
            .field("stop", &self.stop.is_some())
            .finish()
    }
}

/// A response status code, `100` through `599` (RFC 9110 §15).
///
/// A host that saw no status — a browser's opaque redirect reports `0` — reports
/// [`HttpOutcome::OpaqueRedirect`] instead; there is no status value standing for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HttpStatus(u16);

impl HttpStatus {
    /// `200 OK`.
    pub const OK: Self = Self(200);
    /// `302 Found`.
    pub const FOUND: Self = Self(302);

    /// The status `code`, or `None` when it is not a three-digit code from `100` through
    /// `599`.
    #[must_use]
    pub const fn new(code: u16) -> Option<Self> {
        if code >= 100 && code <= 599 {
            Some(Self(code))
        } else {
            None
        }
    }

    /// The numeric code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }

    /// Whether the status is a success (`2xx`).
    #[must_use]
    pub const fn is_success(self) -> bool {
        self.0 >= 200 && self.0 < 300
    }

    /// Whether the status is one the Fetch standard follows to its `Location`: `301`,
    /// `302`, `303`, `307` or `308` (WHATWG Fetch, "redirect status"). `304 Not
    /// Modified` and the other `3xx` codes are not.
    #[must_use]
    pub const fn is_redirect(self) -> bool {
        matches!(self.0, 301 | 302 | 303 | 307 | 308)
    }
}

impl core::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HTTP {}", self.0)
    }
}

/// A response the host received for an [`HttpExchangeRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpResponse {
    /// The response status.
    pub status: HttpStatus,
    /// The response headers in the order received; names compare without regard to
    /// ASCII case.
    pub headers: Vec<(String, String)>,
    /// The whole response body.
    pub body: Vec<u8>,
    /// The URL the host finally fetched, when the host followed redirects of its own and
    /// can report where it ended. `None` means the response answers the request's own
    /// target.
    ///
    /// `LOAD` parses the body against this URL, so relative IRIs in a redirected document
    /// resolve against the document's real location; when the host reports none, the
    /// base is the IRI of the hop that was requested.
    ///
    /// Redirect hops a host follows itself are not checked against the service
    /// catalog: only the IRIs this crate asks the host to fetch are authorized. A host
    /// that must keep every hop under the catalog's policy returns each redirect as a
    /// response and lets [`HttpGraphResolver`] follow it.
    pub final_url: Option<String>,
}

impl HttpResponse {
    /// A response with `status` and `body`, no headers and no final URL.
    #[must_use]
    pub const fn new(status: HttpStatus, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
            final_url: None,
        }
    }

    /// Append the response header `name: value`.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Record the URL the host finally fetched.
    #[must_use]
    pub fn with_final_url(mut self, url: impl Into<String>) -> Self {
        self.final_url = Some(url.into());
        self
    }

    /// The first value of the response header `name`, matched without regard to ASCII
    /// case.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// What an [`HttpExchange`] produced for a request it carried out.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HttpOutcome {
    /// The response the host received.
    Response(HttpResponse),
    /// The host was redirected and cannot see the redirect's status or `Location`: the
    /// opaque-redirect response a browser returns for a request sent without following
    /// redirects. Neither adapter guesses at it: `SERVICE` refuses it as
    /// [`RemoteError::OpaqueRedirect`] and `LOAD` as [`LoadError::OpaqueRedirect`].
    OpaqueRedirect,
    /// The host answered a `LOAD` fetch with a dataset it already holds, parsed by the
    /// host itself. `SERVICE` has no use for a dataset and reports it as
    /// [`RemoteError::HostFault`].
    Dataset(Arc<RdfDataset>),
}

/// Why an [`HttpExchange`] produced no outcome. None of these is an HTTP status: a status
/// the host received is an [`HttpResponse`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpFailure {
    /// The target could not be reached or read: unreachable, refused, reset, or the
    /// host's own timeout. `SERVICE` reports it as [`RemoteError::Transport`] and `LOAD`
    /// as [`LoadError::Transport`].
    Transport(String),
    /// The host refused the request by its own policy, naming no catalog capability.
    /// `SERVICE` reports it as [`RemoteError::HostDenied`] and `LOAD` as
    /// [`LoadError::HostDenied`].
    Denied(String),
    /// The host's handler failed to answer: it threw, rejected, or answered with
    /// something the host protocol does not define. `SERVICE` reports it as
    /// [`RemoteError::HostFault`] and `LOAD` as [`LoadError::Fault`].
    Fault(String),
    /// The host received the answer and could not read it into the form the operation
    /// needs. `SERVICE` reports it as [`RemoteError::Decode`] and `LOAD` as
    /// [`LoadError::Decode`].
    Decode(String),
    /// This execution's stop signal ([`HttpExchangeRequest::stop`]) fired and the host
    /// abandoned the exchange. Never silenced: `SERVICE` reports it as
    /// [`RemoteError::Governed`] and `LOAD` as [`LoadError::Governed`].
    Governed(TrippedGovernor),
    /// The exchange completed after this execution's stop signal fired, and the host
    /// discarded the response. `SERVICE` reports it as
    /// [`RemoteError::GovernedAfterCompletion`], which withdraws the positional-prefix
    /// claim; `LOAD` reports it as [`LoadError::Governed`].
    GovernedAfterCompletion(TrippedGovernor),
    /// The host cannot allocate another logical exchange identifier. An execution
    /// failure, never silenced by `SERVICE SILENT` ([`RemoteError::ExchangeIdExhausted`]);
    /// `LOAD` reports it as [`LoadError::Fault`].
    ExchangeIdExhausted,
}

impl HttpFailure {
    /// The `SERVICE` error this failure is for the request to `endpoint`.
    fn into_service_error(self, endpoint: &str) -> RemoteError {
        match self {
            Self::Transport(message) => RemoteError::Transport(message),
            Self::Denied(message) => RemoteError::HostDenied {
                endpoint: endpoint.to_owned(),
                message,
            },
            Self::Fault(message) => RemoteError::HostFault {
                endpoint: endpoint.to_owned(),
                message,
            },
            Self::Decode(message) => RemoteError::Decode(message),
            Self::Governed(governor) => RemoteError::Governed(governor),
            Self::GovernedAfterCompletion(governor) => {
                RemoteError::GovernedAfterCompletion(governor)
            }
            Self::ExchangeIdExhausted => RemoteError::ExchangeIdExhausted,
        }
    }

    /// The `LOAD` error this failure is.
    fn into_load_error(self) -> LoadError {
        match self {
            Self::Transport(message) => LoadError::Transport(message),
            Self::Denied(message) => LoadError::HostDenied(message),
            Self::Fault(message) => LoadError::Fault(message),
            Self::Decode(message) => LoadError::Decode(message),
            Self::Governed(governor) | Self::GovernedAfterCompletion(governor) => {
                LoadError::Governed(governor)
            }
            Self::ExchangeIdExhausted => {
                LoadError::Fault(crate::EvalError::ExchangeIdExhausted.to_string())
            }
        }
    }
}

/// Why a `SERVICE` exchange produced no body.
enum ServiceBodyFailure {
    /// The classified error, whose text is already built.
    Error(RemoteError),
    /// A non-success status; its message is built through the admitted allocation path.
    Status(HttpStatus),
}

/// The SPARQL Results body a `SERVICE` exchange produced, or the failure it is.
fn service_body(
    endpoint: &str,
    result: Result<HttpOutcome, HttpFailure>,
) -> Result<Vec<u8>, ServiceBodyFailure> {
    let outcome = result
        .map_err(|failure| ServiceBodyFailure::Error(failure.into_service_error(endpoint)))?;
    match outcome {
        HttpOutcome::Response(response) if response.status.is_success() => Ok(response.body),
        HttpOutcome::Response(response) => Err(ServiceBodyFailure::Status(response.status)),
        HttpOutcome::OpaqueRedirect => {
            Err(ServiceBodyFailure::Error(RemoteError::OpaqueRedirect {
                endpoint: endpoint.to_owned(),
            }))
        }
        HttpOutcome::Dataset(_) => Err(ServiceBodyFailure::Error(RemoteError::HostFault {
            endpoint: endpoint.to_owned(),
            message: "the host answered a SERVICE request with a dataset".to_owned(),
        })),
    }
}

/// The host's HTTP exchange, used by [`HttpRemoteQuerySource`] for `SERVICE` and by
/// [`HttpGraphResolver`] for `LOAD`.
///
/// Native binaries can implement this with any HTTP client; wasm hosts with `fetch`. The
/// core evaluator depends only on this trait, so it remains portable. A host that follows
/// no redirects itself returns each redirect as the response it is; `LOAD` follows it
/// and `SERVICE` reports it as the transport failure of an unexpected status.
pub trait HttpExchange {
    /// Carry out `request`.
    ///
    /// # Errors
    ///
    /// The [`HttpFailure`] that explains why the host produced no outcome.
    fn exchange(&self, request: HttpExchangeRequest<'_>) -> Result<HttpOutcome, HttpFailure>;

    /// Certified peak live bytes for an opaque host exchange, including body and dynamic
    /// error storage. Native [`Self::exchange_admitted`] overrides need no bound.
    fn workspace_certificate(
        &self,
        _request: HttpExchangeRequest<'_>,
    ) -> Option<crate::remote::ServiceWorkspaceCertificate> {
        None
    }

    /// The native or certified body door used by bounded `SERVICE` production.
    ///
    /// The default carries out [`Self::exchange`] under the certified grant and maps its
    /// outcome onto the `SERVICE` seam: a `2xx` response is the body; any other status
    /// is [`RemoteError::Transport`]; [`HttpOutcome::OpaqueRedirect`] is
    /// [`RemoteError::OpaqueRedirect`]; a dataset is [`RemoteError::HostFault`]; and each
    /// [`HttpFailure`] is the variant its documentation names.
    ///
    /// # Errors
    ///
    /// Refuses an unpriced opaque exchange before calling it.
    fn exchange_admitted(
        &self,
        request: HttpExchangeRequest<'_>,
        workspace: crate::WorkspaceCapability,
    ) -> Result<AdmittedHttpBody, crate::remote::ServiceResolutionError> {
        use crate::remote::{AdmittedRemoteError, ServiceResolutionError};
        let grant = if workspace.is_bounded() {
            let certificate = self
                .workspace_certificate(request)
                .ok_or(crate::EvalError::WorkspaceUnpriced("opaque HTTP exchange"))?;
            Some(workspace.charge(certificate.peak_bytes)?)
        } else {
            None
        };
        match service_body(request.target, self.exchange(request)) {
            Ok(body) => Ok(AdmittedHttpBody {
                body,
                storage: match grant {
                    Some(grant) => HttpStorage::Certified(grant),
                    None => HttpStorage::Native(crate::workspace::LexicalFrame::new(&workspace)),
                },
            }),
            Err(ServiceBodyFailure::Status(status)) => {
                drop(grant);
                Err(ServiceResolutionError::Invocation(
                    AdmittedRemoteError::message(
                        purrdf_core::SilencedKind::Transport,
                        format_args!("<{}> answered {status}", request.target),
                        &workspace,
                    )?,
                ))
            }
            Err(ServiceBodyFailure::Error(error)) => {
                Err(ServiceResolutionError::Invocation(match grant {
                    Some(grant) => AdmittedRemoteError::certified(error, grant, &workspace)?,
                    None => AdmittedRemoteError::native(
                        error,
                        crate::workspace::LexicalFrame::new(&workspace),
                    )?,
                }))
            }
        }
    }
}

impl<F> HttpExchange for F
where
    F: for<'a> Fn(HttpExchangeRequest<'a>) -> Result<HttpOutcome, HttpFailure>,
{
    fn exchange(&self, request: HttpExchangeRequest<'_>) -> Result<HttpOutcome, HttpFailure> {
        self(request)
    }
}

/// Parses a fetched `LOAD` document into a frozen dataset.
///
/// This crate carries no RDF parser, so the host that installs an [`HttpGraphResolver`]
/// supplies one. The error is the parser's message, reported verbatim as
/// [`LoadError::Decode`].
pub trait DocumentDecoder {
    /// Parse `body`, whose `Content-Type` was `media_type` as received, resolving
    /// relative IRIs against `base`.
    ///
    /// # Errors
    ///
    /// The parser's message when `body` is not a document of `media_type`.
    fn decode(&self, body: &[u8], media_type: &str, base: &str) -> Result<Arc<RdfDataset>, String>;
}

impl<F> DocumentDecoder for F
where
    F: Fn(&[u8], &str, &str) -> Result<Arc<RdfDataset>, String>,
{
    fn decode(&self, body: &[u8], media_type: &str, base: &str) -> Result<Arc<RdfDataset>, String> {
        self(body, media_type, base)
    }
}

/// A [`GraphResolver`] that fetches each `LOAD` source over an injected [`HttpExchange`]
/// and parses it with an injected [`DocumentDecoder`].
///
/// # Policy comes before every hop
///
/// With a [`ServiceCatalog`] installed, every hop the resolver follows — the source and
/// every location a redirect names — is authorized against it before the exchange is
/// asked. A host that follows redirects itself shows the resolver only the first hop, so a
/// host must return each redirect to the resolver for every destination to pass catalog
/// authorization: a `LOAD`
/// needs [`ServiceCapability::Network`] (and [`ServiceCapability::Credentials`] for a
/// credential), exactly the policy a `SERVICE` meets, and a refusal is the catalog's
/// [`LoadError::Denied`] with nothing fetched. The authorized profile supplies the hop's
/// headers, credential, User-Agent and timeout, so a redirect to another origin carries
/// only what that origin's own profile grants.
///
/// # Redirects are followed here
///
/// A response with a redirect status ([`HttpStatus::is_redirect`]) and a `Location` is
/// followed by this resolver, not by the host: the location is resolved against the
/// response's [`HttpResponse::final_url`], or the hop's IRI when the host reports none
/// (RFC 3986 §5.2), re-authorized, and fetched as a fresh exchange, up to
/// [`MAX_LOAD_REDIRECTS`] hops. Each further hop is a host fetch admitted against the
/// request's `RemoteRequests` ceiling through [`GraphResolver::resolve_admitted`] before it
/// is dispatched, and the request's stop signal is polled after every completed hop,
/// before its response is classified. A redirect without a `Location`, a location that does not
/// resolve, and a hop past the limit are each [`LoadError::Transport`]; an
/// [`HttpOutcome::OpaqueRedirect`] is [`LoadError::OpaqueRedirect`].
///
/// # The document
///
/// A `2xx` response is parsed by the decoder with its `Content-Type` and with the
/// response's [`HttpResponse::final_url`] as the base, or the hop's IRI when the host
/// reports none. A `2xx` response without a `Content-Type` is [`LoadError::Decode`]; any
/// other status is [`LoadError::Transport`]. An [`HttpOutcome::Dataset`] is the document
/// as it stands.
#[derive(Debug, Clone)]
pub struct HttpGraphResolver<E, D> {
    exchange: E,
    decoder: D,
    accept: String,
    timeout: Duration,
    user_agent: Option<String>,
    catalog: Option<ServiceCatalog>,
}

impl<E, D> HttpGraphResolver<E, D> {
    /// A resolver that asks `exchange` for each document with `accept` as its `Accept`
    /// header, parses it with `decoder`, uses the default 30s timeout, sends no
    /// User-Agent of its own, and applies no per-service policy.
    #[must_use]
    pub fn new(exchange: E, decoder: D, accept: impl Into<String>) -> Self {
        Self {
            exchange,
            decoder,
            accept: accept.into(),
            timeout: DEFAULT_TIMEOUT,
            user_agent: None,
            catalog: None,
        }
    }

    /// Override the per-request timeout. A per-service [`ServiceProfile::with_timeout`]
    /// overrides this in turn.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send `user_agent` with every fetch. A per-service profile's User-Agent overrides
    /// it in turn.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Gate every hop through `catalog`, and take each hop's headers, credential,
    /// timeout and User-Agent from the profile it authorizes.
    #[must_use]
    pub fn with_catalog(mut self, catalog: ServiceCatalog) -> Self {
        self.catalog = Some(catalog);
        self
    }

    /// The per-service policy, when one is configured.
    #[must_use]
    pub const fn catalog(&self) -> Option<&ServiceCatalog> {
        self.catalog.as_ref()
    }
}

impl<E: HttpExchange, D: DocumentDecoder> HttpGraphResolver<E, D> {
    /// Fetch one hop: the response, or the document the host already holds.
    fn fetch(
        &self,
        iri: &str,
        stop: Option<&dyn StopSignal>,
        admission: Option<&dyn FetchAdmission>,
    ) -> Result<Result<HttpResponse, Arc<RdfDataset>>, LoadError> {
        let profile = match &self.catalog {
            Some(catalog) => Some(
                catalog
                    .authorize(
                        iri,
                        ServiceCapabilities::granting([ServiceCapability::Network]),
                    )
                    .map_err(LoadError::Denied)?,
            ),
            None => None,
        };
        let profile_headers = match profile {
            Some(profile) => profile
                .request_headers_with_memory(&mut purrdf_lex::allocation::Memory::new(
                    &mut purrdf_lex::allocation::Resident,
                ))
                .map_err(LoadError::Storage)?,
            None => Vec::new(),
        };
        let user_agent = profile
            .and_then(ServiceProfile::user_agent)
            .or(self.user_agent.as_deref());
        let accept = [("accept", self.accept.as_str())];
        let with_agent;
        let fixed_headers: &[(&'static str, &str)] = match user_agent {
            Some(agent) => {
                with_agent = [accept[0], ("user-agent", agent)];
                &with_agent
            }
            None => &accept,
        };
        if let Some(admission) = admission {
            admission.admit().map_err(LoadError::Governed)?;
        }
        let outcome = self
            .exchange
            .exchange(HttpExchangeRequest {
                method: HttpMethod::Get,
                target: iri,
                fixed_headers,
                profile_headers: &profile_headers,
                body: &[],
                timeout: profile
                    .and_then(ServiceProfile::timeout)
                    .unwrap_or(self.timeout),
                stop,
            })
            .map_err(HttpFailure::into_load_error)?;
        match outcome {
            HttpOutcome::Response(response) => Ok(Ok(response)),
            HttpOutcome::Dataset(dataset) => Ok(Err(dataset)),
            HttpOutcome::OpaqueRedirect => Err(LoadError::OpaqueRedirect(iri.to_owned())),
        }
    }
}

impl<E: HttpExchange, D: DocumentDecoder> GraphResolver for HttpGraphResolver<E, D> {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        self.resolve_admitted(request, &|| Ok(()))
    }

    fn resolve_admitted(
        &self,
        request: GraphResolveRequest<'_>,
        admission: &dyn FetchAdmission,
    ) -> Result<Arc<RdfDataset>, LoadError> {
        let mut iri = request.iri.to_owned();
        let mut hops = 0_u32;
        loop {
            // The evaluator charged the first fetch; every redirect hop is admitted here.
            let hop_admission = (hops > 0).then_some(admission);
            let fetched = self.fetch(&iri, request.stop, hop_admission)?;
            if let Some(cause) = request.stop.and_then(StopSignal::poll) {
                return Err(LoadError::Governed(TrippedGovernor::Stopped { cause }));
            }
            let response = match fetched {
                Ok(response) => response,
                Err(dataset) => return Ok(dataset),
            };
            if response.status.is_redirect() {
                let Some(location) = response.header("location") else {
                    return Err(LoadError::Transport(format!(
                        "<{iri}> answered {} without a Location",
                        response.status
                    )));
                };
                hops += 1;
                if hops > MAX_LOAD_REDIRECTS {
                    return Err(LoadError::Transport(format!(
                        "exceeded {MAX_LOAD_REDIRECTS} redirect hops; the last, from <{iri}>, \
                         named {location:?}"
                    )));
                }
                let base = response.final_url.as_deref().unwrap_or(&iri);
                iri = redirect_target(base, location).map_err(LoadError::Transport)?;
                continue;
            }
            if !response.status.is_success() {
                return Err(LoadError::Transport(format!(
                    "<{iri}> answered {}",
                    response.status
                )));
            }
            let Some(media_type) = response.header("content-type") else {
                return Err(LoadError::Decode(format!(
                    "<{iri}> answered {} without a Content-Type",
                    response.status
                )));
            };
            let base = response.final_url.as_deref().unwrap_or(&iri);
            return self
                .decoder
                .decode(&response.body, media_type, base)
                .map_err(LoadError::Decode);
        }
    }
}

/// The IRI a redirect from `from` to `location` names: the location resolved against
/// the URL the response came from (RFC 3986 §5.2).
fn redirect_target(from: &str, location: &str) -> Result<String, String> {
    purrdf_iri::parse(from)
        .and_then(|base| base.resolve(location))
        .map(|target| target.as_str().to_owned())
        .map_err(|error| {
            format!("<{from}> redirected to {location:?}, which is not a resolvable IRI: {error}")
        })
}

#[derive(Debug)]
enum HttpStorage {
    Native(crate::workspace::LexicalFrame),
    Certified(crate::WorkspaceAllocation),
}

/// Immutable response bytes retaining the exchange's original allocation grant.
#[derive(Debug)]
pub struct AdmittedHttpBody {
    body: Vec<u8>,
    storage: HttpStorage,
}
impl AdmittedHttpBody {
    /// Borrow the original bytes while their grant remains live.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.body
    }
    /// Move a resident body out while refusing to detach any bounded grant.
    /// # Errors
    /// Returns the intact original carrier when its producer is bounded.
    #[expect(
        clippy::result_large_err,
        reason = "bounded refusal returns the original bytes and their original grant inline, with no allocation after the extraction was refused"
    )]
    pub fn try_into_resident(self) -> Result<Vec<u8>, Self> {
        let bounded = match &self.storage {
            HttpStorage::Native(frame) => frame.workspace().is_bounded(),
            HttpStorage::Certified(grant) => grant.is_bounded(),
        };
        if bounded { Err(self) } else { Ok(self.body) }
    }
    /// Construct exchange buffers through their original native allocation owner.
    /// All fresh buffers and dynamic errors must use `memory` or an independent
    /// admitted owner. Input configuration owned by the caller is borrowed.
    /// # Errors
    /// Preserves the first physical refusal or retained invocation failure.
    pub fn try_build(
        workspace: &crate::WorkspaceCapability,
        build: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission>,
        ) -> Result<Vec<u8>, crate::remote::ServiceBuildError>,
    ) -> Result<Self, crate::remote::ServiceResolutionError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let result = {
            let admission: &mut dyn purrdf_lex::allocation::Admission = &mut frame;
            build(&mut purrdf_lex::allocation::Memory::new(admission))
        };
        if let Some(error) = frame.take_failure() {
            return Err(error.into());
        }
        match result {
            Ok(body) => Ok(Self {
                body,
                storage: HttpStorage::Native(frame),
            }),
            Err(error) => Err(crate::remote::build_error(error, frame)),
        }
    }
}

impl<T: HttpExchange> HttpRemoteQuerySource<T> {
    fn resolve_native(
        &self,
        request: ServiceRequest<'_>,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<crate::remote::AdmittedResolvedBindings, crate::remote::ServiceResolutionError>
    {
        use crate::remote::{
            AdmittedRemoteError, AdmittedResolvedBindings, ServiceResolutionError,
        };
        use purrdf_lex::allocation::Memory;
        if let Some(trip) = request.stop_trip() {
            return Err(crate::remote::invocation_error(trip, workspace)?);
        }
        let ServiceRequest {
            endpoint,
            query_text,
            stop,
            max_intermediate_cells,
            ..
        } = request;
        struct Headers {
            values: Vec<(String, String)>,
            _frame: crate::workspace::LexicalFrame,
        }
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let profile = {
            let mut memory = Memory::new(&mut frame);
            match &self.catalog {
                Some(catalog) => catalog
                    .authorize_with_memory(
                        endpoint,
                        ServiceCapabilities::granting([
                            ServiceCapability::Query,
                            ServiceCapability::Network,
                        ]),
                        &mut memory,
                    )
                    .map(Some),
                None => Ok(None),
            }
        };
        let profile = match profile {
            Ok(profile) => profile,
            Err(error) => return Err(crate::remote::build_error(error, frame)),
        };
        let headers = {
            let live = frame.admitted_bytes();
            let mut memory = Memory::resume(&mut frame, live);
            match profile {
                Some(profile) => {
                    profile
                        .request_headers_with_memory(&mut memory)
                        .map_err(|error| {
                            memory
                                .admission_mut()
                                .storage_error(error, "SERVICE HTTP headers")
                        })?
                }
                None => Vec::new(),
            }
        };
        let headers = Headers {
            values: headers,
            _frame: frame,
        };
        let fixed_headers = [
            ("content-type", "application/sparql-query"),
            ("accept", "application/sparql-results+json"),
            (
                "user-agent",
                profile
                    .and_then(ServiceProfile::user_agent)
                    .unwrap_or(&self.user_agent),
            ),
        ];
        let body = self.exchange.exchange_admitted(
            HttpExchangeRequest {
                method: HttpMethod::Post,
                target: endpoint,
                fixed_headers: &fixed_headers,
                profile_headers: &headers.values,
                body: query_text.as_bytes(),
                timeout: profile
                    .and_then(ServiceProfile::timeout)
                    .unwrap_or(self.timeout),
                stop: stop.map(|signal| &**signal),
            },
            workspace.clone(),
        )?;
        if let Some(cause) = stop.and_then(|signal| signal.poll()) {
            return Err(crate::remote::invocation_error(
                RemoteError::GovernedAfterCompletion(TrippedGovernor::Stopped { cause }),
                workspace,
            )?);
        }
        // Decode under its ORIGINAL producer frame, holding the original HTTP
        // body and request headers until every decoder-owned value has settled.
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let parsed = {
            let mut memory = Memory::new(&mut frame);
            purrdf_sparql_results::from_json_with_memory(
                body.bytes(),
                max_intermediate_cells,
                &mut memory,
            )
        };
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(purrdf_sparql_results::ReadError::Storage(error)) => {
                return Err(ServiceResolutionError::Operational(
                    frame.storage_error(error, "SERVICE results JSON"),
                ));
            }
            Err(purrdf_sparql_results::ReadError::Lexical(error)) => {
                if let purrdf_sparql_results::Error::JsonRelease { cause, .. }
                | purrdf_sparql_results::Error::TermRelease { cause, .. }
                | purrdf_sparql_results::Error::Storage(cause) = &error
                {
                    let cause = *cause;
                    // Destroy every original decoder payload before its frame
                    // can release the grant. Physical refusal never formats a
                    // replacement invocation diagnostic or requests more storage.
                    drop(error);
                    return Err(ServiceResolutionError::Operational(
                        frame.storage_error(cause, "SERVICE results JSON"),
                    ));
                }

                let live = frame.admitted_bytes();
                let text = Memory::resume(&mut frame, live)
                    .format(&format_args!(
                        "SPARQL-results JSON from <{endpoint}>: {error}"
                    ))
                    .map_err(|error| frame.storage_error(error, "SERVICE JSON diagnostic"))?;
                drop(error);
                // Decoder error scratch has died; a conservative original
                // grant remains until the retained invocation diagnostic dies.
                return Err(ServiceResolutionError::Invocation(
                    AdmittedRemoteError::native(RemoteError::Decode(text), frame)?,
                ));
            }
        };
        let attempted = if parsed.truncated {
            let rows = u64::try_from(parsed.solutions.rows.len())
                .map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?;
            let columns = u64::try_from(parsed.solutions.variables.len().max(1))
                .map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?;
            Some(
                rows.checked_add(1)
                    .and_then(|rows| rows.checked_mul(columns))
                    .ok_or(crate::EvalError::WorkspaceBoundOverflow)?,
            )
        } else {
            None
        };
        let live = frame.admitted_bytes();
        let bindings = {
            let mut memory = Memory::resume(&mut frame, live);
            let mut variables = Vec::new();
            for name in &parsed.solutions.variables {
                let name = workspace.authored_text(name)?;
                memory
                    .push(&mut variables, Variable::from_admitted(name))
                    .map_err(|error| {
                        memory
                            .admission_mut()
                            .storage_error(error, "SERVICE result variables")
                    })?;
            }
            for name in parsed.solutions.variables {
                memory.release_string(name).map_err(|error| {
                    memory
                        .admission_mut()
                        .storage_error(error, "SERVICE decoded variable release")
                })?;
            }
            // The original outer variable-vector capacity remains conservatively
            // charged until this response is destroyed. No census fee occurs.
            ResolvedBindings {
                variables,
                rows: parsed.solutions.rows,
                cell_limit_exceeded_at: attempted,
            }
        };
        Ok(AdmittedResolvedBindings::from_native(
            bindings, frame, workspace,
        ))
    }
}

/// A [`ServiceResolver`] that forwards queries to a remote SPARQL endpoint over
/// an injected [`HttpExchange`]. Reusable across endpoints because the endpoint URL
/// is per-call.
///
/// # Capability gating is opt-in, and adds no byte until it is opted into
///
/// With no [`ServiceCatalog`] this source contacts whatever endpoint it is handed, with
/// the timeout and User-Agent it was built with and no extra headers — exactly as it did
/// before catalogs existed, byte for byte. [`Self::with_catalog`] turns on per-service
/// policy: every request must then be authorized for
/// [`ServiceCapability::Query`] **and** [`ServiceCapability::Network`] — the second
/// because this source is precisely the one that opens a socket — and the authorized
/// profile supplies the request's extra headers, credential, timeout and User-Agent.
#[derive(Debug, Clone)]
pub struct HttpRemoteQuerySource<T> {
    exchange: T,
    timeout: Duration,
    user_agent: String,
    catalog: Option<ServiceCatalog>,
}

impl<T> HttpRemoteQuerySource<T> {
    /// A source with the default 30s timeout and no per-service policy.
    #[must_use]
    pub fn new(exchange: T) -> Self {
        Self {
            exchange,
            timeout: DEFAULT_TIMEOUT,
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            catalog: None,
        }
    }

    /// Override the per-request timeout. A per-service
    /// [`ServiceProfile::with_timeout`] overrides this in turn.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Gate every request through `catalog`, and take each request's headers,
    /// credential, timeout and User-Agent from the profile it authorizes.
    #[must_use]
    pub fn with_catalog(mut self, catalog: ServiceCatalog) -> Self {
        self.catalog = Some(catalog);
        self
    }

    /// The per-service policy, when one is configured.
    #[must_use]
    pub const fn catalog(&self) -> Option<&ServiceCatalog> {
        self.catalog.as_ref()
    }
}

impl<T> ServiceResolver for HttpRemoteQuerySource<T>
where
    T: HttpExchange,
{
    crate::remote::resident_service_resolve!(
    /// # The signal is polled before the exchange is touched
    ///
    /// An already-expired deadline **prevents** the request: the poll below happens before
    /// [`HttpExchange::exchange`] is called, so a caller whose budget is spent never issues
    /// a request it cannot wait for. The signal is then handed to the host in
    /// [`HttpExchangeRequest::stop`], which is the only way it can act during the exchange
    /// itself.
    ///
    /// # …and the policy is applied before it, too
    ///
    /// The [`ServiceCatalog`] authorization below likewise happens before
    /// [`HttpExchange::exchange`], so a service denied [`ServiceCapability::Network`] does
    /// not have its socket opened and then discarded — the host is never reached at
    /// all. That ordering is the entire difference between a policy and an audit log.
    );
    fn resolve_admitted(
        &self,
        request: ServiceRequest<'_>,
        workspace: crate::WorkspaceCapability,
    ) -> Result<crate::remote::AdmittedResolvedBindings, crate::remote::ServiceResolutionError>
    {
        self.resolve_native(request, &workspace)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// The default User-Agent is the one the host is actually handed, and it names this
    /// crate's own version — observed on the wire rather than read off the constant, so a
    /// constructor that stopped using the constant would fail here too.
    #[test]
    fn default_user_agent_names_the_crate_version() {
        let seen: Mutex<Option<String>> = Mutex::new(None);
        let exchange = |request: HttpExchangeRequest<'_>| -> Result<HttpOutcome, HttpFailure> {
            *seen.lock().expect("lock") = request.fixed_header("user-agent").map(str::to_owned);
            Ok(HttpOutcome::Response(HttpResponse::new(
                HttpStatus::OK,
                br#"{"head":{"vars":[]},"results":{"bindings":[]}}"#.to_vec(),
            )))
        };
        let source = HttpRemoteQuerySource::new(exchange);
        source
            .resolve(ServiceRequest::new(
                "http://example.org/sparql",
                "SELECT * WHERE { ?s ?p ?o }",
            ))
            .expect("the host answers an empty result");
        let agent = seen
            .lock()
            .expect("lock")
            .clone()
            .expect("the host was called with a User-Agent");
        assert!(
            agent.contains(env!("CARGO_PKG_VERSION")),
            "the default User-Agent {agent:?} does not name version {}",
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            agent,
            format!(
                "purrdf-sparql-eval/{} (SERVICE federation)",
                env!("CARGO_PKG_VERSION")
            )
        );
    }
}
