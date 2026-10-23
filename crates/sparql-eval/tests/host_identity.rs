// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original host IDs through the public operational SPARQL boundary.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_core::{
    CanonHash, DatasetView, FallibleDatasetView, GraphMatch, OwnedWorkspaceReservation, QuadIds,
    RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermGuard, TermRef, TermValue,
    TripleDataset, TripleReadError, ViewOperationStatus, ViewTermId, WorkspaceReservation,
    try_canonicalize_flat_view,
};
use purrdf_sparql_eval::{EvalCtx, FallibleSparqlError, NativeSparqlEngine, QueryOptions, eval};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct HostId(u64);

impl ViewTermId for HostId {
    type JoinKeyAtom = u128;

    fn encode(self) -> u128 {
        u128::from(self.0)
    }

    fn encode_computed(index: u32) -> u128 {
        (1_u128 << 64) | u128::from(index)
    }
}

const A: HostId = HostId(u64::MAX);
const B: HostId = HostId(0);
const P: HostId = HostId(1 << 62);
const Q: HostId = HostId((1 << 62) + 1);
const TRIPLE: HostId = HostId((1 << 63) + 7);
const INTEGER: HostId = HostId(11);
const DATE_TIME: HostId = HostId(12);
const NUMBER_LOW: HostId = HostId((2 << 60) + 2);
const NUMBER_HIGH: HostId = HostId((2 << 60) + 1);
const TIME_LOW: HostId = HostId((3 << 60) + 2);
const TIME_HIGH: HostId = HostId((3 << 60) + 1);

#[derive(Clone, Copy, Debug)]
enum Certificate {
    Unavailable,
    Ordered,
    Refused,
    Sticky,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Deleted(HostId);

impl std::fmt::Display for Deleted {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "host generation lost term {:?}", self.0)
    }
}

impl std::error::Error for Deleted {}

const LIVE_LIMIT: u64 = 1 << 20;

struct HostReservation {
    live: Arc<Mutex<u64>>,
    bytes: u64,
}

impl WorkspaceReservation for HostReservation {
    type Error = Deleted;

    fn resize(&mut self, payload: u64) -> Result<(), Deleted> {
        let bytes = payload
            .checked_add(u64::try_from(size_of::<Self>()).map_err(|_| Deleted(B))?)
            .ok_or(Deleted(B))?;
        let mut live = self.live.lock().expect("admission lock");
        *live = live
            .checked_sub(self.bytes)
            .and_then(|live| live.checked_add(bytes))
            .filter(|new| *new <= LIVE_LIMIT)
            .ok_or(Deleted(B))?;
        drop(live);
        self.bytes = bytes;
        Ok(())
    }
}

impl Drop for HostReservation {
    fn drop(&mut self) {
        *self.live.lock().expect("admission lock") -= self.bytes;
    }
}

struct Pin<'a> {
    term: TermRef<'static, HostId>,
    live: &'a AtomicUsize,
}

impl TermGuard<HostId> for Pin<'_> {
    fn term(&self) -> TermRef<'_, HostId> {
        self.term
    }
}

impl Drop for Pin<'_> {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::Relaxed);
    }
}

struct Host {
    rows: Vec<QuadIds<HostId>>,
    pins: AtomicUsize,
    resolutions: AtomicUsize,
    probes: Mutex<Vec<[Option<HostId>; 3]>>,
    failure: Mutex<Option<Deleted>>,
    delete_at_resolve: Option<HostId>,
    certificate: Certificate,
    certificates: AtomicUsize,
    bounded: bool,
    live: Arc<Mutex<u64>>,
}

impl Host {
    fn fixture() -> Self {
        Self {
            rows: vec![
                QuadIds {
                    s: A,
                    p: P,
                    o: B,
                    g: None,
                },
                QuadIds {
                    s: B,
                    p: Q,
                    o: TRIPLE,
                    g: None,
                },
            ],
            pins: AtomicUsize::new(0),
            resolutions: AtomicUsize::new(0),
            probes: Mutex::new(Vec::new()),
            failure: Mutex::new(None),
            delete_at_resolve: None,
            certificate: Certificate::Unavailable,
            certificates: AtomicUsize::new(0),
            bounded: false,
            live: Arc::new(Mutex::new(0)),
        }
    }

    fn fail(&self, id: HostId) -> Deleted {
        *self
            .failure
            .lock()
            .expect("failure lock")
            .get_or_insert(Deleted(id))
    }
}

impl DatasetView for Host {
    type Id = HostId;
    type ReadError = Deleted;
    type TermGuard<'a> = Pin<'a>;
    type ProbePlan = ();

    fn read_error(&self) -> Option<Deleted> {
        *self.failure.lock().expect("failure lock")
    }

    fn storage_live_budget(&self) -> Option<u64> {
        self.bounded.then_some(LIVE_LIMIT)
    }

    fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Option<Result<OwnedWorkspaceReservation<Deleted>, Deleted>> {
        self.storage_live_budget()?;
        Some((|| {
            let mut reservation = HostReservation {
                live: Arc::clone(&self.live),
                bytes: 0,
            };
            reservation.resize(bytes)?;
            let owner = purrdf_core::small::try_boxed(reservation).map_err(|_| Deleted(B))?;
            Ok(owner as OwnedWorkspaceReservation<Deleted>)
        })())
    }

    fn max_owned_term_bytes(&self) -> Option<u64> {
        Some(1024)
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds<HostId>> + '_ {
        self.rows
            .iter()
            .copied()
            .take_while(|_| self.read_error().is_none())
    }

    fn resolve(&self, id: HostId) -> Result<Pin<'_>, Deleted> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        self.resolutions.fetch_add(1, Ordering::Relaxed);
        if self.delete_at_resolve == Some(id) {
            return Err(self.fail(id));
        }
        let term = match id {
            A => TermRef::Iri("http://example.org/a"),
            B => TermRef::Iri("http://example.org/b"),
            P => TermRef::Iri("http://example.org/p"),
            Q => TermRef::Iri("http://example.org/q"),
            TRIPLE => TermRef::Triple { s: A, p: P, o: B },
            INTEGER => TermRef::Iri(purrdf_xsd::datatype::XSD_INTEGER),
            DATE_TIME => TermRef::Iri(purrdf_xsd::datatype::XSD_DATE_TIME),
            NUMBER_LOW | NUMBER_HIGH | TIME_LOW | TIME_HIGH => {
                let (lexical, datatype) = match id {
                    NUMBER_LOW => ("2", INTEGER),
                    NUMBER_HIGH => ("10", INTEGER),
                    TIME_LOW => ("2024-01-01T00:00:00Z", DATE_TIME),
                    _ => ("2024-01-02T00:00:00Z", DATE_TIME),
                };
                TermRef::Literal {
                    lexical,
                    datatype,
                    language: None,
                    direction: None,
                }
            }
            _ => return Err(self.fail(id)),
        };
        self.pins.fetch_add(1, Ordering::Relaxed);
        Ok(Pin {
            term,
            live: &self.pins,
        })
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<HostId>, Deleted> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        Ok(match value {
            TermValue::Iri(iri) => match iri.as_str() {
                "http://example.org/a" => Some(A),
                "http://example.org/b" => Some(B),
                "http://example.org/p" => Some(P),
                "http://example.org/q" => Some(Q),
                purrdf_xsd::datatype::XSD_INTEGER => Some(INTEGER),
                purrdf_xsd::datatype::XSD_DATE_TIME => Some(DATE_TIME),
                _ => None,
            },
            TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            } => match (datatype.as_str(), lexical_form.as_str()) {
                (purrdf_xsd::datatype::XSD_INTEGER, "2") => Some(NUMBER_LOW),
                (purrdf_xsd::datatype::XSD_INTEGER, "10") => Some(NUMBER_HIGH),
                (purrdf_xsd::datatype::XSD_DATE_TIME, "2024-01-01T00:00:00Z") => Some(TIME_LOW),
                (purrdf_xsd::datatype::XSD_DATE_TIME, "2024-01-02T00:00:00Z") => Some(TIME_HIGH),
                _ => None,
            },
            TermValue::Triple { s, p, o }
                if matches!((&**s, &**p, &**o), (TermValue::Iri(s), TermValue::Iri(p), TermValue::Iri(o))
                    if s == "http://example.org/a" && p == "http://example.org/p" && o == "http://example.org/b") =>
            {
                Some(TRIPLE)
            }
            _ => None,
        })
    }

    purrdf_core::fixed_store_capabilities!(quoted_triples);

    fn certified_value_order(
        &self,
        left: HostId,
        right: HostId,
    ) -> Result<Option<std::cmp::Ordering>, Deleted> {
        self.certificates.fetch_add(1, Ordering::Relaxed);
        match self.certificate {
            Certificate::Unavailable => return Ok(None),
            Certificate::Refused => return Err(Deleted(left)),
            Certificate::Sticky => {
                self.fail(left);
            }
            Certificate::Ordered => {}
        }
        // The host's type-tagged descending encoding has the opposite direction
        // from raw ID order. Unsupported pairs supply no certificate.
        Ok(match (left, right) {
            (NUMBER_LOW, NUMBER_HIGH) | (TIME_LOW, TIME_HIGH) => Some(std::cmp::Ordering::Less),
            (NUMBER_HIGH, NUMBER_LOW) | (TIME_HIGH, TIME_LOW) => Some(std::cmp::Ordering::Greater),
            _ => None,
        })
    }

    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<HostId>) {}

    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<HostId>,
        p: Option<HostId>,
        o: Option<HostId>,
        g: GraphMatch<HostId>,
    ) -> impl Iterator<Item = QuadIds<HostId>> + '_ {
        self.probes.lock().expect("probe lock").push([s, p, o]);
        self.quads_for_pattern(s, p, o, g)
    }

    fn term_count(&self) -> u64 {
        // A sparse nonresident dictionary need not fit a native u32 interner.
        (1_u64 << 40) + 5
    }
}

impl FallibleDatasetView for Host {
    type Error = Deleted;
    type Evidence = usize;

    fn operation_status(&self) -> ViewOperationStatus<Deleted, usize> {
        let evidence = self.resolutions.load(Ordering::Relaxed);
        match self.read_error() {
            Some(error) => ViewOperationStatus::Failed { error, evidence },
            None => ViewOperationStatus::Ready { evidence },
        }
    }
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn original_host_ids_join_and_expand_triples_without_native_interning() {
    let host = Host::fixture();
    let result = NativeSparqlEngine::new().query_fallible_view(
        &host,
        request("SELECT ?s ?t WHERE { ?s <http://example.org/p> ?mid . ?mid <http://example.org/q> ?t }"),
        QueryOptions::EMPTY,
    ).expect("host SELECT");
    let (_, rows) = result.result.solutions().expect("SELECT shape");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Some(TermValue::iri("http://example.org/a")));
    assert!(matches!(rows[0][1], Some(TermValue::Triple { .. })));
    let probes = host.probes.lock().expect("probe lock");
    assert!(
        probes
            .iter()
            .any(|probe| probe[0] == Some(B) && probe[1] == Some(Q))
    );
    assert!(
        probes
            .iter()
            .flatten()
            .flatten()
            .all(|id| [A, B, P, Q, TRIPLE].contains(id))
    );
    drop(probes);
    assert_eq!(host.pins.load(Ordering::Relaxed), 0);
    assert_ne!(HostId::encode_computed(0), B.encode());
    assert_ne!(HostId::encode_computed(u32::MAX), A.encode());
}

fn host_deletion_discards_partial_output_and_preserves_first_typed_cause() {
    let mut host = Host::fixture();
    host.delete_at_resolve = Some(TRIPLE);
    let engine = NativeSparqlEngine::new();
    let result = engine.query_fallible_view(
        &host,
        request("SELECT ?t WHERE { ?s <http://example.org/q> ?t }"),
        QueryOptions::EMPTY,
    );
    assert!(matches!(
        result,
        Err(FallibleSparqlError::Operational {
            error: Deleted(TRIPLE),
            ..
        })
    ));
    assert_eq!(host.pins.load(Ordering::Relaxed), 0);
    assert_eq!(host.fail(A), Deleted(TRIPLE));
    let empty = engine.query_fallible_view(
        &host,
        request("ASK { ?s <http://example.org/missing> ?o }"),
        QueryOptions::EMPTY,
    );
    assert!(matches!(
        empty,
        Err(FallibleSparqlError::Operational {
            error: Deleted(TRIPLE),
            ..
        })
    ));
}

fn certified_numeric_and_temporal_order_matches_fallback_and_keeps_source_failure() {
    let engine = NativeSparqlEngine::new();
    for (low, high) in [(NUMBER_LOW, NUMBER_HIGH), (TIME_LOW, TIME_HIGH)] {
        assert!(low > high, "raw ID order must disagree with value order");
        for bounded in [false, true] {
            for certificate in [
                Certificate::Unavailable,
                Certificate::Ordered,
                Certificate::Refused,
                Certificate::Sticky,
            ] {
                let mut host = Host::fixture();
                host.certificate = certificate;
                host.bounded = bounded;
                host.rows = vec![
                    QuadIds {
                        s: A,
                        p: P,
                        o: low,
                        g: None,
                    },
                    QuadIds {
                        s: A,
                        p: Q,
                        o: high,
                        g: None,
                    },
                ];
                // Arc forwarding is part of the public host seam, not a test adapter.
                let host = Arc::new(host);
                let result = engine.query_fallible_view(
                &host,
                request("ASK { ?s <http://example.org/p> ?low ; <http://example.org/q> ?high . FILTER(?low < ?high) }"),
                QueryOptions::EMPTY,
            );
                match certificate {
                    Certificate::Unavailable | Certificate::Ordered => {
                        assert_eq!(
                            result.expect("complete comparison").result.boolean(),
                            Some(true)
                        );
                    }
                    Certificate::Refused | Certificate::Sticky => {
                        assert!(
                            matches!(result, Err(FallibleSparqlError::Operational { error: Deleted(id), .. }) if id == low),
                            "{certificate:?}, bounded={bounded}, {low:?}: {result:?}"
                        );
                    }
                }
                assert_eq!(host.certificates.load(Ordering::Relaxed), 1);
                assert_eq!(host.pins.load(Ordering::Relaxed), 0);
            }
        }
    }
}

fn carrier_pair(populated: bool) -> (Arc<RdfDataset>, TripleDataset) {
    let build = || {
        let mut builder = RdfDatasetBuilder::default();
        if populated {
            let a = builder.intern_iri("urn:a").unwrap();
            let b = builder.intern_iri("urn:b").unwrap();
            let p = builder.intern_iri("urn:p").unwrap();
            builder.push_quad(a, p, b, None).unwrap();
            builder.push_quad(b, p, a, None).unwrap();
        }
        builder.freeze().unwrap()
    };
    let quad = build();
    let triple = TripleDataset::try_from(Arc::try_unwrap(build()).unwrap()).unwrap();
    (quad, triple)
}

fn triple_graph_support_precedes_empty_and_nested_query_shortcuts() {
    let engine = NativeSparqlEngine::new();
    for populated in [false, true] {
        let (quad, triple) = carrier_pair(populated);
        for query in [
            "SELECT * WHERE { GRAPH <urn:absent> { ?s ?p ?o } }",
            "ASK { GRAPH ?g { ?s ?p ?o } }",
            "CONSTRUCT { ?s ?p ?o } WHERE { GRAPH <urn:absent> { ?s ?p ?o } }",
            "SELECT * WHERE { ?s <urn:missing> ?o . GRAPH <urn:absent> { ?s ?p ?o } }",
            "ASK { FILTER EXISTS { GRAPH <urn:absent> {} } }",
            "SELECT * WHERE { ?s <urn:missing> ?o . FILTER NOT EXISTS { GRAPH ?g { ?s ?p ?o } } }",
            "SELECT * WHERE { { SELECT * WHERE { GRAPH ?g { ?s ?p ?o } } } }",
            "SELECT * FROM <urn:absent> WHERE { ?s ?p ?o }",
            "ASK FROM NAMED <urn:absent> { ?s ?p ?o }",
        ] {
            let prepared = engine.prepare_query(query, None).unwrap();
            for use_prepared in [false, true] {
                let read = triple.read().unwrap();
                let result = if use_prepared {
                    engine.query_prepared_fallible_view(&read, &prepared, &[], QueryOptions::EMPTY)
                } else {
                    engine.query_fallible_view(&read, request(query), QueryOptions::EMPTY)
                };
                assert!(
                    matches!(
                        result,
                        Err(FallibleSparqlError::Operational {
                            error: TripleReadError::NamedGraphsUnsupported,
                            ..
                        })
                    ),
                    "{query}, populated={populated}, prepared={use_prepared}: {result:?}"
                );
                assert_eq!(
                    read.read_error(),
                    Some(TripleReadError::NamedGraphsUnsupported)
                );
                let healthy = triple.read().unwrap();
                assert!(
                    engine
                        .query_fallible_view(
                            &healthy,
                            request("SELECT * WHERE { ?s ?p ?o }"),
                            QueryOptions::EMPTY
                        )
                        .is_ok()
                );
            }
            // The same syntactically valid operation is supported by the existing
            // quad carrier even when its dictionary has no graph name.
            assert!(
                engine
                    .query_with_options_view(quad.as_ref(), request(query), QueryOptions::EMPTY)
                    .is_ok()
            );
            if query.contains("GRAPH") {
                let read = triple.read().unwrap();
                let mut context = EvalCtx::new(&read);
                assert!(eval(prepared.query().pattern(), &mut context).is_err());
                assert_eq!(
                    read.read_error(),
                    Some(TripleReadError::NamedGraphsUnsupported)
                );
            }
        }
    }
}

fn triple_default_queries_preserve_bags_unbound_cells_paths_and_graph_results() {
    let (quad, triple) = carrier_pair(true);
    let engine = NativeSparqlEngine::new();
    for query in [
        "SELECT ?s ?o WHERE { ?s <urn:p> ?o } ORDER BY ?s ?o",
        "SELECT ?s ?missing WHERE { ?s <urn:p> ?o OPTIONAL { ?s <urn:missing> ?missing } } ORDER BY ?s",
        "SELECT ?s WHERE { { ?s <urn:p> ?o } UNION { ?s <urn:p> ?o } } ORDER BY ?s",
        "SELECT ?s ?o WHERE { ?s <urn:p>+ ?o } ORDER BY ?s ?o",
        "SELECT (COUNT(*) AS ?n) WHERE { ?s <urn:p> ?o }",
        "ASK { ?s <urn:p> ?o . FILTER EXISTS { ?o <urn:p> ?s } }",
        "CONSTRUCT { ?s <urn:derived> ?o } WHERE { ?s <urn:p> ?o }",
    ] {
        let expected = engine
            .query_with_options_view(quad.as_ref(), request(query), QueryOptions::EMPTY)
            .unwrap();
        let read = triple.read().unwrap();
        let actual = engine
            .query_fallible_view(&read, request(query), QueryOptions::EMPTY)
            .unwrap()
            .result;
        match expected {
            SparqlResult::Solutions {
                variables, rows, ..
            } => {
                let (actual_variables, actual_rows) = actual.solutions().unwrap();
                assert_eq!(actual_variables, variables, "{query}");
                assert_eq!(actual_rows, rows, "{query}");
            }
            SparqlResult::Boolean(expected) => {
                assert_eq!(actual.boolean(), Some(expected), "{query}");
            }
            SparqlResult::Graph(expected) => assert_eq!(
                try_canonicalize_flat_view(actual.graph().unwrap(), CanonHash::Sha256)
                    .unwrap()
                    .nquads,
                try_canonicalize_flat_view(expected.as_ref(), CanonHash::Sha256)
                    .unwrap()
                    .nquads,
                "{query}"
            ),
        }
    }
}

fn triple_local_graph_support_stops_at_the_remote_service_boundary() {
    use purrdf_sparql_eval::InProcessServiceResolver;
    let mut remote = RdfDatasetBuilder::new();
    let graph = remote.intern_iri("urn:remote:graph").unwrap();
    let subject = remote.intern_iri("urn:remote:subject").unwrap();
    let predicate = remote.intern_iri("urn:remote:predicate").unwrap();
    let value = remote
        .intern_literal(purrdf_core::RdfLiteral::simple("remote-graph"))
        .unwrap();
    remote
        .push_quad(subject, predicate, value, Some(graph))
        .unwrap();
    let resolver = InProcessServiceResolver::new().with_endpoint(
        "https://example.org/in-process/sparql",
        remote.freeze().unwrap(),
    );
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_remote(Some(&resolver));

    for populated in [false, true] {
        let (_, triple) = carrier_pair(populated);
        for (query, remote_only) in [
            (
                "SELECT ?n WHERE { SERVICE <https://example.org/in-process/sparql> { GRAPH <urn:remote:graph> { <urn:remote:subject> <urn:remote:predicate> ?n } } }",
                true,
            ),
            (
                "SELECT ?n WHERE { SERVICE SILENT <https://example.org/in-process/sparql> { GRAPH ?g { <urn:remote:subject> <urn:remote:predicate> ?n } } }",
                true,
            ),
            (
                "SELECT ?n WHERE { VALUES ?n { \"remote-graph\" } FILTER EXISTS { SERVICE <https://example.org/in-process/sparql> { GRAPH <urn:remote:graph> { <urn:remote:subject> <urn:remote:predicate> ?n } } } }",
                true,
            ),
            (
                "SELECT ?n WHERE { SERVICE <https://example.org/in-process/sparql> { GRAPH <urn:remote:graph> { <urn:remote:subject> <urn:remote:predicate> ?n } } GRAPH <urn:local:graph> {} }",
                false,
            ),
            (
                "SELECT ?n FROM NAMED <urn:local:graph> WHERE { SERVICE <https://example.org/in-process/sparql> { GRAPH <urn:remote:graph> { <urn:remote:subject> <urn:remote:predicate> ?n } } }",
                false,
            ),
        ] {
            let prepared = engine.prepare_query(query, None).unwrap();
            for use_prepared in [false, true] {
                let read = triple.read().unwrap();
                let result = if use_prepared {
                    engine.query_prepared_fallible_view(&read, &prepared, &[], options)
                } else {
                    engine.query_fallible_view(
                        &read,
                        SparqlRequest {
                            query,
                            base_iri: None,
                            substitutions: &[],
                        },
                        options,
                    )
                };
                if remote_only {
                    let result = result.expect("the remote quad carrier supports named graphs");
                    let (variables, rows) = result.result.solutions().unwrap();
                    assert_eq!(variables, ["n"]);
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows[0].len(), 1);
                    assert!(matches!(&rows[0][0],
                        Some(TermValue::Literal { lexical_form, .. }) if lexical_form == "remote-graph"));
                    assert_eq!(read.read_error(), None);
                } else {
                    assert!(matches!(
                        result,
                        Err(FallibleSparqlError::Operational {
                            error: TripleReadError::NamedGraphsUnsupported,
                            ..
                        })
                    ));
                    assert_eq!(
                        read.read_error(),
                        Some(TripleReadError::NamedGraphsUnsupported)
                    );
                }
            }
        }
    }
}

purrdf_testkit::harness_main!(
    original_host_ids_join_and_expand_triples_without_native_interning,
    host_deletion_discards_partial_output_and_preserves_first_typed_cause,
    certified_numeric_and_temporal_order_matches_fallback_and_keeps_source_failure,
    triple_graph_support_precedes_empty_and_nested_query_shortcuts,
    triple_local_graph_support_stops_at_the_remote_service_boundary,
    triple_default_queries_preserve_bags_unbound_cells_paths_and_graph_results,
);
