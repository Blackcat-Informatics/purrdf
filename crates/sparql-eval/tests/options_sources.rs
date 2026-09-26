// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The per-request `SERVICE` and `LOAD` sources carried by [`QueryOptions`].
//!
//! [`QueryOptions::remote`] is the `SERVICE` source of one request — a query on any lane,
//! or an UPDATE's `WHERE` — and [`QueryOptions::load`] is the `LOAD` source of one UPDATE.
//! Every treatment here is paired with a neighbour whose observable answer differs from
//! it, so an assertion cannot pass because the source was silently ignored: a `SERVICE`
//! that federated contributes rows a swallowed one does not, and a `LOAD` that fetched
//! from one resolver inserts a quad the other resolver does not hold.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{
    DatasetMut, GraphMatchValue, MutableDataset, RdfDataset, RdfDatasetBuilder, RdfDiagnostic,
    ResourceDimension, SparqlRequest, SparqlResult, TermValue, TrippedGovernor,
};
use purrdf_sparql_eval::{
    GovernedOutcome, GovernedUpdateOutcome, GraphResolveRequest, GraphResolver,
    InProcessServiceResolver, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

/// The federated endpoint every fixture names.
const ENDPOINT: &str = "http://example.org/sparql";

/// The document every `LOAD` names.
const DOC: &str = "http://example.org/doc";

/// A full `http://example.org/` IRI for `local`.
fn ex(local: &str) -> String {
    format!("http://example.org/{local}")
}

/// A frozen default-graph dataset of IRI triples.
fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = builder.intern_iri(&ex(s));
        let p = builder.intern_iri(&ex(p));
        let o = builder.intern_iri(&ex(o));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze fixture")
}

/// The local store: two subjects, each knowing one object. Only `x` has a name at the
/// endpoint, so a federated join keeps `a` and drops `b`.
fn local() -> Arc<RdfDataset> {
    dataset(&[("a", "knows", "x"), ("b", "knows", "y")])
}

/// The endpoint's contents: names for `x` and for `z` (which the local store never
/// reaches), and two `q` edges an UPDATE copies across.
fn remote() -> Arc<RdfDataset> {
    dataset(&[
        ("x", "name", "nx"),
        ("z", "name", "nz"),
        ("r1", "q", "o1"),
        ("r2", "q", "o2"),
    ])
}

/// The in-process resolver answering [`ENDPOINT`] from [`remote`].
fn resolver() -> InProcessServiceResolver {
    InProcessServiceResolver::new().with_endpoint(ENDPOINT, remote())
}

/// The federated join: local `knows` edges joined with remote names.
fn join_query(silent: bool) -> String {
    let silent = if silent { "SILENT " } else { "" };
    format!(
        "SELECT ?s ?n WHERE {{ ?s <http://example.org/knows> ?o \
         SERVICE {silent}<{ENDPOINT}> {{ ?o <http://example.org/name> ?n }} }}"
    )
}

fn request(text: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query: text,
        base_iri: None,
        substitutions: &[],
    }
}

/// A term rendered for comparison: an IRI by its string, anything else by `Debug`.
fn render(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => iri.clone(),
        other => format!("{other:?}"),
    }
}

/// The `?s ?n` rows of a SELECT result, as a set of rendered cells.
fn rows(result: &SparqlResult) -> BTreeSet<(Option<String>, Option<String>)> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("expected solutions, got {result:?}");
    };
    assert_eq!(variables, &["s".to_owned(), "n".to_owned()]);
    rows.iter()
        .map(|row| (row[0].as_ref().map(render), row[1].as_ref().map(render)))
        .collect()
}

/// The one row the federated join produces.
fn joined() -> BTreeSet<(Option<String>, Option<String>)> {
    BTreeSet::from([(Some(ex("a")), Some(ex("nx")))])
}

/// Every default-graph triple the store holds, rendered.
fn quads(store: &Arc<RdfDataset>) -> BTreeSet<(String, String, String)> {
    MutableDataset::new(Arc::clone(store))
        .quads_for_pattern(None, None, None, GraphMatchValue::Any)
        .iter()
        .map(|quad| {
            assert_eq!(quad.g, None, "every fixture quad is in the default graph");
            (render(&quad.s), render(&quad.p), render(&quad.o))
        })
        .collect()
}

/// A rendered triple of `http://example.org/` IRIs.
fn triple(s: &str, p: &str, o: &str) -> (String, String, String) {
    (ex(s), ex(p), ex(o))
}

/// The local store's own triples.
fn local_triples() -> BTreeSet<(String, String, String)> {
    BTreeSet::from([triple("a", "knows", "x"), triple("b", "knows", "y")])
}

/// The federating INSERT: copy every remote `q` edge into a local `p` edge.
fn federating_insert(silent: bool) -> String {
    let silent = if silent { "SILENT " } else { "" };
    format!(
        "INSERT {{ ?s <http://example.org/p> ?o }} WHERE {{ SERVICE {silent}<{ENDPOINT}> \
         {{ ?s <http://example.org/q> ?o }} }}"
    )
}

/// The store after [`federating_insert`] applied the remote rows.
fn federated_triples() -> BTreeSet<(String, String, String)> {
    let mut expected = local_triples();
    expected.insert(triple("r1", "p", "o1"));
    expected.insert(triple("r2", "p", "o2"));
    expected
}

/// A `LOAD` host that answers every IRI with one fixed document.
#[derive(Debug)]
struct FixedDocument {
    document: Arc<RdfDataset>,
}

impl FixedDocument {
    /// A resolver whose document is the single triple `ex:{s} ex:{p} ex:{o}`.
    fn new(s: &str, p: &str, o: &str) -> Self {
        Self {
            document: dataset(&[(s, p, o)]),
        }
    }
}

impl GraphResolver for FixedDocument {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        assert_eq!(request.iri, DOC, "LOAD names the fixture document");
        Ok(Arc::clone(&self.document))
    }
}

/// Run `update` ungoverned over a fresh [`local`] store under `options`.
fn update(
    engine: &NativeSparqlEngine,
    update: &str,
    options: QueryOptions<'_>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut store = local();
    engine.update_with_options(&mut store, request(update), options)?;
    Ok(store)
}

#[test]
fn options_remote_joins_remote_rows() {
    let resolver = resolver();
    let engine = NativeSparqlEngine::new();
    let federated = engine
        .query_with_options_view(
            &*local(),
            request(&join_query(false)),
            QueryOptions::new().with_remote(Some(&resolver)),
        )
        .expect("the federated join evaluates");
    assert_eq!(rows(&federated), joined());

    // Neighbour: the same join through a source that cannot reach the endpoint. The
    // endpoint fails, so `SERVICE SILENT` contributes the join identity and both local
    // subjects survive with `?n` unbound — a different answer, which is what makes the
    // treatment's single named row evidence of federation.
    let unreachable = InProcessServiceResolver::new();
    let unfederated = engine
        .query_with_options_view(
            &*local(),
            request(&join_query(true)),
            QueryOptions::new().with_remote(Some(&unreachable)),
        )
        .expect("SERVICE SILENT over an endpoint that fails is the join identity");
    let identity = BTreeSet::from([(Some(ex("a")), None), (Some(ex("b")), None)]);
    assert_eq!(rows(&unfederated), identity);
    assert_ne!(rows(&unfederated), rows(&federated));

    // With no source at all the engine was given nowhere to send the request: no
    // endpoint failed, so `SILENT` does not swallow it.
    let err = engine
        .query_with_options_view(&*local(), request(&join_query(true)), QueryOptions::EMPTY)
        .expect_err("SERVICE SILENT without a source is refused");
    assert_eq!(
        err.message,
        format!(
            "SERVICE federation error: no remote query source configured for SERVICE \
             <{ENDPOINT}>; SILENT does not apply: it tolerates an endpoint that fails, and \
             no endpoint was reached — configure a remote query source for it"
        )
    );
}

#[test]
fn options_remote_matches_query_with_source() {
    let resolver = resolver();
    let engine = NativeSparqlEngine::new();
    let through_options = engine
        .query_with_options_view(
            &*local(),
            request(&join_query(false)),
            QueryOptions::new().with_remote(Some(&resolver)),
        )
        .expect("the options spelling evaluates");
    let through_source = engine
        .query_with_source(
            &local(),
            request(&join_query(false)),
            &resolver,
            QueryOptions::EMPTY,
        )
        .expect("the source spelling evaluates");
    assert_eq!(rows(&through_options), joined());
    assert_eq!(rows(&through_source), rows(&through_options));

    // The explicit source wins over a different `options.remote`: an endpoint-less
    // resolver in the options, the real one passed explicitly, answers as the real one.
    let empty = InProcessServiceResolver::new();
    let explicit_wins = engine
        .query_with_source(
            &local(),
            request(&join_query(false)),
            &resolver,
            QueryOptions::new().with_remote(Some(&empty)),
        )
        .expect("the explicit source answers");
    assert_eq!(rows(&explicit_wins), joined());

    // Neighbour: the endpoint-less resolver on its own is really consulted, and it
    // cannot answer the endpoint — so the win above is the explicit source's doing.
    let err = engine
        .query_with_options_view(
            &*local(),
            request(&join_query(false)),
            QueryOptions::new().with_remote(Some(&empty)),
        )
        .expect_err("a resolver with no endpoint cannot answer a non-SILENT SERVICE");
    assert!(
        err.message
            .contains(&format!("no in-memory endpoint <{ENDPOINT}>")),
        "unexpected diagnostic: {err:?}"
    );
}

#[test]
fn options_remote_reaches_the_governed_entry() {
    let resolver = resolver();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::new().with_remote(Some(&resolver));
    let outcome = engine
        .query_governed(
            &local(),
            request(&join_query(false)),
            options,
            &QueryGovernors::METERED,
        )
        .expect("the governed federated join evaluates");
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &outcome
    else {
        panic!("a metered run cannot trip: {outcome:?}");
    };
    assert_eq!(rows(result), joined());
    assert_eq!(
        evidence.consumed_in(ResourceDimension::RemoteRequests),
        1,
        "the SERVICE call is one remote request"
    );

    // The same request under a remote-request ceiling of zero trips on exactly that
    // dimension, which only a SERVICE call that was really dispatched can do.
    let capped = engine
        .query_governed(
            &local(),
            request(&join_query(false)),
            options,
            &QueryGovernors::UNBOUNDED.with_max_remote_requests(0),
        )
        .expect("a trip is an outcome, not an error");
    assert_eq!(
        capped.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::RemoteRequests,
            limit: 0,
            consumed: 1,
        })
    );
}

#[test]
fn update_where_federates_through_options_remote() {
    let resolver = resolver();
    let store = update(
        &NativeSparqlEngine::new(),
        &federating_insert(false),
        QueryOptions::new().with_remote(Some(&resolver)),
    )
    .expect("the federated INSERT applies");
    assert_eq!(quads(&store), federated_triples());
}

#[test]
fn update_where_service_silent_without_a_source_is_a_hard_error_too() {
    let err = update(
        &NativeSparqlEngine::new(),
        &federating_insert(true),
        QueryOptions::EMPTY,
    )
    .expect_err("SERVICE SILENT without a source is refused");
    assert_eq!(err.code, "native-sparql-update-eval");
    assert!(
        err.message.contains(&format!(
            "no remote query source configured for SERVICE <{ENDPOINT}>; SILENT does not apply"
        )),
        "{err:?}"
    );

    // Neighbours: the same SILENT request through a source whose endpoint fails inserts
    // nothing and succeeds, and WITH a source that answers it inserts the remote rows,
    // so the unchanged store is the failing endpoint's doing, not the request's.
    let unreachable = InProcessServiceResolver::new();
    let store = update(
        &NativeSparqlEngine::new(),
        &federating_insert(true),
        QueryOptions::new().with_remote(Some(&unreachable)),
    )
    .expect("SERVICE SILENT over an endpoint that fails succeeds");
    assert_eq!(quads(&store), local_triples());
    let resolver = resolver();
    let federated = update(
        &NativeSparqlEngine::new(),
        &federating_insert(true),
        QueryOptions::new().with_remote(Some(&resolver)),
    )
    .expect("the federated SILENT INSERT applies");
    assert_eq!(quads(&federated), federated_triples());
    assert_ne!(quads(&federated), quads(&store));
}

#[test]
fn update_where_service_without_a_source_is_a_hard_error() {
    let err = update(
        &NativeSparqlEngine::new(),
        &federating_insert(false),
        QueryOptions::EMPTY,
    )
    .expect_err("a non-SILENT SERVICE with no source cannot be answered");
    assert_eq!(err.code, "native-sparql-update-eval");
    assert_eq!(
        err.message,
        format!(
            "SERVICE federation error: no remote query source configured for SERVICE <{ENDPOINT}>"
        )
    );
}

#[test]
fn update_governed_where_federation_is_governed() {
    let resolver = resolver();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::new().with_remote(Some(&resolver));

    let mut store = local();
    let before = Arc::clone(&store);
    let outcome = engine
        .update_governed(
            &mut store,
            request(&federating_insert(false)),
            options,
            &QueryGovernors::UNBOUNDED.with_max_remote_requests(0),
        )
        .expect("a trip is an outcome, not an error");
    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::RemoteRequests,
            limit: 0,
            consumed: 1,
        })
    );
    assert!(
        Arc::ptr_eq(&store, &before),
        "a tripped UPDATE leaves the caller's handle untouched"
    );
    assert_eq!(quads(&store), local_triples());

    // Neighbour: no ceiling, and the same request applies the remote rows.
    let mut applied = local();
    let outcome = engine
        .update_governed(
            &mut applied,
            request(&federating_insert(false)),
            options,
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the ungoverned-ceiling run applies");
    assert!(matches!(outcome, GovernedUpdateOutcome::Applied { .. }));
    assert_eq!(quads(&applied), federated_triples());
    assert_ne!(quads(&applied), quads(&store));
}

#[test]
fn options_load_resolves_without_an_engine_resolver() {
    let document = FixedDocument::new("docS", "docP", "docO");
    let engine = NativeSparqlEngine::new();
    let load = format!("LOAD <{DOC}>");
    let store = update(
        &engine,
        &load,
        QueryOptions::new().with_load(Some(&document)),
    )
    .expect("LOAD resolves through the request's source");
    let mut expected = local_triples();
    expected.insert(triple("docS", "docP", "docO"));
    assert_eq!(quads(&store), expected);

    // Neighbour: the same engine and request with no source at all is the documented
    // hard error, so the inserted quad above came from `options.load`.
    let err = update(&engine, &load, QueryOptions::EMPTY)
        .expect_err("LOAD with neither resolver cannot fetch");
    assert_eq!(err.code, "native-sparql-load-no-resolver");
}

#[test]
fn options_load_overrides_the_engine_resolver() {
    let engine = NativeSparqlEngine::new().with_resolver(Arc::new(FixedDocument::new(
        "engineS", "engineP", "engineO",
    )));
    let per_request = FixedDocument::new("requestS", "requestP", "requestO");
    let load = format!("LOAD <{DOC}>");

    let overridden = update(
        &engine,
        &load,
        QueryOptions::new().with_load(Some(&per_request)),
    )
    .expect("LOAD resolves through the request's source");
    let mut expected = local_triples();
    expected.insert(triple("requestS", "requestP", "requestO"));
    assert_eq!(quads(&overridden), expected);

    // Neighbour: without `options.load` the engine's own resolver still answers, and
    // its document is a different quad.
    let fallback = update(&engine, &load, QueryOptions::EMPTY)
        .expect("LOAD falls back to the engine's resolver");
    let mut expected = local_triples();
    expected.insert(triple("engineS", "engineP", "engineO"));
    assert_eq!(quads(&fallback), expected);
    assert_ne!(quads(&fallback), quads(&overridden));
}
