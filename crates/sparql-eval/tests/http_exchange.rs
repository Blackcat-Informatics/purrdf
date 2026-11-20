// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `SERVICE` and `LOAD` through the one caller-supplied [`HttpExchange`].
//!
//! Every test drives one host adapter, [`Host`], through both consumers of the exchange:
//! [`HttpRemoteQuerySource`] for `SERVICE` and [`HttpGraphResolver`] for `LOAD`. The host
//! answers from a route table and records each request it is asked to carry out, so "the
//! request was refused before the exchange" is told apart from "the exchange ran and its
//! answer was refused". Each refusal is executed beside a neighbouring request that must
//! still succeed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_core::ResourceDimension;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SilencedKind, SilencedTarget, SparqlRequest,
    StopCause, TermValue, TrippedGovernor,
};
use purrdf_sparql_eval::{
    CancellationFlag, GraphResolveRequest, GraphResolver, HttpExchange, HttpExchangeRequest,
    HttpFailure, HttpGraphResolver, HttpMethod, HttpOutcome, HttpRemoteQuerySource, HttpResponse,
    HttpStatus, LoadError, NativeSparqlEngine, QueryGovernors, QueryOptions, RemoteError,
    ServiceCapabilities, ServiceCapability, ServiceCatalog, ServiceProfile, ServiceRequest,
    ServiceResolver, StopSignal,
};

const PREDICATE: &str = "http://example.org/body";

/// A SPARQL Results body binding `?n` once.
const RESULTS: &[u8] = br#"{"head":{"vars":["n"]},"results":{"bindings":[
    {"n":{"type":"literal","value":"remote"}}
]}}"#;

/// What [`Host`] answers for one target.
#[derive(Debug, Clone)]
enum Route {
    /// A `200` response with this body and Content-Type, and optionally a final URL.
    Document {
        body: &'static [u8],
        content_type: Option<&'static str>,
        final_url: Option<&'static str>,
    },
    /// A response with this status and `Location`.
    Redirect(u16, Option<&'static str>),
    /// A response with this status and an empty body.
    Status(u16),
    /// The opaque redirect a browser reports for an unfollowed cross-origin redirect.
    Opaque,
    /// A dataset the host already holds.
    Dataset,
    /// A typed failure.
    Fail(HttpFailure),
}

/// One request as the host saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    method: HttpMethod,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// A network-free host adapter: answers from `routes` (by target), records every
/// request it carries out, and fails a target it has no route for as a transport failure.
#[derive(Debug)]
struct Host {
    routes: Vec<(&'static str, Route)>,
    seen: Mutex<Vec<Seen>>,
}

impl Host {
    fn new(routes: Vec<(&'static str, Route)>) -> Self {
        Self {
            routes,
            seen: Mutex::new(Vec::new()),
        }
    }

    fn targets(&self) -> Vec<String> {
        self.seen
            .lock()
            .expect("lock")
            .iter()
            .map(|seen| seen.target.clone())
            .collect()
    }

    fn take(&self) -> Vec<Seen> {
        std::mem::take(&mut *self.seen.lock().expect("lock"))
    }
}

impl HttpExchange for &Host {
    fn exchange(&self, request: HttpExchangeRequest<'_>) -> Result<HttpOutcome, HttpFailure> {
        self.seen.lock().expect("lock").push(Seen {
            method: request.method,
            target: request.target.to_owned(),
            headers: request
                .headers()
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            body: request.body.to_vec(),
        });
        let route = self
            .routes
            .iter()
            .find(|(target, _)| *target == request.target)
            .map(|(_, route)| route.clone());
        let status = |code| HttpStatus::new(code).expect("a fixture status is valid");
        match route {
            Some(Route::Document {
                body,
                content_type,
                final_url,
            }) => {
                let mut response = HttpResponse::new(HttpStatus::OK, body.to_vec());
                if let Some(content_type) = content_type {
                    response = response.with_header("Content-Type", content_type);
                }
                if let Some(url) = final_url {
                    response = response.with_final_url(url);
                }
                Ok(HttpOutcome::Response(response))
            }
            Some(Route::Redirect(code, location)) => {
                let mut response = HttpResponse::new(status(code), Vec::new());
                if let Some(location) = location {
                    response = response.with_header("Location", location);
                }
                Ok(HttpOutcome::Response(response))
            }
            Some(Route::Status(code)) => Ok(HttpOutcome::Response(HttpResponse::new(
                status(code),
                Vec::new(),
            ))),
            Some(Route::Opaque) => Ok(HttpOutcome::OpaqueRedirect),
            Some(Route::Dataset) => Ok(HttpOutcome::Dataset(document("held", "urn:held"))),
            Some(Route::Fail(failure)) => Err(failure),
            None => Err(HttpFailure::Transport(format!(
                "no route for {}",
                request.target
            ))),
        }
    }
}

/// A one-quad dataset `<base> <PREDICATE> "text"`.
fn document(text: &str, base: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri(base).expect("base IRI");
    let predicate = builder.intern_iri(PREDICATE).expect("predicate IRI");
    let object = builder
        .intern_literal(RdfLiteral::simple(text))
        .expect("literal");
    builder
        .push_quad(subject, predicate, object, None)
        .expect("quad");
    builder.freeze().expect("freeze")
}

/// A decoder for `text/plain` only: the body becomes one literal about the base IRI, so
/// a test can read back both the bytes and the base the resolver parsed them against.
fn decode(body: &[u8], media_type: &str, base: &str) -> Result<Arc<RdfDataset>, String> {
    if media_type != "text/plain" {
        return Err(format!("unsupported media type {media_type:?}"));
    }
    let text = std::str::from_utf8(body).map_err(|error| error.to_string())?;
    Ok(document(text, base))
}

type Decoder = fn(&[u8], &str, &str) -> Result<Arc<RdfDataset>, String>;

fn resolver(host: &Host) -> HttpGraphResolver<&Host, Decoder> {
    HttpGraphResolver::new(host, decode as Decoder, "text/plain")
}

fn load(resolver: &dyn GraphResolver, iri: &str) -> Result<Arc<RdfDataset>, LoadError> {
    resolver.resolve(GraphResolveRequest { iri, stop: None })
}

fn service(source: &dyn ServiceResolver, endpoint: &str) -> Result<usize, RemoteError> {
    source
        .resolve(ServiceRequest::new(
            endpoint,
            "SELECT ?n WHERE { ?s ?p ?n }",
        ))
        .map(|bindings| bindings.rows.len())
}

/// Assert that `dataset` is the document [`decode`] produced from `text` against `base`.
fn assert_document(dataset: &RdfDataset, base: &str, text: &str) {
    assert_eq!(dataset.quad_count(), 1);
    assert!(
        dataset
            .term_id_by_value(&TermValue::Iri(base.to_owned()))
            .is_some(),
        "the document was parsed against <{base}>"
    );
    assert!(
        dataset
            .term_id_by_value(&TermValue::simple_literal(text))
            .is_some(),
        "the document carries {text:?}"
    );
}

/// A catalog that grants `query` and `network` to exactly these `open.example` targets,
/// each adding one profile header and its own User-Agent.
fn open_catalog() -> ServiceCatalog {
    [
        "https://open.example/sparql",
        "https://open.example/doc",
        "https://open.example/escape",
    ]
    .into_iter()
    .fold(ServiceCatalog::default(), |catalog, endpoint| {
        catalog.with_service(
            endpoint,
            ServiceProfile::new(ServiceCapabilities::granting([
                ServiceCapability::Query,
                ServiceCapability::Network,
            ]))
            .with_header("X-Profile", "open")
            .with_user_agent("purrdf-test/1.0"),
        )
    })
}

/// The one host carries both operations, each with its own method, fixed headers in send
/// order, the catalog profile's headers after them, and its body.
#[test]
fn service_and_load_share_one_exchange_with_their_own_request_shapes() {
    let host = Host::new(vec![
        (
            "https://open.example/sparql",
            Route::Document {
                body: RESULTS,
                content_type: Some("application/sparql-results+json"),
                final_url: None,
            },
        ),
        (
            "https://open.example/doc",
            Route::Document {
                body: b"loaded",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ),
    ]);
    let source = HttpRemoteQuerySource::new(&host).with_catalog(open_catalog());
    let graphs = resolver(&host).with_catalog(open_catalog());

    assert_eq!(service(&source, "https://open.example/sparql"), Ok(1));
    let loaded = load(&graphs, "https://open.example/doc").expect("the document loads");
    assert_document(&loaded, "https://open.example/doc", "loaded");

    let owned = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|&(name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    };
    assert_eq!(
        host.take(),
        vec![
            Seen {
                method: HttpMethod::Post,
                target: "https://open.example/sparql".to_owned(),
                headers: owned(&[
                    ("content-type", "application/sparql-query"),
                    ("accept", "application/sparql-results+json"),
                    ("user-agent", "purrdf-test/1.0"),
                    ("X-Profile", "open"),
                ]),
                body: b"SELECT ?n WHERE { ?s ?p ?n }".to_vec(),
            },
            Seen {
                method: HttpMethod::Get,
                target: "https://open.example/doc".to_owned(),
                headers: owned(&[
                    ("accept", "text/plain"),
                    ("user-agent", "purrdf-test/1.0"),
                    ("X-Profile", "open"),
                ]),
                body: Vec::new(),
            },
        ]
    );
}

/// A catalog refusal happens before the exchange, for `SERVICE`, for `LOAD`, and for a
/// `LOAD` redirect into a refused origin, while the allowed neighbour is fetched.
#[test]
fn a_catalog_denial_never_reaches_the_host_beside_an_allowed_neighbour() {
    let host = Host::new(vec![
        (
            "https://open.example/sparql",
            Route::Document {
                body: RESULTS,
                content_type: None,
                final_url: None,
            },
        ),
        (
            "https://open.example/doc",
            Route::Document {
                body: b"allowed",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ),
        (
            "https://open.example/escape",
            Route::Redirect(302, Some("https://closed.example/doc")),
        ),
    ]);
    let source = HttpRemoteQuerySource::new(&host).with_catalog(open_catalog());
    let graphs = resolver(&host).with_catalog(open_catalog());

    assert!(matches!(
        service(&source, "https://closed.example/sparql"),
        Err(RemoteError::Denied(_))
    ));
    assert!(matches!(
        load(&graphs, "https://closed.example/doc"),
        Err(LoadError::Denied(_))
    ));
    assert!(host.targets().is_empty(), "a denial issues no request");

    assert_eq!(service(&source, "https://open.example/sparql"), Ok(1));
    assert!(load(&graphs, "https://open.example/doc").is_ok());
    let Err(LoadError::Denied(denial)) = load(&graphs, "https://open.example/escape") else {
        panic!("a redirect into a refused origin is the catalog's denial");
    };
    assert_eq!(denial.endpoint(), "https://closed.example/doc");
    assert_eq!(denial.withheld(), ServiceCapability::Network);
    assert_eq!(
        host.targets(),
        [
            "https://open.example/sparql",
            "https://open.example/doc",
            "https://open.example/escape",
        ]
    );
}

/// Each typed host failure reaches `SERVICE` and `LOAD` as the variant it names, never as
/// a status, and a neighbouring target on the same host still answers.
#[test]
fn each_host_failure_is_its_typed_error_for_service_and_load() {
    let stopped = TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    };
    let cases = [
        (
            HttpFailure::Transport("reset".to_owned()),
            RemoteError::Transport("reset".to_owned()),
            LoadError::Transport("reset".to_owned()),
        ),
        (
            HttpFailure::Denied("rate limit".to_owned()),
            RemoteError::HostDenied {
                endpoint: "https://open.example/failing".to_owned(),
                message: "rate limit".to_owned(),
            },
            LoadError::HostDenied("rate limit".to_owned()),
        ),
        (
            HttpFailure::Fault("handler threw".to_owned()),
            RemoteError::HostFault {
                endpoint: "https://open.example/failing".to_owned(),
                message: "handler threw".to_owned(),
            },
            LoadError::Fault("handler threw".to_owned()),
        ),
        (
            HttpFailure::Decode("unreadable".to_owned()),
            RemoteError::Decode("unreadable".to_owned()),
            LoadError::Decode("unreadable".to_owned()),
        ),
        (
            HttpFailure::Governed(stopped),
            RemoteError::Governed(stopped),
            LoadError::Governed(stopped),
        ),
        (
            HttpFailure::GovernedAfterCompletion(stopped),
            RemoteError::GovernedAfterCompletion(stopped),
            LoadError::Governed(stopped),
        ),
    ];
    for (failure, service_error, load_error) in cases {
        let host = Host::new(vec![
            ("https://open.example/failing", Route::Fail(failure.clone())),
            (
                "https://open.example/sparql",
                Route::Document {
                    body: RESULTS,
                    content_type: None,
                    final_url: None,
                },
            ),
            (
                "https://open.example/doc",
                Route::Document {
                    body: b"fine",
                    content_type: Some("text/plain"),
                    final_url: None,
                },
            ),
        ]);
        let source = HttpRemoteQuerySource::new(&host);
        let graphs = resolver(&host);
        assert_eq!(
            service(&source, "https://open.example/failing"),
            Err(service_error),
            "{failure:?}"
        );
        assert_eq!(
            load(&graphs, "https://open.example/failing").map(|_| ()),
            Err(load_error),
            "{failure:?}"
        );
        assert_eq!(service(&source, "https://open.example/sparql"), Ok(1));
        assert!(load(&graphs, "https://open.example/doc").is_ok());
    }
}

/// An opaque redirect has no status to read, so neither operation guesses: `SERVICE`
/// and `LOAD` each refuse it by name, and an ordinary response from the same host is
/// still accepted.
#[test]
fn an_opaque_redirect_is_refused_by_name_by_service_and_load() {
    let host = Host::new(vec![
        ("https://open.example/opaque", Route::Opaque),
        (
            "https://open.example/sparql",
            Route::Document {
                body: RESULTS,
                content_type: None,
                final_url: None,
            },
        ),
        (
            "https://open.example/doc",
            Route::Document {
                body: b"visible",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ),
    ]);
    let source = HttpRemoteQuerySource::new(&host);
    let graphs = resolver(&host);

    let refused = service(&source, "https://open.example/opaque").expect_err("refused");
    assert_eq!(
        refused,
        RemoteError::OpaqueRedirect {
            endpoint: "https://open.example/opaque".to_owned()
        }
    );
    assert!(refused.to_string().contains("opaque redirect"), "{refused}");
    assert_eq!(service(&source, "https://open.example/sparql"), Ok(1));

    let refused = load(&graphs, "https://open.example/opaque").expect_err("refused");
    assert_eq!(
        refused,
        LoadError::OpaqueRedirect("https://open.example/opaque".to_owned())
    );
    assert_eq!(refused.code(), "native-sparql-load-failed");
    assert!(refused.to_string().contains("opaque redirect"), "{refused}");
    assert!(load(&graphs, "https://open.example/doc").is_ok());
}

/// A `LOAD` hop that is redirected again into an opaque redirect is refused with the
/// IRI of the hop that was opaque.
#[test]
fn an_opaque_redirect_after_a_followed_hop_names_that_hop() {
    let host = Host::new(vec![
        (
            "https://open.example/moved",
            Route::Redirect(301, Some("/opaque")),
        ),
        ("https://open.example/opaque", Route::Opaque),
    ]);
    assert_eq!(
        load(&resolver(&host), "https://open.example/moved").map(|_| ()),
        Err(LoadError::OpaqueRedirect(
            "https://open.example/opaque".to_owned()
        ))
    );
}

/// `SERVICE SILENT` absorbs an opaque redirect and a host failure as the invocation
/// failures they are, recording each; without `SILENT` the same refusal fails the query.
#[test]
fn service_silent_absorbs_exchange_refusals_and_records_them() {
    let host = Host::new(vec![
        ("https://open.example/opaque", Route::Opaque),
        (
            "https://open.example/denying",
            Route::Fail(HttpFailure::Denied("not today".to_owned())),
        ),
        (
            "https://open.example/sparql",
            Route::Document {
                body: RESULTS,
                content_type: None,
                final_url: None,
            },
        ),
    ]);
    let source = HttpRemoteQuerySource::new(&host);
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let run = |query: &str| {
        NativeSparqlEngine::new().query_governed_with_source(
            &dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            &source,
            &QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED,
        )
    };
    let outcome = run("SELECT * WHERE { \
         SERVICE SILENT <https://open.example/opaque> { ?s ?p ?o } \
         SERVICE SILENT <https://open.example/denying> { ?s ?p ?o } \
         SERVICE <https://open.example/sparql> { ?s ?p ?n } }")
    .expect("SILENT absorbs both refusals");
    let silenced: Vec<(String, SilencedKind)> = outcome
        .evidence()
        .silenced
        .iter()
        .map(|record| (record.target.name().to_owned(), record.kind))
        .collect();
    assert_eq!(
        silenced,
        [
            (
                "https://open.example/denying".to_owned(),
                SilencedKind::HostDenied
            ),
            (
                "https://open.example/opaque".to_owned(),
                SilencedKind::Transport
            ),
        ]
    );
    let error = run("SELECT * WHERE { SERVICE <https://open.example/opaque> { ?s ?p ?o } }")
        .expect_err("without SILENT the refusal fails the query");
    assert!(
        error.message.contains("opaque redirect"),
        "{}",
        error.message
    );
}

/// `LOAD SILENT` absorbs an opaque redirect as a transport failure, recorded on the
/// evidence; the neighbouring `LOAD` in the same request is applied.
#[test]
fn load_silent_absorbs_an_opaque_redirect_beside_an_applied_load() {
    let host = Host::new(vec![
        ("https://open.example/opaque", Route::Opaque),
        (
            "https://open.example/doc",
            Route::Document {
                body: b"applied",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ),
    ]);
    let graphs = resolver(&host);
    let mut dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let outcome = NativeSparqlEngine::new()
        .update_governed(
            &mut dataset,
            SparqlRequest {
                query: "LOAD SILENT <https://open.example/opaque> ; \
                        LOAD <https://open.example/doc>",
                base_iri: None,
                substitutions: &[],
            },
            &QueryOptions::EMPTY.with_load(Some(&graphs)),
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the update runs");
    assert!(outcome.is_applied(), "{outcome:?}");
    let silenced: Vec<_> = outcome.evidence().silenced.iter().cloned().collect();
    assert_eq!(silenced.len(), 1, "{silenced:?}");
    assert_eq!(
        silenced[0].target,
        SilencedTarget::Load {
            iri: "https://open.example/opaque".to_owned()
        }
    );
    assert_eq!(silenced[0].kind, SilencedKind::Transport);
    assert_eq!(dataset.quad_count(), 1);
}

/// `LOAD` follows a relative redirect to its resolved target and parses the document
/// against the URL the host reports it finally fetched; without one, against the IRI of
/// the hop that answered.
#[test]
fn load_follows_redirects_and_parses_against_the_final_url() {
    let host = Host::new(vec![
        (
            "https://open.example/a/start",
            Route::Redirect(307, Some("../b/doc")),
        ),
        (
            "https://open.example/b/doc",
            Route::Document {
                body: b"redirected",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ),
        (
            "https://open.example/mirror",
            Route::Document {
                body: b"mirrored",
                content_type: Some("text/plain"),
                final_url: Some("https://cdn.example/real"),
            },
        ),
    ]);
    let graphs = resolver(&host);
    let followed = load(&graphs, "https://open.example/a/start").expect("followed");
    assert_document(&followed, "https://open.example/b/doc", "redirected");
    let mirrored = load(&graphs, "https://open.example/mirror").expect("mirrored");
    assert_document(&mirrored, "https://cdn.example/real", "mirrored");
}

/// The refusals of the `LOAD` response path, each beside the neighbouring response the
/// same rule accepts.
#[test]
fn load_response_refusals_each_beside_an_accepted_neighbour() {
    let doc = Route::Document {
        body: b"ok",
        content_type: Some("text/plain"),
        final_url: None,
    };
    let host = Host::new(vec![
        ("https://open.example/ok", doc),
        // A status that is not a success is a transport failure; 2xx is accepted.
        ("https://open.example/missing", Route::Status(404)),
        // 304 is not a redirect status, even with a Location; 303 is.
        (
            "https://open.example/not-modified",
            Route::Redirect(304, Some("/ok")),
        ),
        (
            "https://open.example/see-other",
            Route::Redirect(303, Some("/ok")),
        ),
        // A redirect status without a Location cannot be followed.
        ("https://open.example/nowhere", Route::Redirect(302, None)),
        // A document without a Content-Type cannot be parsed; with one it can.
        (
            "https://open.example/untyped",
            Route::Document {
                body: b"ok",
                content_type: None,
                final_url: None,
            },
        ),
        // The decoder's own refusal is the decode failure, verbatim.
        (
            "https://open.example/turtle",
            Route::Document {
                body: b"ok",
                content_type: Some("text/turtle"),
                final_url: None,
            },
        ),
        // A dataset the host already holds is the document as it stands.
        ("https://open.example/held", Route::Dataset),
    ]);
    let graphs = resolver(&host);
    assert!(load(&graphs, "https://open.example/ok").is_ok());
    assert!(load(&graphs, "https://open.example/see-other").is_ok());
    assert!(load(&graphs, "https://open.example/held").is_ok());
    assert_eq!(
        load(&graphs, "https://open.example/missing").map(|_| ()),
        Err(LoadError::Transport(
            "<https://open.example/missing> answered HTTP 404".to_owned()
        ))
    );
    assert!(matches!(
        load(&graphs, "https://open.example/not-modified"),
        Err(LoadError::Transport(_))
    ));
    assert_eq!(
        load(&graphs, "https://open.example/nowhere").map(|_| ()),
        Err(LoadError::Transport(
            "<https://open.example/nowhere> answered HTTP 302 without a Location".to_owned()
        ))
    );
    assert!(matches!(
        load(&graphs, "https://open.example/untyped"),
        Err(LoadError::Decode(_))
    ));
    assert_eq!(
        load(&graphs, "https://open.example/turtle").map(|_| ()),
        Err(LoadError::Decode(
            "unsupported media type \"text/turtle\"".to_owned()
        ))
    );
}

/// `LOAD` follows exactly `MAX_LOAD_REDIRECTS` hops and refuses the next.
#[test]
fn load_follows_the_redirect_limit_and_refuses_one_hop_more() {
    let chain = |hops: usize| {
        let names: Vec<&'static str> = (0..=hops)
            .map(|hop| &*Box::leak(format!("https://open.example/{hop}").into_boxed_str()))
            .collect();
        let mut routes: Vec<(&'static str, Route)> = names
            .windows(2)
            .map(|pair| (pair[0], Route::Redirect(302, Some(pair[1]))))
            .collect();
        routes.push((
            names[hops],
            Route::Document {
                body: b"end",
                content_type: Some("text/plain"),
                final_url: None,
            },
        ));
        Host::new(routes)
    };
    let limit = usize::try_from(purrdf_sparql_eval::remote_http::MAX_LOAD_REDIRECTS)
        .expect("a small limit");
    let within = chain(limit);
    assert!(load(&resolver(&within), "https://open.example/0").is_ok());
    let beyond = chain(limit + 1);
    assert!(matches!(
        load(&resolver(&beyond), "https://open.example/0"),
        Err(LoadError::Transport(message)) if message.starts_with("exceeded 5 redirect hops")
    ));
}

/// A `SERVICE` response must be a success: a redirect or an error status is the
/// transport failure it is, and is never followed.
#[test]
fn a_service_response_that_is_not_a_success_is_a_transport_failure() {
    let host = Host::new(vec![
        (
            "https://open.example/moved",
            Route::Redirect(302, Some("https://open.example/sparql")),
        ),
        (
            "https://open.example/sparql",
            Route::Document {
                body: RESULTS,
                content_type: None,
                final_url: None,
            },
        ),
    ]);
    let source = HttpRemoteQuerySource::new(&host);
    assert_eq!(
        service(&source, "https://open.example/moved"),
        Err(RemoteError::Transport(
            "<https://open.example/moved> answered HTTP 302".to_owned()
        ))
    );
    assert_eq!(host.targets(), ["https://open.example/moved"]);
    assert_eq!(service(&source, "https://open.example/sparql"), Ok(1));
}

/// A status code outside `100..=599` is not a status, so no response can carry one.
#[test]
fn a_status_outside_the_three_digit_range_is_refused() {
    assert_eq!(HttpStatus::new(0), None);
    assert_eq!(HttpStatus::new(600), None);
    assert_eq!(HttpStatus::new(99), None);
    assert_eq!(HttpStatus::new(100).map(HttpStatus::code), Some(100));
    assert_eq!(HttpStatus::new(599).map(HttpStatus::code), Some(599));
}

/// A stop signal that fires while the host holds one hop prevents the next hop: the
/// redirect is not followed, and the `LOAD` is the governor's stop, not a transport
/// failure. The same chain with no stop fired is followed to its document.
#[test]
fn a_stop_fired_during_a_hop_prevents_the_next_redirect_hop() {
    let flag = CancellationFlag::new();
    let signal: Arc<dyn StopSignal> = Arc::new(flag.clone());
    let hops = Mutex::new(Vec::<String>::new());
    let chain = |cancel: bool| {
        let flag = flag.clone();
        let hops = &hops;
        move |request: HttpExchangeRequest<'_>| -> Result<HttpOutcome, HttpFailure> {
            hops.lock().expect("lock").push(request.target.to_owned());
            if request.target.ends_with("/start") {
                if cancel {
                    flag.cancel();
                }
                let response = HttpResponse::new(HttpStatus::FOUND, Vec::new())
                    .with_header("Location", "/end");
                return Ok(HttpOutcome::Response(response));
            }
            Ok(HttpOutcome::Response(
                HttpResponse::new(HttpStatus::OK, b"end".to_vec())
                    .with_header("Content-Type", "text/plain"),
            ))
        }
    };
    let request = |iri| GraphResolveRequest {
        iri,
        stop: Some(&*signal),
    };
    let quiet = HttpGraphResolver::new(chain(false), decode as Decoder, "text/plain");
    assert!(quiet.resolve(request("https://open.example/start")).is_ok());
    assert_eq!(hops.lock().expect("lock").len(), 2);
    hops.lock().expect("lock").clear();
    let stopped = HttpGraphResolver::new(chain(true), decode as Decoder, "text/plain");
    assert_eq!(
        stopped
            .resolve(request("https://open.example/start"))
            .map(|_| ()),
        Err(LoadError::Governed(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }))
    );
    assert_eq!(
        hops.lock().expect("lock").len(),
        1,
        "the second hop is never fetched"
    );
}

/// A relative `Location` is resolved against the URL the host reports for the hop it
/// already followed, and against the hop's IRI when it reports none. The catalog still
/// authorizes the resulting target: a catalog that grants only the hop's own start IRI
/// refuses the host-reported destination before it is fetched.
#[test]
fn a_relative_location_resolves_against_the_hosts_final_url() {
    let targets = Mutex::new(Vec::<String>::new());
    let exchange = |report_final: bool| {
        let targets = &targets;
        move |request: HttpExchangeRequest<'_>| -> Result<HttpOutcome, HttpFailure> {
            targets
                .lock()
                .expect("lock")
                .push(request.target.to_owned());
            if request.target.ends_with("/start") {
                let mut response =
                    HttpResponse::new(HttpStatus::FOUND, Vec::new()).with_header("Location", "end");
                if report_final {
                    response = response.with_final_url("https://moved.example/y/mid");
                }
                return Ok(HttpOutcome::Response(response));
            }
            Ok(HttpOutcome::Response(
                HttpResponse::new(HttpStatus::OK, b"end".to_vec())
                    .with_header("Content-Type", "text/plain"),
            ))
        }
    };
    let grant = |endpoint: &str| {
        ServiceCatalog::default().with_service(
            endpoint,
            ServiceProfile::new(ServiceCapabilities::granting([ServiceCapability::Network])),
        )
    };
    let catalog = grant("https://open.example/x/start")
        .with_service(
            "https://moved.example/y/end",
            ServiceProfile::new(ServiceCapabilities::granting([ServiceCapability::Network])),
        )
        .with_service(
            "https://open.example/x/end",
            ServiceProfile::new(ServiceCapabilities::granting([ServiceCapability::Network])),
        );

    let reported = HttpGraphResolver::new(exchange(true), decode as Decoder, "text/plain")
        .with_catalog(catalog.clone());
    let dataset = load(&reported, "https://open.example/x/start").expect("the final URL base");
    assert_document(&dataset, "https://moved.example/y/end", "end");
    assert_eq!(
        *targets.lock().expect("lock"),
        [
            "https://open.example/x/start",
            "https://moved.example/y/end"
        ]
    );

    targets.lock().expect("lock").clear();
    let unreported = HttpGraphResolver::new(exchange(false), decode as Decoder, "text/plain")
        .with_catalog(catalog);
    let dataset = load(&unreported, "https://open.example/x/start").expect("the hop base");
    assert_document(&dataset, "https://open.example/x/end", "end");

    // Authorization applies to the resolved target: with only the hop's own base granted,
    // the host-reported destination is refused before it is fetched.
    targets.lock().expect("lock").clear();
    let narrow = HttpGraphResolver::new(exchange(true), decode as Decoder, "text/plain")
        .with_catalog(grant("https://open.example/x/start"));
    assert!(matches!(
        load(&narrow, "https://open.example/x/start"),
        Err(LoadError::Denied(_))
    ));
    assert_eq!(targets.lock().expect("lock").len(), 1);
}

/// A stop that fires during the final `200` hop is the governor's stop, and the body is
/// never decoded. The same hop with no stop is decoded.
#[test]
fn a_stop_fired_during_the_final_hop_is_never_decoded() {
    let flag = CancellationFlag::new();
    let signal: Arc<dyn StopSignal> = Arc::new(flag.clone());
    let decoded = AtomicUsize::new(0);
    let decoder = |body: &[u8], media_type: &str, base: &str| {
        decoded.fetch_add(1, Ordering::SeqCst);
        decode(body, media_type, base)
    };
    let exchange = |cancel: bool| {
        let flag = flag.clone();
        move |_request: HttpExchangeRequest<'_>| -> Result<HttpOutcome, HttpFailure> {
            if cancel {
                flag.cancel();
            }
            Ok(HttpOutcome::Response(
                HttpResponse::new(HttpStatus::OK, b"doc".to_vec())
                    .with_header("Content-Type", "text/plain"),
            ))
        }
    };
    let request = GraphResolveRequest {
        iri: "https://open.example/doc",
        stop: Some(&*signal),
    };
    let quiet = HttpGraphResolver::new(exchange(false), &decoder, "text/plain");
    assert!(quiet.resolve(request).is_ok());
    assert_eq!(decoded.load(Ordering::SeqCst), 1);

    let stopped = HttpGraphResolver::new(exchange(true), &decoder, "text/plain");
    assert_eq!(
        stopped.resolve(request).map(|_| ()),
        Err(LoadError::Governed(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }))
    );
    assert_eq!(
        decoded.load(Ordering::SeqCst),
        1,
        "the cancelled body is not decoded"
    );
}

/// `max_remote_requests` bounds host fetches: with a ceiling of 1 a `LOAD` whose first
/// response is a redirect is refused before the second fetch is dispatched, and with a
/// ceiling of 2 the same chain loads.
#[test]
fn the_remote_request_ceiling_bounds_each_redirect_hop() {
    let run = |ceiling: u64| {
        let fetches = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&fetches);
        let exchange = move |request: HttpExchangeRequest<'_>| -> Result<HttpOutcome, HttpFailure> {
            counter.fetch_add(1, Ordering::SeqCst);
            if request.target.ends_with("/start") {
                return Ok(HttpOutcome::Response(
                    HttpResponse::new(HttpStatus::FOUND, Vec::new())
                        .with_header("Location", "/end"),
                ));
            }
            Ok(HttpOutcome::Response(
                HttpResponse::new(HttpStatus::OK, b"end".to_vec())
                    .with_header("Content-Type", "text/plain"),
            ))
        };
        let resolver = HttpGraphResolver::new(exchange, decode as Decoder, "text/plain");
        let engine = NativeSparqlEngine::new().with_resolver(Arc::new(resolver));
        let mut dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
        let outcome = engine
            .update_governed(
                &mut dataset,
                SparqlRequest {
                    query: "LOAD <https://open.example/start>",
                    base_iri: None,
                    substitutions: &[],
                },
                &QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_max_remote_requests(ceiling),
            )
            .expect("a tripped governor is an outcome");
        (outcome.tripped(), fetches.load(Ordering::SeqCst), dataset)
    };

    let (tripped, fetches, dataset) = run(1);
    assert_eq!(
        tripped,
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::RemoteRequests,
            limit: 1,
            consumed: 2,
        })
    );
    assert_eq!(
        fetches, 1,
        "the redirect hop is refused before it is fetched"
    );
    assert_eq!(dataset.quad_count(), 0);

    let (tripped, fetches, dataset) = run(2);
    assert_eq!(tripped, None);
    assert_eq!(fetches, 2);
    assert_eq!(dataset.quad_count(), 1);
}
