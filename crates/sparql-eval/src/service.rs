// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-service context for the `SERVICE` seam: [`ServiceCatalog`], [`ServiceProfile`],
//! [`ServiceCapabilities`], [`ServiceCredential`], and the two resolvers built on them.
//!
//! # What already existed, and what this adds
//!
//! The dependency-inversion seam is not new. [`ServiceResolver`] (in [`crate::remote`],
//! where it is the trait [`EvalCtx`](crate::EvalCtx) holds) has always let a host decide
//! how a `SERVICE` clause is answered, and it has always had two implementations proving
//! the inversion is real — an HTTP one and an in-memory one. What it did **not** have was
//! anywhere to put per-service context: its request was four positional arguments
//! (endpoint, query text, stop signal, cell ceiling), so a host could choose *a* resolver
//! but could not tell that resolver anything about the individual service it was about to
//! contact. Headers, credentials, and — the part with teeth — whether this deployment
//! permits that service at all had no home.
//!
//! This module is that home. [`ServiceRequest`] carries the whole request as one value,
//! and a [`ServiceCatalog`] maps a service IRI to the [`ServiceProfile`] that governs it.
//!
//! # Per-service context belongs on the resolver, never in the service IRI
//!
//! A service IRI is a name, not a credential store. Encoding a bearer token, a tenant
//! header, or a "this one is allowed" marker into the IRI would put that context into the
//! *query text*, where it is visible to whoever wrote the query, is serialized into plans
//! and receipts, and travels with a nested `SERVICE` body. So all of it lives on the
//! resolver instead, keyed by endpoint.
//!
//! # Capability gating, and what "in-process federation" buys
//!
//! A capability is a thing the resolver is permitted to *do* while resolving one service:
//! [`ServiceCapability::Query`] (resolve it at all), [`ServiceCapability::Network`]
//! (perform network I/O to do so), and [`ServiceCapability::Credentials`] (attach the
//! profile's credential). Withholding `Network` is what makes `SERVICE`-shaped
//! composition available without `SERVICE`-shaped risk: [`InProcessServiceResolver`]
//! answers from datasets already in memory and holds no transport of any kind, so a query
//! may be written against a federation seam that provably never opens a socket.
//!
//! There is no `ServiceCapabilities::ALL`, deliberately. A blanket grant written once
//! would silently widen the moment a capability is added to the enum, which is precisely
//! the audit a capability set exists to make impossible; a host names what it means.
//!
//! # The `SILENT` contract
//!
//! SPARQL 1.1 §10 says a `SERVICE SILENT` clause whose endpoint cannot be reached "will
//! be considered to have matched with a single, empty, solution" — the join identity, so
//! the surrounding query proceeds unchanged. This crate keeps that promise exactly, and
//! confines it to what it is a promise *about*. Three outcomes, three answers:
//!
//! | Outcome | Non-silent `SERVICE` | `SERVICE SILENT` |
//! |---|---|---|
//! | The endpoint is unreachable, or its response undecodable ([`RemoteError::Transport`], [`RemoteError::Decode`], [`RemoteError::Disabled`]) | [`EvalError::Remote`](crate::EvalError) | join identity |
//! | A capability was denied ([`RemoteError::Denied`]) | [`EvalError::ServiceDenied`](crate::EvalError) | [`EvalError::ServiceDenied`](crate::EvalError) |
//! | This engine's own governor tripped ([`RemoteError::Governed`], [`RemoteError::GovernedAfterCompletion`]) | truncation | truncation |
//!
//! The first and last rows are the pre-existing rule, unchanged: `SILENT` is a statement
//! about an endpoint the caller does not control, never about the caller's own budget, so
//! a governor trip reached through a `SERVICE` clause propagates as a truncation whether
//! or not `SILENT` is written (see [`crate::remote`]).
//!
//! **The middle row is decided by that same principle, and it is not configurable.** A
//! capability denial is a decision taken on *this* side of the seam — by the host running
//! this engine, deterministically, before any endpoint was consulted. It is therefore
//! exactly like a governor trip and nothing like an unreachable endpoint, and `SILENT`
//! does not get to swallow it. Swallowing one would put the join identity into a
//! surrounding join, making it a no-op, and hand back an answer that looks complete and
//! is wrong — permanently and identically on every run, rather than transiently, so
//! nothing would ever produce a symptom.
//!
//! There is deliberately no knob that softens this. A host that genuinely wants a blocked
//! service to behave like an unreachable one can already say so with total precision, by
//! returning [`RemoteError::Transport`] from its own resolver: that is an honest claim
//! that the endpoint did not answer, and `SILENT` swallows it under the first row above.
//! The expressive power is in the error type, so a visibility flag would have added no
//! reach — only a second way to spell an existing one, and with it the possibility of two
//! callers running the same query over the same data through the same resolver and
//! getting different answers.
//!
//! The denial row holds at **every depth**. An [`InProcessServiceResolver`] evaluates a
//! forwarded body itself, so a denial raised by a `SERVICE` nested inside one travels back
//! out through that inner evaluation's error channel; it is carried as the structured
//! [`EvalError::ServiceDenied`](crate::EvalError) and reclassified as
//! [`RemoteError::Denied`] on the way out, never flattened into a message. Flattening it
//! would make a nested denial silenceable while the identical denial one level up is not.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use purrdf_core::RdfDataset;

use crate::DetHashMap;
use crate::remote::{RemoteError, ResolvedBindings, ServiceRequest, ServiceResolver};

// ── Capabilities ─────────────────────────────────────────────────────────────────

/// One thing a [`ServiceResolver`] may be permitted to do while resolving one service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ServiceCapability {
    /// Resolve this service at all. Withholding it denies the service outright.
    Query,
    /// Perform network I/O while resolving it. Withholding it is what makes an
    /// in-process façade a *provable* one rather than a promise: a resolver that would
    /// have opened a socket refuses instead of opening it.
    Network,
    /// Attach the profile's [`ServiceCredential`] to the request.
    ///
    /// A profile that carries a credential while withholding this capability is a
    /// configuration contradiction, and [`ServiceCatalog::authorize`] refuses it rather
    /// than sending the request unauthenticated: dropping the credential would produce a
    /// request that looks like the configured one, is not, and then fails — or, worse,
    /// succeeds against whatever public subset the service exposes — for a reason nothing
    /// in the configuration explains.
    Credentials,
}

impl ServiceCapability {
    /// Every capability, in the fixed ascending order [`ServiceCapabilities`] reports
    /// them. The order is part of the contract: it is what makes a denial a function of
    /// the configuration rather than of a hash seed.
    const ORDER: [Self; 3] = [Self::Query, Self::Network, Self::Credentials];

    /// This capability's single-bit mask within a [`ServiceCapabilities`] set.
    const fn bit(self) -> u8 {
        match self {
            Self::Query => 1,
            Self::Network => 1 << 1,
            Self::Credentials => 1 << 2,
        }
    }
}

impl fmt::Display for ServiceCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Query => "query",
            Self::Network => "network",
            Self::Credentials => "credentials",
        })
    }
}

/// The set of [`ServiceCapability`] grants held by one [`ServiceProfile`].
///
/// There is intentionally no `ALL`: see this module's documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceCapabilities(u8);

impl ServiceCapabilities {
    /// No capability at all — a service that may not even be queried.
    pub const NONE: Self = Self(0);

    /// The set granting exactly `capabilities`.
    #[must_use]
    pub fn granting(capabilities: impl IntoIterator<Item = ServiceCapability>) -> Self {
        capabilities.into_iter().fold(Self::NONE, Self::grant)
    }

    /// This set plus `capability`.
    #[must_use]
    pub const fn grant(self, capability: ServiceCapability) -> Self {
        Self(self.0 | capability.bit())
    }

    /// Whether `capability` is granted.
    #[must_use]
    pub const fn allows(self, capability: ServiceCapability) -> bool {
        self.0 & capability.bit() != 0
    }

    /// The granted capabilities, in [`ServiceCapability`]'s fixed order.
    pub fn iter(self) -> impl Iterator<Item = ServiceCapability> {
        ServiceCapability::ORDER
            .into_iter()
            .filter(move |&capability| self.allows(capability))
    }

    /// The first capability in `self` that `granted` withholds, in the fixed order —
    /// `None` when `granted` covers all of it.
    fn first_withheld_by(self, granted: Self) -> Option<ServiceCapability> {
        self.iter().find(|&capability| !granted.allows(capability))
    }
}

// ── Credentials ──────────────────────────────────────────────────────────────────

/// Credential material for one service.
///
/// Its [`Debug`] redacts every secret, so a profile or catalog printed into a log or a
/// diagnostic cannot leak one. [`Self::header`] is the only way to get the secret back
/// out, and it is named so a reader can see exactly where that happens.
#[derive(Clone)]
#[non_exhaustive]
pub enum ServiceCredential {
    /// `Authorization: Bearer <token>`.
    Bearer(String),
    /// `Authorization: Basic <base64(username:password)>` (RFC 7617).
    Basic {
        /// The user id. RFC 7617 §2 forbids a colon in it; [`Self::header`] asserts that.
        username: String,
        /// The password.
        password: String,
    },
    /// An arbitrary credential-bearing header, for schemes with no standard spelling
    /// (`X-Api-Key`, a signed tenant assertion, and so on).
    Header {
        /// The header name.
        name: String,
        /// The header value.
        value: String,
    },
}

impl fmt::Debug for ServiceCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bearer(_) => f.write_str("Bearer(<redacted>)"),
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("password", &"<redacted>")
                .finish(),
            Self::Header { name, .. } => f
                .debug_struct("Header")
                .field("name", name)
                .field("value", &"<redacted>")
                .finish(),
        }
    }
}

impl ServiceCredential {
    /// The `(name, value)` header pair that carries this credential.
    ///
    /// The returned value **is** the secret; it is built only where a request is about to
    /// be handed to a transport.
    ///
    /// # Panics
    ///
    /// Panics if a [`Self::Basic`] username contains a colon, which RFC 7617 §2 forbids
    /// because the colon is the field separator — encoding one anyway would silently move
    /// part of the username into the password, producing a credential that is wrong in a
    /// way no error message would ever mention. A colon in the *password* is legal and
    /// encodes normally.
    #[must_use]
    pub fn header(&self) -> (String, String) {
        match self {
            Self::Bearer(token) => ("Authorization".to_owned(), format!("Bearer {token}")),
            Self::Basic { username, password } => {
                assert!(
                    !username.contains(':'),
                    "an HTTP Basic user id may not contain a colon (RFC 7617 §2): the colon is \
                     the field separator, so encoding one would move part of the user id into \
                     the password"
                );
                let encoded = base64_standard(format!("{username}:{password}").as_bytes());
                ("Authorization".to_owned(), format!("Basic {encoded}"))
            }
            Self::Header { name, value } => (name.clone(), value.clone()),
        }
    }
}

/// RFC 4648 §4 base64 with the standard alphabet and `=` padding.
///
/// Hand-rolled rather than a dependency: it is twenty lines, it is on the wasm path, and
/// HTTP Basic is its only caller.
fn base64_standard(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

// ── Denials ──────────────────────────────────────────────────────────────────────

/// A [`ServiceResolver`] refused a service because a capability was withheld.
///
/// Never swallowed by `SERVICE SILENT` — see this module's `SILENT` contract. Carries no
/// credential material and no request text: it is written into query diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDenial {
    /// The service IRI that was refused.
    endpoint: String,
    /// The capability whose absence caused the refusal.
    withheld: ServiceCapability,
    /// Why the capability was not available, in the words the diagnostic shows.
    detail: String,
}

impl ServiceDenial {
    /// A denial of `endpoint` for the want of `withheld`.
    #[must_use]
    pub fn new(
        endpoint: impl Into<String>,
        withheld: ServiceCapability,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            withheld,
            detail: detail.into(),
        }
    }

    /// The service IRI that was refused.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// The capability whose absence caused the refusal.
    #[must_use]
    pub const fn withheld(&self) -> ServiceCapability {
        self.withheld
    }

    /// Why the capability was not available.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ServiceDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            endpoint,
            withheld,
            detail,
        } = self;
        write!(
            f,
            "<{endpoint}> withholds the {withheld} capability: {detail}"
        )
    }
}

// ── Per-service profiles and the catalog ─────────────────────────────────────────

/// Reject a header name or value that could forge a second header.
///
/// Narrow by construction: only NUL, CR and LF are refused, because those are the three
/// bytes that end a header line or terminate a C string. Every other byte a real
/// deployment uses — spaces, `=`, `;`, `,`, `/`, `+`, RFC 9110 token punctuation such as
/// `` !#$%&'*+-.^_`|~ ``, and non-ASCII in a value — passes through untouched. A stricter
/// check here would refuse working configurations for no gain: the transport is the HTTP
/// framer, and applies whatever further rules its client library has.
fn assert_header_safe(kind: &str, text: &str) {
    assert!(
        !text.contains(['\r', '\n', '\0']),
        "a service header {kind} may not contain CR, LF or NUL — that would forge a second \
         header on the wire; got {text:?}"
    );
}

/// Reject credential material that could forge a second header, **without echoing it**.
///
/// The same three bytes [`assert_header_safe`] refuses, and for the same reason — but the
/// message names only `part`, never the text. A credential's whole design is that its
/// secret does not reach a log ([`ServiceCredential`]'s [`Debug`] redacts it), and a panic
/// message is a log: quoting the offending token here would leak on the one path
/// specifically built not to.
fn assert_credential_safe(part: &str, text: &str) {
    assert!(
        !text.contains(['\r', '\n', '\0']),
        "a service credential's {part} may not contain CR, LF or NUL — that would forge a \
         second header on the wire (the value is withheld from this message because it is \
         credential material)"
    );
}

/// The per-service context a [`ServiceResolver`] applies to one endpoint: what it may do,
/// and what it sends.
#[derive(Debug, Clone)]
pub struct ServiceProfile {
    /// What the resolver may do for this service.
    capabilities: ServiceCapabilities,
    /// Extra request headers, in the order they were added. A resolver appends the
    /// credential header (if any) after these.
    headers: Vec<(String, String)>,
    /// This service's credential, if it has one.
    credential: Option<ServiceCredential>,
    /// Per-request timeout, overriding the resolver's own.
    timeout: Option<Duration>,
    /// User-Agent value, overriding the resolver's own.
    user_agent: Option<String>,
}

impl ServiceProfile {
    /// A profile granting exactly `capabilities`, with no headers, no credential and no
    /// overrides.
    ///
    /// `capabilities` is a required argument rather than something that defaults: a
    /// profile whose grants were implicit would be a policy this crate invented on the
    /// host's behalf.
    #[must_use]
    pub const fn new(capabilities: ServiceCapabilities) -> Self {
        Self {
            capabilities,
            headers: Vec::new(),
            credential: None,
            timeout: None,
            user_agent: None,
        }
    }

    /// Add a request header. Repeated names are kept, in order, exactly as HTTP allows.
    ///
    /// # Panics
    ///
    /// Panics if `name` or `value` contains CR, LF or NUL, which would forge a second
    /// header on the wire.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let (name, value) = (name.into(), value.into());
        assert_header_safe("name", &name);
        assert_header_safe("value", &value);
        self.headers.push((name, value));
        self
    }

    /// Attach this service's credential.
    ///
    /// # Panics
    ///
    /// Panics if the credential carries bytes that would forge a second header, or if a
    /// [`ServiceCredential::Basic`] user id contains the colon RFC 7617 §2 forbids. Both
    /// are checked **here**, at configuration time, rather than only where the header is
    /// rendered: rendering happens inside [`ServiceResolver::resolve`] while a query is
    /// running, and on `wasm32-unknown-unknown` a panic there takes the whole instance
    /// down mid-evaluation. The message never quotes the credential.
    ///
    /// Only the parts that reach the wire **verbatim** are checked for CR/LF/NUL —
    /// a bearer token and an arbitrary header's name and value. A
    /// [`ServiceCredential::Basic`] username and password are deliberately *not*:
    /// [`ServiceCredential::header`] base64-encodes them, whose output cannot contain any
    /// of those bytes, so refusing them would reject working credentials for no gain.
    #[must_use]
    pub fn with_credential(mut self, credential: ServiceCredential) -> Self {
        match &credential {
            ServiceCredential::Bearer(token) => assert_credential_safe("bearer token", token),
            ServiceCredential::Header { name, value } => {
                // The NAME is not secret — `Debug` prints it — but it is echoed through
                // the header-safety message rather than this one only because that
                // message is the more useful of the two; the value never is.
                assert_header_safe("name", name);
                assert_credential_safe("header value", value);
            }
            ServiceCredential::Basic { username, .. } => assert!(
                !username.contains(':'),
                "an HTTP Basic user id may not contain a colon (RFC 7617 §2): the colon is \
                 the field separator, so encoding one would move part of the user id into \
                 the password"
            ),
        }
        self.credential = Some(credential);
        self
    }

    /// Override the resolver's per-request timeout for this service.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Override the resolver's User-Agent for this service.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// What the resolver may do for this service.
    #[must_use]
    pub const fn capabilities(&self) -> ServiceCapabilities {
        self.capabilities
    }

    /// The extra request headers, in the order they were added.
    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// This service's credential, if it has one.
    #[must_use]
    pub const fn credential(&self) -> Option<&ServiceCredential> {
        self.credential.as_ref()
    }

    /// The per-request timeout override, if any.
    #[must_use]
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// The User-Agent override, if any.
    #[must_use]
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// The complete, ordered header list a request to this service carries: the profile's
    /// own headers followed by the credential header, when one is configured **and** the
    /// [`ServiceCapability::Credentials`] grant is present.
    ///
    /// The capability is re-checked here rather than assumed, so this is correct on a
    /// profile that never went through [`ServiceCatalog::authorize`]. It is a backstop,
    /// not a supported way to drop a credential: `authorize` refuses the credential-
    /// without-the-grant configuration outright, so a request is never *issued* with the
    /// credential quietly missing.
    #[must_use]
    pub fn request_headers(&self) -> Vec<(String, String)> {
        let mut headers = self.headers.clone();
        if self.capabilities.allows(ServiceCapability::Credentials)
            && let Some(credential) = &self.credential
        {
            headers.push(credential.header());
        }
        headers
    }
}

/// Maps a service IRI to its [`ServiceProfile`]. **Deny by default**: a service with no
/// entry and no configured fallback is refused.
///
/// A catalog is the whole per-service policy in one value, so what a resolver will do is
/// inspectable in one place rather than spread across the resolver, the query text, and
/// the environment.
#[derive(Debug, Clone, Default)]
pub struct ServiceCatalog {
    /// Per-endpoint profiles.
    profiles: DetHashMap<String, ServiceProfile>,
    /// The profile applied to a service with no entry of its own, when one is configured.
    fallback: Option<ServiceProfile>,
}

impl ServiceCatalog {
    /// An empty catalog: every service is denied.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `profile` for the service IRI `endpoint`.
    #[must_use]
    pub fn with_service(mut self, endpoint: impl Into<String>, profile: ServiceProfile) -> Self {
        self.profiles.insert(endpoint.into(), profile);
        self
    }

    /// Apply `profile` to every service with no entry of its own.
    ///
    /// The explicit opt-out of deny-by-default, spelled as its own call so a catalog that
    /// grants unknown services says so in one readable line.
    #[must_use]
    pub fn with_fallback(mut self, profile: ServiceProfile) -> Self {
        self.fallback = Some(profile);
        self
    }

    /// The profile that governs `endpoint`, without checking any capability.
    #[must_use]
    pub fn profile_for(&self, endpoint: &str) -> Option<&ServiceProfile> {
        self.profiles.get(endpoint).or(self.fallback.as_ref())
    }

    /// The profile that governs `endpoint`, once it has granted every capability in
    /// `needs`.
    ///
    /// The reported capability is the first one in [`ServiceCapability`]'s fixed order
    /// that the profile withholds, so the denial is a function of the configuration and
    /// not of iteration order.
    ///
    /// # Errors
    ///
    /// Returns a [`ServiceDenial`] when `endpoint` has no profile, when the profile
    /// withholds one of `needs`, or when the profile carries a credential without the
    /// [`ServiceCapability::Credentials`] grant that would let it be sent.
    pub fn authorize(
        &self,
        endpoint: &str,
        needs: ServiceCapabilities,
    ) -> Result<&ServiceProfile, ServiceDenial> {
        let Some(profile) = self.profile_for(endpoint) else {
            return Err(ServiceDenial::new(
                endpoint,
                ServiceCapability::Query,
                "no profile is configured for this service, and the catalog has no fallback",
            ));
        };
        if let Some(withheld) = needs.first_withheld_by(profile.capabilities()) {
            return Err(ServiceDenial::new(
                endpoint,
                withheld,
                "the service profile does not grant it",
            ));
        }
        if profile.credential().is_some()
            && !profile
                .capabilities()
                .allows(ServiceCapability::Credentials)
        {
            return Err(ServiceDenial::new(
                endpoint,
                ServiceCapability::Credentials,
                "the service profile carries a credential but withholds the capability that \
                 would let it be sent; the request is refused rather than issued \
                 unauthenticated, which would be a different request than the one configured",
            ));
        }
        Ok(profile)
    }
}

// ── The in-process resolver ──────────────────────────────────────────────────────

/// An in-memory [`ServiceResolver`] that **dog-foods the native engine**: each endpoint
/// IRI maps to a local [`RdfDataset`], and a forwarded query is parsed and evaluated
/// against it with [`NativeSparqlEngine`](crate::NativeSparqlEngine) semantics.
/// Deterministic and network-free — the test/conformance vehicle for `SERVICE`, and the
/// in-process federation seam for a host that wants one.
///
/// # It cannot reach the network, structurally
///
/// This type owns two things — a map of datasets and an optional [`ServiceCatalog`] — and
/// no transport, no socket, no URL client, and no injected callback of any kind. There is
/// no code path through it that performs I/O, so "in-process" is a property of the type
/// rather than a promise about how it was configured.
///
/// # Capability gating
///
/// With no catalog it answers any endpoint it was given a dataset for, which is the
/// deterministic offline vehicle the conformance harness and this crate's own tests use,
/// and is unchanged from before catalogs existed. [`Self::with_catalog`] turns gating on:
/// the catalog is consulted for every resolution, **including the nested ones** a
/// `SERVICE` inside a forwarded body performs, because a nested body is resolved by
/// threading `self` — the gated resolver — back into the forwarded evaluation.
#[derive(Debug, Default)]
pub struct InProcessServiceResolver {
    /// Endpoint IRI → the dataset that answers it.
    datasets: DetHashMap<String, Arc<RdfDataset>>,
    /// The per-service policy, when one is configured.
    catalog: Option<ServiceCatalog>,
}

impl InProcessServiceResolver {
    /// An empty resolver with no endpoints and no catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `dataset` as the contents of `endpoint`.
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>, dataset: Arc<RdfDataset>) -> Self {
        self.datasets.insert(endpoint.into(), dataset);
        self
    }

    /// Gate every resolution — nested ones included — through `catalog`.
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

    /// The dataset registered for `endpoint`, if any.
    #[must_use]
    pub fn dataset(&self, endpoint: &str) -> Option<&Arc<RdfDataset>> {
        self.datasets.get(endpoint)
    }
}

impl ServiceResolver for InProcessServiceResolver {
    /// # The forwarded evaluation is governed by the caller's signal
    ///
    /// The forwarded query is a whole evaluation of its own, and an in-memory endpoint is
    /// not a bounded amount of work — a cyclic property path or a cross product costs the
    /// same here as it does anywhere else. So the caller's [`StopSignal`](crate::governor::StopSignal) is installed on
    /// the forwarded context, which polls it at every operator boundary, and a signal that
    /// fires part-way through is reported as [`RemoteError::Governed`] rather than as a
    /// decode failure. Reporting it as a failure would make it silenceable, which is
    /// exactly the laundering `SILENT` must not perform.
    ///
    /// The forwarded evaluation carries the caller's intermediate-cell ceiling, so an
    /// in-memory endpoint cannot materialize a bag the caller already bounded. It carries
    /// no *charge* ceilings: fuel spent here is already charged at the calling seam, per
    /// request and per ingested row, and charging it twice would make one query's budget
    /// depend on how a federation happened to be split up.
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        // The governor above outranks the policy below deliberately, and in this order:
        // a stop that has already fired is a fact about this execution, and both are
        // non-silenceable, so reporting the stop keeps the certificate naming the
        // governor that actually ended the query.
        if let Some(catalog) = &self.catalog {
            catalog.authorize(
                request.endpoint,
                ServiceCapabilities::granting([ServiceCapability::Query]),
            )?;
        }
        let dataset = self.datasets.get(request.endpoint).ok_or_else(|| {
            RemoteError::Transport(format!("no in-memory endpoint <{}>", request.endpoint))
        })?;
        crate::remote::evaluate_in_memory(dataset, request, self)
    }
}

// ── The router ───────────────────────────────────────────────────────────────────

/// A [`ServiceResolver`] that dispatches each service IRI to a different resolver.
///
/// This is how a host composes a mixed federation: some services answered in process by
/// an [`InProcessServiceResolver`], the rest handed to a network resolver — with the
/// routing table, rather than the query text, deciding which is which. A service with no
/// route and no fallback is denied.
pub struct ServiceRouter<'a> {
    /// Endpoint IRI → the resolver that answers it.
    routes: DetHashMap<String, &'a (dyn ServiceResolver + Sync)>,
    /// The resolver used for a service with no route of its own.
    fallback: Option<&'a (dyn ServiceResolver + Sync)>,
}

impl fmt::Debug for ServiceRouter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut routes: Vec<&str> = self.routes.keys().map(String::as_str).collect();
        routes.sort_unstable();
        f.debug_struct("ServiceRouter")
            .field("routes", &routes)
            .field("fallback", &self.fallback.map(|_| "<resolver>"))
            .finish()
    }
}

impl Default for ServiceRouter<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ServiceRouter<'a> {
    /// A router with no routes and no fallback: every service is denied.
    #[must_use]
    pub fn new() -> Self {
        Self {
            routes: DetHashMap::default(),
            fallback: None,
        }
    }

    /// Route `endpoint` to `resolver`.
    #[must_use]
    pub fn with_route(
        mut self,
        endpoint: impl Into<String>,
        resolver: &'a (dyn ServiceResolver + Sync),
    ) -> Self {
        self.routes.insert(endpoint.into(), resolver);
        self
    }

    /// Send every unrouted service to `resolver`.
    #[must_use]
    pub fn with_fallback(mut self, resolver: &'a (dyn ServiceResolver + Sync)) -> Self {
        self.fallback = Some(resolver);
        self
    }
}

impl ServiceResolver for ServiceRouter<'_> {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        let resolver = self
            .routes
            .get(request.endpoint)
            .copied()
            .or(self.fallback)
            .ok_or_else(|| {
                RemoteError::Denied(ServiceDenial::new(
                    request.endpoint,
                    ServiceCapability::Query,
                    "no resolver is routed to this service, and the router has no fallback",
                ))
            })?;
        resolver.resolve(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_4648_test_vectors() {
        // RFC 4648 §10, verbatim: the padding boundaries are where a hand-rolled encoder
        // goes wrong, and every one of them is exercised here.
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(
                base64_standard(input.as_bytes()),
                expected,
                "input {input:?}"
            );
        }
        // A byte outside ASCII exercises the high bits of the 24-bit group.
        assert_eq!(base64_standard(&[0xff, 0xef, 0xbf]), "/++/");
    }

    #[test]
    fn a_basic_credential_renders_the_rfc_7617_header() {
        let (name, value) = ServiceCredential::Basic {
            username: "Aladdin".to_owned(),
            password: "open sesame".to_owned(),
        }
        .header();
        assert_eq!(name, "Authorization");
        // RFC 7617 §2's own example.
        assert_eq!(value, "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==");
    }

    #[test]
    fn a_credential_never_prints_its_secret() {
        let profile = ServiceProfile::new(ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Credentials,
        ]))
        .with_credential(ServiceCredential::Bearer("s3cr3t-token".to_owned()));
        let printed = format!("{profile:?}");
        assert!(
            !printed.contains("s3cr3t-token"),
            "a profile printed into a log must not carry the token: {printed}"
        );
        assert!(printed.contains("<redacted>"), "got {printed}");

        let printed = format!(
            "{:?}",
            ServiceCredential::Basic {
                username: "user".to_owned(),
                password: "hunter2".to_owned(),
            }
        );
        assert!(!printed.contains("hunter2"), "got {printed}");
        assert!(printed.contains("user"), "the user id is not the secret");

        let printed = format!(
            "{:?}",
            ServiceCredential::Header {
                name: "X-Api-Key".to_owned(),
                value: "abcd-key".to_owned(),
            }
        );
        assert!(!printed.contains("abcd-key"), "got {printed}");
        assert!(printed.contains("X-Api-Key"), "the name is not the secret");
    }

    #[test]
    fn capability_sets_report_exactly_what_was_granted() {
        let none = ServiceCapabilities::NONE;
        assert_eq!(none.iter().count(), 0);
        for capability in ServiceCapability::ORDER {
            assert!(!none.allows(capability), "{capability} must not be granted");
        }

        let set = ServiceCapabilities::granting([
            ServiceCapability::Credentials,
            ServiceCapability::Query,
        ]);
        // Exact, not "at least": a set that reported a capability nobody granted is the
        // whole failure mode a capability set exists to prevent.
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            vec![ServiceCapability::Query, ServiceCapability::Credentials],
            "the reported set is exactly the granted one, in the fixed order"
        );
        assert!(!set.allows(ServiceCapability::Network));
        assert_eq!(
            ServiceCapability::ORDER.len(),
            3,
            "every capability needs a bit and a place in ORDER; adding one without \
             extending ORDER would drop it out of iteration and out of every denial"
        );
        // Every bit is distinct: a collision would make one grant imply another.
        let mut bits: Vec<u8> = ServiceCapability::ORDER.iter().map(|c| c.bit()).collect();
        bits.sort_unstable();
        bits.dedup();
        assert_eq!(bits.len(), ServiceCapability::ORDER.len());
    }

    #[test]
    fn the_first_withheld_capability_is_reported_in_the_fixed_order() {
        let needs = ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Network,
            ServiceCapability::Credentials,
        ]);
        assert_eq!(
            needs.first_withheld_by(ServiceCapabilities::NONE),
            Some(ServiceCapability::Query)
        );
        assert_eq!(
            needs.first_withheld_by(ServiceCapabilities::granting([ServiceCapability::Query])),
            Some(ServiceCapability::Network)
        );
        assert_eq!(needs.first_withheld_by(needs), None);
    }

    #[test]
    fn an_uncatalogued_service_is_denied_and_a_listed_one_is_not() {
        let catalog = ServiceCatalog::new();
        let denial = catalog
            .authorize(
                "https://example.org/sparql",
                ServiceCapabilities::granting([ServiceCapability::Query]),
            )
            .expect_err("an empty catalog denies everything");
        assert_eq!(denial.withheld(), ServiceCapability::Query);
        assert_eq!(denial.endpoint(), "https://example.org/sparql");

        // …and the neighbouring VALID case: the same catalog with the service listed
        // authorizes it. A denial that fired for everything would prove nothing.
        let catalog = ServiceCatalog::new().with_service(
            "https://example.org/sparql",
            ServiceProfile::new(ServiceCapabilities::granting([ServiceCapability::Query])),
        );
        let profile = catalog
            .authorize(
                "https://example.org/sparql",
                ServiceCapabilities::granting([ServiceCapability::Query]),
            )
            .expect("a listed service with the grant it needs must be authorized");
        assert!(profile.capabilities().allows(ServiceCapability::Query));
    }

    #[test]
    fn a_fallback_profile_is_the_explicit_opt_out_of_deny_by_default() {
        let catalog = ServiceCatalog::new().with_fallback(ServiceProfile::new(
            ServiceCapabilities::granting([ServiceCapability::Query]),
        ));
        catalog
            .authorize(
                "https://example.org/never-listed",
                ServiceCapabilities::granting([ServiceCapability::Query]),
            )
            .expect("the fallback covers an unlisted service");
        // The fallback grants Query and nothing else, so Network is still refused —
        // a fallback is not a blanket grant.
        let denial = catalog
            .authorize(
                "https://example.org/never-listed",
                ServiceCapabilities::granting([
                    ServiceCapability::Query,
                    ServiceCapability::Network,
                ]),
            )
            .expect_err("the fallback grants Query only");
        assert_eq!(denial.withheld(), ServiceCapability::Network);
    }

    #[test]
    fn a_credential_without_its_capability_is_refused_rather_than_dropped() {
        let catalog = ServiceCatalog::new().with_service(
            "https://example.org/sparql",
            ServiceProfile::new(ServiceCapabilities::granting([
                ServiceCapability::Query,
                ServiceCapability::Network,
            ]))
            .with_credential(ServiceCredential::Bearer("token".to_owned())),
        );
        let denial = catalog
            .authorize(
                "https://example.org/sparql",
                ServiceCapabilities::granting([
                    ServiceCapability::Query,
                    ServiceCapability::Network,
                ]),
            )
            .expect_err("sending the request unauthenticated is not a repair");
        assert_eq!(denial.withheld(), ServiceCapability::Credentials);

        // The neighbouring VALID case: grant the capability and the same profile
        // authorizes, and its request headers carry the credential.
        let catalog = ServiceCatalog::new().with_service(
            "https://example.org/sparql",
            ServiceProfile::new(ServiceCapabilities::granting([
                ServiceCapability::Query,
                ServiceCapability::Network,
                ServiceCapability::Credentials,
            ]))
            .with_credential(ServiceCredential::Bearer("token".to_owned())),
        );
        let profile = catalog
            .authorize(
                "https://example.org/sparql",
                ServiceCapabilities::granting([
                    ServiceCapability::Query,
                    ServiceCapability::Network,
                ]),
            )
            .expect("the granted profile authorizes");
        assert_eq!(
            profile.request_headers(),
            vec![("Authorization".to_owned(), "Bearer token".to_owned())]
        );
    }

    #[test]
    fn request_headers_are_the_profile_headers_then_the_credential() {
        let profile = ServiceProfile::new(ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Credentials,
        ]))
        .with_header("X-Tenant", "acme")
        .with_header("X-Tenant", "acme-secondary")
        .with_credential(ServiceCredential::Header {
            name: "X-Api-Key".to_owned(),
            value: "k".to_owned(),
        });
        assert_eq!(
            profile.request_headers(),
            vec![
                ("X-Tenant".to_owned(), "acme".to_owned()),
                ("X-Tenant".to_owned(), "acme-secondary".to_owned()),
                ("X-Api-Key".to_owned(), "k".to_owned()),
            ],
            "repeated names survive in order, and the credential comes last"
        );

        // Without the grant the credential is absent from the rendered list — the
        // catalog refuses that configuration before a request is built, so this is the
        // belt to `authorize`'s braces rather than a supported way to drop a credential.
        let ungranted =
            ServiceProfile::new(ServiceCapabilities::granting([ServiceCapability::Query]))
                .with_credential(ServiceCredential::Bearer("t".to_owned()));
        assert_eq!(ungranted.request_headers(), [] as [_; 0]);
    }

    #[test]
    fn a_header_that_could_forge_a_second_one_is_rejected() {
        for (name, value) in [
            ("X-Evil\r\nX-Injected", "1"),
            ("X-Evil", "1\r\nX-Injected: 2"),
            ("X-Evil", "1\nX-Injected: 2"),
            ("X-Evil\0", "1"),
        ] {
            let outcome = std::panic::catch_unwind(|| {
                ServiceProfile::new(ServiceCapabilities::NONE).with_header(name, value)
            });
            assert!(outcome.is_err(), "{name:?}: {value:?} must be refused");
        }
    }

    #[test]
    fn ordinary_and_awkward_but_legal_headers_are_not_rejected() {
        // The over-refusal guard for the check above: RFC 9110 token punctuation in a
        // name, and spaces / separators / non-ASCII in a value, are all legal and in use.
        let profile = ServiceProfile::new(ServiceCapabilities::NONE)
            .with_header("X-Purrdf-Trace!#$%&'*+-.^_`|~", "a b=c;d,e/f+g")
            .with_header("Accept-Language", "fr-CA, en;q=0.8")
            .with_header("X-Note", "café — naïve");
        assert_eq!(
            profile.headers().len(),
            3,
            "a narrow injection guard must not become a header allow-list"
        );
    }

    #[test]
    fn a_credential_that_could_forge_a_second_header_is_refused_at_configuration_time() {
        // The higher-privilege sibling of `with_header`'s guard, and the one that mattered
        // more: a credential is appended to the very same header list, so leaving it
        // unchecked let the ONE field a host is most careful about be the one that could
        // forge a header. Checked in `with_credential` rather than only in `header()`
        // because `header()` runs inside `resolve` while a query is executing.
        for credential in [
            ServiceCredential::Bearer("t\r\nX-Injected: 1".to_owned()),
            ServiceCredential::Bearer("t\0".to_owned()),
            ServiceCredential::Header {
                name: "X-Api-Key".to_owned(),
                value: "k\r\nX-Injected: 1".to_owned(),
            },
            ServiceCredential::Header {
                name: "X-Api-Key\r\nX-Injected".to_owned(),
                value: "k".to_owned(),
            },
        ] {
            let outcome = std::panic::catch_unwind(|| {
                ServiceProfile::new(ServiceCapabilities::NONE).with_credential(credential.clone())
            });
            let payload = outcome.expect_err("the credential must be refused");
            // The refusal must not become the leak: a panic message is a log, and this is
            // the one type whose whole design is that its secret never reaches one.
            let rendered = payload
                .downcast_ref::<String>()
                .map_or_else(String::new, Clone::clone);
            for secret in ["t\r\nX-Injected: 1", "k\r\nX-Injected: 1", "t\0"] {
                assert!(
                    !rendered.contains(secret),
                    "the panic message quoted credential material: {rendered}"
                );
            }
        }
    }

    #[test]
    fn ordinary_and_awkward_but_legal_credentials_are_not_rejected() {
        // The over-refusal guard for the check above. A bearer token is base64url-ish in
        // practice but is not required to be, and a Basic password may legally contain
        // ANY byte — including the CR/LF that the verbatim paths refuse — because
        // `header()` base64-encodes it, and base64 output cannot forge a header.
        let profile = ServiceProfile::new(ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Credentials,
        ]))
        .with_credential(ServiceCredential::Bearer(
            "eyJhbGciOiJIUzI1NiJ9.e30.abc-_~+/=".to_owned(),
        ));
        assert_eq!(profile.request_headers().len(), 1);

        let profile = ServiceProfile::new(ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Credentials,
        ]))
        .with_credential(ServiceCredential::Basic {
            username: "user".to_owned(),
            password: "p\r\nass\0word — café".to_owned(),
        });
        let (name, value) = profile.request_headers().remove(0);
        assert_eq!(name, "Authorization");
        assert!(
            !value.contains(['\r', '\n', '\0']),
            "base64 cannot forge a header, which is why the bytes are allowed: {value}"
        );
        assert_eq!(
            value,
            format!(
                "Basic {}",
                base64_standard("user:p\r\nass\0word — café".as_bytes())
            )
        );

        // A credential-bearing header with RFC 9110 token punctuation and a non-ASCII
        // value is legal and in use, and must survive.
        let profile = ServiceProfile::new(ServiceCapabilities::granting([
            ServiceCapability::Query,
            ServiceCapability::Credentials,
        ]))
        .with_credential(ServiceCredential::Header {
            name: "X-Api-Key!#$%&'*+-.^_`|~".to_owned(),
            value: "a b=c;d,e/f+g café".to_owned(),
        });
        assert_eq!(profile.request_headers().len(), 1);
    }

    #[test]
    fn a_basic_username_with_a_colon_is_refused_rather_than_silently_split() {
        let outcome = std::panic::catch_unwind(|| {
            ServiceCredential::Basic {
                username: "user:name".to_owned(),
                password: "p".to_owned(),
            }
            .header()
        });
        assert!(
            outcome.is_err(),
            "RFC 7617 §2 forbids a colon in the user id"
        );
        // The neighbouring VALID case: a colon in the PASSWORD is explicitly allowed by
        // RFC 7617 and must still encode.
        let (_, value) = ServiceCredential::Basic {
            username: "user".to_owned(),
            password: "pass:word".to_owned(),
        }
        .header();
        assert_eq!(
            value,
            format!("Basic {}", base64_standard(b"user:pass:word"))
        );
    }
}
