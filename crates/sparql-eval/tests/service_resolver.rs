// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-service context on the `SERVICE` seam: capability gating, headers, credentials,
//! and the `SILENT` contract through both resolver implementations.
//!
//! Every "no network access occurred" assertion here is made with a **live network path
//! present and counting**: the transport that would have been used increments a counter
//! before it does anything else, so a zero is evidence the request was prevented rather
//! than evidence the test forgot to wire one up. Each such test is paired with a
//! neighbouring case that drives the same counter above zero, so the zero cannot be
//! satisfied vacuously.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{
    HttpRemoteQuerySource, HttpRequest, InProcessServiceResolver, NativeSparqlEngine, QueryOptions,
    RemoteError, ServiceCapabilities, ServiceCapability, ServiceCatalog, ServiceCredential,
    ServiceProfile, ServiceRequest, ServiceResolver, ServiceRouter,
};

/// The in-process service, and the one every gated fixture lists.
const LOCAL_EP: &str = "https://example.org/in-process/sparql";
/// A service the in-process resolver has no dataset for, used for the network path.
const NET_EP: &str = "https://example.org/remote/sparql";

/// One row, so a swallowed `SERVICE` (the join identity) is distinguishable from a
/// dropped one (zero rows).
fn local_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let knows = b.intern_iri("https://example.org/vocab#knows");
    let a = b.intern_iri("https://example.org/a");
    let x = b.intern_iri("https://example.org/x");
    b.push_quad(a, knows, x, None);
    b.freeze().expect("freeze")
}

/// `:x :name "in-process"` — the dataset behind [`LOCAL_EP`].
fn service_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let name = b.intern_iri("https://example.org/vocab#name");
    let x = b.intern_iri("https://example.org/x");
    let value = b.intern_literal(RdfLiteral::simple("in-process"));
    b.push_quad(x, name, value, None);
    b.freeze().expect("freeze")
}

/// A SPARQL Results body binding `?n` once.
const REMOTE_JSON: &[u8] = br#"{"head":{"vars":["n"]},"results":{"bindings":[
    {"n":{"type":"literal","value":"from-the-network"}}
]}}"#;

/// An HTTP transport that counts every exchange it is asked to perform and records the
/// headers it was handed.
#[derive(Debug, Default)]
struct SpyTransport {
    /// How many exchanges were requested.
    posts: AtomicUsize,
    /// The headers of the most recent request.
    headers: Mutex<Vec<(String, String)>>,
    /// The user agent and timeout of the most recent request.
    agent_and_timeout: Mutex<Option<(String, Duration)>>,
}

impl SpyTransport {
    /// How many exchanges this transport has been asked to perform.
    fn posts(&self) -> usize {
        self.posts.load(Ordering::Relaxed)
    }

    /// The headers of the most recent request.
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.lock().expect("lock").clone()
    }
}

impl purrdf_sparql_eval::HttpTransport for &SpyTransport {
    fn post(&self, request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
        // Counted FIRST, before any inspection can bail out: a test asserting zero is
        // then asserting the request was never issued, not that it was declined here.
        self.posts.fetch_add(1, Ordering::Relaxed);
        *self.headers.lock().expect("lock") = request.headers.to_vec();
        *self.agent_and_timeout.lock().expect("lock") =
            Some((request.user_agent.to_owned(), request.timeout));
        Ok(REMOTE_JSON.to_vec())
    }
}

/// Run `query` against [`local_dataset`] with `resolver` injected and `base_iri` in
/// scope for relative IRI references.
fn run_with_base(
    resolver: &(dyn ServiceResolver + Sync),
    query: &str,
    base_iri: Option<&str>,
) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    NativeSparqlEngine::new().query_with_source(
        &local_dataset(),
        SparqlRequest {
            query,
            base_iri,
            substitutions: &[],
        },
        resolver,
        QueryOptions::EMPTY,
    )
}

/// Run `query` against [`local_dataset`] with `resolver` injected and no base IRI.
fn run(
    resolver: &(dyn ServiceResolver + Sync),
    query: &str,
) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    run_with_base(resolver, query, None)
}

/// The solution rows of `result`.
fn rows(result: &SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// A `SELECT` whose `SERVICE` clause targets `endpoint`.
fn service_query(endpoint: &str, silent: bool) -> String {
    let silent = if silent { "SILENT " } else { "" };
    format!(
        "SELECT ?o ?n WHERE {{ ?s <https://example.org/vocab#knows> ?o \
         SERVICE {silent}<{endpoint}> {{ ?x <https://example.org/vocab#name> ?n }} }}"
    )
}

/// A profile granting exactly `capabilities`.
fn profile(capabilities: &[ServiceCapability]) -> ServiceProfile {
    ServiceProfile::new(ServiceCapabilities::granting(capabilities.iter().copied()))
}

// ── The in-process resolver performs no network I/O ──────────────────────────────

#[test]
fn an_in_process_service_is_answered_without_the_network_transport_being_touched() {
    // A router with a LIVE network path as its fallback: the in-process service must be
    // answered without that path being used at all.
    let spy = SpyTransport::default();
    let network = HttpRemoteQuerySource::new(&spy);
    let in_process = InProcessServiceResolver::new().with_endpoint(LOCAL_EP, service_dataset());
    let router = ServiceRouter::new()
        .with_route(LOCAL_EP, &in_process)
        .with_fallback(&network);

    let result =
        run(&router, &service_query(LOCAL_EP, false)).expect("the in-process service answers");
    assert_eq!(
        rows(&result),
        1,
        "the answer came from the in-memory dataset"
    );
    assert_eq!(
        spy.posts(),
        0,
        "resolving an in-process service must not perform network I/O"
    );

    // The neighbouring case that proves the zero above is a real observation: the SAME
    // router, the SAME transport, a service that is NOT routed in process — and the
    // counter moves. Without this, `posts() == 0` would also pass for a router that
    // never resolved anything.
    let result = run(&router, &service_query(NET_EP, false)).expect("the fallback answers");
    assert_eq!(rows(&result), 1);
    assert_eq!(
        spy.posts(),
        1,
        "the network path is live and reachable — the zero above was a prevented request"
    );
}

#[test]
fn withholding_the_network_capability_prevents_the_exchange_rather_than_discarding_it() {
    let spy = SpyTransport::default();
    let denied = HttpRemoteQuerySource::new(&spy).with_catalog(
        ServiceCatalog::new().with_service(NET_EP, profile(&[ServiceCapability::Query])),
    );
    let err = run(&denied, &service_query(NET_EP, false))
        .expect_err("a service denied the network capability cannot be resolved");
    assert!(
        err.message.contains("withholds the network capability"),
        "the diagnostic must name the capability that was withheld: {}",
        err.message
    );
    assert_eq!(
        spy.posts(),
        0,
        "the policy runs BEFORE the transport: a denied service never opens a socket"
    );

    // The neighbouring VALID case: the same catalog with `Network` granted resolves, and
    // the transport is reached exactly once.
    let allowed =
        HttpRemoteQuerySource::new(&spy).with_catalog(ServiceCatalog::new().with_service(
            NET_EP,
            profile(&[ServiceCapability::Query, ServiceCapability::Network]),
        ));
    let result = run(&allowed, &service_query(NET_EP, false))
        .expect("granting Network must let the very same query through");
    assert_eq!(rows(&result), 1);
    assert_eq!(spy.posts(), 1);
}

#[test]
fn a_catalog_gates_a_service_nested_inside_a_forwarded_body_too() {
    // The bypass this closes: an in-process resolver evaluates a forwarded body itself,
    // so a `SERVICE` NESTED in that body is resolved by a second pass through the
    // resolver. If the nested pass used an ungated dataset map, a query could reach an
    // endpoint the catalog refuses at the top level simply by nesting one level down.
    let inner_ep = "https://example.org/in-process/inner";
    let in_process = InProcessServiceResolver::new()
        .with_endpoint(LOCAL_EP, service_dataset())
        .with_endpoint(inner_ep, service_dataset())
        // Only the OUTER service is listed. The inner one has a dataset but no profile.
        .with_catalog(
            ServiceCatalog::new().with_service(LOCAL_EP, profile(&[ServiceCapability::Query])),
        );

    let err = run(
        &in_process,
        &format!(
            "SELECT ?n WHERE {{ SERVICE <{LOCAL_EP}> {{ \
             SERVICE <{inner_ep}> {{ ?x <https://example.org/vocab#name> ?n }} }} }}"
        ),
    )
    .expect_err("the nested service is uncatalogued and must be denied");
    assert!(
        err.message.contains("withholds the query capability"),
        "the nested resolution must go through the same gate: {}",
        err.message
    );

    // The neighbouring VALID case: list the inner service too and the identical nested
    // query answers. Without this, the denial above could be a nested `SERVICE` that
    // simply never works.
    let in_process = InProcessServiceResolver::new()
        .with_endpoint(LOCAL_EP, service_dataset())
        .with_endpoint(inner_ep, service_dataset())
        .with_catalog(
            ServiceCatalog::new()
                .with_service(LOCAL_EP, profile(&[ServiceCapability::Query]))
                .with_service(inner_ep, profile(&[ServiceCapability::Query])),
        );
    let result = run(
        &in_process,
        &format!(
            "SELECT ?n WHERE {{ SERVICE <{LOCAL_EP}> {{ \
             SERVICE <{inner_ep}> {{ ?x <https://example.org/vocab#name> ?n }} }} }}"
        ),
    )
    .expect("a nested service that IS catalogued must resolve normally");
    assert_eq!(rows(&result), 1);
}

// ── The SILENT contract ──────────────────────────────────────────────────────────

#[test]
fn silent_swallows_an_unreachable_endpoint_through_both_resolvers() {
    // Row one of the contract table, both implementations. One local row, so the join
    // identity (a no-op join) keeps it and a dropped clause would not.
    let unreachable = HttpRemoteQuerySource::new(|_: HttpRequest<'_>| {
        Err(RemoteError::Transport("connection refused".to_owned()))
    });
    let result = run(&unreachable, &service_query(NET_EP, true))
        .expect("SILENT swallows an unreachable endpoint");
    assert_eq!(
        rows(&result),
        1,
        "the join identity leaves the surrounding query unchanged"
    );
    let err = run(&unreachable, &service_query(NET_EP, false))
        .expect_err("without SILENT the same failure aborts the query");
    assert!(err.message.contains("SERVICE"), "got {}", err.message);

    // The in-process resolver reports an endpoint it has no dataset for the same way.
    let empty = InProcessServiceResolver::new();
    let result =
        run(&empty, &service_query(LOCAL_EP, true)).expect("SILENT swallows a missing endpoint");
    assert_eq!(rows(&result), 1);
    run(&empty, &service_query(LOCAL_EP, false))
        .expect_err("without SILENT the same failure aborts the query");
}

#[test]
fn silent_never_swallows_a_capability_denial_through_either_resolver() {
    // Row two of the contract table: a denial is a decision taken on THIS side of the
    // seam, so `SILENT` — which promises only to tolerate an endpoint that does not
    // answer — does not hide it. If it did, the surrounding join would become a no-op
    // and the answer would look complete and be wrong on every single run.
    let spy = SpyTransport::default();
    let network = HttpRemoteQuerySource::new(&spy).with_catalog(
        ServiceCatalog::new().with_service(NET_EP, profile(&[ServiceCapability::Query])),
    );
    let err =
        run(&network, &service_query(NET_EP, true)).expect_err("SILENT must not swallow a denial");
    assert!(
        err.message.contains("withholds the network capability"),
        "got {}",
        err.message
    );
    assert_eq!(spy.posts(), 0);

    let in_process = InProcessServiceResolver::new()
        .with_endpoint(LOCAL_EP, service_dataset())
        .with_catalog(ServiceCatalog::new());
    let err = run(&in_process, &service_query(LOCAL_EP, true))
        .expect_err("SILENT must not swallow a denial here either");
    assert!(
        err.message.contains("withholds the query capability"),
        "got {}",
        err.message
    );

    // The neighbouring VALID case for BOTH: grant the capability and the identical
    // SILENT query answers. A refusal that fired for every SILENT query would pass the
    // assertions above while proving nothing.
    let in_process = InProcessServiceResolver::new()
        .with_endpoint(LOCAL_EP, service_dataset())
        .with_catalog(
            ServiceCatalog::new().with_service(LOCAL_EP, profile(&[ServiceCapability::Query])),
        );
    let result = run(&in_process, &service_query(LOCAL_EP, true))
        .expect("a granted SILENT service resolves normally");
    assert_eq!(rows(&result), 1);

    let network =
        HttpRemoteQuerySource::new(&spy).with_catalog(ServiceCatalog::new().with_service(
            NET_EP,
            profile(&[ServiceCapability::Query, ServiceCapability::Network]),
        ));
    let result = run(&network, &service_query(NET_EP, true))
        .expect("a granted SILENT service resolves normally");
    assert_eq!(rows(&result), 1);
    assert_eq!(spy.posts(), 1);
}

#[test]
fn an_unrouted_service_is_denied_by_the_router_even_under_silent() {
    let router = ServiceRouter::new();
    let err = run(&router, &service_query(NET_EP, true))
        .expect_err("a router with no route and no fallback denies");
    assert!(
        err.message
            .contains("no resolver is routed to this service"),
        "got {}",
        err.message
    );

    // Neighbouring VALID case: add the route and the same query answers.
    let in_process = InProcessServiceResolver::new().with_endpoint(NET_EP, service_dataset());
    let router = ServiceRouter::new().with_route(NET_EP, &in_process);
    assert_eq!(
        rows(&run(&router, &service_query(NET_EP, true)).expect("the routed service answers")),
        1
    );
}

// ── Per-service headers, credentials and overrides ───────────────────────────────

#[test]
fn a_profiles_headers_and_credential_reach_the_transport_in_order() {
    let spy = SpyTransport::default();
    let source = HttpRemoteQuerySource::new(&spy)
        .with_timeout(Duration::from_secs(11))
        .with_catalog(
            ServiceCatalog::new().with_service(
                NET_EP,
                profile(&[
                    ServiceCapability::Query,
                    ServiceCapability::Network,
                    ServiceCapability::Credentials,
                ])
                .with_header("X-Tenant", "acme")
                .with_header("Accept-Language", "fr-CA, en;q=0.8")
                .with_credential(ServiceCredential::Basic {
                    username: "Aladdin".to_owned(),
                    password: "open sesame".to_owned(),
                })
                .with_user_agent("purrdf-test/1.0")
                .with_timeout(Duration::from_secs(7)),
            ),
        );
    run(&source, &service_query(NET_EP, false)).expect("the granted service resolves");

    // Exact, not "contains": a header list that silently gained or lost an entry is
    // precisely the accounting error this asserts against.
    assert_eq!(
        spy.headers(),
        vec![
            ("X-Tenant".to_owned(), "acme".to_owned()),
            ("Accept-Language".to_owned(), "fr-CA, en;q=0.8".to_owned()),
            (
                "Authorization".to_owned(),
                "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==".to_owned()
            ),
        ],
        "the profile's headers in order, then the credential"
    );
    let (agent, timeout) = spy
        .agent_and_timeout
        .lock()
        .expect("lock")
        .clone()
        .expect("a request was made");
    assert_eq!(agent, "purrdf-test/1.0", "the profile overrides the source");
    assert_eq!(timeout, Duration::from_secs(7));
}

#[test]
fn a_source_with_no_catalog_sends_exactly_what_it_always_did() {
    // The byte-level compatibility guard: configuring nothing must change nothing. A
    // source with no catalog adds no headers and keeps its own agent and timeout.
    let spy = SpyTransport::default();
    let source = HttpRemoteQuerySource::new(&spy).with_timeout(Duration::from_secs(11));
    run(&source, &service_query(NET_EP, false)).expect("an ungated source resolves any service");

    assert!(
        spy.headers().is_empty(),
        "no catalog means no extra headers: {:?}",
        spy.headers()
    );
    let (agent, timeout) = spy
        .agent_and_timeout
        .lock()
        .expect("lock")
        .clone()
        .expect("a request was made");
    assert!(agent.contains("purrdf-sparql-eval"), "got {agent}");
    assert_eq!(timeout, Duration::from_secs(11));
}

#[test]
fn a_catalogued_profile_that_adds_nothing_also_sends_nothing_extra() {
    // The other half of the compatibility guard: gating a service is not, by itself, a
    // change to the request. Only what a profile actually carries is added.
    let spy = SpyTransport::default();
    let source = HttpRemoteQuerySource::new(&spy).with_catalog(ServiceCatalog::new().with_service(
        NET_EP,
        profile(&[ServiceCapability::Query, ServiceCapability::Network]),
    ));
    run(&source, &service_query(NET_EP, false)).expect("the granted service resolves");
    assert!(spy.headers().is_empty(), "got {:?}", spy.headers());
}

// ── A service IRI is resolved by the workspace's one base layer ──────────────────

/// A resolver that records every endpoint it is handed and resolves nothing.
#[derive(Debug, Default)]
struct EndpointSpy(Mutex<Vec<String>>);

impl ServiceResolver for EndpointSpy {
    fn resolve(
        &self,
        request: ServiceRequest<'_>,
    ) -> Result<purrdf_sparql_eval::ResolvedBindings, RemoteError> {
        self.0
            .lock()
            .expect("lock")
            .push(request.endpoint.to_owned());
        Err(RemoteError::Transport("observed".to_owned()))
    }
}

impl EndpointSpy {
    /// The endpoints this resolver has been handed, in order.
    fn seen(&self) -> Vec<String> {
        self.0.lock().expect("lock").clone()
    }
}

#[test]
fn a_relative_service_iri_is_refused_without_a_base_and_resolved_with_one() {
    // A service IRI is an IRI like any other, so it goes through the workspace's single
    // RFC 3986 resolution layer rather than through a rule this seam invented: a relative
    // reference with no base in scope is the shared `iri-relative-no-base` hard error,
    // raised while the query is parsed and therefore before any resolver is consulted.
    // A resolver keys its per-service profile off `ServiceRequest::endpoint`, so a raw
    // relative string arriving here would silently miss every catalog entry — a denial,
    // or under a fallback an unintended grant, for a reason nothing in the catalog says.
    let spy = EndpointSpy::default();
    let query = "SELECT ?n WHERE { SERVICE <sparql> { ?x <https://example.org/vocab#name> ?n } }";
    let err = run_with_base(&spy, query, None)
        .expect_err("a relative service IRI with no base cannot be resolved");
    // The engine's own parse code on the outside, the shared layer's `iri-relative-no-base`
    // carried verbatim on the inside — the SERVICE seam contributes no rule of its own.
    assert_eq!(err.code, "native-sparql-query-parse", "got {err:?}");
    assert!(
        err.message.contains("iri-relative-no-base"),
        "the shared base layer raises this, not the SERVICE seam: {err:?}"
    );
    assert!(
        spy.seen().is_empty(),
        "the refusal precedes evaluation, so no resolver saw a relative endpoint: {:?}",
        spy.seen()
    );

    // …and `SILENT` does not soften it. `SILENT` is a promise about an endpoint that does
    // not answer, and an endpoint IRI that cannot be resolved is a malformed query rather
    // than an endpoint that failed — swallowing it to the join identity would answer a
    // query nobody wrote.
    let silent =
        "SELECT ?n WHERE { SERVICE SILENT <sparql> { ?x <https://example.org/vocab#name> ?n } }";
    let err = run_with_base(&spy, silent, None)
        .expect_err("SILENT does not make an unresolvable endpoint IRI resolvable");
    assert!(err.message.contains("iri-relative-no-base"), "got {err:?}");
    assert!(spy.seen().is_empty(), "got {:?}", spy.seen());

    // The neighbouring VALID case: the identical query with a base in scope resolves, and
    // the resolver is handed the ABSOLUTE endpoint — the form a catalog is keyed on.
    run_with_base(&spy, query, Some("https://example.org/remote/"))
        .expect_err("the spy resolves nothing, but the endpoint reached it");
    assert_eq!(
        spy.seen(),
        vec!["https://example.org/remote/sparql".to_owned()],
        "the seam receives the resolved absolute IRI, never the relative reference"
    );

    // And an in-query `BASE` works the same way, through the same layer.
    let spy = EndpointSpy::default();
    run_with_base(
        &spy,
        &format!("BASE <https://example.org/remote/> {query}"),
        None,
    )
    .expect_err("the spy resolves nothing, but the endpoint reached it");
    assert_eq!(spy.seen(), vec![NET_EP.to_owned()]);
}

#[test]
fn the_silent_flag_reaches_the_resolver() {
    // The flag is carried so a POLICY can depend on it — refusing `SERVICE SILENT`
    // against a credentialed service is the motivating rule, because an auth failure
    // swallowed to the join identity is a silent wrong answer. This pins that the flag
    // actually arrives, in both states.
    #[derive(Debug, Default)]
    struct SilentSpy(Mutex<Vec<bool>>);

    impl ServiceResolver for SilentSpy {
        fn resolve(
            &self,
            request: ServiceRequest<'_>,
        ) -> Result<purrdf_sparql_eval::ResolvedBindings, RemoteError> {
            self.0.lock().expect("lock").push(request.silent);
            Err(RemoteError::Transport("observed".to_owned()))
        }
    }

    let spy = SilentSpy::default();
    run(&spy, &service_query(NET_EP, true)).expect("SILENT swallows the transport error");
    run(&spy, &service_query(NET_EP, false)).expect_err("the non-silent clause fails");
    assert_eq!(
        *spy.0.lock().expect("lock"),
        vec![true, false],
        "the resolver sees the clause's own SILENT flag, not a fixed value"
    );
}
