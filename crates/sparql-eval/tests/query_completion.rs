// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// The pre-binding lane is deprecated and inert; these tests still name it.
#![allow(deprecated)]

//! Generic query publication must include reads after the last algebra node.
mod support;

use purrdf_core::TermGuard as _;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, OwnedWorkspaceReservation, QuadIds, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, SparqlEngine, SparqlRequest, SparqlResult,
    StopCause, TermId, TermValue, TrippedGovernor, ViewOperationStatus, WorkspaceReservation,
};
use purrdf_sparql_eval::{
    CancellationFlag, FallibleScopedResult, FallibleSparqlError, FallibleSparqlResult,
    GovernedEvidence, GovernorState, NativeSparqlEngine, PartialAnswers, PreparedQuery,
    QueryGovernors, QueryOptions, RetainedQueryExplanation, ShaclPrebinding, StopSignal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusedRead {
    FinalRead(RefusalSite),
    Workspace,
    Lookup,
}
impl std::fmt::Display for RefusedRead {
    /// Preserves the read site and distinguishes direct admission causes in diagnostics.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FinalRead(site) => write!(out, "source refused at {site:?}"),
            Self::Workspace => out.write_str("source refused execution workspace"),
            Self::Lookup => out.write_str("source refused admission lookup"),
        }
    }
}
impl std::error::Error for RefusedRead {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusalSite {
    Never,
    Iterator,
    AfterRow,
    Term,
    Reporting,
    Execution,
    ExecutionInitial,
    ExecutionGrowth,
    ExecutionSticky,
    ReportingSticky,
    Lookup,
    Checkpoint,
    FinalCheckpoint,
    BoundedHealthy,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Receipt {
    failed: bool,
    yielded: usize,
    checkpoints: usize,
    reservations: usize,
    execution_attempts: usize,
    reporting_guards: usize,
    execution_guards: usize,
    reads: usize,
    growth_attempts: usize,
    owned_bytes: u64,
}
#[derive(Debug, Clone)]
struct Source {
    resident: Arc<RdfDataset>,
    state: Arc<SourceState>,
}
#[derive(Debug)]
struct SourceState {
    failed: AtomicBool,
    yielded: AtomicUsize,
    checkpoints: AtomicUsize,
    site: RefusalSite,
    reservations: AtomicUsize,
    execution_attempts: AtomicUsize,
    reporting_guards: AtomicUsize,
    execution_guards: AtomicUsize,
    reads: AtomicUsize,
    last_checkpoint: Mutex<Receipt>,
    growth_attempts: AtomicUsize,
    owned_bytes: AtomicU64,
    peak_owned_bytes: AtomicU64,
    term_bound: u64,
    live_limit: u64,
    final_checkpoint: AtomicUsize,
}
impl SourceState {
    /// Requires the final cache, execution, and published-result owners to release
    /// the same original account without inventing a second bounded permit.
    fn assert_released(&self) {
        assert_eq!(self.reporting_guards.load(Ordering::Relaxed), 0);
        assert_eq!(self.execution_guards.load(Ordering::Relaxed), 0);
        assert_eq!(self.owned_bytes.load(Ordering::Relaxed), 0);
    }
}
impl Drop for SourceState {
    fn drop(&mut self) {
        self.assert_released();
    }
}
impl std::ops::Deref for Source {
    type Target = SourceState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
impl Source {
    /// Creates a one-row source that refuses at one selected operational boundary.
    fn at(site: RefusalSite) -> Self {
        Self {
            resident: support::local_dataset([("s", "p", "o")]),
            state: Arc::new(SourceState {
                failed: AtomicBool::new(false),
                yielded: AtomicUsize::new(0),
                checkpoints: AtomicUsize::new(0),
                site,
                reservations: AtomicUsize::new(0),
                execution_attempts: AtomicUsize::new(0),
                reporting_guards: AtomicUsize::new(0),
                execution_guards: AtomicUsize::new(0),
                reads: AtomicUsize::new(0),
                last_checkpoint: Mutex::new(Receipt::default()),
                growth_attempts: AtomicUsize::new(0),
                owned_bytes: AtomicU64::new(0),
                peak_owned_bytes: AtomicU64::new(0),
                term_bound: 256,
                live_limit: u64::MAX,
                final_checkpoint: AtomicUsize::new(0),
            }),
        }
    }
    /// Latches a source failure so later checkpoints must retain its root cause.
    fn refuse(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }
    /// Requires the published receipt to match the last checkpoint before cleanup.
    fn assert_receipt(&self, receipt: &Receipt) {
        assert_eq!(
            receipt,
            &*self.last_checkpoint.lock().expect("checkpoint receipt")
        );
    }
}
struct Reservation<'a> {
    source: &'a Source,
    execution: bool,
}
impl WorkspaceReservation for Reservation<'_> {
    type Error = RefusedRead;
    /// Keeps this lifetime-tracking fixture independent of byte-capacity failures.
    fn resize(&mut self, _: u64) -> Result<(), RefusedRead> {
        Ok(())
    }
}
impl Drop for Reservation<'_> {
    /// Releases exactly one owner, exposing leaked or duplicate guard lifetimes.
    fn drop(&mut self) {
        let live = if self.execution {
            &self.source.execution_guards
        } else {
            &self.source.reporting_guards
        };
        assert!(live.fetch_sub(1, Ordering::Relaxed) > 0);
    }
}
/// This provider owns its state, and admits its concrete erased box before
/// construction. The fixed box charge cannot be removed by resize(0).
struct OwnedReservation {
    source: Arc<SourceState>,
    bytes: u64,
    control: u64,
}
impl WorkspaceReservation for OwnedReservation {
    type Error = RefusedRead;

    fn resize(&mut self, bytes: u64) -> Result<(), RefusedRead> {
        if bytes > self.bytes {
            let prior = self.source.growth_attempts.fetch_add(1, Ordering::Relaxed);
            if matches!(
                (self.source.site, prior),
                (RefusalSite::ExecutionInitial, 0) | (RefusalSite::ExecutionGrowth, 1)
            ) {
                return Err(RefusedRead::Workspace);
            }
        }
        let previous = self
            .bytes
            .checked_add(self.control)
            .ok_or(RefusedRead::Workspace)?;
        let next = bytes
            .checked_add(self.control)
            .ok_or(RefusedRead::Workspace)?;
        if next >= previous {
            let prior = self
                .source
                .owned_bytes
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |live| {
                    live.checked_add(next - previous)
                        .filter(|bytes| *bytes <= self.source.live_limit)
                })
                .map_err(|_| RefusedRead::Workspace)?;
            self.source
                .peak_owned_bytes
                .fetch_max(prior + (next - previous), Ordering::Relaxed);
        } else {
            self.source
                .owned_bytes
                .fetch_sub(previous - next, Ordering::Relaxed);
        }
        self.bytes = bytes;
        Ok(())
    }
}
impl Drop for OwnedReservation {
    fn drop(&mut self) {
        let total = self
            .bytes
            .checked_add(self.control)
            .expect("admitted provider layout");
        assert!(self.source.owned_bytes.fetch_sub(total, Ordering::Relaxed) >= total);
        assert!(self.source.reporting_guards.fetch_sub(1, Ordering::Relaxed) > 0);
    }
}
impl DatasetView for Source {
    type Id = TermId;
    type ReadError = RefusedRead;
    type TermGuard<'a> = <RdfDataset as DatasetView>::TermGuard<'a>;
    type ProbePlan = ();

    fn statistics(&self) -> Option<&dyn purrdf_core::SnapshotStatistics<TermId>> {
        self.read_error().is_none().then_some(())?;
        self.resident.statistics()
    }
    /// Enables bounded admission without letting capacity mask the scripted refusal.
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
        .then_some(self.live_limit)
    }
    /// Supplies the finite term bound required to price this one-row fixture.
    fn max_owned_term_bytes(&self) -> Option<u64> {
        Some(self.term_bound)
    }
    /// Tracks reporting and execution owners and injects direct or sticky refusals.
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = RefusedRead> + '_, RefusedRead> {
        self.reservations.fetch_add(1, Ordering::Relaxed);
        assert_eq!(bytes, 0, "unbounded scoped admission has no byte claim");
        // The first borrowed permit owns ingress reporting. Execution is a
        // separate zero-byte provider operation while that original permit lives.
        let execution = self.reporting_guards.load(Ordering::Relaxed) != 0;
        if execution {
            self.execution_attempts.fetch_add(1, Ordering::Relaxed);
        }
        let refused = match self.site {
            RefusalSite::Reporting | RefusalSite::ReportingSticky => !execution,
            RefusalSite::Execution | RefusalSite::ExecutionSticky => execution,
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
    /// Bounded output obtains the actual provider-owned reservation. Native
    /// grow callbacks, not guessed fixed byte amounts, select refusal phases.
    fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Option<Result<OwnedWorkspaceReservation<RefusedRead>, RefusedRead>> {
        self.storage_live_budget()?;
        Some((|| {
            self.reservations.fetch_add(1, Ordering::Relaxed);
            if matches!(
                self.site,
                RefusalSite::Reporting | RefusalSite::ReportingSticky
            ) {
                if self.site == RefusalSite::ReportingSticky {
                    self.refuse();
                }
                return Err(RefusedRead::Workspace);
            }
            let control =
                u64::try_from(size_of::<OwnedReservation>()).map_err(|_| RefusedRead::Workspace)?;
            let total = bytes.checked_add(control).ok_or(RefusedRead::Workspace)?;
            let prior = self
                .owned_bytes
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |live| {
                    live.checked_add(total)
                        .filter(|bytes| *bytes <= self.live_limit)
                })
                .map_err(|_| RefusedRead::Workspace)?;
            self.peak_owned_bytes
                .fetch_max(prior + total, Ordering::Relaxed);
            self.reporting_guards.fetch_add(1, Ordering::Relaxed);
            // A failed native box factory drops this original reservation and
            // refunds the admission; no owned output or guard was published.
            let reservation = OwnedReservation {
                source: self.state.clone(),
                bytes,
                control,
            };
            let boxed =
                purrdf_core::small::try_boxed(reservation).map_err(|_| RefusedRead::Workspace)?;
            Ok(boxed as OwnedWorkspaceReservation<RefusedRead>)
        })())
    }

    /// Exposes the latched read site without advancing the publication checkpoint.
    fn read_error(&self) -> Option<RefusedRead> {
        self.failed
            .load(Ordering::Relaxed)
            .then_some(RefusedRead::FinalRead(self.site))
    }
    /// Injects failure before the sole row or at stream end after yielding it.
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        let mut rows = self.resident.as_ref().quads();
        std::iter::from_fn(move || {
            self.reads.fetch_add(1, Ordering::Relaxed);
            let row = rows.next();
            match (self.site, row) {
                (RefusalSite::Iterator, _) | (RefusalSite::AfterRow, None) => {
                    self.refuse();
                    None
                }
                (_, Some(row)) => {
                    self.yielded.fetch_add(1, Ordering::Relaxed);
                    Some(row)
                }
                (_, None) => None,
            }
        })
    }
    /// Injects a sticky term-read failure to exercise checked result materialization.
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, RefusedRead> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if self.site == RefusalSite::Term {
            self.refuse();
            return Err(RefusedRead::FinalRead(self.site));
        }
        Ok(
            <RdfDataset as DatasetView>::resolve(self.resident.as_ref(), id)
                .expect("resident term resolution is infallible"),
        )
    }
    /// Refuses admission lookup directly without creating a sticky source failure.
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
    /// Captures live owners and rows, with faults at admitted execution or the second sample.
    fn operation_status(&self) -> ViewOperationStatus<RefusedRead, Receipt> {
        let prior = self.checkpoints.fetch_add(1, Ordering::Relaxed);
        if (self.site == RefusalSite::Checkpoint
            && self.execution_guards.load(Ordering::Relaxed) > 0)
            || (self.site == RefusalSite::FinalCheckpoint
                && prior + 1 == self.final_checkpoint.load(Ordering::Relaxed))
        {
            self.refuse();
        }
        let evidence = Receipt {
            failed: self.failed.load(Ordering::Relaxed),
            yielded: self.yielded.load(Ordering::Relaxed),
            checkpoints: prior + 1,
            reservations: self.reservations.load(Ordering::Relaxed),
            execution_attempts: self.execution_attempts.load(Ordering::Relaxed),
            reporting_guards: self.reporting_guards.load(Ordering::Relaxed),
            execution_guards: self.execution_guards.load(Ordering::Relaxed),
            reads: self.reads.load(Ordering::Relaxed),
            growth_attempts: self.growth_attempts.load(Ordering::Relaxed),
            owned_bytes: self.owned_bytes.load(Ordering::Relaxed),
        };
        *self.last_checkpoint.lock().expect("checkpoint receipt") = evidence;
        match self.read_error() {
            Some(error) => ViewOperationStatus::Failed { error, evidence },
            None => ViewOperationStatus::Ready { evidence },
        }
    }
}
const QUERY: &str = "SELECT ?s WHERE { ?s ?p ?o }";
/// Requests default-dataset evaluation without a base or caller substitutions.
fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
/// Extracts the storage checkpoint uniformly from plain and governed egresses.
trait ViewReceipt {
    /// Selects the storage receipt used to verify publication and guard ownership.
    fn view_receipt(&self) -> &Receipt;
}
impl ViewReceipt for Receipt {
    /// The plain egress carries its storage checkpoint directly.
    fn view_receipt(&self) -> &Receipt {
        self
    }
}
impl ViewReceipt for GovernedEvidence<Receipt> {
    /// Keeps storage ownership checks separate from governor consumption evidence.
    fn view_receipt(&self) -> &Receipt {
        &self.view
    }
}
/// Requires an exact operational cause and checkpoint without a partial answer.
fn assert_source_refusal<T: std::fmt::Debug, V: std::fmt::Debug + ViewReceipt>(
    result: Result<T, FallibleSparqlError<RefusedRead, V>>,
    source: &Source,
    expected: RefusedRead,
) -> FallibleSparqlError<RefusedRead, V> {
    let error =
        result.expect_err("an incomplete source cannot publish any answer or budget outcome");
    assert_eq!(
        error.operational_error(),
        Some(&expected),
        "actual source outcome: {error:?}"
    );
    assert!(error.diagnostic().is_none());
    assert!(error.partial_answers().is_none());
    assert!(error.tripped().is_none());
    source.assert_receipt(error.evidence().view_receipt());
    assert_eq!(source.execution_guards.load(Ordering::Relaxed), 0);
    if source.storage_live_budget().is_none() {
        source.assert_released();
    }
    // A bounded engine may retain its admitted key scratch after failure. Its
    // original account must live until that cache is destroyed, not until this
    // receipt is sampled. SourceState checks exact zero at final destruction.
    error
}
const LOOKUP_QUERY: &str = "SELECT ?s WHERE { ?s <https://example.org/p> ?o }";
const GRAPH_LOOKUP_QUERY: &str = "SELECT ?s WHERE { GRAPH <https://example.org/g> { ?s ?p ?o } }";
const CONSTRUCT: &str = "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }";

#[derive(Clone, Copy)]
enum QueryRoute {
    Owned,
    Prepared,
    Governed,
    PreparedGoverned,
    Shared,
    GovernedSource,
    PreparedGovernedSource,
    SharedSource,
}
const LOCAL_ROUTES: [QueryRoute; 5] = [
    QueryRoute::Owned,
    QueryRoute::Prepared,
    QueryRoute::Governed,
    QueryRoute::PreparedGoverned,
    QueryRoute::Shared,
];
const SOURCE_ROUTES: [QueryRoute; 3] = [
    QueryRoute::GovernedSource,
    QueryRoute::PreparedGovernedSource,
    QueryRoute::SharedSource,
];
struct RefusingService(AtomicUsize);
impl purrdf_sparql_eval::remote::ServiceResolver for RefusingService {
    /// The callback allocates nothing and returns only the static Disabled variant.
    fn workspace_certificate(
        &self,
        _: purrdf_sparql_eval::remote::ServiceRequest<'_>,
    ) -> Option<purrdf_sparql_eval::remote::ServiceWorkspaceCertificate> {
        Some(purrdf_sparql_eval::remote::ServiceWorkspaceCertificate { peak_bytes: 0 })
    }
    /// Records attempted SERVICE work while respecting a stop before dispatch.
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
/// Checks typed refusal across query routes without allowing outbound SERVICE work.
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
        QueryRoute::Owned | QueryRoute::Prepared => {
            let result = if matches!(route, QueryRoute::Owned) {
                engine.query_fallible_view(source, request(query), QueryOptions::EMPTY)
            } else {
                engine.query_prepared_fallible_view(source, &prepared, &[], QueryOptions::EMPTY)
            };
            assert_source_refusal(result, source, expected);
        }
        _ => {
            let result = match route {
                QueryRoute::Governed => engine.query_governed_fallible_view(
                    source,
                    request(query),
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                ),
                QueryRoute::PreparedGoverned => engine.query_prepared_governed_fallible_view(
                    source,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                ),
                QueryRoute::Shared => engine.query_prepared_governed_fallible_in_operation(
                    source,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &state,
                ),
                QueryRoute::GovernedSource => engine.query_governed_fallible_with_source_view(
                    source,
                    request(query),
                    &remote,
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                ),
                QueryRoute::PreparedGovernedSource => engine
                    .query_prepared_governed_fallible_with_source_view(
                        source,
                        &prepared,
                        &[],
                        &remote,
                        QueryOptions::EMPTY,
                        &QueryGovernors::METERED,
                    ),
                QueryRoute::SharedSource => engine
                    .query_prepared_governed_fallible_with_source_in_operation(
                        source,
                        &prepared,
                        &[],
                        &remote,
                        QueryOptions::EMPTY,
                        &state,
                    ),
                QueryRoute::Owned | QueryRoute::Prepared => unreachable!("governed route"),
            };
            let error = assert_source_refusal(result, source, expected);
            if matches!(route, QueryRoute::Shared | QueryRoute::SharedSource)
                || source.site == RefusalSite::Reporting
            {
                assert_eq!(error.evidence().governors, state.evidence());
            }
        }
    }
    assert_eq!(remote.0.load(Ordering::Relaxed), 0);
}
/// Iterator and term failures must prevent publication across owned and governed routes.
fn typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        for route in LOCAL_ROUTES {
            assert_query_refusal(
                &engine,
                &Source::at(site),
                route,
                QUERY,
                RefusedRead::FinalRead(site),
            );
        }
    }
}
/// Preflight preserves a latched storage cause before parsing or empty-pattern success.
fn preexisting_source_failure_outranks_parse_and_empty_pattern_success() {
    let engine = NativeSparqlEngine::new();
    let source = Source::at(RefusalSite::Never);
    source.refuse();
    assert_source_refusal(
        engine.query_fallible_view(&source, request("not SPARQL"), QueryOptions::EMPTY),
        &source,
        RefusedRead::FinalRead(source.site),
    );
    let empty = engine
        .prepare_query("SELECT * WHERE {}", None)
        .expect("prepare empty pattern");
    assert_source_refusal(
        engine.query_prepared_fallible_view(&source, &empty, &[], QueryOptions::EMPTY),
        &source,
        RefusedRead::FinalRead(source.site),
    );
    assert_source_refusal(
        engine.query_governed_fallible_view(
            &source,
            request("not SPARQL"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        &source,
        RefusedRead::FinalRead(source.site),
    );
    assert_source_refusal(
        engine.query_prepared_governed_fallible_view(
            &source,
            &empty,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        &source,
        RefusedRead::FinalRead(source.site),
    );
}
/// Visitor reads remain checked, and a retained execution can retry on a healthy source.
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
            RefusedRead::FinalRead(source.site),
        );
    }
    let source = Source::at(RefusalSite::Never);
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
            source.refuse();
            7
        }),
        &source,
        RefusedRead::FinalRead(source.site),
    );
    let source = Source::at(RefusalSite::Never);
    let (value, receipt) = engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7)
        .expect("retained workspace answers valid neighbor");
    assert_eq!(value, 7);
    assert_held_receipt(&source, &receipt);
}
/// Reporting refusal stays operational and precedes parsing, execution, and reads.
fn refused_reporting_admission_remains_typed_even_before_sticky_failure() {
    let engine = NativeSparqlEngine::new();
    for route in LOCAL_ROUTES.into_iter().chain(SOURCE_ROUTES) {
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
/// Direct reservation and lookup refusals retain their cause without a sticky failure.
fn direct_execution_admission_failures_keep_their_typed_cause_and_exact_evidence() {
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
            let engine = NativeSparqlEngine::new();
            let source = Source::at(site);
            assert_query_refusal(&engine, &source, route, query, expected);
            assert!(source.read_error().is_none());
            let receipt = *source.last_checkpoint.lock().expect("checkpoint receipt");
            assert!(!receipt.failed);
            assert_eq!(receipt.reporting_guards, 1);
            assert_eq!(receipt.execution_guards, 0);
            assert_eq!(
                receipt.execution_attempts,
                usize::from(source.storage_live_budget().is_none())
            );
            match site {
                RefusalSite::ExecutionInitial => assert_eq!(receipt.growth_attempts, 1),
                RefusalSite::ExecutionGrowth => assert_eq!(receipt.growth_attempts, 2),
                RefusalSite::Lookup => assert!(receipt.growth_attempts > 0),
                _ => {}
            }
            if site != RefusalSite::Lookup {
                assert_eq!(receipt.reads, 0, "admission refusal precedes evaluation");
            }
            drop(engine);
            source.assert_released();
        }
    }
    let engine = NativeSparqlEngine::new();
    for route in SOURCE_ROUTES {
        assert_query_refusal(
            &engine,
            &Source::at(RefusalSite::Execution),
            route,
            QUERY,
            RefusedRead::Workspace,
        );
    }
}
/// A graph-name forecast and the actual query must preserve a direct lookup
/// refusal, even when the provider has no sticky failure and no graph exists.
fn named_graph_lookup_refusal_never_certifies_an_empty_scope() {
    for route in LOCAL_ROUTES {
        let engine = NativeSparqlEngine::new();
        let source = Source::at(RefusalSite::Lookup);
        assert_query_refusal(
            &engine,
            &source,
            route,
            GRAPH_LOOKUP_QUERY,
            RefusedRead::Lookup,
        );
        assert!(source.read_error().is_none());
        assert!(source.reads.load(Ordering::Relaxed) > 0);
        drop(engine);
        source.assert_released();
    }
    for route in EXPLAIN_ROUTES {
        let engine = NativeSparqlEngine::new();
        let source = Source::at(RefusalSite::Lookup);
        let error = assert_source_refusal(
            explain(&engine, &source, route, GRAPH_LOOKUP_QUERY),
            &source,
            RefusedRead::Lookup,
        );
        assert!(source.read_error().is_none());
        assert!(error.evidence().reads > 0);
        drop(error);
        drop(engine);
        source.assert_released();
    }
}
/// A latched storage root takes precedence over the admission error that exposed it.
fn sticky_operational_root_outranks_the_direct_admission_error() {
    let engine = NativeSparqlEngine::new();
    for site in [RefusalSite::ReportingSticky, RefusalSite::ExecutionSticky] {
        for route in LOCAL_ROUTES.into_iter().chain(SOURCE_ROUTES) {
            let source = Source::at(site);
            assert_query_refusal(
                &engine,
                &source,
                route,
                QUERY,
                RefusedRead::FinalRead(source.site),
            );
            assert_eq!(
                source.read_error(),
                Some(RefusedRead::FinalRead(source.site))
            );
        }
    }
}
/// Admission failure prevents visitor entry and leaves the retained handle reusable.
fn refused_execution_never_invokes_a_scoped_visitor_and_the_handle_can_retry() {
    for site in [
        RefusalSite::Reporting,
        RefusalSite::Execution,
        RefusalSite::ExecutionInitial,
        RefusalSite::ExecutionGrowth,
        RefusalSite::ExecutionSticky,
        RefusalSite::Lookup,
    ] {
        let engine = NativeSparqlEngine::new();
        let query = if site == RefusalSite::Lookup {
            LOOKUP_QUERY
        } else {
            QUERY
        };
        let expected = match site {
            RefusalSite::ExecutionSticky => RefusedRead::FinalRead(site),
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
        drop(execution);
        drop(engine);
        source.assert_released();
        healthy.assert_released();
    }
}
/// Requires successful reads and workspace ownership through the final checkpoint.
fn assert_held_receipt(source: &Source, receipt: &Receipt) {
    assert_workspace_receipt(source, receipt);
    assert!(receipt.reads > 0);
}
/// Checks the actual last checkpoint, with one original bounded account or the
/// unbounded provider's separately held reporting and execution permits.
fn assert_workspace_receipt(source: &Source, receipt: &Receipt) {
    source.assert_receipt(receipt);
    assert!(!receipt.failed);
    assert_eq!(receipt.reporting_guards, 1);
    if source.storage_live_budget().is_some() {
        assert_eq!(
            receipt.execution_guards, 0,
            "bounded phases share the original account"
        );
        assert_eq!(receipt.execution_attempts, 0);
        assert_eq!(receipt.reservations, 1, "no second bounded account");
        assert!(receipt.owned_bytes > 0);
    } else {
        assert_eq!(
            receipt.execution_guards, 1,
            "zero-byte execution permit is operational"
        );
        assert_eq!(receipt.execution_attempts, 1);
        assert_eq!(receipt.reservations, 2);
        assert_eq!(receipt.owned_bytes, 0);
    }
}
/// Rejects vacuous success and checks the one-row result's final storage receipt.
fn assert_complete_query<V: ViewReceipt>(
    source: &Source,
    answer: purrdf_sparql_eval::CompleteSparqlResult<V>,
    engine: NativeSparqlEngine,
    prepared: Arc<PreparedQuery>,
    state: Arc<GovernorState>,
) {
    assert_eq!(answer.result.solutions().expect("SELECT shape").1.len(), 1);
    assert_held_receipt(source, answer.evidence.view_receipt());
    // The result is the remaining publication owner only after operational
    // governor handles, the prepared plan, and the engine's admitted key/cache
    // buffers have been destroyed.
    drop(state);
    drop(prepared);
    drop(engine);
    let (result, evidence) = answer.into_parts();
    let clone = result.clone();
    let solutions = result.into_solutions().expect("SELECT extraction");
    assert_eq!(solutions.as_parts().1.len(), 1);
    let retained = source.owned_bytes.load(Ordering::Relaxed);
    if source.storage_live_budget().is_some() {
        assert!(
            retained > 0,
            "result extraction keeps the original admitted allocation"
        );
        assert_eq!(source.reporting_guards.load(Ordering::Relaxed), 1);
    } else {
        source.assert_released();
    }
    drop(evidence);
    assert_eq!(source.owned_bytes.load(Ordering::Relaxed), retained);
    drop(clone);
    assert_eq!(source.owned_bytes.load(Ordering::Relaxed), retained);
    drop(solutions);
    source.assert_released();
}
/// A conservative certificate must not price each output cell at its maximum.
fn materialization_source() -> Source {
    let mut source = Source::at(RefusalSite::BoundedHealthy);
    let state = Arc::get_mut(&mut source.state).expect("new source has one owner");
    state.term_bound = 128_256;
    state.live_limit = 8 * 1024 * 1024;
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p").unwrap();
    for index in 0..512 {
        let subject = builder
            .intern_iri(&format!("http://example.org/s/{index}"))
            .unwrap();
        let mut object = builder
            .intern_iri(&format!("http://example.org/o/{index}"))
            .unwrap();
        if index == 1 {
            object = builder
                .intern_literal(RdfLiteral::language_tagged("x".repeat(4096), "en"))
                .unwrap();
        } else if index == 2 {
            object = builder.intern_triple(subject, predicate, object).unwrap();
            object = builder.intern_triple(subject, predicate, object).unwrap();
        } else if matches!(index, 3 | 4) {
            object = builder
                .intern_literal(RdfLiteral::simple("repeated short literal"))
                .unwrap();
        }
        builder.push_quad(subject, predicate, object, None).unwrap();
    }
    source.resident = builder.freeze().unwrap();
    source
}

/// Materialization preserves complete RDF values and the original retained owner.
fn materialized_existing_cells_use_actual_payload_admission() {
    for query in [
        "SELECT ?s ?p ?o WHERE { ?s ?p ?o }",
        "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o",
        "SELECT ?s ?p ?o (STR(?s) AS ?computed) WHERE { ?s ?p ?o }",
    ] {
        let source = materialization_source();
        let SparqlResult::Solutions {
            variables: expected_variables,
            rows: expected_rows,
            ..
        } = NativeSparqlEngine::new()
            .query(&source.resident, request(query))
            .expect("resident oracle")
        else {
            panic!("the oracle query is SELECT");
        };
        let engine = NativeSparqlEngine::new();
        let answer = engine
            .query_governed_fallible_view(
                &source,
                request(query),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("short cells fit actual admission despite a conservative view certificate");
        let (variables, rows) = answer.result.solutions().expect("complete SELECT");
        assert_eq!(variables, expected_variables);
        assert_eq!(rows, expected_rows);
        assert_eq!(rows.len(), 512);
        assert!(source.peak_owned_bytes.load(Ordering::Relaxed) <= source.live_limit);
        let state = source.state.clone();
        let resident = Arc::downgrade(&source.resident);
        drop(engine);
        drop(source);
        assert!(
            resident.upgrade().is_none(),
            "the source dataset is destroyed"
        );
        let clone = answer.result.clone();
        drop(answer);
        assert_eq!(clone.solutions().unwrap().1, expected_rows);
        assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
        drop(clone);
        state.assert_released();
    }
}

/// Certified partial rows retain their payload after the source context ends.
fn partial_materialized_cells_keep_actual_payload_admission() {
    let source = materialization_source();
    let engine = NativeSparqlEngine::new();
    let error = engine
        .query_governed_fallible_view(
            &source,
            request("SELECT ?s ?p ?o WHERE { ?s ?p ?o }"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_answers(7),
        )
        .expect_err("the answer governor stops a positive query");
    let FallibleSparqlError::BudgetExhausted {
        partial: PartialAnswers::Certain(partial),
        ..
    } = error
    else {
        panic!("the result must retain a certified lower bound");
    };
    let state = source.state.clone();
    let resident = Arc::downgrade(&source.resident);
    drop(source);
    drop(engine);
    assert!(
        resident.upgrade().is_none(),
        "the source dataset is destroyed"
    );
    assert_eq!(partial.result().solutions().unwrap().1.len(), 7);
    assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
    drop(partial);
    state.assert_released();
}

/// Successful query routes retain workspace through publication for bounded and unbounded views.
fn healthy_query_routes_hold_execution_admission_through_final_checkpoint() {
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        for route in LOCAL_ROUTES {
            let engine = NativeSparqlEngine::new();
            let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
            let source = Source::at(site);
            let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
            match route {
                QueryRoute::Owned => assert_complete_query(
                    &source,
                    engine
                        .query_fallible_view(&source, request(QUERY), QueryOptions::EMPTY)
                        .expect("healthy owned query"),
                    engine,
                    prepared,
                    state,
                ),
                QueryRoute::Prepared => assert_complete_query(
                    &source,
                    engine
                        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                        .expect("healthy prepared query"),
                    engine,
                    prepared,
                    state,
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
                    engine,
                    prepared,
                    state,
                ),
                QueryRoute::PreparedGoverned => assert_complete_query(
                    &source,
                    engine
                        .query_prepared_governed_fallible_view(
                            &source,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &QueryGovernors::METERED,
                        )
                        .expect("healthy per-call prepared query"),
                    engine,
                    prepared,
                    state,
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
                    assert_complete_query(&source, answer, engine, prepared, state);
                }
                QueryRoute::GovernedSource
                | QueryRoute::PreparedGovernedSource
                | QueryRoute::SharedSource => {
                    unreachable!("local route set")
                }
            }
            let last = *source.last_checkpoint.lock().expect("checkpoint receipt");
            assert_eq!(source.checkpoints.load(Ordering::Relaxed), last.checkpoints);
            assert!(
                last.checkpoints >= 2,
                "ingress and final publication are checked"
            );
        }
    }
}
/// A final checkpoint can invalidate evaluated rows and visitor output before publication.
fn final_checkpoint_failure_discards_success_and_keeps_workspace_held() {
    let engine = NativeSparqlEngine::new();
    for route in LOCAL_ROUTES {
        let source = Source::at(RefusalSite::Checkpoint);
        assert_query_refusal(
            &engine,
            &source,
            route,
            QUERY,
            RefusedRead::FinalRead(source.site),
        );
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
        RefusedRead::FinalRead(source.site),
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
/// Seeds a destination with a sentinel quad to detect any partial graph publication.
fn destination() -> RdfDatasetBuilder {
    let mut destination = RdfDatasetBuilder::new();
    let s = destination
        .intern_iri("https://example.org/retained")
        .unwrap();
    let p = destination.intern_iri("https://example.org/edge").unwrap();
    let o = destination.intern_iri("https://example.org/value").unwrap();
    destination.push_quad(s, p, o, None).unwrap();
    destination
}
/// Graph failures preserve the destination; successful publication appends only after checking.
fn graph_admission_and_final_checkpoint_failures_leave_destination_unchanged() {
    let now = coordinate_time();
    let options = QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0);
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
                options,
                &state,
                &mut destination,
            ),
            &source,
            if matches!(site, RefusalSite::Reporting | RefusalSite::Execution) {
                RefusedRead::Workspace
            } else {
                RefusedRead::FinalRead(source.site)
            },
        );
        if matches!(
            site,
            RefusalSite::Reporting | RefusalSite::Execution | RefusalSite::ExecutionSticky
        ) {
            assert_eq!(
                error.evidence().coordinates,
                purrdf_sparql_eval::QueryCoordinates::NotStarted
            );
        } else {
            assert_supplied_coordinates(&error.evidence().coordinates);
        }
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
            options,
            &state,
            &mut destination,
        )
        .expect("healthy graph publication");
    assert_supplied_coordinates(&evidence.coordinates);
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
    GovernedRequest,
}
const EXPLAIN_ROUTES: [ExplainRoute; 4] = [
    ExplainRoute::Default,
    ExplainRoute::Options,
    ExplainRoute::QuietStop,
    ExplainRoute::GovernedRequest,
];
#[allow(
    clippy::result_large_err,
    reason = "test helper preserves the public operational result shared by all three EXPLAIN entries"
)]
/// Dispatches each checked EXPLAIN entry with equivalent options and an inert stop.
fn explain(
    engine: &NativeSparqlEngine,
    source: &Source,
    route: ExplainRoute,
    query: &str,
) -> FallibleScopedResult<RetainedQueryExplanation, RefusedRead, Receipt> {
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
        ExplainRoute::GovernedRequest => engine
            .explain_governed_fallible_view(
                source,
                request(query),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .map(|(explanation, evidence)| (explanation, evidence.view))
            .map_err(|error| error.map_evidence(|evidence| evidence.view)),
    }
}

/// Request bindings and every numeric ceiling reach the original measuring run.
fn governed_explain_matches_bound_query_receipts_and_original_ownership() {
    let bindings = [
        (
            "number".to_owned(),
            TermValue::Literal {
                lexical_form: "41".to_owned(),
                datatype: purrdf_xsd::datatype::XSD_INTEGER.to_owned(),
                language: None,
                direction: None,
            },
        ),
        ("s".to_owned(), TermValue::iri(format!("{}s", support::EX))),
    ];
    for query in [
        "SELECT ?s (?number + 1 AS ?next) WHERE { ?s <p> ?o }",
        "ASK { ?s <p> ?o FILTER(?number = 41) }",
        "CONSTRUCT { ?s <p> ?number } WHERE { ?s <p> ?o }",
        "DESCRIBE ?s WHERE { ?s <p> ?o FILTER(?number = 41) }",
        "SELECT ?s ?o WHERE { ?s <p>+ ?o }",
        "SELECT (?number + ?x AS ?next) WHERE { VALUES ?x { 1 2 3 } } ORDER BY ?next",
    ] {
        for governors in [
            QueryGovernors::UNBOUNDED,
            QueryGovernors::METERED,
            QueryGovernors::METERED.with_fuel(0),
            QueryGovernors::METERED.with_fuel(1),
            QueryGovernors::METERED.with_fuel(2),
            QueryGovernors::METERED.with_fuel(3),
            QueryGovernors::METERED.with_fuel(7),
            QueryGovernors::METERED.with_fuel(16),
            QueryGovernors::METERED.with_fuel(64),
            QueryGovernors::METERED.with_max_answers(0),
            QueryGovernors::METERED.with_max_answers(1),
            QueryGovernors::METERED.with_max_intermediate_cells(0),
            QueryGovernors::METERED.with_max_intermediate_cells(1),
            QueryGovernors::METERED.with_max_scratch_bytes(0),
            QueryGovernors::METERED.with_max_scratch_bytes(1),
            QueryGovernors::METERED.with_max_remote_requests(0),
            QueryGovernors::METERED.with_max_remote_requests(1),
        ] {
            let mut source = Source::at(RefusalSite::BoundedHealthy);
            Arc::get_mut(&mut source.state).unwrap().live_limit = 8 * 1024 * 1024;
            let engine = NativeSparqlEngine::new();
            let request = SparqlRequest {
                query,
                base_iri: Some(support::EX),
                substitutions: &bindings,
            };
            let direct = engine.query_governed_fallible_view(
                &source,
                request,
                QueryOptions::EMPTY,
                &governors,
            );
            let expected = match &direct {
                Ok(answer) => answer.evidence.governors.clone(),
                Err(error) => {
                    assert!(matches!(error, FallibleSparqlError::BudgetExhausted { .. }));
                    error.evidence().governors.clone()
                }
            };
            let prepared = engine.prepare_query(query, request.base_iri).unwrap();
            let prepared_answer = engine.query_prepared_governed_fallible_view(
                &source,
                &prepared,
                &bindings,
                QueryOptions::EMPTY,
                &governors,
            );
            let prepared_evidence = match &prepared_answer {
                Ok(answer) => &answer.evidence.governors,
                Err(error) => &error.evidence().governors,
            };
            assert_eq!(prepared_evidence, &expected, "prepared request: {query}");
            let (explanation, receipt) = engine
                .explain_governed_fallible_view(&source, request, QueryOptions::EMPTY, &governors)
                .expect("a healthy measurement retains complete or stopped evidence");
            assert_eq!(receipt.governors, expected, "{query}");
            assert_eq!(explanation.evidence(), &expected);
            if governors.is_engaged_in(purrdf_core::ResourceDimension::Fuel) {
                assert_eq!(
                    explanation
                        .ledger()
                        .iter()
                        .map(purrdf_sparql_eval::NodeCharges::fuel_total)
                        .sum::<u64>(),
                    expected.consumed.get(purrdf_core::ResourceDimension::Fuel),
                    "the ledger numbers the actual substituted measuring tree: {query}"
                );
            } else {
                assert!(
                    explanation.ledger().iter().any(|node| node.rows > 0),
                    "disengaging fuel does not erase measured output rows: {query}"
                );
            }
            let state = source.state.clone();
            let resident = Arc::downgrade(&source.resident);
            drop(direct);
            drop(prepared_answer);
            drop(prepared);
            drop(engine);
            drop(source);
            assert!(resident.upgrade().is_none());
            assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
            let clone = explanation.clone();
            drop(explanation);
            assert_eq!(clone.evidence(), &expected);
            drop(clone);
            drop(expected);
            drop(receipt);
            state.assert_released();
        }
    }
}

/// EXPLAIN enforces an absent provider and zero remote allowance on the native path.
fn governed_explain_preserves_remote_refusal_and_zero_request_limit() {
    for (configured, governors) in [
        (false, QueryGovernors::METERED),
        (false, QueryGovernors::METERED.with_max_remote_requests(0)),
        (true, QueryGovernors::METERED),
        (true, QueryGovernors::METERED.with_max_remote_requests(0)),
    ] {
        let source = Source::at(RefusalSite::BoundedHealthy);
        let engine = NativeSparqlEngine::new();
        let provider = RefusingService(AtomicUsize::new(0));
        let options =
            QueryOptions::EMPTY.with_remote(if configured { Some(&provider) } else { None });
        let request =
            request("SELECT ?s WHERE { SERVICE <https://example.org/remote> { ?s ?p ?o } }");
        let direct = engine
            .query_governed_fallible_view(&source, request, options, &governors)
            .expect_err("a missing provider cannot complete a remote query");
        let measured = engine.explain_governed_fallible_view(&source, request, options, &governors);
        match &direct {
            FallibleSparqlError::BudgetExhausted { .. } => {
                let (explanation, receipt) = measured.expect("a governor stop has a ledger");
                assert_eq!(&receipt.governors, &direct.evidence().governors);
                assert_eq!(explanation.evidence(), &receipt.governors);
            }
            FallibleSparqlError::Query { .. } => {
                let error = measured.expect_err("the original provider refusal remains fatal");
                assert_eq!(
                    error.diagnostic().unwrap().to_string(),
                    direct.diagnostic().unwrap().to_string()
                );
                assert_eq!(error.evidence().governors, direct.evidence().governors);
                assert!(error.partial_answers().is_none());
            }
            other => panic!("unexpected native remote outcome: {other:?}"),
        }
        let admitted_calls = if configured
            && governors
                .limits()
                .get(purrdf_core::ResourceDimension::RemoteRequests)
                > 0
        {
            2
        } else {
            0
        };
        assert_eq!(provider.0.load(Ordering::Relaxed), admitted_calls);
    }
}
/// Every EXPLAIN entry preserves admission causes and publishes no measuring ledger.
fn explain_admission_failures_remain_typed_without_an_explanation() {
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
            RefusalSite::ReportingSticky | RefusalSite::ExecutionSticky => {
                RefusedRead::FinalRead(site)
            }
            _ => RefusedRead::Workspace,
        };
        let query = if site == RefusalSite::Lookup {
            LOOKUP_QUERY
        } else {
            QUERY
        };
        for route in EXPLAIN_ROUTES {
            let engine = NativeSparqlEngine::new();
            let source = Source::at(site);
            let error =
                assert_source_refusal(explain(&engine, &source, route, query), &source, expected);
            let receipt = error.evidence();
            assert_eq!(
                receipt.failed,
                matches!(expected, RefusedRead::FinalRead(_))
            );
            assert_eq!(receipt.execution_guards, 0);
            let attempts = usize::from(matches!(
                site,
                RefusalSite::Execution | RefusalSite::ExecutionSticky
            ));
            assert_eq!(receipt.execution_attempts, attempts);
            match site {
                RefusalSite::ExecutionInitial => assert_eq!(receipt.growth_attempts, 1),
                RefusalSite::ExecutionGrowth => assert_eq!(receipt.growth_attempts, 2),
                RefusalSite::Lookup => assert!(receipt.growth_attempts > 0),
                RefusalSite::Reporting | RefusalSite::ReportingSticky => {
                    assert_eq!(receipt.growth_attempts, 0);
                }
                _ => {}
            }
            if site != RefusalSite::Lookup {
                assert_eq!(receipt.reads, 0);
            }
            drop(error);
            drop(engine);
            source.assert_released();
        }
    }
}
/// A preexisting storage failure wins before EXPLAIN can parse malformed text.
fn explain_preflight_failure_outranks_malformed_query() {
    let engine = NativeSparqlEngine::new();
    for route in EXPLAIN_ROUTES {
        let source = Source::at(RefusalSite::Never);
        source.refuse();
        let error = assert_source_refusal(
            explain(&engine, &source, route, "not SPARQL"),
            &source,
            RefusedRead::FinalRead(source.site),
        );
        assert_eq!(error.evidence().reservations, 0);
        assert_eq!(error.evidence().reads, 0);
    }
}
/// Measuring reads and the final checkpoint can each invalidate the entire explanation.
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
                RefusedRead::FinalRead(source.site),
            );
            assert_eq!(error.evidence().reporting_guards, 1);
            assert_eq!(error.evidence().execution_guards, 1);
            assert!(error.evidence().reads > 0);
        }
    }
}
/// Checked EXPLAIN preserves resident bytes and governor evidence while holding workspace.
fn healthy_explain_is_identical_to_resident_and_holds_admission_through_publication() {
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        for route in EXPLAIN_ROUTES {
            let engine = NativeSparqlEngine::new();
            let source = Source::at(site);
            let expected = engine
                .explain_query(&source.resident, QUERY, None)
                .expect("resident explanation");
            let (actual, receipt) =
                explain(&engine, &source, route, QUERY).expect("healthy operational explanation");
            support::assert_scan_explanation_matches_resident(&actual, &expected.render());
            assert_eq!(actual.evidence(), expected.evidence());
            assert_eq!(actual.evidence().tripped, None);
            assert_held_receipt(&source, &receipt);
            drop(expected);
            drop(engine);
            if site == RefusalSite::BoundedHealthy {
                assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
                let clone = actual.clone();
                drop(actual);
                assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
                drop(clone);
            } else {
                drop(actual);
            }
            source.assert_released();
        }
    }
    let engine = NativeSparqlEngine::new();
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
    support::assert_scan_explanation_matches_resident(&actual, &expected.render());
    assert_held_receipt(&source, &receipt);
    drop(expected);
    drop(engine);
    drop(actual);
    source.assert_released();
}
/// Query diagnostics retain exact final evidence unless a later storage root supersedes them.
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
        RefusedRead::FinalRead(source.site),
    );
    assert_eq!(error.evidence().execution_guards, 1);
}
/// Admitted configuration reaches the native parser before any SERVICE dispatch.
fn explain_admitted_supplied_inputs_preserve_parse_failure_before_reading() {
    let engine = NativeSparqlEngine::new();
    let remote = RefusingService(AtomicUsize::new(0));
    for options in [
        QueryOptions::EMPTY.with_bnode_mint_prefix(Some("configured-mint")),
        QueryOptions::EMPTY.with_remote(Some(&remote)),
    ] {
        let source = Source::at(RefusalSite::BoundedHealthy);
        let error = engine
            .explain_query_with_options_fallible_view(&source, "not SPARQL", None, options)
            .expect_err("native parser refuses malformed query text");
        assert_eq!(
            error
                .diagnostic()
                .expect("retained native parse diagnostic")
                .code,
            "native-sparql-query-parse"
        );
        assert!(error.operational_error().is_none());
        source.assert_receipt(error.evidence());
        assert!(error.evidence().reservations >= 1);
        assert_eq!(error.evidence().reporting_guards, 1);
        assert_eq!(error.evidence().execution_attempts, 0);
        assert_eq!(error.evidence().reads, 0);
        assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
        let clone = error.clone();
        drop(error);
        assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
        drop(clone);
        source.assert_released();
    }
    assert_eq!(remote.0.load(Ordering::Relaxed), 0);
}
#[derive(Debug)]
struct FailureAndStop(Arc<Source>);
impl StopSignal for FailureAndStop {
    /// Presents cancellation and a newly latched storage root in the same observation.
    fn poll(&self) -> Option<StopCause> {
        self.0.refuse();
        Some(StopCause::Cancelled)
    }
}
/// Cancellation preserves a healthy stopped explanation, but concurrent storage failure wins.
fn explain_stop_retains_stopped_receipt_while_operational_failure_takes_precedence() {
    for site in [RefusalSite::Never, RefusalSite::BoundedHealthy] {
        let engine = NativeSparqlEngine::new();
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
        support::assert_scan_explanation_matches_resident(&actual, &expected.render());
        assert_workspace_receipt(&source, &receipt);
        drop(expected);
        drop(engine);
        let clone = actual.clone();
        if site == RefusalSite::BoundedHealthy {
            assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
        }
        drop(actual);
        if site == RefusalSite::BoundedHealthy {
            assert!(source.owned_bytes.load(Ordering::Relaxed) > 0);
        }
        drop(clone);
        source.assert_released();
    }
    let engine = NativeSparqlEngine::new();
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
        RefusedRead::FinalRead(source.site),
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
        RefusedRead::FinalRead(source.site),
    );
    assert_eq!(error.evidence().execution_guards, 1);
}
const COORDINATE_TIME: &str = "-12345-06-07T08:09:10.123456789123456789123456789+05:30";

fn coordinate_time() -> purrdf_xsd::temporal::DateTime {
    let purrdf_xsd::XsdValue::DateTime(now) =
        purrdf_xsd::parse(COORDINATE_TIME, purrdf_xsd::XsdDatatype::DateTime).unwrap()
    else {
        panic!("timestamp")
    };
    now
}

fn assert_supplied_coordinates(coordinates: &purrdf_sparql_eval::QueryCoordinates) {
    let purrdf_sparql_eval::QueryCoordinates::Started(inputs) = coordinates else {
        panic!("constructed context")
    };
    assert!(
        inputs
            .now()
            .unwrap()
            .same_representation(&coordinate_time())
    );
    assert_eq!(inputs.entropy_seed(), Some(0));
}

/// Select governor ownership while preserving the same plan, source, and publication boundary.
#[allow(
    clippy::result_large_err,
    reason = "the test drives both public entries through their unchanged typed reporting boundary"
)]
fn prepared_governed(
    engine: &NativeSparqlEngine,
    source: &Source,
    prepared: &PreparedQuery,
    governors: &QueryGovernors,
    shared: bool,
) -> FallibleSparqlResult<RefusedRead, GovernedEvidence<Receipt>> {
    let now = coordinate_time();
    let options = QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0);
    if shared {
        let state = Arc::new(GovernorState::new(governors));
        engine.query_prepared_governed_fallible_in_operation(source, prepared, &[], options, &state)
    } else {
        engine.query_prepared_governed_fallible_view(source, prepared, &[], options, governors)
    }
}

/// A row observed before a lazy stream refusal must never escape as a certified partial answer.
fn prepared_governed_queries_discard_rows_produced_before_an_iterator_failure() {
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
    for shared in [false, true] {
        let source = Source::at(RefusalSite::AfterRow);
        let refusal = assert_source_refusal(
            prepared_governed(
                &engine,
                &source,
                &prepared,
                &QueryGovernors::METERED,
                shared,
            ),
            &source,
            RefusedRead::FinalRead(RefusalSite::AfterRow),
        );
        assert_supplied_coordinates(&refusal.evidence().coordinates);
        assert_eq!(source.yielded.load(Ordering::Relaxed), 1);
        let checkpoint = source.checkpoints.load(Ordering::Relaxed);
        assert!(checkpoint >= 2);
        assert!(refusal.evidence().view.failed);
        assert_eq!(refusal.evidence().view.yielded, 1);
        assert_eq!(refusal.evidence().view.checkpoints, checkpoint);
    }
}

#[derive(Clone, Copy)]
enum ReadyOutcome {
    Complete,
    Query,
    Budget,
}

/// Final-checkpoint refusal replaces completion, diagnostics, and exhaustion without losing meters.
fn the_final_prepared_checkpoint_outranks_every_ready_evaluator_outcome() {
    let engine = NativeSparqlEngine::new();
    for (query, governors, expected) in [
        (QUERY, QueryGovernors::METERED, ReadyOutcome::Complete),
        (
            "SELECT * WHERE { SERVICE <http://example.org/service> { ?s ?p ?o } }",
            QueryGovernors::METERED,
            ReadyOutcome::Query,
        ),
        (
            "SELECT ?s WHERE { VALUES ?s { <http://example.org/a> <http://example.org/b> } }",
            QueryGovernors::METERED.with_max_answers(1),
            ReadyOutcome::Budget,
        ),
    ] {
        let prepared = engine.prepare_query(query, None).expect("prepare fixture");
        for shared in [false, true] {
            let healthy = Source::at(RefusalSite::Never);
            let healthy_outcome =
                prepared_governed(&engine, &healthy, &prepared, &governors, shared);
            match &healthy_outcome {
                Ok(complete) => assert_supplied_coordinates(&complete.evidence.coordinates),
                Err(error) => assert_supplied_coordinates(&error.evidence().coordinates),
            }
            let healthy_governors = match (expected, healthy_outcome) {
                (ReadyOutcome::Complete, Ok(complete)) => {
                    assert_eq!(
                        complete.result.solutions().expect("SELECT shape").1.len(),
                        1
                    );
                    assert!(!complete.evidence.view.failed);
                    complete.evidence.governors.clone()
                }
                (
                    ReadyOutcome::Query,
                    Err(FallibleSparqlError::Query {
                        diagnostic,
                        evidence,
                    }),
                ) => {
                    assert_eq!(diagnostic.code, "native-sparql-service-unconfigured");
                    assert!(!evidence.view.failed);
                    evidence.governors
                }
                (
                    ReadyOutcome::Budget,
                    Err(FallibleSparqlError::BudgetExhausted {
                        tripped,
                        partial,
                        evidence,
                    }),
                ) => {
                    assert_eq!(evidence.governors.tripped, Some(tripped));
                    assert_eq!(
                        partial
                            .result()
                            .expect("certified prefix")
                            .result()
                            .solutions()
                            .expect("SELECT shape")
                            .1
                            .len(),
                        1
                    );
                    assert!(!evidence.view.failed);
                    evidence.governors
                }
                (_, other) => {
                    panic!("healthy source produced the wrong evaluator outcome: {other:?}")
                }
            };
            let final_checkpoint = healthy.checkpoints.load(Ordering::Relaxed);
            assert!(final_checkpoint >= 2);
            healthy.assert_released();

            let source = Source::at(RefusalSite::FinalCheckpoint);
            source
                .final_checkpoint
                .store(final_checkpoint, Ordering::Relaxed);
            let refusal = assert_source_refusal(
                prepared_governed(&engine, &source, &prepared, &governors, shared),
                &source,
                RefusedRead::FinalRead(RefusalSite::FinalCheckpoint),
            );
            assert_supplied_coordinates(&refusal.evidence().coordinates);
            assert_eq!(source.checkpoints.load(Ordering::Relaxed), final_checkpoint);
            assert!(refusal.evidence().view.failed);
            assert_eq!(refusal.evidence().view.checkpoints, final_checkpoint);
            assert_eq!(refusal.evidence().governors, healthy_governors);
        }
    }
}

/// EXPLAIN captures its actual context even on forecast refusal, and keeps it through read failures.
fn governed_explain_coordinates_survive_forecast_and_publication_refusals() {
    let engine = NativeSparqlEngine::new();
    let now = coordinate_time();
    let options = QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0);
    for site in [
        RefusalSite::Never,
        RefusalSite::Iterator,
        RefusalSite::Checkpoint,
        RefusalSite::Reporting,
    ] {
        let source = Source::at(site);
        let result = engine.explain_governed_fallible_view(
            &source,
            request(QUERY),
            options,
            &QueryGovernors::METERED,
        );
        if site == RefusalSite::Never {
            let (_, evidence) = result.expect("healthy measurement");
            assert_supplied_coordinates(&evidence.coordinates);
        } else {
            let expected = if site == RefusalSite::Reporting {
                RefusedRead::Workspace
            } else {
                RefusedRead::FinalRead(site)
            };
            let error = assert_source_refusal(result, &source, expected);
            if site == RefusalSite::Reporting {
                assert_eq!(
                    error.evidence().coordinates,
                    purrdf_sparql_eval::QueryCoordinates::NotStarted
                );
            } else {
                assert_supplied_coordinates(&error.evidence().coordinates);
            }
        }
    }
    let source = Source::at(RefusalSite::Never);
    let (_, evidence) = engine
        .explain_governed_fallible_view(
            &source,
            request(QUERY),
            options,
            &QueryGovernors::METERED.with_max_intermediate_cells(0),
        )
        .unwrap();
    assert_supplied_coordinates(&evidence.coordinates);
    assert!(evidence.governors.tripped.is_some());
}

/// Real materialized publication retains only its original coordinate owner after results drop.
fn materialized_coordinate_receipts_balance_with_retained_engine_and_plan() {
    for prepared_route in [false, true] {
        let source = Source::at(RefusalSite::BoundedHealthy);
        let state = source.state.clone();
        let engine = NativeSparqlEngine::new();
        let query = "SELECT (1 AS ?value) WHERE {}";
        let prepared = engine.prepare_query(query, None).unwrap();
        let run = |options| {
            if prepared_route {
                engine.query_prepared_governed_fallible_view(
                    &source,
                    &prepared,
                    &[],
                    options,
                    &QueryGovernors::METERED,
                )
            } else {
                engine.query_governed_fallible_view(
                    &source,
                    request(query),
                    options,
                    &QueryGovernors::METERED,
                )
            }
            .unwrap()
        };
        // Warm the existing immutable plan/cache owners, with no supplied coordinate.
        drop(run(QueryOptions::EMPTY));
        let baseline = state.owned_bytes.load(Ordering::Relaxed);
        let purrdf_xsd::XsdValue::DateTime(now) = purrdf_xsd::parse(
            &format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096)),
            purrdf_xsd::XsdDatatype::DateTime,
        )
        .unwrap() else {
            panic!("wide timestamp")
        };
        let mut retained = None;
        for iteration in 0..4 {
            let result = run(QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0));
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let coordinates = result.evidence.coordinates.clone();
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            drop(result);
            assert!(state.owned_bytes.load(Ordering::Relaxed) > baseline);
            let purrdf_sparql_eval::QueryCoordinates::Started(inputs) = &coordinates else {
                panic!("started materialized query")
            };
            assert!(inputs.now().unwrap().same_representation(&now));
            assert_eq!(inputs.entropy_seed(), Some(0));
            if iteration == 3 {
                retained = Some(coordinates);
            } else {
                drop(coordinates);
                assert_eq!(state.owned_bytes.load(Ordering::Relaxed), baseline);
            }
        }
        drop(now);
        drop(prepared);
        drop(engine);
        drop(source);
        assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
        drop(retained);
        state.assert_released();
    }
}

/// The resident public projection still owns both original scoped permits at
/// its last checked_read. Its infallible source keeps admission observable.
struct ExplainProjection<'a> {
    source: &'a Source,
    final_reporting: AtomicUsize,
    final_execution: AtomicUsize,
}
struct ProjectionReservation<R>(R);
impl<R: WorkspaceReservation<Error = RefusedRead>> WorkspaceReservation
    for ProjectionReservation<R>
{
    type Error = std::convert::Infallible;

    fn resize(&mut self, bytes: u64) -> Result<(), Self::Error> {
        self.0
            .resize(bytes)
            .expect("healthy projection reservation");
        Ok(())
    }
}
impl DatasetView for ExplainProjection<'_> {
    type Id = TermId;
    type ReadError = std::convert::Infallible;
    type TermGuard<'a>
        = <RdfDataset as DatasetView>::TermGuard<'a>
    where
        Self: 'a;
    type ProbePlan = ();

    fn statistics(&self) -> Option<&dyn purrdf_core::SnapshotStatistics<TermId>> {
        self.source.statistics()
    }

    fn read_error(&self) -> Option<Self::ReadError> {
        assert!(self.source.read_error().is_none());
        self.final_reporting.store(
            self.source.reporting_guards.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.final_execution.store(
            self.source.execution_guards.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        None
    }
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        Ok(ProjectionReservation(
            self.source
                .reserve_workspace(bytes)
                .expect("healthy projection ingress"),
        ))
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.source.quads()
    }
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.source.reads.fetch_add(1, Ordering::Relaxed);
        <RdfDataset as DatasetView>::resolve(self.source.resident.as_ref(), id)
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Self::ReadError> {
        self.source.reads.fetch_add(1, Ordering::Relaxed);
        <RdfDataset as DatasetView>::term_id_by_value(self.source.resident.as_ref(), value)
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.source.capabilities()
    }
    fn term_count(&self) -> u64 {
        self.source.term_count()
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
/// Exercise the actual public Infallible view door. Operational EXPLAIN refusal
/// stays in the separately tested typed fallible door.
fn public_explain_projection_holds_real_execution_permit_through_final_read() {
    let engine = NativeSparqlEngine::new();
    for (query, prepared) in [(QUERY, true), ("not SPARQL", false)] {
        let source = Source::at(RefusalSite::Never);
        let view = ExplainProjection {
            source: &source,
            final_reporting: AtomicUsize::new(0),
            final_execution: AtomicUsize::new(0),
        };
        let result = engine.explain_query_view(&view, query, None);
        if prepared {
            result.expect("healthy public EXPLAIN projection");
            assert_eq!(source.execution_attempts.load(Ordering::Relaxed), 1);
            assert_eq!(view.final_execution.load(Ordering::Relaxed), 1);
        } else {
            let diagnostic = result.expect_err("parse precedes execution admission");
            assert_eq!(diagnostic.code, "native-sparql-query-parse");
            assert_eq!(source.execution_attempts.load(Ordering::Relaxed), 0);
            assert_eq!(source.reads.load(Ordering::Relaxed), 0);
            assert_eq!(view.final_execution.load(Ordering::Relaxed), 0);
        }
        assert_eq!(view.final_reporting.load(Ordering::Relaxed), 1);
        source.assert_released();
    }
}

const STORED_RELATION: &str = "http://example.org/stored-records";
const STORED_QUERY: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/stored-records> ?o }";

#[derive(Debug, Default)]
struct StoredPulls {
    examined: AtomicUsize,
    unmetered: AtomicUsize,
    admission_failed: AtomicBool,
    cancellation: Option<(CancellationFlag, usize)>,
    poison_on_stop: bool,
}

/// The producer borrows the captured original view; its iterator borrows it too.
struct StoredProducer<'query> {
    snapshot: &'query Source,
    modes: [purrdf_sparql_eval::BindingPattern; 1],
    generation: Arc<str>,
    pulls: Arc<StoredPulls>,
}
impl<'query> StoredProducer<'query> {
    fn new(snapshot: &'query Source, generation: Arc<str>, pulls: Arc<StoredPulls>) -> Self {
        Self {
            snapshot,
            modes: [purrdf_sparql_eval::PfArity::new(1, 1).all_free_mode()],
            generation,
            pulls,
        }
    }
}
impl purrdf_sparql_eval::PropertyFunction for StoredProducer<'_> {
    fn volatility(&self) -> purrdf_sparql_eval::Volatility {
        purrdf_sparql_eval::Volatility::Stable
    }
    fn arity(&self) -> purrdf_sparql_eval::PfArity {
        purrdf_sparql_eval::PfArity::new(1, 1)
    }
    fn modes(&self) -> &[purrdf_sparql_eval::BindingPattern] {
        &self.modes
    }
    fn rows_per_invocation(&self, _: purrdf_sparql_eval::BindingPattern) -> u64 {
        self.snapshot.resident.len_hint().unwrap()
    }
    purrdf_sparql_eval::resident_property_open!();
    fn open_admitted<'cursor>(
        &self,
        args: &purrdf_sparql_eval::PfArgs<'_>,
        _: Option<u64>,
        workspace: purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<purrdf_sparql_eval::WorkspaceCursor<'cursor>, purrdf_sparql_eval::EvalError>
    where
        Self: 'cursor,
    {
        let mut bindings = purrdf_sparql_eval::AdmittedVec::with_capacity(2, &workspace)?;
        for value in args.flattened() {
            bindings.push(value.map(|value| workspace.clone_term(value)).transpose()?)?;
        }
        purrdf_sparql_eval::WorkspaceCursor::new(
            StoredCursor {
                snapshot: self.snapshot,
                rows: self.snapshot.quads(),
                generation: self.generation.clone(),
                pulls: self.pulls.clone(),
                bindings,
                workspace: workspace.clone(),
            },
            &workspace,
            "borrowed stored-record cursor",
        )
    }
}
struct StoredCursor<'query, I> {
    snapshot: &'query Source,
    rows: I,
    generation: Arc<str>,
    pulls: Arc<StoredPulls>,
    bindings: purrdf_sparql_eval::AdmittedVec<Option<purrdf_sparql_eval::WorkspaceTerm>>,
    workspace: purrdf_sparql_eval::WorkspaceCapability,
}
impl<I: Iterator<Item = QuadIds>> StoredCursor<'_, I> {
    fn pull(
        &mut self,
        context: Option<&purrdf_sparql_eval::NativeFnContext<'_>>,
    ) -> Result<Option<purrdf_sparql_eval::AdmittedPfRow>, purrdf_sparql_eval::EvalError> {
        loop {
            for _ in 0..32 {
                if let Some(context) = context {
                    if context
                        .checkpoint()
                        .and_then(|()| context.charge_work(1))
                        .is_err()
                    {
                        if self.pulls.poison_on_stop {
                            self.snapshot.refuse();
                        }
                        return Ok(None);
                    }
                } else {
                    self.pulls.unmetered.fetch_add(1, Ordering::Relaxed);
                }
                let work = self.pulls.examined.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some((cancel, at)) = &self.pulls.cancellation
                    && work == *at
                {
                    cancel.cancel();
                }
            }
            let Some(row) = self.rows.next() else {
                return Ok(None);
            };
            let admitted = (|| {
                let mut terms = purrdf_sparql_eval::AdmittedVec::with_capacity(2, &self.workspace)?;
                for id in [row.s, row.o] {
                    let guard = self
                        .snapshot
                        .resolve(id)
                        .map_err(|_| purrdf_sparql_eval::EvalError::WorkspaceStopped)?;
                    let purrdf_core::TermRef::Iri(iri) = guard.term() else {
                        panic!("IRI fixture")
                    };
                    terms.push(self.workspace.iri(iri)?)?;
                }
                Ok::<_, purrdf_sparql_eval::EvalError>(terms)
            })();
            if admitted.is_err() {
                self.pulls.admission_failed.store(true, Ordering::Relaxed);
            }
            let terms = admitted?;
            if self
                .bindings
                .iter()
                .zip(terms.iter())
                .any(|(bound, value)| bound.as_ref().is_some_and(|bound| **bound != **value))
            {
                continue;
            }
            return purrdf_sparql_eval::AdmittedPfRow::from_terms(
                terms.into_iter(),
                &self.workspace,
            )
            .map(Some);
        }
    }
}
impl<I: Iterator<Item = QuadIds>> purrdf_sparql_eval::PfCursor for StoredCursor<'_, I> {
    fn next(&mut self) -> Result<Option<purrdf_sparql_eval::PfRow>, purrdf_sparql_eval::EvalError> {
        self.pull(None)?
            .map(purrdf_sparql_eval::AdmittedPfRow::try_into_resident)
            .transpose()
    }
    fn next_metered(
        &mut self,
        context: &purrdf_sparql_eval::NativeFnContext<'_>,
    ) -> Result<Option<purrdf_sparql_eval::PfRow>, purrdf_sparql_eval::EvalError> {
        self.pull(Some(context))?
            .map(purrdf_sparql_eval::AdmittedPfRow::try_into_resident)
            .transpose()
    }
    fn next_admitted(
        &mut self,
    ) -> Result<Option<purrdf_sparql_eval::AdmittedPfRow>, purrdf_sparql_eval::EvalError> {
        self.pull(None)
    }
    fn next_admitted_metered(
        &mut self,
        context: &purrdf_sparql_eval::NativeFnContext<'_>,
    ) -> Result<Option<purrdf_sparql_eval::AdmittedPfRow>, purrdf_sparql_eval::EvalError> {
        self.pull(Some(context))
    }
    fn generation(&self) -> purrdf_sparql_eval::IndexGeneration {
        purrdf_sparql_eval::IndexGeneration::Declared(self.generation.clone())
    }
}

fn stored_environment<'query>(
    producer: Arc<dyn purrdf_sparql_eval::PropertyFunction + 'query>,
) -> purrdf_sparql_eval::ExtensionEnv<'query> {
    let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
    registry.register(STORED_RELATION, producer);
    purrdf_sparql_eval::ExtensionEnv::empty()
        .with_relations(registry)
        .unwrap()
}

/// Both callback and cursor borrow the retired snapshot; results retain only owned rows.
fn borrowed_producers_keep_frozen_snapshot_and_original_result_owners() {
    let engine = NativeSparqlEngine::new();
    let live = Mutex::new(Source::at(RefusalSite::BoundedHealthy));
    let mut accounts = Vec::new();
    for (operation, owned) in [false, true, false].into_iter().enumerate() {
        let captured = {
            let current = live.lock().unwrap();
            if owned {
                // The resident-only memory relation keeps its existing owned
                // contract; borrowed storage producers use the bounded view.
                let mut neighbor = Source::at(RefusalSite::Never);
                neighbor.resident = current.resident.clone();
                neighbor
            } else {
                current.clone()
            }
        };
        let expected = engine
            .query(
                &captured.resident,
                request("SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }"),
            )
            .unwrap();
        let mut replacement = Source::at(RefusalSite::BoundedHealthy);
        let subject = format!("fresh-{operation}");
        let object = format!("new-{operation}");
        replacement.resident = support::local_dataset([(subject.as_str(), "p", object.as_str())]);
        *live.lock().unwrap() = replacement;
        let state = captured.state.clone();
        let resident = Arc::downgrade(&captured.resident);
        let pulls = Arc::new(StoredPulls::default());
        let generation: Arc<str> = format!("captured-{operation}").into();
        let producer: Arc<dyn purrdf_sparql_eval::PropertyFunction + '_> = if owned {
            let rows = expected
                .solutions()
                .unwrap()
                .1
                .iter()
                .map(|row| row.iter().map(|cell| cell.clone().unwrap()).collect())
                .collect();
            let owned: Arc<dyn purrdf_sparql_eval::PropertyFunction> =
                Arc::new(purrdf_sparql_eval::MemoryRelation::new(1, 1, rows).unwrap());
            owned
        } else {
            Arc::new(StoredProducer::new(
                &captured,
                generation.clone(),
                pulls.clone(),
            ))
        };
        let environment = stored_environment(producer);
        let answer = engine
            .query_governed_fallible_view(
                &captured,
                request(STORED_QUERY),
                QueryOptions::EMPTY.with_env(&environment),
                &QueryGovernors::METERED,
            )
            .unwrap();
        assert_eq!(answer.result.solutions(), expected.solutions());
        assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
        assert_eq!(
            pulls.examined.load(Ordering::Relaxed),
            if owned { 0 } else { 64 }
        );
        let purrdf_sparql_eval::GovernedOutcome::Complete {
            result, relations, ..
        } = engine
            .query_governed(
                &captured.resident,
                request(STORED_QUERY),
                QueryOptions::EMPTY.with_env(&environment),
                &QueryGovernors::METERED,
            )
            .unwrap()
        else {
            panic!("healthy witnessed native query")
        };
        assert_eq!(result.solutions(), expected.solutions());
        assert!(
            owned
                || relations
                    .witness
                    .get(STORED_RELATION)
                    .unwrap()
                    .generations
                    .contains(&purrdf_sparql_eval::IndexGeneration::Declared(generation))
        );
        assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
        drop(result);
        drop(relations);
        drop(environment);
        drop(captured);
        assert!(resident.upgrade().is_none());
        let clone = answer.result.clone();
        drop(answer);
        assert_eq!(clone.solutions(), expected.solutions());
        if !owned {
            assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
        }
        drop(clone);
        accounts.push(state);
    }
    // Cached plans retain their original admission grants, never the snapshot.
    drop(engine);
    for account in accounts {
        account.assert_released();
    }
}

/// Captured source references outlive the temporary producer and registry handles.
fn borrowed_cursor_keeps_its_snapshot_after_registry_handles_are_released() {
    let source = Source::at(RefusalSite::BoundedHealthy);
    let engine = NativeSparqlEngine::new();
    let pulls = Arc::new(StoredPulls::default());
    let mut cursor = {
        let environment = stored_environment(Arc::new(StoredProducer::new(
            &source,
            Arc::from("independent-snapshot"),
            pulls.clone(),
        )));
        let options = QueryOptions::EMPTY.with_env(&environment);
        let prepared = engine
            .prepare_query_with_options(STORED_QUERY, None, options)
            .unwrap();
        engine.open_call_cursor(&prepared, options).unwrap()
    };
    let row = cursor.next_row(&*source.resident).unwrap().unwrap();
    assert_eq!(
        row,
        vec![
            Some(TermValue::iri("http://example.org/s")),
            Some(TermValue::iri("http://example.org/o"))
        ]
    );
    assert!(cursor.next_row(&*source.resident).unwrap().is_none());
    assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
    assert_eq!(pulls.examined.load(Ordering::Relaxed), 64);
    drop(cursor);
    drop(engine);
    source.assert_released();
}

/// Work performed inside one pull observes the original operation's stop state.
fn borrowed_producers_stop_inside_pulls_and_preserve_storage_failure_precedence() {
    for cancelled in [false, true] {
        for poison_on_stop in [false, true] {
            let engine = NativeSparqlEngine::new();
            let source = Source::at(RefusalSite::BoundedHealthy);
            let cancellation = CancellationFlag::new();
            let pulls = Arc::new(StoredPulls {
                cancellation: cancelled.then(|| (cancellation.clone(), 2)),
                poison_on_stop,
                ..StoredPulls::default()
            });
            let environment = stored_environment(Arc::new(StoredProducer::new(
                &source,
                Arc::from("stopped-snapshot"),
                pulls.clone(),
            )));
            let governors = if cancelled {
                QueryGovernors::METERED.with_stop_signal(Arc::new(cancellation))
            } else {
                QueryGovernors::METERED.with_fuel(16)
            };
            let answer = engine.query_governed_fallible_view(
                &source,
                request(STORED_QUERY),
                QueryOptions::EMPTY.with_env(&environment),
                &governors,
            );
            let examined = pulls.examined.load(Ordering::Relaxed);
            assert!(
                examined > 0 && examined < 32,
                "work before stop: {examined}"
            );
            assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
            if poison_on_stop {
                let error =
                    assert_source_refusal(answer, &source, RefusedRead::FinalRead(source.site));
                assert!(error.evidence().governors.tripped.is_some());
            } else {
                let error = answer.expect_err("a stopped pull cannot publish completion");
                let FallibleSparqlError::BudgetExhausted {
                    tripped, evidence, ..
                } = &error
                else {
                    panic!("typed governor stop: {error:?}");
                };
                assert_eq!(evidence.governors.tripped, Some(*tripped));
                if cancelled {
                    assert_eq!(
                        *tripped,
                        TrippedGovernor::Stopped {
                            cause: StopCause::Cancelled
                        }
                    );
                    assert_eq!(examined, 2);
                }
                assert!(!evidence.view.failed);
                source.assert_receipt(&evidence.view);
            }
            drop(environment);
            drop(engine);
            source.assert_released();
        }
    }
}

/// Lazy producer faults remain failures through modifiers and left/union branches.
fn borrowed_producers_discard_candidates_when_the_original_source_fails() {
    for site in [
        RefusalSite::Iterator,
        RefusalSite::Term,
        RefusalSite::AfterRow,
    ] {
        for query in [
            STORED_QUERY,
            "SELECT ?s ?o WHERE { ?s <http://example.org/stored-records> ?o } LIMIT 2",
            "SELECT ?s ?o WHERE { VALUES ?seed { 1 } OPTIONAL { ?s <http://example.org/stored-records> ?o } }",
            "SELECT ?s ?o WHERE { { ?s <http://example.org/stored-records> ?o } UNION { VALUES (?s ?o) { (<http://example.org/a> <http://example.org/b>) } } }",
        ] {
            let engine = NativeSparqlEngine::new();
            let source = Source::at(site);
            let pulls = Arc::new(StoredPulls::default());
            let environment = stored_environment(Arc::new(StoredProducer::new(
                &source,
                Arc::from("failing-snapshot"),
                pulls.clone(),
            )));
            assert_source_refusal(
                engine.query_governed_fallible_view(
                    &source,
                    request(query),
                    QueryOptions::EMPTY.with_env(&environment),
                    &QueryGovernors::METERED,
                ),
                &source,
                RefusedRead::FinalRead(site),
            );
            assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
            drop(environment);
            drop(engine);
            source.assert_released();
        }
    }
}

/// A physical refusal during cell birth keeps the provider's original typed cause.
fn borrowed_producers_refuse_unaffordable_rows_and_release_original_admission() {
    fn run(limit: u64) -> (u64, bool) {
        let engine = NativeSparqlEngine::new();
        let mut source = Source::at(RefusalSite::BoundedHealthy);
        let subject = "s".repeat(16_384);
        source.resident = support::local_dataset([(subject.as_str(), "p", "o")]);
        let state = Arc::get_mut(&mut source.state).unwrap();
        state.live_limit = limit;
        state.term_bound = 32_768;
        let pulls = Arc::new(StoredPulls::default());
        let environment = stored_environment(Arc::new(StoredProducer::new(
            &source,
            Arc::from("bounded-snapshot"),
            pulls.clone(),
        )));
        let answer = engine.query_governed_fallible_view(
            &source,
            request(STORED_QUERY),
            QueryOptions::EMPTY.with_env(&environment),
            &QueryGovernors::METERED,
        );
        let refused_inside_cursor = pulls.admission_failed.load(Ordering::Relaxed);
        if refused_inside_cursor {
            assert_source_refusal(answer, &source, RefusedRead::Workspace);
        } else {
            if limit == u64::MAX {
                assert!(answer.is_ok(), "healthy bounded control: {answer:?}");
            }
            drop(answer);
        }
        let peak = source.peak_owned_bytes.load(Ordering::Relaxed);
        assert!(peak <= limit);
        drop(environment);
        drop(engine);
        source.assert_released();
        (peak, refused_inside_cursor)
    }
    let (healthy_peak, refused) = run(u64::MAX);
    assert!(!refused);
    let refused_inside_cursor = (256..healthy_peak).step_by(128).any(|limit| run(limit).1);
    assert!(
        refused_inside_cursor,
        "sweep must reach an admitted cursor's cell allocation"
    );
}

/// A certified prefix owns rows after its borrowed provider and snapshot are gone.
fn borrowed_producers_keep_partial_rows_after_the_query_scope_ends() {
    let engine = NativeSparqlEngine::new();
    let mut source = Source::at(RefusalSite::BoundedHealthy);
    source.resident = support::local_dataset([("s1", "p", "o1"), ("s2", "p", "o2")]);
    let state = source.state.clone();
    let resident = Arc::downgrade(&source.resident);
    let pulls = Arc::new(StoredPulls::default());
    let environment = stored_environment(Arc::new(StoredProducer::new(
        &source,
        Arc::from("partial-snapshot"),
        pulls.clone(),
    )));
    let error = engine
        .query_governed_fallible_view(
            &source,
            request(STORED_QUERY),
            QueryOptions::EMPTY.with_env(&environment),
            &QueryGovernors::METERED.with_max_answers(1),
        )
        .expect_err("answer ceiling preserves a prefix");
    let FallibleSparqlError::BudgetExhausted {
        partial: PartialAnswers::Certain(partial),
        ..
    } = error
    else {
        panic!("positive producer must retain a certified partial result");
    };
    assert_eq!(pulls.unmetered.load(Ordering::Relaxed), 0);
    drop(environment);
    drop(source);
    drop(engine);
    assert!(resident.upgrade().is_none());
    assert_eq!(partial.result().solutions().unwrap().1.len(), 1);
    assert!(state.owned_bytes.load(Ordering::Relaxed) > 0);
    drop(partial);
    state.assert_released();
}

purrdf_testkit::harness_main!(
    borrowed_cursor_keeps_its_snapshot_after_registry_handles_are_released,
    borrowed_producers_refuse_unaffordable_rows_and_release_original_admission,
    borrowed_producers_keep_partial_rows_after_the_query_scope_ends,
    borrowed_producers_stop_inside_pulls_and_preserve_storage_failure_precedence,
    borrowed_producers_discard_candidates_when_the_original_source_fails,
    borrowed_producers_keep_frozen_snapshot_and_original_result_owners,
    governed_explain_matches_bound_query_receipts_and_original_ownership,
    governed_explain_preserves_remote_refusal_and_zero_request_limit,
    materialized_existing_cells_use_actual_payload_admission,
    partial_materialized_cells_keep_actual_payload_admission,
    public_explain_projection_holds_real_execution_permit_through_final_read,
    typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures,
    preexisting_source_failure_outranks_parse_and_empty_pattern_success,
    retained_prepared_visits_remain_inside_the_checked_publication_scope,
    refused_reporting_admission_remains_typed_even_before_sticky_failure,
    direct_execution_admission_failures_keep_their_typed_cause_and_exact_evidence,
    named_graph_lookup_refusal_never_certifies_an_empty_scope,
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
    explain_admitted_supplied_inputs_preserve_parse_failure_before_reading,
    explain_stop_retains_stopped_receipt_while_operational_failure_takes_precedence,
    prepared_governed_queries_discard_rows_produced_before_an_iterator_failure,
    the_final_prepared_checkpoint_outranks_every_ready_evaluator_outcome,
    governed_explain_coordinates_survive_forecast_and_publication_refusals,
    materialized_coordinate_receipts_balance_with_retained_engine_and_plan,
);
