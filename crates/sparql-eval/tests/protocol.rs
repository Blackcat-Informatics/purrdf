// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SPARQL 1.1 Protocol request surface, from the public API only.
//!
//! Every refusal is paired with a neighbouring request that is valid and is asserted to
//! parse, so a refusal that reaches one step too far fails here rather than in front of
//! a client. The dataset parameters are asserted through the engine: the spliced text is
//! executed, and every restricted answer is compared against the unrestricted answer of
//! the same request without the parameter — which the fixture makes different.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;
use purrdf_sparql_eval::protocol::{
    OperationKind, ProtocolError, ProtocolRequest, ResultKind, format_media_type, negotiate,
};

const EX: &str = "http://example.org/";
const FORM: Option<&str> = Some("application/x-www-form-urlencoded");
const SPARQL_QUERY: Option<&str> = Some("application/sparql-query");
const SPARQL_UPDATE: Option<&str> = Some("application/sparql-update");

fn get(query_string: &str) -> Result<ProtocolRequest, ProtocolError> {
    ProtocolRequest::parse("GET", None, Some(query_string), b"")
}

fn post(
    content_type: Option<&str>,
    query_string: Option<&str>,
    body: &str,
) -> Result<ProtocolRequest, ProtocolError> {
    ProtocolRequest::parse("POST", content_type, query_string, body.as_bytes())
}

/// Percent-encode everything but the unreserved characters, as a form encoder would.
fn enc(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            write!(out, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    out
}

fn ok(result: Result<ProtocolRequest, ProtocolError>) -> ProtocolRequest {
    result.unwrap_or_else(|err| panic!("a valid request was refused: {err}"))
}

fn err(result: Result<ProtocolRequest, ProtocolError>) -> ProtocolError {
    match result {
        Ok(request) => panic!("an invalid request was accepted: {request:?}"),
        Err(err) => err,
    }
}

// ── request forms ────────────────────────────────────────────────────────────────────

#[test]
fn a_query_via_get_is_read_from_the_query_string() {
    let query = "SELECT ?s WHERE { ?s ?p \"a b+c\" }";
    let request = ok(get(&format!(
        "?query={}&default-graph-uri={}&default-graph-uri={}&named-graph-uri={}&timeout=5",
        enc(query),
        enc("http://example.org/g1"),
        enc("http://example.org/g2"),
        enc("http://example.org/n")
    )));
    assert_eq!(request.kind(), OperationKind::Query);
    assert_eq!(request.text(), query);
    assert_eq!(
        request.default_graph_uris(),
        ["http://example.org/g1", "http://example.org/g2"],
        "repeatable, in request order"
    );
    assert_eq!(request.named_graph_uris(), ["http://example.org/n"]);
    assert_eq!(request.using_graph_uris(), [] as [String; 0]);
    assert_eq!(
        request.extra_parameters(),
        [("timeout".to_owned(), "5".to_owned())],
        "a parameter the protocol does not define is kept, not dropped"
    );

    // `+` is a space in URL-encoded text; `%2B` is a plus. The leading `?` is optional.
    let request = ok(get("query=ASK+%7B+%3Fs+%3Fp+1%2B1+%7D"));
    assert_eq!(request.text(), "ASK { ?s ?p 1+1 }");
}

#[test]
fn a_query_via_post_directly_reads_the_body_and_the_query_string_parameters() {
    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let request = ok(post(
        SPARQL_QUERY,
        Some("default-graph-uri=http%3A%2F%2Fexample.org%2Fg1"),
        query,
    ));
    assert_eq!(request.kind(), OperationKind::Query);
    assert_eq!(request.text(), query);
    assert_eq!(request.default_graph_uris(), ["http://example.org/g1"]);

    // Media types are case-insensitive, and parameters other than charset are ignored.
    for content_type in [
        "Application/SPARQL-Query",
        "application/sparql-query; charset=utf-8",
        "application/sparql-query;charset=\"UTF-8\"",
        "application/sparql-query ; profile=x",
    ] {
        let request = ok(post(Some(content_type), None, query));
        assert_eq!(request.text(), query, "for `{content_type}`");
    }
}

#[test]
fn a_query_via_post_with_url_encoded_parameters_reads_the_body() {
    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let request = ok(post(
        FORM,
        None,
        &format!(
            "query={}&named-graph-uri={}&named-graph-uri={}",
            enc(query),
            enc("http://example.org/n1"),
            enc("http://example.org/n2")
        ),
    ));
    assert_eq!(request.kind(), OperationKind::Query);
    assert_eq!(request.text(), query);
    assert_eq!(
        request.named_graph_uris(),
        ["http://example.org/n1", "http://example.org/n2"]
    );

    // A parameter a client also wrote in the URL is honoured, not dropped.
    let request = ok(post(
        Some("Application/X-WWW-Form-Urlencoded; charset=UTF-8"),
        Some("default-graph-uri=http%3A%2F%2Fexample.org%2Fg"),
        &format!("query={}", enc(query)),
    ));
    assert_eq!(request.default_graph_uris(), ["http://example.org/g"]);
}

#[test]
fn an_update_via_post_with_url_encoded_parameters_reads_the_body() {
    let update = "INSERT DATA { <http://example.org/s> <http://example.org/p> \"é\" }";
    let request = ok(post(
        FORM,
        None,
        &format!(
            "update={}&using-graph-uri={}&using-named-graph-uri={}",
            enc(update),
            enc("http://example.org/g"),
            enc("http://example.org/n")
        ),
    ));
    assert_eq!(request.kind(), OperationKind::Update);
    assert_eq!(request.text(), update);
    assert_eq!(request.using_graph_uris(), ["http://example.org/g"]);
    assert_eq!(request.using_named_graph_uris(), ["http://example.org/n"]);
    assert_eq!(request.result_kind(), Ok(None));
}

#[test]
fn an_update_via_post_directly_reads_the_body() {
    let update = "CLEAR DEFAULT";
    let request = ok(post(
        SPARQL_UPDATE,
        Some("using-named-graph-uri=http%3A%2F%2Fexample.org%2Fn"),
        update,
    ));
    assert_eq!(request.kind(), OperationKind::Update);
    assert_eq!(request.text(), update);
    assert_eq!(request.using_named_graph_uris(), ["http://example.org/n"]);

    // An empty update is a valid request (a request with no operations), not a missing one.
    let request = ok(post(SPARQL_UPDATE, None, ""));
    assert_eq!(request.text(), "");
}

#[test]
fn the_result_kind_is_read_from_the_query_form() {
    for (query, kind) in [
        ("SELECT * { ?s ?p ?o }", ResultKind::Solutions),
        ("ask { ?s ?p ?o }", ResultKind::Boolean),
        ("CONSTRUCT WHERE { ?s ?p ?o }", ResultKind::Graph),
        ("DESCRIBE <http://example.org/s>", ResultKind::Graph),
        (
            "BASE <http://example.org/>\nPREFIX select: <http://example.org/select#>\n\
             VERSION \"1.2\"\nconstruct { ?s select:p ?o } WHERE { ?s select:p ?o }",
            ResultKind::Graph,
        ),
    ] {
        let request = ok(post(SPARQL_QUERY, None, query));
        assert_eq!(request.result_kind(), Ok(Some(kind)), "for `{query}`");
    }
    let request = ok(post(SPARQL_QUERY, None, "PREFIX ex: <http://example.org/>"));
    assert!(matches!(
        request.result_kind(),
        Err(ProtocolError::MalformedOperation { .. })
    ));
}

// ── refusals, each with a valid neighbour ───────────────────────────────────────────

#[test]
fn a_method_other_than_get_or_post_is_refused() {
    let q = format!("query={}", enc("ASK {}"));
    for method in ["PUT", "DELETE", "get", "HEAD"] {
        let refused = err(ProtocolRequest::parse(method, None, Some(&q), b""));
        assert_eq!(
            refused,
            ProtocolError::UnsupportedMethod {
                method: method.to_owned()
            }
        );
        assert_eq!(refused.name(), "UnsupportedMethod");
    }
    ok(ProtocolRequest::parse("GET", None, Some(&q), b""));
}

#[test]
fn a_post_without_a_content_type_is_refused() {
    assert_eq!(
        err(post(None, None, "ASK {}")),
        ProtocolError::MissingContentType
    );
    ok(post(SPARQL_QUERY, None, "ASK {}"));
}

#[test]
fn a_post_with_an_unsupported_content_type_is_refused() {
    let refused = err(post(Some("Text/Plain; charset=utf-8"), None, "ASK {}"));
    assert_eq!(
        refused,
        ProtocolError::UnsupportedContentType {
            content_type: "text/plain".to_owned()
        }
    );
    assert!(refused.to_string().contains("`text/plain`"), "{refused}");
    ok(post(Some("application/sparql-query"), None, "ASK {}"));
}

#[test]
fn a_charset_other_than_utf8_is_refused() {
    for content_type in [
        "application/sparql-query; charset=iso-8859-1",
        "application/x-www-form-urlencoded; charset=\"windows-1252\"",
    ] {
        assert!(
            matches!(
                err(post(Some(content_type), None, "query=ASK+%7B%7D")),
                ProtocolError::UnsupportedCharset { .. }
            ),
            "for `{content_type}`"
        );
    }
    ok(post(
        Some("application/sparql-query; CHARSET=Utf-8"),
        None,
        "ASK {}",
    ));
    ok(post(
        Some("application/x-www-form-urlencoded; charset=\"UTF-8\""),
        None,
        "query=ASK+%7B%7D",
    ));
}

#[test]
fn a_body_that_is_not_utf8_is_refused() {
    let refused = err(ProtocolRequest::parse(
        "POST",
        SPARQL_QUERY,
        None,
        b"ASK { ?s ?p \"\xff\" }",
    ));
    assert_eq!(refused, ProtocolError::NonUtf8Body { valid_up_to: 13 });
    let request = ok(ProtocolRequest::parse(
        "POST",
        SPARQL_QUERY,
        None,
        "ASK { ?s ?p \"é\" }".as_bytes(),
    ));
    assert_eq!(request.text(), "ASK { ?s ?p \"é\" }");

    // A form body is held to the same rule.
    assert!(matches!(
        err(ProtocolRequest::parse("POST", FORM, None, b"query=\xff")),
        ProtocolError::MalformedForm { .. }
    ));
}

#[test]
fn a_get_with_a_body_is_refused() {
    let q = format!("query={}", enc("ASK {}"));
    assert_eq!(
        err(ProtocolRequest::parse("GET", None, Some(&q), b"x")),
        ProtocolError::BodyOnGet
    );
    ok(ProtocolRequest::parse("GET", None, Some(&q), b""));
}

#[test]
fn url_encoded_text_that_does_not_decode_is_refused() {
    for bad in [
        "query=ASK%zz",
        "query=ASK%2",
        "query=%",
        "query=%FF",
        "query%G0=x",
    ] {
        assert!(
            matches!(err(get(bad)), ProtocolError::MalformedForm { .. }),
            "for `{bad}`"
        );
        assert!(
            matches!(
                err(post(FORM, None, bad)),
                ProtocolError::MalformedForm { .. }
            ),
            "for form `{bad}`"
        );
    }
    // The neighbours: a literal `%` spelled `%25`, a two-byte UTF-8 sequence, lower-case
    // hex digits, an empty pair and a trailing `&`.
    assert_eq!(ok(get("query=ASK%25")).text(), "ASK%");
    assert_eq!(ok(get("query=%c3%a9")).text(), "é");
    assert_eq!(ok(get("&query=ASK+%7B%7D&")).text(), "ASK {}");
}

#[test]
fn a_request_with_no_operation_is_refused() {
    assert_eq!(
        err(get("default-graph-uri=http%3A%2F%2Fexample.org%2Fg")),
        ProtocolError::MissingOperation
    );
    assert_eq!(err(get("")), ProtocolError::MissingOperation);
    assert_eq!(
        err(ProtocolRequest::parse("GET", None, None, b"")),
        ProtocolError::MissingOperation
    );
    assert_eq!(
        err(post(
            FORM,
            None,
            "default-graph-uri=http%3A%2F%2Fexample.org%2Fg"
        )),
        ProtocolError::MissingOperation
    );
    ok(get(
        "query=ASK+%7B%7D&default-graph-uri=http%3A%2F%2Fexample.org%2Fg",
    ));
    ok(post(FORM, None, "update=CLEAR+ALL"));
}

#[test]
fn a_request_with_more_than_one_operation_is_refused() {
    for (content_type, query_string, body) in [
        (FORM, None, "query=ASK+%7B%7D&update=CLEAR+ALL"),
        (FORM, None, "query=ASK+%7B%7D&query=ASK+%7B%7D"),
        (FORM, None, "update=CLEAR+ALL&update=CLEAR+ALL"),
        (FORM, Some("query=ASK+%7B%7D"), "query=ASK+%7B%7D"),
        (SPARQL_QUERY, Some("query=ASK+%7B%7D"), "ASK {}"),
        (SPARQL_QUERY, Some("update=CLEAR+ALL"), "ASK {}"),
        (SPARQL_UPDATE, Some("update=CLEAR+ALL"), "CLEAR ALL"),
        (SPARQL_UPDATE, Some("query=ASK+%7B%7D"), "CLEAR ALL"),
    ] {
        let refused = err(post(content_type, query_string, body));
        assert!(
            matches!(refused, ProtocolError::AmbiguousOperation { .. }),
            "for {content_type:?} {query_string:?} `{body}`: {refused:?}"
        );
    }
    assert!(matches!(
        err(get("query=ASK+%7B%7D&query=ASK+%7B%7D")),
        ProtocolError::AmbiguousOperation { .. }
    ));
    // The neighbours: one operation each, with the dataset parameters beside it.
    ok(post(FORM, None, "query=ASK+%7B%7D"));
    ok(post(FORM, None, "update=CLEAR+ALL"));
    ok(post(
        SPARQL_QUERY,
        Some("default-graph-uri=http%3A%2F%2Fexample.org%2Fg"),
        "ASK {}",
    ));
    ok(post(
        SPARQL_UPDATE,
        Some("using-graph-uri=http%3A%2F%2Fexample.org%2Fg"),
        "CLEAR ALL",
    ));
    ok(get("query=ASK+%7B%7D"));
}

#[test]
fn an_update_via_get_is_refused() {
    let refused = err(get("update=CLEAR+ALL"));
    assert_eq!(refused, ProtocolError::QueryViaGetIsNotUpdate);
    assert_eq!(refused.parameter(), Some("update"));
    assert_eq!(
        err(get("query=ASK+%7B%7D&update=CLEAR+ALL")),
        ProtocolError::QueryViaGetIsNotUpdate
    );
    let request = ok(post(FORM, None, "update=CLEAR+ALL"));
    assert_eq!(request.kind(), OperationKind::Update);
}

#[test]
fn query_dataset_parameters_on_an_update_are_refused() {
    for (parameter, pair) in [
        (
            "default-graph-uri",
            "default-graph-uri=http%3A%2F%2Fexample.org%2Fg",
        ),
        (
            "named-graph-uri",
            "named-graph-uri=http%3A%2F%2Fexample.org%2Fg",
        ),
    ] {
        let refused = err(post(FORM, None, &format!("update=CLEAR+ALL&{pair}")));
        assert_eq!(
            refused,
            ProtocolError::DatasetParametersOnUpdate { parameter }
        );
        assert_eq!(refused.parameter(), Some(parameter));
        assert!(matches!(
            err(post(SPARQL_UPDATE, Some(pair), "CLEAR ALL")),
            ProtocolError::DatasetParametersOnUpdate { .. }
        ));
        // The same parameter on a query is the neighbour that parses.
        ok(post(FORM, None, &format!("query=ASK+%7B%7D&{pair}")));
    }
}

#[test]
fn update_dataset_parameters_on_a_query_are_refused() {
    for (parameter, pair) in [
        (
            "using-graph-uri",
            "using-graph-uri=http%3A%2F%2Fexample.org%2Fg",
        ),
        (
            "using-named-graph-uri",
            "using-named-graph-uri=http%3A%2F%2Fexample.org%2Fg",
        ),
    ] {
        assert_eq!(
            err(get(&format!("query=ASK+%7B%7D&{pair}"))),
            ProtocolError::UsingParametersOnQuery { parameter }
        );
        assert!(matches!(
            err(post(SPARQL_QUERY, Some(pair), "ASK {}")),
            ProtocolError::UsingParametersOnQuery { .. }
        ));
        ok(post(FORM, None, &format!("update=CLEAR+ALL&{pair}")));
    }
}

#[test]
fn a_dataset_parameter_that_is_not_an_absolute_iri_is_refused() {
    for (value, encoded) in [
        ("g1", "g1"),
        ("/g1", "%2Fg1"),
        (
            "http://example.org/<x>",
            "http%3A%2F%2Fexample.org%2F%3Cx%3E",
        ),
        ("http://example.org/a b", "http%3A%2F%2Fexample.org%2Fa+b"),
    ] {
        let refused = err(get(&format!(
            "query=ASK+%7B%7D&default-graph-uri={encoded}"
        )));
        let ProtocolError::InvalidGraphIri {
            parameter,
            value: reported,
            ..
        } = &refused
        else {
            panic!("`{value}`: {refused:?}");
        };
        assert_eq!(*parameter, "default-graph-uri");
        assert_eq!(reported, value);
        assert!(refused.to_string().contains(value), "{refused}");
    }
    let refused = err(post(FORM, None, "update=CLEAR+ALL&using-named-graph-uri=x"));
    assert_eq!(refused.parameter(), Some("using-named-graph-uri"));
    // Absolute IRIs of other schemes, and a non-ASCII IRI, are the valid neighbours.
    for encoded in [
        "http%3A%2F%2Fexample.org%2Fg1",
        "urn%3Aexample%3Ag1",
        "http%3A%2F%2Fexample.org%2Fcaf%C3%A9",
    ] {
        ok(get(&format!(
            "query=ASK+%7B%7D&default-graph-uri={encoded}"
        )));
    }
}

// ── negotiation ─────────────────────────────────────────────────────────────────────

#[test]
fn negotiation_follows_weights_wildcards_specificity_and_preference_order() {
    use ResultKind::{Graph, Solutions};
    let table: &[(Option<&str>, ResultKind, Option<&str>)] = &[
        // No Accept header, or an empty one: the defaults.
        (None, Solutions, Some("json")),
        (None, Graph, Some("turtle")),
        (Some(""), Solutions, Some("json")),
        (Some("  "), Graph, Some("turtle")),
        // Exact media types, case-insensitively.
        (
            Some("application/sparql-results+json"),
            Solutions,
            Some("json"),
        ),
        (
            Some("application/sparql-results+xml"),
            Solutions,
            Some("xml"),
        ),
        (Some("TEXT/CSV"), Solutions, Some("csv")),
        (Some("text/tab-separated-values"), Solutions, Some("tsv")),
        (Some("text/turtle"), Graph, Some("turtle")),
        (Some("application/trig"), Graph, Some("trig")),
        (Some("application/n-triples"), Graph, Some("ntriples")),
        (Some("application/n-quads"), Graph, Some("nquads")),
        (Some("application/ld+json"), Graph, Some("jsonld")),
        // Wildcards: `*/*` takes the preferred format; `type/*` the preferred one of that type.
        (Some("*/*"), Solutions, Some("json")),
        (Some("*/*"), Graph, Some("turtle")),
        (Some("text/*"), Solutions, Some("csv")),
        (Some("application/*"), Graph, Some("trig")),
        // Weights: the higher one wins, whatever the order of the header.
        (
            Some("text/csv;q=0.5, application/sparql-results+xml;q=0.9"),
            Solutions,
            Some("xml"),
        ),
        (
            Some("application/sparql-results+json;q=0.1, text/tab-separated-values"),
            Solutions,
            Some("tsv"),
        ),
        (
            Some("application/n-quads;q=1.0, text/turtle;q=0.999"),
            Graph,
            Some("nquads"),
        ),
        // Specificity: an exact range's weight overrides a wildcard's for that type.
        (
            Some("*/*;q=0.9, application/sparql-results+json;q=0.1"),
            Solutions,
            Some("xml"),
        ),
        (Some("text/*;q=0.8, text/csv;q=0.2"), Solutions, Some("tsv")),
        (Some("*/*, text/turtle;q=0"), Graph, Some("trig")),
        // q=0 excludes, including through a wildcard.
        (Some("application/sparql-results+json;q=0"), Solutions, None),
        (Some("*/*;q=0"), Graph, None),
        (
            Some("text/*;q=0, application/sparql-results+xml;q=0.3"),
            Solutions,
            Some("xml"),
        ),
        // Ties go to the server's preference order.
        (
            Some("text/csv, application/sparql-results+xml"),
            Solutions,
            Some("xml"),
        ),
        (
            Some("application/ld+json;q=0.5, application/n-triples;q=0.5"),
            Graph,
            Some("ntriples"),
        ),
        // Media-type parameters make a range more specific; the weight is still read.
        (
            Some("text/turtle;charset=utf-8;q=0.4, */*;q=0.3"),
            Graph,
            Some("turtle"),
        ),
        // Nothing acceptable.
        (Some("text/html"), Solutions, None),
        (Some("application/sparql-results+json"), Graph, None),
        (Some("text/turtle"), Solutions, None),
        (Some("image/*"), Graph, None),
        // Malformed elements are skipped; a well-formed neighbour still counts.
        (
            Some("text/csv;q=2, text/tab-separated-values"),
            Solutions,
            Some("tsv"),
        ),
        (Some("text/csv;q=0.5555"), Solutions, None),
        (Some("*/csv"), Solutions, None),
        (Some("garbage, application/trig"), Graph, Some("trig")),
    ];
    for &(accept, kind, expected) in table {
        assert_eq!(
            negotiate(accept, kind),
            expected,
            "Accept: {accept:?} for {kind:?}"
        );
    }
}

#[test]
fn every_negotiated_format_has_its_media_type() {
    for (token, media) in [
        ("json", "application/sparql-results+json"),
        ("xml", "application/sparql-results+xml"),
        ("csv", "text/csv"),
        ("tsv", "text/tab-separated-values"),
        ("turtle", "text/turtle"),
        ("trig", "application/trig"),
        ("ntriples", "application/n-triples"),
        ("nquads", "application/n-quads"),
        ("jsonld", "application/ld+json"),
    ] {
        assert_eq!(format_media_type(token), Some(media));
        let kind = if ["json", "xml", "csv", "tsv"].contains(&token) {
            ResultKind::Solutions
        } else {
            ResultKind::Graph
        };
        assert_eq!(negotiate(Some(media), kind), Some(token));
    }
    assert_eq!(format_media_type("rdfxml"), None);
}

// ── dataset parameters, through the engine ──────────────────────────────────────────

/// The default graph holds `ex:a` and `ex:c`; graph `ex:g1` holds `ex:a`; graph `ex:g2`
/// holds `ex:b` — so the unrestricted default-graph answer ({a, c}), `FROM ex:g1` ({a})
/// and `FROM ex:g2` ({b}) are all different.
fn fixture() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let quad = |builder: &mut RdfDatasetBuilder, s: &str, o: &str, g: Option<&str>| {
        let s = builder.intern_iri(&format!("{EX}{s}"));
        let o = builder.intern_iri(&format!("{EX}{o}"));
        let g = g.map(|g| builder.intern_iri(&format!("{EX}{g}")));
        builder.push_quad(s, p, o, g);
    };
    quad(&mut builder, "a", "x1", None);
    quad(&mut builder, "c", "x0", None);
    quad(&mut builder, "a", "x1", Some("g1"));
    quad(&mut builder, "b", "x2", Some("g2"));
    builder.freeze().expect("freeze fixture")
}

fn sparql(text: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query: text,
        base_iri: None,
        substitutions: &[],
    }
}

/// Run a request's effective text as a query and return its rows as sorted strings of
/// local names, one string per row.
fn rows(dataset: &Arc<RdfDataset>, request: &ProtocolRequest) -> Vec<String> {
    let text = request
        .effective_text()
        .unwrap_or_else(|err| panic!("effective text: {err}"));
    let result = NativeSparqlEngine::new()
        .query(dataset, sparql(&text))
        .unwrap_or_else(|err| panic!("`{text}` evaluates: {err:?}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers solutions");
    };
    let mut out: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Some(TermValue::Iri(iri)) => iri.strip_prefix(EX).unwrap_or(iri).to_owned(),
                    other => format!("{other:?}"),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    out.sort();
    out
}

fn query_request(query: &str, params: &str) -> ProtocolRequest {
    ok(get(&format!("query={}{params}", enc(query))))
}

fn graph_param(name: &str, graph: &str) -> String {
    format!("&{name}={}", enc(&format!("{EX}{graph}")))
}

#[test]
fn default_graph_uri_restricts_the_default_graph() {
    let dataset = fixture();
    let query = "SELECT ?s WHERE { ?s <http://example.org/p> ?o }";
    let unrestricted = query_request(query, "");
    assert_eq!(
        unrestricted.effective_text().as_deref(),
        Ok(query),
        "no parameter, no splice"
    );
    assert_eq!(rows(&dataset, &unrestricted), ["a", "c"]);

    let restricted = query_request(query, &graph_param("default-graph-uri", "g1"));
    assert_eq!(
        restricted.effective_text().as_deref(),
        Ok("SELECT ?s  FROM <http://example.org/g1> WHERE { ?s <http://example.org/p> ?o }"),
        "the clause is written at the query's dataset position; nothing else changes"
    );
    assert_eq!(rows(&dataset, &restricted), ["a"]);

    // Two default graphs are merged.
    let merged = query_request(
        query,
        &format!(
            "{}{}",
            graph_param("default-graph-uri", "g1"),
            graph_param("default-graph-uri", "g2")
        ),
    );
    assert_eq!(rows(&dataset, &merged), ["a", "b"]);

    // A WHERE-less group and a prologue are handled the same way.
    let terse = "PREFIX ex: <http://example.org/>\nSELECT ?s{ ?s ex:p ?o }";
    assert_eq!(rows(&dataset, &query_request(terse, "")), ["a", "c"]);
    assert_eq!(
        rows(
            &dataset,
            &query_request(terse, &graph_param("default-graph-uri", "g2"))
        ),
        ["b"]
    );
}

#[test]
fn default_graph_uri_overrides_the_querys_own_from() {
    let dataset = fixture();
    let query = "SELECT ?s FROM <http://example.org/g2> WHERE { ?s <http://example.org/p> ?o }";
    // The neighbour: the query's own clause, honoured when no parameter overrides it.
    assert_eq!(rows(&dataset, &query_request(query, "")), ["b"]);
    let overridden = query_request(query, &graph_param("default-graph-uri", "g1"));
    assert_eq!(
        overridden.effective_text().as_deref(),
        Ok("SELECT ?s  FROM <http://example.org/g1> WHERE { ?s <http://example.org/p> ?o }"),
        "the query's own FROM is removed, not merged"
    );
    assert_eq!(rows(&dataset, &overridden), ["a"]);

    // A named-graph parameter alone also replaces the whole dataset: the default graph
    // is then empty (SPARQL 1.1 Query §13.2).
    let named_only = query_request(query, &graph_param("named-graph-uri", "g1"));
    assert_eq!(rows(&dataset, &named_only), [] as [String; 0]);
}

#[test]
fn named_graph_uri_restricts_graph_patterns() {
    let dataset = fixture();
    let query = "SELECT ?g ?s WHERE { GRAPH ?g { ?s <http://example.org/p> ?o } }";
    assert_eq!(rows(&dataset, &query_request(query, "")), ["g1 a", "g2 b"]);
    assert_eq!(
        rows(
            &dataset,
            &query_request(query, &graph_param("named-graph-uri", "g2"))
        ),
        ["g2 b"]
    );
    assert_eq!(
        rows(
            &dataset,
            &query_request(
                query,
                &format!(
                    "{}{}",
                    graph_param("named-graph-uri", "g1"),
                    graph_param("default-graph-uri", "g2")
                ),
            )
        ),
        ["g1 a"]
    );
    // Through a form POST, with the query's own FROM NAMED overridden.
    let own = "SELECT ?g ?s FROM NAMED <http://example.org/g1> WHERE { GRAPH ?g { ?s <http://example.org/p> ?o } }";
    assert_eq!(rows(&dataset, &query_request(own, "")), ["g1 a"]);
    let request = ok(post(
        FORM,
        None,
        &format!("query={}{}", enc(own), graph_param("named-graph-uri", "g2")),
    ));
    assert_eq!(rows(&dataset, &request), ["g2 b"]);
}

#[test]
fn the_dataset_parameters_apply_to_every_query_form() {
    let dataset = fixture();
    let engine = NativeSparqlEngine::new();
    let g1 = graph_param("default-graph-uri", "g1");
    // ASK: `ex:c` is in the default graph but not in g1.
    let ask = "ASK { <http://example.org/c> <http://example.org/p> ?o }";
    for (params, expected) in [("", true), (g1.as_str(), false)] {
        let text = query_request(ask, params)
            .effective_text()
            .expect("splices");
        let result = engine.query(&dataset, sparql(&text)).expect("evaluates");
        assert!(
            matches!(result, SparqlResult::Boolean(b) if b == expected),
            "`{text}`"
        );
    }
    // CONSTRUCT (both forms): the graph has one triple under g1, two without.
    for construct in [
        "CONSTRUCT { ?s <http://example.org/p> ?o } WHERE { ?s <http://example.org/p> ?o }",
        "CONSTRUCT WHERE { ?s <http://example.org/p> ?o }",
    ] {
        for (params, expected) in [("", 2), (g1.as_str(), 1)] {
            let text = query_request(construct, params)
                .effective_text()
                .expect("splices");
            let SparqlResult::Graph(graph) =
                engine.query(&dataset, sparql(&text)).expect("evaluates")
            else {
                panic!("a CONSTRUCT answers a graph");
            };
            assert_eq!(graph.quad_count(), expected, "`{text}`");
        }
    }
}

/// Run a request's effective text as an update over a fresh fixture and return the
/// subjects marked `ex:seen` (in any graph), and those the default graph still has `ex:p`
/// for.
fn after_update(request: &ProtocolRequest) -> (Vec<String>, Vec<String>) {
    let mut dataset = fixture();
    let text = request
        .effective_text()
        .unwrap_or_else(|err| panic!("effective text: {err}"));
    NativeSparqlEngine::new()
        .update(&mut dataset, sparql(&text))
        .unwrap_or_else(|err| panic!("`{text}` applies: {err:?}"));
    let seen = query_request(
        "SELECT ?s WHERE { { ?s <http://example.org/seen> ?o } UNION \
         { GRAPH ?g { ?s <http://example.org/seen> ?o } } }",
        "",
    );
    let edges = query_request("SELECT ?s WHERE { ?s <http://example.org/p> ?o }", "");
    (rows(&dataset, &seen), rows(&dataset, &edges))
}

fn update_request(update: &str, params: &str) -> ProtocolRequest {
    ok(post(FORM, None, &format!("update={}{params}", enc(update))))
}

#[test]
fn using_graph_uri_restricts_an_updates_where() {
    let update =
        "INSERT { ?s <http://example.org/seen> true } WHERE { ?s <http://example.org/p> ?o }";
    let unrestricted = update_request(update, "");
    assert_eq!(after_update(&unrestricted).0, ["a", "c"]);

    let restricted = update_request(update, &graph_param("using-graph-uri", "g2"));
    assert_eq!(
        restricted.effective_text().as_deref(),
        Ok(
            "INSERT { ?s <http://example.org/seen> true }  USING <http://example.org/g2> WHERE { ?s <http://example.org/p> ?o }"
        )
    );
    assert_eq!(after_update(&restricted).0, ["b"]);

    // USING NAMED makes a graph addressable in the WHERE, and only that graph.
    let named = "INSERT { ?s <http://example.org/seen> true } WHERE { GRAPH ?g { ?s <http://example.org/p> ?o } }";
    assert_eq!(after_update(&update_request(named, "")).0, ["a", "b"]);
    assert_eq!(
        after_update(&update_request(
            named,
            &graph_param("using-named-graph-uri", "g1")
        ))
        .0,
        ["a"]
    );

    // Every operation with a WHERE receives the clauses; one without is left alone.
    let several = "INSERT DATA { <http://example.org/d> <http://example.org/p> <http://example.org/x3> } ;\n\
                   DELETE { ?s <http://example.org/p> ?o } INSERT { ?s <http://example.org/seen> true } \
                   WHERE { ?s <http://example.org/p> ?o }";
    let (seen, edges) = after_update(&update_request(several, ""));
    assert_eq!(seen, ["a", "c", "d"]);
    assert_eq!(
        edges,
        [] as [String; 0],
        "every default-graph edge matched and was deleted"
    );
    let (seen, edges) = after_update(&update_request(
        several,
        &graph_param("using-graph-uri", "g1"),
    ));
    assert_eq!(seen, ["a"], "only the g1 match is marked");
    assert_eq!(
        edges,
        ["c", "d"],
        "only the g1 match is deleted from the default graph"
    );
}

#[test]
fn using_graph_uri_applies_to_the_delete_where_shorthand() {
    let update = "DELETE WHERE { ?s <http://example.org/p> ?o } # trailing comment";
    // The neighbour: without the parameter every default-graph edge goes.
    assert_eq!(
        after_update(&update_request(update, "")).1,
        [] as [String; 0]
    );
    let restricted = update_request(update, &graph_param("using-graph-uri", "g1"));
    assert_eq!(
        restricted.effective_text().as_deref(),
        Ok(
            "DELETE { ?s <http://example.org/p> ?o }\nUSING <http://example.org/g1> WHERE { ?s <http://example.org/p> ?o } # trailing comment"
        ),
        "the shorthand is written as the long form it is defined as"
    );
    assert_eq!(
        after_update(&restricted).1,
        ["c"],
        "only the edge g1 also holds is deleted from the default graph"
    );
}

#[test]
fn using_parameters_conflict_with_an_updates_own_using_or_with() {
    let g1 = graph_param("using-graph-uri", "g1");
    for (update, clause) in [
        (
            "INSERT { ?s <http://example.org/seen> true } USING <http://example.org/g2> WHERE { ?s <http://example.org/p> ?o }",
            "USING",
        ),
        (
            "INSERT { ?s <http://example.org/seen> true } USING NAMED <http://example.org/g2> WHERE { GRAPH ?g { ?s <http://example.org/p> ?o } }",
            "USING",
        ),
        (
            "WITH <http://example.org/g2> INSERT { ?s <http://example.org/seen> true } WHERE { ?s <http://example.org/p> ?o }",
            "WITH",
        ),
    ] {
        let request = update_request(update, &g1);
        let refused = request
            .effective_text()
            .expect_err("the protocol forbids the combination");
        assert_eq!(
            refused,
            ProtocolError::UsingConflictsWithClause {
                parameter: "using-graph-uri",
                operation: 0,
                clause,
            }
        );
        assert_eq!(refused.name(), "UsingConflictsWithClause");
        // The neighbour: the same update without the parameter runs over its own
        // dataset, g2 — so it marks `ex:b`, where the parameter would have meant `ex:a`.
        assert_eq!(
            after_update(&update_request(update, "")).0,
            ["b"],
            "`{update}`"
        );
    }
    // The conflict is found in whichever operation has the clause.
    let second = "INSERT DATA { <http://example.org/d> <http://example.org/p> <http://example.org/x3> } ;\n\
                  WITH <http://example.org/g2> DELETE { ?s ?p ?o } WHERE { ?s ?p ?o }";
    assert!(matches!(
        update_request(second, &graph_param("using-named-graph-uri", "g1")).effective_text(),
        Err(ProtocolError::UsingConflictsWithClause {
            parameter: "using-named-graph-uri",
            operation: 1,
            clause: "WITH"
        })
    ));
}

#[test]
fn an_operation_that_does_not_parse_is_reported_only_when_parameters_must_be_applied() {
    let broken = "SELECT WHERE";
    let request = query_request(broken, &graph_param("default-graph-uri", "g1"));
    let refused = request
        .effective_text()
        .expect_err("no position to splice at");
    assert!(
        matches!(
            &refused,
            ProtocolError::MalformedOperation {
                kind: OperationKind::Query,
                ..
            }
        ),
        "{refused:?}"
    );
    // Without parameters the text is handed on untouched, for the engine to report.
    assert_eq!(
        query_request(broken, "").effective_text().as_deref(),
        Ok(broken)
    );
    let request = update_request("INSERT WHERE", &graph_param("using-graph-uri", "g1"));
    assert!(matches!(
        request.effective_text(),
        Err(ProtocolError::MalformedOperation {
            kind: OperationKind::Update,
            ..
        })
    ));
}

#[test]
fn every_error_names_itself_and_explains_itself() {
    let errors = [
        err(ProtocolRequest::parse("PUT", None, None, b"")),
        err(post(None, None, "")),
        err(post(Some("text/plain"), None, "")),
        err(post(
            Some("application/sparql-query; charset=latin1"),
            None,
            "",
        )),
        err(ProtocolRequest::parse("POST", SPARQL_QUERY, None, b"\xff")),
        err(ProtocolRequest::parse("GET", None, Some("query=x"), b"x")),
        err(get("query=%")),
        err(get("")),
        err(get("query=a&query=b")),
        err(get("update=x")),
        err(post(
            FORM,
            None,
            "update=x&default-graph-uri=http%3A%2F%2Fexample.org%2F",
        )),
        err(get("query=x&using-graph-uri=http%3A%2F%2Fexample.org%2F")),
        err(get("query=x&default-graph-uri=x")),
    ];
    let names: Vec<&str> = errors.iter().map(ProtocolError::name).collect();
    assert_eq!(
        names,
        [
            "UnsupportedMethod",
            "MissingContentType",
            "UnsupportedContentType",
            "UnsupportedCharset",
            "NonUtf8Body",
            "BodyOnGet",
            "MalformedForm",
            "MissingOperation",
            "AmbiguousOperation",
            "QueryViaGetIsNotUpdate",
            "DatasetParametersOnUpdate",
            "UsingParametersOnQuery",
            "InvalidGraphIri",
        ]
    );
    for error in &errors {
        assert_ne!(error.to_string(), "");
    }
}
