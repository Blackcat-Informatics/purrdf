// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generic query publication must include reads after the last algebra node.
mod support;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder,
    RdfStoreCapabilities, SparqlRequest, StopCause, TermId, TermValue, TrippedGovernor,
    ViewOperationStatus, WorkspaceReservation,
};
use purrdf_sparql_eval::{
    CancellationFlag, FallibleScopedResult, FallibleSparqlError, GovernedEvidence, GovernorState,
    NativeSparqlEngine, QueryExplanation, QueryGovernors, QueryOptions, ShaclPrebinding,
    StopSignal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusedRead {
    FinalRead,
    Workspace,
    Lookup,
}
impl std::fmt::Display for RefusedRead {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::FinalRead => "source refused the final read",
            Self::Workspace => "source refused execution workspace",
            Self::Lookup => "source refused admission lookup",
        })
    }
}
impl std::error::Error for RefusedRead {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusalSite {
    Never,
    Iterator,
    Term,
    Reporting,
    Execution,
    ExecutionInitial,
    ExecutionGrowth,
    ExecutionSticky,
    ReportingSticky,
    Lookup,
    Checkpoint,
    BoundedHealthy,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Receipt {
    failed: bool,
    reservations: usize,
    execution_attempts: usize,
    reporting_guards: usize,
    execution_guards: usize,
    reads: usize,
}
#[derive(Debug)]
struct Source {
    resident: Arc<RdfDataset>,
    failed: AtomicBool,
    site: RefusalSite,
    reservations: AtomicUsize,
    execution_attempts: AtomicUsize,
    reporting_guards: AtomicUsize,
    execution_guards: AtomicUsize,
    reads: AtomicUsize,
    last_checkpoint: Mutex<Receipt>,
}
impl Source {
    fn at(site: RefusalSite) -> Self {
        Self {
            resident: support::local_dataset([("s", "p", "o")]),
            failed: AtomicBool::new(false),
            site,
            reservations: AtomicUsize::new(0),
            execution_attempts: AtomicUsize::new(0),
            reporting_guards: AtomicUsize::new(0),
            execution_guards: AtomicUsize::new(0),
            reads: AtomicUsize::new(0),
            last_checkpoint: Mutex::new(Receipt::default()),
        }
    }
    fn refuse(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }
    fn assert_released(&self) {
        assert_eq!(self.reporting_guards.load(Ordering::Relaxed), 0);
        assert_eq!(self.execution_guards.load(Ordering::Relaxed), 0);
    }
    fn assert_receipt(&self, receipt: &Receipt) {
        assert_eq!(
            receipt,
            &*self.last_checkpoint.lock().expect("checkpoint receipt")
        );
        self.assert_released();
    }
}
struct Reservation<'a> {
    source: &'a Source,
    execution: bool,
}
impl WorkspaceReservation for Reservation<'_> {
    type Error = RefusedRead;
    fn resize(&mut self, _: u64) -> Result<(), RefusedRead> {
        Ok(())
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        let live = if self.execution {
            &self.source.execution_guards
        } else {
            &self.source.reporting_guards
        };
        assert!(live.fetch_sub(1, Ordering::Relaxed) > 0);
    }
}
impl DatasetView for Source {
    type Id = TermId;
    type ReadError = RefusedRead;
    type TermGuard<'a> = <RdfDataset as DatasetView>::TermGuard<'a>;
    type ProbePlan = ();
    fn storage_live_budget(&self) -> Option<u64> {
        matches!(
            self.site,
            RefusalSite::Reporting
                | RefusalSite::ReportingSticky
                | RefusalSite::ExecutionInitial
                | RefusalSite::ExecutionGrowth
                | RefusalSite::Lookup
                | RefusalSite::BoundedHealthy
        )
        .then_some(u64::MAX)
    }
    fn max_owned_term_bytes(&self) -> Option<u64> {
        Some(256)
    }
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = RefusedRead> + '_, RefusedRead> {
        self.reservations.fetch_add(1, Ordering::Relaxed);
        // An unbounded adapter has zero-byte reservations. Its execution guard
        // is the one requested while both reporting admissions are still held.
        let execution = if self.storage_live_budget().is_some() {
            bytes != 8192
        } else {
            self.reporting_guards.load(Ordering::Relaxed) == 2
        };
        let attempt = if execution {
            self.execution_attempts.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };
        let refused = match self.site {
            RefusalSite::Reporting | RefusalSite::ReportingSticky => !execution,
            RefusalSite::Execution | RefusalSite::ExecutionSticky => execution,
            RefusalSite::ExecutionInitial => execution && attempt == 0,
            RefusalSite::ExecutionGrowth => execution && attempt == 1,
            _ => false,
        };
        if refused {
            if matches!(
                self.site,
                RefusalSite::ReportingSticky | RefusalSite::ExecutionSticky
            ) {
                self.refuse();
            }
            return Err(RefusedRead::Workspace);
        }
        let live = if execution {
            &self.execution_guards
        } else {
            &self.reporting_guards
        };
        live.fetch_add(1, Ordering::Relaxed);
        Ok(Reservation {
            source: self,
            execution,
        })
    }
    fn read_error(&self) -> Option<RefusedRead> {
        self.failed
            .load(Ordering::Relaxed)
            .then_some(RefusedRead::FinalRead)
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.resident.as_ref().quads().filter(|_| {
            self.reads.fetch_add(1, Ordering::Relaxed);
            if self.site == RefusalSite::Iterator {
                self.refuse();
                false
            } else {
                true
            }
        })
    }
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, RefusedRead> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if self.site == RefusalSite::Term {
            self.refuse();
            return Err(RefusedRead::FinalRead);
        }
        Ok(
            <RdfDataset as DatasetView>::resolve(self.resident.as_ref(), id)
                .expect("resident term resolution is infallible"),
        )
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, RefusedRead> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if self.site == RefusalSite::Lookup {
            return Err(RefusedRead::Lookup);
        }
        Ok(
            <RdfDataset as DatasetView>::term_id_by_value(self.resident.as_ref(), value)
                .expect("resident reverse lookup is infallible"),
        )
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.resident.capabilities()
    }
    fn term_count(&self) -> u64 {
        self.resident.term_count()
    }
    fn len_hint(&self) -> Option<u64> {
        Some(1)
    }
    fn cardinality_estimate(
        &self,
        _: Option<TermId>,
        _: Option<TermId>,
        _: Option<TermId>,
        _: GraphMatch,
    ) -> u64 {
        1
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
}
impl FallibleDatasetView for Source {
    type Error = RefusedRead;
    type Evidence = Receipt;
    fn operation_status(&self) -> ViewOperationStatus<RefusedRead, Receipt> {
        if self.site == RefusalSite::Checkpoint && self.execution_guards.load(Ordering::Relaxed) > 0
        {
            self.refuse();
        }
        let evidence = Receipt {
            failed: self.failed.load(Ordering::Relaxed),
            reservations: self.reservations.load(Ordering::Relaxed),
            execution_attempts: self.execution_attempts.load(Ordering::Relaxed),
            reporting_guards: self.reporting_guards.load(Ordering::Relaxed),
            execution_guards: self.execution_guards.load(Ordering::Relaxed),
            reads: self.reads.load(Ordering::Relaxed),
        };
        *self.last_checkpoint.lock().expect("checkpoint receipt") = evidence;
        match self.read_error() {
            Some(error) => ViewOperationStatus::Failed { error, evidence },
            None => ViewOperationStatus::Ready { evidence },
        }
    }
}
const QUERY: &str = "SELECT ?s WHERE { ?s ?p ?o }";
fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
trait ViewReceipt {
    fn view_receipt(&self) -> &Receipt;
}
impl ViewReceipt for Receipt {
    fn view_receipt(&self) -> &Receipt {
        self
    }
}
impl ViewReceipt for GovernedEvidence<Receipt> {
    fn view_receipt(&self) -> &Receipt {
        &self.view
    }
}
fn assert_source_refusal<T: std::fmt::Debug, V: std::fmt::Debug + ViewReceipt>(
    result: Result<T, FallibleSparqlError<RefusedRead, V>>,
    source: &Source,
    expected: RefusedRead,
) -> FallibleSparqlError<RefusedRead, V> {
    let error =
        result.expect_err("an incomplete source cannot publish any answer or budget outcome");
    assert_eq!(error.operational_error(), Some(&expected));
    assert!(error.diagnostic().is_none());
    assert!(error.partial_answers().is_none());
    source.assert_receipt(error.evidence().view_receipt());
    error
}
const LOOKUP_QUERY: &str = "SELECT ?s WHERE { ?s <https://example.org/p> ?o }";
const CONSTRUCT: &str = "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }";

#[derive(Clone, Copy)]
enum QueryRoute {
    Owned,
    Prepared,
    Governed,
    Shared,
    GovernedSource,
    SharedSource,
}
const LOCAL_ROUTES: [QueryRoute; 4] = [
    QueryRoute::Owned,
    QueryRoute::Prepared,
    QueryRoute::Governed,
    QueryRoute::Shared,
];
struct RefusingService(AtomicUsize);
impl purrdf_sparql_eval::remote::ServiceResolver for RefusingService {
    fn resolve(
        &self,
        request: purrdf_sparql_eval::remote::ServiceRequest<'_>,
    ) -> Result<purrdf_sparql_eval::remote::ResolvedBindings, purrdf_sparql_eval::remote::RemoteError>
    {
        if let Some(tripped) = request.stop_trip() {
            return Err(tripped);
        }
        self.0.fetch_add(1, Ordering::Relaxed);
        Err(purrdf_sparql_eval::remote::RemoteError::Disabled)
    }
}
fn assert_query_refusal(
    engine: &NativeSparqlEngine,
    source: &Source,
    route: QueryRoute,
    query: &str,
    expected: RefusedRead,
) {
    let prepared = engine.prepare_query(query, None).expect("prepare fixture");
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let remote = RefusingService(AtomicUsize::new(0));
    match route {
        QueryRoute::Owned => {
            assert_source_refusal(
                engine.query_fallible_view(source, request(query), QueryOptions::EMPTY),
                source,
                expected,
            );
        }
        QueryRoute::Prepared => {
            assert_source_refusal(
                engine.query_prepared_fallible_view(source, &prepared, &[], QueryOptions::EMPTY),
                source,
                expected,
            );
        }
        QueryRoute::Governed => {
            assert_source_refusal(
                engine.query_governed_fallible_view(
                    source,
                    request(query),
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                ),
                source,
                expected,
            );
        }
        QueryRoute::Shared => {
            let error = assert_source_refusal(
                engine.query_prepared_governed_fallible_in_operation(
                    source,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &state,
                ),
                source,
                expected,
            );
            assert_eq!(error.evidence().governors, state.evidence());
        }
        QueryRoute::GovernedSource => {
            assert_source_refusal(
                engine.query_governed_fallible_with_source_view(
                    source,
                    request(query),
                    &remote,
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                ),
                source,
                expected,
            );
        }
        QueryRoute::SharedSource => {
            let error = assert_source_refusal(
                engine.query_prepared_governed_fallible_with_source_in_operation(
                    source,
                    &prepared,
                    &[],
                    &remote,
                    QueryOptions::EMPTY,
                    &state,
                ),
                source,
                expected,
            );
            assert_eq!(error.evidence().governors, state.evidence());
        }
    }
    assert_eq!(remote.0.load(Ordering::Relaxed), 0);
}
fn typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        for route in LOCAL_ROUTES {
            assert_query_refusal(
                &engine,
                &Source::at(site),
                route,
                QUERY,
                RefusedRead::FinalRead,
            );
        }
    }
}
fn preexisting_source_failure_outranks_parse_and_empty_pattern_success() {
    let engine = NativeSparqlEngine::new();
    let source = Source::at(RefusalSite::Never);
    source.refuse();
    assert_source_refusal(
        engine.query_fallible_view(&source, request("not SPARQL"), QueryOptions::EMPTY),
        &source,
        RefusedRead::FinalRead,
    );
    let empty = engine
        .prepare_query("SELECT * WHERE {}", None)
        .expect("prepare empty pattern");
    assert_source_refusal(
        engine.query_prepared_fallible_view(&source, &empty, &[], QueryOptions::EMPTY),
        &source,
        RefusedRead::FinalRead,
    );
    assert_source_refusal(
        engine.query_governed_fallible_view(
            &source,
            request("not SPARQL"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        &source,
        RefusedRead::FinalRead,
    );
}
fn retained_prepared_visits_remain_inside_the_checked_publication_scope() {
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        let source = Source::at(site);
        let subject = source.resident.quads().next().expect("fixture row").s;
        assert_source_refusal(
            engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
                let _read = source.resolve(subject);
                7
            }),
            &source,
            RefusedRead::FinalRead,
        );
    }
    let source = Source::at(RefusalSite::Never);
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
            source.refuse();
            7
        }),
        &source,
        RefusedRead::FinalRead,
    );
    let source = Source::at(RefusalSite::Never);
    let (value, receipt) = engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7)
        .expect("retained workspace answers valid neighbor");
    assert_eq!(value, 7);
    assert_held_receipt(&source, &receipt);
}
fn refused_reporting_admission_remains_typed_even_before_sticky_failure() {
    let engine = NativeSparqlEngine::new();
    for route in LOCAL_ROUTES {
        let source = Source::at(RefusalSite::Reporting);
        assert_query_refusal(&engine, &source, route, QUERY, RefusedRead::Workspace);
        assert!(source.read_error().is_none());
        assert_eq!(source.reads.load(Ordering::Relaxed), 0);
        assert_eq!(source.execution_attempts.load(Ordering::Relaxed), 0);
    }
    let source = Source::at(RefusalSite::Reporting);
    assert_source_refusal(
        engine.query_fallible_view(&source, request("not SPARQL"), QueryOptions::EMPTY),
        &source,
        RefusedRead::Workspace,
    );
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7),
        &source,
        RefusedRead::Workspace,
    );
    assert!(source.read_error().is_none());
}
fn direct_execution_admission_failures_keep_their_typed_cause_and_exact_evidence() {
    let engine = NativeSparqlEngine::new();
    for site in [
        RefusalSite::Execution,
        RefusalSite::ExecutionInitial,
        RefusalSite::ExecutionGrowth,
        RefusalSite::Lookup,
    ] {
        let query = if site == RefusalSite::Lookup {
            LOOKUP_QUERY
        } else {
            QUERY
        };
        let expected = if site == RefusalSite::Lookup {
            RefusedRead::Lookup
        } else {
            RefusedRead::Workspace
        };
        for route in LOCAL_ROUTES {
            let source = Source::at(site);
            assert_query_refusal(&engine, &source, route, query, expected);
            assert!(source.read_error().is_none());
            let receipt = *source.last_checkpoint.lock().expect("checkpoint receipt");
            assert!(!receipt.failed);
            assert_eq!(receipt.reporting_guards, 1);
            assert_eq!(receipt.execution_guards, 0);
            assert_eq!(
                receipt.execution_attempts,
                if site == RefusalSite::ExecutionGrowth {
                    2
                } else {
                    1
                }
            );
            if site != RefusalSite::Lookup {
                assert_eq!(receipt.reads, 0, "admission refusal precedes evaluation");
            }
        }
    }
    for route in [QueryRoute::GovernedSource, QueryRoute::SharedSource] {
        assert_query_refusal(
            &engine,
            &Source::at(RefusalSite::Execution),
            route,
            QUERY,
            RefusedRead::Workspace,
        );
    }
}
fn sticky_operational_root_outranks_the_direct_admission_error() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::ReportingSticky, RefusalSite::ExecutionSticky] {
        for route in LOCAL_ROUTES {
            let source = Source::at(site);
            assert_query_refusal(&engine, &source, route, QUERY, RefusedRead::FinalRead);
            assert_eq!(source.read_error(), Some(RefusedRead::FinalRead));
        }
    }
}
fn refused_execution_never_invokes_a_scoped_visitor_and_the_handle_can_retry() {
    let engine = NativeSparqlEngine::new();
    for site in [
        RefusalSite::Reporting,
        RefusalSite::Execution,
        RefusalSite::ExecutionInitial,
        RefusalSite::ExecutionGrowth,
        RefusalSite::ExecutionSticky,
        RefusalSite::Lookup,
    ] {
        let query = if site == RefusalSite::Lookup {
            LOOKUP_QUERY
        } else {
            QUERY
        };
        let expected = match site {
            RefusalSite::ExecutionSticky => RefusedRead::FinalRead,
            RefusalSite::Lookup => RefusedRead::Lookup,
            _ => RefusedRead::Workspace,
        };
        let mut execution = engine
            .prepare_execution(query, None, &[], QueryOptions::EMPTY)
            .expect("prepare execution");
        let source = Source::at(site);
        let visited = AtomicBool::new(false);
        assert_source_refusal(
            engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
                visited.store(true, Ordering::Relaxed);
                7
            }),
            &source,
            expected,
        );
        assert!(!visited.load(Ordering::Relaxed));
        let healthy = Source::at(RefusalSite::Never);
        let (value, receipt) = engine
            .execute_fallible(&mut execution, &healthy, QueryOptions::EMPTY, |_| {
                visited.store(true, Ordering::Relaxed);
                7
            })
            .expect("an admission refusal does not consume the retained handle");
        assert!(visited.load(Ordering::Relaxed));
        assert_eq!(value, 7);
        assert_held_receipt(&healthy, &receipt);
    }
}
fn assert_held_receipt(source: &Source, receipt: &Receipt) {
    assert_workspace_receipt(source, receipt);
    assert!(receipt.reads > 0);
}
fn assert_workspace_receipt(source: &Source, receipt: &Receipt) {
    source.assert_receipt(receipt);
    assert!(!receipt.failed);
    assert_eq!(receipt.reporting_guards, 1);
    assert_eq!(receipt.execution_guards, 1);
    let executions = if source.storage_live_budget().is_some() {
        2
    } else {
        1
    };
    assert_eq!(receipt.execution_attempts, executions);
    assert_eq!(receipt.reservations, executions + 2);
}
fn assert_complete_query<V: ViewReceipt>(
    source: &Source,
    answer: purrdf_sparql_eval::CompleteSparqlResult<V>,
) {
    assert!(
        matches!(answer.result, purrdf_core::SparqlResult::Solutions { rows, .. } if rows.len() == 1)
    );
    assert_held_receipt(source, answer.evidence.view_receipt());
}
fn healthy_query_routes_hold_execution_admission_through_final_checkpoint() {
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        for route in LOCAL_ROUTES {
            let source = Source::at(site);
            let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
            match route {
                QueryRoute::Owned => assert_complete_query(
                    &source,
                    engine
                        .query_fallible_view(&source, request(QUERY), QueryOptions::EMPTY)
                        .expect("healthy owned query"),
                ),
                QueryRoute::Prepared => assert_complete_query(
                    &source,
                    engine
                        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                        .expect("healthy prepared query"),
                ),
                QueryRoute::Governed => assert_complete_query(
                    &source,
                    engine
                        .query_governed_fallible_view(
                            &source,
                            request(QUERY),
                            QueryOptions::EMPTY,
                            &QueryGovernors::METERED,
                        )
                        .expect("healthy governed query"),
                ),
                QueryRoute::Shared => {
                    let answer = engine
                        .query_prepared_governed_fallible_in_operation(
                            &source,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &state,
                        )
                        .expect("healthy shared operation");
                    assert_eq!(answer.evidence.governors, state.evidence());
                    assert_complete_query(&source, answer);
                }
                QueryRoute::GovernedSource | QueryRoute::SharedSource => {
                    unreachable!("local route set")
                }
            }
        }
    }
}
fn final_checkpoint_failure_discards_success_and_keeps_workspace_held() {
    let engine = NativeSparqlEngine::new();
    for route in LOCAL_ROUTES {
        let source = Source::at(RefusalSite::Checkpoint);
        assert_query_refusal(&engine, &source, route, QUERY, RefusedRead::FinalRead);
        let receipt = *source.last_checkpoint.lock().expect("checkpoint receipt");
        assert!(receipt.failed);
        assert_eq!(receipt.execution_guards, 1);
        assert_eq!(receipt.reporting_guards, 1);
        assert!(receipt.reads > 0);
    }
    let source = Source::at(RefusalSite::Checkpoint);
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    let visited = AtomicBool::new(false);
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
            visited.store(true, Ordering::Relaxed);
            7
        }),
        &source,
        RefusedRead::FinalRead,
    );
    assert!(
        visited.load(Ordering::Relaxed),
        "the refusal is after the visit"
    );
    assert_eq!(
        source
            .last_checkpoint
            .lock()
            .expect("checkpoint receipt")
            .execution_guards,
        1
    );
}
fn destination() -> RdfDatasetBuilder {
    let mut destination = RdfDatasetBuilder::new();
    let s = destination.intern_iri("https://example.org/retained");
    let p = destination.intern_iri("https://example.org/edge");
    let o = destination.intern_iri("https://example.org/value");
    destination.push_quad(s, p, o, None);
    destination
}
fn graph_admission_and_final_checkpoint_failures_leave_destination_unchanged() {
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(CONSTRUCT, None)
        .expect("prepare construct");
    let expected = destination().freeze().expect("expected destination");
    for site in [
        RefusalSite::Reporting,
        RefusalSite::Execution,
        RefusalSite::ExecutionSticky,
        RefusalSite::Checkpoint,
        RefusalSite::Iterator,
        RefusalSite::Term,
    ] {
        let source = Source::at(site);
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let mut destination = destination();
        let error = assert_source_refusal(
            engine.construct_prepared_fallible_in_operation_into_view(
                &source,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &state,
                &mut destination,
            ),
            &source,
            if matches!(site, RefusalSite::Reporting | RefusalSite::Execution) {
                RefusedRead::Workspace
            } else {
                RefusedRead::FinalRead
            },
        );
        assert_eq!(error.evidence().governors, state.evidence());
        let actual = destination.freeze().expect("unchanged destination");
        assert_eq!(actual.term_count(), expected.term_count());
        assert_eq!(
            actual.owned_quads().collect::<Vec<_>>(),
            expected.owned_quads().collect::<Vec<_>>()
        );
        if site == RefusalSite::Checkpoint {
            assert_eq!(error.evidence().view.execution_guards, 1);
        }
    }
    let source = Source::at(RefusalSite::Never);
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let mut destination = destination();
    let (stats, evidence) = engine
        .construct_prepared_fallible_in_operation_into_view(
            &source,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &state,
            &mut destination,
        )
        .expect("healthy graph publication");
    assert_eq!(stats.statements, 1);
    assert_eq!(evidence.governors, state.evidence());
    assert_held_receipt(&source, &evidence.view);
    assert_eq!(
        destination.freeze().expect("published graph").quad_count(),
        2
    );
}
#[derive(Clone, Copy)]
enum ExplainRoute {
    Default,
    Options,
    QuietStop,
}
const EXPLAIN_ROUTES: [ExplainRoute; 3] = [
    ExplainRoute::Default,
    ExplainRoute::Options,
    ExplainRoute::QuietStop,
];
#[allow(
    clippy::result_large_err,
    reason = "test helper preserves the public operational result shared by all three EXPLAIN entries"
)]
fn explain(
    engine: &NativeSparqlEngine,
    source: &Source,
    route: ExplainRoute,
    query: &str,
) -> FallibleScopedResult<QueryExplanation, RefusedRead, Receipt> {
    match route {
        ExplainRoute::Default => engine.explain_query_fallible_view(source, query, None),
        ExplainRoute::Options => engine.explain_query_with_options_fallible_view(
            source,
            query,
            None,
            QueryOptions::EMPTY,
        ),
        ExplainRoute::QuietStop => engine.explain_query_with_stop_signal_fallible_view(
            source,
            query,
            None,
            QueryOptions::EMPTY,
            Arc::new(CancellationFlag::new()),
        ),
    }
}
fn explain_admission_failures_remain_typed_without_an_explanation() {
    let engine = NativeSparqlEngine::new();
    for site in [
        RefusalSite::Reporting,
        RefusalSite::Execution,
        RefusalSite::ExecutionInitial,
        RefusalSite::ExecutionGrowth,
        RefusalSite::Lookup,
        RefusalSite::ReportingSticky,
        RefusalSite::ExecutionSticky,
    ] {
        let expected = match site {
            RefusalSite::Lookup => RefusedRead::Lookup,
            RefusalSite::ReportingSticky | RefusalSite::ExecutionSticky => RefusedRead::FinalRead,
            _ => RefusedRead::Workspace,
        };
        let query = if site == RefusalSite::Lookup {
            LOOKUP_QUERY
        } else {
            QUERY
        };
        for route in EXPLAIN_ROUTES {
            let source = Source::at(site);
            let error =
                assert_source_refusal(explain(&engine, &source, route, query), &source, expected);
            let receipt = error.evidence();
            assert_eq!(receipt.failed, expected == RefusedRead::FinalRead);
            assert_eq!(receipt.execution_guards, 0);
            let attempts = match site {
                RefusalSite::Reporting | RefusalSite::ReportingSticky => 0,
                RefusalSite::ExecutionGrowth => 2,
                _ => 1,
            };
            assert_eq!(receipt.execution_attempts, attempts);
            if site != RefusalSite::Lookup {
                assert_eq!(receipt.reads, 0);
            }
        }
    }
}
fn explain_preflight_failure_outranks_malformed_query() {
    let engine = NativeSparqlEngine::new();
    for route in EXPLAIN_ROUTES {
        let source = Source::at(RefusalSite::Never);
        source.refuse();
        let error = assert_source_refusal(
            explain(&engine, &source, route, "not SPARQL"),
            &source,
            RefusedRead::FinalRead,
        );
        assert_eq!(error.evidence().reservations, 0);
        assert_eq!(error.evidence().reads, 0);
    }
}
fn explain_measuring_and_final_checkpoint_failures_publish_no_ledger() {
    let engine = NativeSparqlEngine::new();
    for site in [
        RefusalSite::Iterator,
        RefusalSite::Term,
        RefusalSite::Checkpoint,
    ] {
        let query = if site == RefusalSite::Term {
            "SELECT ?s WHERE { ?s ?p ?o FILTER(STR(?s) = \"http://example.org/s\") }"
        } else {
            QUERY
        };
        for route in EXPLAIN_ROUTES {
            let source = Source::at(site);
            let error = assert_source_refusal(
                explain(&engine, &source, route, query),
                &source,
                RefusedRead::FinalRead,
            );
            assert_eq!(error.evidence().reporting_guards, 1);
            assert_eq!(error.evidence().execution_guards, 1);
            assert!(error.evidence().reads > 0);
        }
    }
}
fn healthy_explain_is_identical_to_resident_and_holds_admission_through_publication() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        for route in EXPLAIN_ROUTES {
            let source = Source::at(site);
            let expected = engine
                .explain_query(&source.resident, QUERY, None)
                .expect("resident explanation");
            let (actual, receipt) =
                explain(&engine, &source, route, QUERY).expect("healthy operational explanation");
            assert_eq!(actual.render(), expected.render());
            assert_eq!(actual.evidence(), expected.evidence());
            assert_eq!(actual.evidence().tripped, None);
            assert_held_receipt(&source, &receipt);
        }
    }
    let source = Source::at(RefusalSite::Never);
    let options = QueryOptions::EMPTY
        .with_prebinding(ShaclPrebinding::Applied)
        .with_bnode_mint_prefix(Some("configured-mint"));
    let expected = engine
        .explain_query_with_options(&source.resident, QUERY, None, options)
        .expect("resident configured explanation");
    let (actual, receipt) = engine
        .explain_query_with_options_fallible_view(&source, QUERY, None, options)
        .expect("operational configured explanation");
    assert_eq!(actual.render(), expected.render());
    assert_held_receipt(&source, &receipt);
}
fn healthy_explain_preserves_parse_and_evaluation_diagnostics_and_final_evidence() {
    let engine = NativeSparqlEngine::new();
    for query in [
        "not SPARQL",
        "SELECT ?s WHERE { SERVICE <https://example.org/service> { ?s ?p ?o } }",
    ] {
        for route in EXPLAIN_ROUTES {
            let source = Source::at(RefusalSite::Never);
            let expected = engine
                .explain_query(&source.resident, query, None)
                .expect_err("resident query diagnostic");
            let error = explain(&engine, &source, route, query)
                .expect_err("healthy operational view retains the query diagnostic");
            let diagnostic = error.diagnostic().expect("query diagnostic");
            assert_eq!(diagnostic, &expected);
            assert!(error.operational_error().is_none());
            assert!(error.partial_answers().is_none());
            source.assert_receipt(error.evidence());
            assert!(!error.evidence().failed);
            assert_eq!(error.evidence().reporting_guards, 1);
            assert_eq!(
                error.evidence().execution_guards,
                usize::from(query != "not SPARQL")
            );
        }
    }
    let source = Source::at(RefusalSite::Checkpoint);
    let error = assert_source_refusal(
        engine.explain_query_fallible_view(
            &source,
            "SELECT ?s WHERE { SERVICE <https://example.org/service> { ?s ?p ?o } }",
            None,
        ),
        &source,
        RefusedRead::FinalRead,
    );
    assert_eq!(error.evidence().execution_guards, 1);
}
fn explain_refuses_unpriced_supplied_inputs_before_parsing_or_reading() {
    let engine = NativeSparqlEngine::new();
    let remote = RefusingService(AtomicUsize::new(0));
    for options in [
        QueryOptions::EMPTY.with_bnode_mint_prefix(Some("configured-mint")),
        QueryOptions::EMPTY.with_remote(Some(&remote)),
    ] {
        let source = Source::at(RefusalSite::BoundedHealthy);
        let cache = engine.plan_cache_stats();
        let error = engine
            .explain_query_with_options_fallible_view(&source, "not SPARQL", None, options)
            .expect_err("unpriced context is refused before parsing");
        assert_eq!(
            error.diagnostic().expect("unpriced diagnostic").code,
            "native-sparql-workspace-unpriced"
        );
        assert!(error.operational_error().is_none());
        source.assert_receipt(error.evidence());
        assert_eq!(error.evidence().reservations, 1);
        assert_eq!(error.evidence().reporting_guards, 1);
        assert_eq!(error.evidence().execution_attempts, 0);
        assert_eq!(error.evidence().reads, 0);
        assert_eq!(engine.plan_cache_stats(), cache);
    }
    assert_eq!(remote.0.load(Ordering::Relaxed), 0);
}
#[derive(Debug)]
struct FailureAndStop(Arc<Source>);
impl StopSignal for FailureAndStop {
    fn poll(&self) -> Option<StopCause> {
        self.0.refuse();
        Some(StopCause::Cancelled)
    }
}
fn explain_stop_retains_stopped_receipt_while_operational_failure_takes_precedence() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        let source = Source::at(site);
        let stop = CancellationFlag::new();
        stop.cancel();
        let expected = engine
            .explain_query_with_stop_signal(
                &source.resident,
                QUERY,
                None,
                QueryOptions::EMPTY,
                Arc::new(stop.clone()),
            )
            .expect("resident cancellation receipt");
        let (actual, receipt) = engine
            .explain_query_with_stop_signal_fallible_view(
                &source,
                QUERY,
                None,
                QueryOptions::EMPTY,
                Arc::new(stop),
            )
            .expect("a stop alone preserves the explanation");
        assert_eq!(
            actual.evidence().tripped,
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );
        assert_eq!(actual.render(), expected.render());
        assert_workspace_receipt(&source, &receipt);
    }
    let source = Source::at(RefusalSite::Checkpoint);
    let stop = CancellationFlag::new();
    stop.cancel();
    let error = assert_source_refusal(
        engine.explain_query_with_stop_signal_fallible_view(
            &source,
            QUERY,
            None,
            QueryOptions::EMPTY,
            Arc::new(stop),
        ),
        &source,
        RefusedRead::FinalRead,
    );
    assert_eq!(error.evidence().execution_guards, 1);
    let source = Arc::new(Source::at(RefusalSite::Never));
    let error = assert_source_refusal(
        engine.explain_query_with_stop_signal_fallible_view(
            source.as_ref(),
            QUERY,
            None,
            QueryOptions::EMPTY,
            Arc::new(FailureAndStop(Arc::clone(&source))),
        ),
        source.as_ref(),
        RefusedRead::FinalRead,
    );
    assert_eq!(error.evidence().execution_guards, 1);
}
purrdf_testkit::harness_main!(
    typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures,
    preexisting_source_failure_outranks_parse_and_empty_pattern_success,
    retained_prepared_visits_remain_inside_the_checked_publication_scope,
    refused_reporting_admission_remains_typed_even_before_sticky_failure,
    direct_execution_admission_failures_keep_their_typed_cause_and_exact_evidence,
    sticky_operational_root_outranks_the_direct_admission_error,
    refused_execution_never_invokes_a_scoped_visitor_and_the_handle_can_retry,
    healthy_query_routes_hold_execution_admission_through_final_checkpoint,
    final_checkpoint_failure_discards_success_and_keeps_workspace_held,
    graph_admission_and_final_checkpoint_failures_leave_destination_unchanged,
    explain_admission_failures_remain_typed_without_an_explanation,
    explain_preflight_failure_outranks_malformed_query,
    explain_measuring_and_final_checkpoint_failures_publish_no_ledger,
    healthy_explain_is_identical_to_resident_and_holds_admission_through_publication,
    healthy_explain_preserves_parse_and_evaluation_diagnostics_and_final_evidence,
    explain_refuses_unpriced_supplied_inputs_before_parsing_or_reading,
    explain_stop_retains_stopped_receipt_while_operational_failure_takes_precedence,
);
