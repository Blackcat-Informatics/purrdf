// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generic query publication must include reads after the last algebra node.
mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, NoopReservation, QuadIds, RdfDataset,
    RdfStoreCapabilities, SparqlRequest, SparqlResult, TermId, TermValue, ViewOperationStatus,
    WorkspaceReservation,
};
use purrdf_sparql_eval::{
    FallibleSparqlError, FallibleSparqlResult, GovernedEvidence, GovernorState, NativeSparqlEngine,
    PreparedQuery, QueryGovernors, QueryOptions,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefusedRead(RefusalSite);
impl std::fmt::Display for RefusedRead {
    /// Preserve the injected read site when an evaluator projects the cause into a diagnostic.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "source refused at {:?}", self.0)
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
    FinalCheckpoint,
}
struct Source {
    resident: Arc<RdfDataset>,
    failed: AtomicBool,
    yielded: AtomicUsize,
    checkpoints: AtomicUsize,
    site: RefusalSite,
}
impl Source {
    /// Start with one healthy resident row and zero observations of the selected refusal site.
    fn at(site: RefusalSite) -> Self {
        Self {
            resident: support::local_dataset([("s", "p", "o")]),
            failed: AtomicBool::new(false),
            yielded: AtomicUsize::new(0),
            checkpoints: AtomicUsize::new(0),
            site,
        }
    }
    /// Latch the refusal so every subsequent publication checkpoint sees the same cause.
    fn refuse(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }
}
impl DatasetView for Source {
    type Id = TermId;
    type ReadError = RefusedRead;
    type TermGuard<'a> = <RdfDataset as DatasetView>::TermGuard<'a>;
    type ProbePlan = ();
    fn storage_live_budget(&self) -> Option<u64> {
        (self.site == RefusalSite::Reporting).then_some(8192)
    }
    /// Refuse reporting admission independently of the sticky lazy-read failure flag.
    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl WorkspaceReservation<Error = RefusedRead> + '_, RefusedRead> {
        if self.site == RefusalSite::Reporting {
            // An adapter may refuse admission before a sticky read failure: the
            // returned operational cause must still remain typed and exact.
            Err(RefusedRead(self.site))
        } else {
            Ok(NoopReservation::<RefusedRead>::default())
        }
    }
    /// Read the latched cause without advancing the publication-checkpoint counter.
    fn read_error(&self) -> Option<RefusedRead> {
        self.failed
            .load(Ordering::Relaxed)
            .then_some(RefusedRead(self.site))
    }
    /// Inject failure either before yielding a row or when a yielded row's stream ends.
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        let mut rows = self.resident.as_ref().quads();
        std::iter::from_fn(move || {
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
    /// Make owned-result materialization the failing read without replacing the term's identity.
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, RefusedRead> {
        if self.site == RefusalSite::Term {
            self.refuse();
            return Err(RefusedRead(self.site));
        }
        Ok(
            <RdfDataset as DatasetView>::resolve(self.resident.as_ref(), id)
                .expect("resident term resolution is infallible"),
        )
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, RefusedRead> {
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
    type Evidence = bool;
    /// Inject the selected final-checkpoint fault only at the second status sample.
    fn operation_status(&self) -> ViewOperationStatus<RefusedRead, bool> {
        let prior = self.checkpoints.fetch_add(1, Ordering::Relaxed);
        if self.site == RefusalSite::FinalCheckpoint && prior == 1 {
            self.refuse();
        }
        match self.read_error() {
            Some(error) => ViewOperationStatus::Failed {
                error,
                evidence: true,
            },
            None => ViewOperationStatus::Ready { evidence: false },
        }
    }
}
const QUERY: &str = "SELECT ?s WHERE { ?s ?p ?o }";
/// Request default-dataset evaluation without a base or caller substitutions.
fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
/// Require the exact operational cause and forbid query or budget outcomes and partial answers.
fn assert_source_refusal<T: std::fmt::Debug, V: std::fmt::Debug>(
    result: Result<T, FallibleSparqlError<RefusedRead, V>>,
    site: RefusalSite,
) -> FallibleSparqlError<RefusedRead, V> {
    let error =
        result.expect_err("an incomplete source cannot publish any answer or budget outcome");
    assert_eq!(error.operational_error(), Some(&RefusedRead(site)));
    assert!(error.diagnostic().is_none());
    assert!(error.partial_answers().is_none());
    assert!(error.tripped().is_none());
    error
}
/// Exercise owned text and prepared fallible boundaries at iterator and term-materialization failures.
fn typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures() {
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
        assert_source_refusal(
            engine.query_prepared_fallible_view(
                &Source::at(site),
                &prepared,
                &[],
                QueryOptions::EMPTY,
            ),
            site,
        );
        assert_source_refusal(
            engine.query_fallible_view(&Source::at(site), request(QUERY), QueryOptions::EMPTY),
            site,
        );
        assert_source_refusal(
            engine.query_governed_fallible_view(
                &Source::at(site),
                request(QUERY),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            ),
            site,
        );
        assert_source_refusal(
            engine.query_prepared_governed_fallible_view(
                &Source::at(site),
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            ),
            site,
        );
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        assert_source_refusal(
            engine.query_prepared_governed_fallible_in_operation(
                &Source::at(site),
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &state,
            ),
            site,
        );
    }
    let source = Source::at(RefusalSite::Never);
    let engine = NativeSparqlEngine::new();
    let answer = engine
        .query_fallible_view(&source, request(QUERY), QueryOptions::EMPTY)
        .expect("valid neighboring source answers");
    assert!(matches!(answer.result,SparqlResult::Solutions { rows,.. } if rows.len()==1));
    assert!(!answer.evidence);
}
/// A failed ingress checkpoint must defeat invalid text and plans that need no data reads.
fn preexisting_source_failure_outranks_parse_and_empty_pattern_success() {
    let source = Source::at(RefusalSite::Never);
    source.refuse();
    let engine = NativeSparqlEngine::new();
    assert_source_refusal(
        engine.query_fallible_view(&source, request("not SPARQL"), QueryOptions::EMPTY),
        RefusalSite::Never,
    );
    let empty = engine
        .prepare_query("SELECT * WHERE {}", None)
        .expect("prepare empty pattern");
    assert_source_refusal(
        engine.query_prepared_fallible_view(&source, &empty, &[], QueryOptions::EMPTY),
        RefusalSite::Never,
    );
    assert_source_refusal(
        engine.query_governed_fallible_view(
            &source,
            request("not SPARQL"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        RefusalSite::Never,
    );
    assert_source_refusal(
        engine.query_prepared_governed_fallible_view(
            &source,
            &empty,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        RefusalSite::Never,
    );
}
/// Reads performed by a retained-result visitor remain covered by the final checkpoint.
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
            site,
        );
    }
    let source = Source::at(RefusalSite::Never);
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| {
            source.refuse();
            7
        }),
        RefusalSite::Never,
    );
    let source = Source::at(RefusalSite::Never);
    assert_eq!(
        engine
            .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7)
            .expect("retained workspace answers valid neighbor"),
        (7, false)
    );
}
/// A reporting reservation's own cause survives even when the source has no latched read failure.
fn refused_reporting_admission_remains_typed_even_before_sticky_failure() {
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
    let source = Source::at(RefusalSite::Reporting);
    assert_source_refusal(
        engine.query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY),
        RefusalSite::Reporting,
    );
    assert_source_refusal(
        engine.query_fallible_view(&source, request("not SPARQL"), QueryOptions::EMPTY),
        RefusalSite::Reporting,
    );
    assert_source_refusal(
        engine.query_governed_fallible_view(
            &source,
            request(QUERY),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        RefusalSite::Reporting,
    );
    let refusal = assert_source_refusal(
        engine.query_prepared_governed_fallible_view(
            &source,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        RefusalSite::Reporting,
    );
    assert!(!refusal.evidence().view);
    assert_eq!(
        refusal.evidence().governors,
        GovernorState::new(&QueryGovernors::METERED).evidence()
    );
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    assert_source_refusal(
        engine.query_prepared_governed_fallible_in_operation(
            &source,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &state,
        ),
        RefusalSite::Reporting,
    );
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    assert_source_refusal(
        engine.execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7),
        RefusalSite::Reporting,
    );
    assert!(source.read_error().is_none());
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
) -> FallibleSparqlResult<RefusedRead, GovernedEvidence<bool>> {
    if shared {
        let state = Arc::new(GovernorState::new(governors));
        engine.query_prepared_governed_fallible_in_operation(
            source,
            prepared,
            &[],
            QueryOptions::EMPTY,
            &state,
        )
    } else {
        engine.query_prepared_governed_fallible_view(
            source,
            prepared,
            &[],
            QueryOptions::EMPTY,
            governors,
        )
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
            RefusalSite::AfterRow,
        );
        assert_eq!(source.yielded.load(Ordering::Relaxed), 1);
        assert_eq!(source.checkpoints.load(Ordering::Relaxed), 2);
        assert!(refusal.evidence().view);
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
            let healthy_governors = match (expected, healthy_outcome) {
                (ReadyOutcome::Complete, Ok(complete)) => {
                    assert!(matches!(
                        complete.result,
                        SparqlResult::Solutions { rows, .. } if rows.len() == 1
                    ));
                    assert!(!complete.evidence.view);
                    complete.evidence.governors
                }
                (
                    ReadyOutcome::Query,
                    Err(FallibleSparqlError::Query {
                        diagnostic,
                        evidence,
                    }),
                ) => {
                    assert_eq!(diagnostic.code, "native-sparql-service-unconfigured");
                    assert!(!evidence.view);
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
                    assert!(matches!(
                        partial.result().expect("certified prefix").result(),
                        SparqlResult::Solutions { rows, .. } if rows.len() == 1
                    ));
                    assert!(!evidence.view);
                    evidence.governors
                }
                (_, other) => {
                    panic!("healthy source produced the wrong evaluator outcome: {other:?}")
                }
            };
            assert_eq!(healthy.checkpoints.load(Ordering::Relaxed), 2);

            let source = Source::at(RefusalSite::FinalCheckpoint);
            let refusal = assert_source_refusal(
                prepared_governed(&engine, &source, &prepared, &governors, shared),
                RefusalSite::FinalCheckpoint,
            );
            assert_eq!(source.checkpoints.load(Ordering::Relaxed), 2);
            assert!(refusal.evidence().view);
            assert_eq!(refusal.evidence().governors, healthy_governors);
        }
    }
}
purrdf_testkit::harness_main!(
    typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures,
    preexisting_source_failure_outranks_parse_and_empty_pattern_success,
    retained_prepared_visits_remain_inside_the_checked_publication_scope,
    refused_reporting_admission_remains_typed_even_before_sticky_failure,
    prepared_governed_queries_discard_rows_produced_before_an_iterator_failure,
    the_final_prepared_checkpoint_outranks_every_ready_evaluator_outcome,
);
