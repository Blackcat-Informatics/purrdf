// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generic query publication must include reads after the last algebra node.
mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, NoopReservation, QuadIds, RdfDataset,
    RdfStoreCapabilities, SparqlRequest, TermId, TermValue, ViewOperationStatus,
    WorkspaceReservation,
};
use purrdf_sparql_eval::{
    FallibleSparqlError, GovernorState, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefusedRead;
impl std::fmt::Display for RefusedRead {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("source refused the final read")
    }
}
impl std::error::Error for RefusedRead {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RefusalSite {
    Never,
    Iterator,
    Term,
    Reporting,
}
struct Source {
    resident: Arc<RdfDataset>,
    failed: AtomicBool,
    site: RefusalSite,
}
impl Source {
    fn at(site: RefusalSite) -> Self {
        Self {
            resident: support::local_dataset([("s", "p", "o")]),
            failed: AtomicBool::new(false),
            site,
        }
    }
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
    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl WorkspaceReservation<Error = RefusedRead> + '_, RefusedRead> {
        if self.site == RefusalSite::Reporting {
            // An adapter may refuse admission before a sticky read failure: the
            // returned operational cause must still remain typed and exact.
            Err(RefusedRead)
        } else {
            Ok(NoopReservation::<RefusedRead>::default())
        }
    }
    fn read_error(&self) -> Option<RefusedRead> {
        self.failed.load(Ordering::Relaxed).then_some(RefusedRead)
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.resident.as_ref().quads().filter(|_| {
            if self.site == RefusalSite::Iterator {
                self.refuse();
                false
            } else {
                true
            }
        })
    }
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, RefusedRead> {
        if self.site == RefusalSite::Term {
            self.refuse();
            return Err(RefusedRead);
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
    fn operation_status(&self) -> ViewOperationStatus<RefusedRead, bool> {
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
fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
fn assert_source_refusal<T: std::fmt::Debug, V: std::fmt::Debug>(
    result: Result<T, FallibleSparqlError<RefusedRead, V>>,
) {
    let error =
        result.expect_err("an incomplete source cannot publish any answer or budget outcome");
    assert_eq!(error.operational_error(), Some(&RefusedRead));
    assert!(error.diagnostic().is_none());
    assert!(error.partial_answers().is_none());
}
fn typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures() {
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
        assert_source_refusal(engine.query_prepared_fallible_view(
            &Source::at(site),
            &prepared,
            &[],
            QueryOptions::EMPTY,
        ));
        assert_source_refusal(engine.query_fallible_view(
            &Source::at(site),
            request(QUERY),
            QueryOptions::EMPTY,
        ));
        assert_source_refusal(engine.query_governed_fallible_view(
            &Source::at(site),
            request(QUERY),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ));
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        assert_source_refusal(engine.query_prepared_governed_fallible_in_operation(
            &Source::at(site),
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &state,
        ));
    }
    let source = Source::at(RefusalSite::Never);
    let engine = NativeSparqlEngine::new();
    let answer = engine
        .query_fallible_view(&source, request(QUERY), QueryOptions::EMPTY)
        .expect("valid neighboring source answers");
    assert!(
        matches!(answer.result,purrdf_core::SparqlResult::Solutions { rows,.. } if rows.len()==1)
    );
    assert!(!answer.evidence);
}
fn preexisting_source_failure_outranks_parse_and_empty_pattern_success() {
    let source = Source::at(RefusalSite::Never);
    source.refuse();
    let engine = NativeSparqlEngine::new();
    assert_source_refusal(engine.query_fallible_view(
        &source,
        request("not SPARQL"),
        QueryOptions::EMPTY,
    ));
    let empty = engine
        .prepare_query("SELECT * WHERE {}", None)
        .expect("prepare empty pattern");
    assert_source_refusal(engine.query_prepared_fallible_view(
        &source,
        &empty,
        &[],
        QueryOptions::EMPTY,
    ));
    assert_source_refusal(engine.query_governed_fallible_view(
        &source,
        request("not SPARQL"),
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    ));
}
fn retained_prepared_visits_remain_inside_the_checked_publication_scope() {
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    for site in [RefusalSite::Iterator, RefusalSite::Term] {
        let source = Source::at(site);
        let subject = source.resident.quads().next().expect("fixture row").s;
        assert_source_refusal(engine.execute_fallible(
            &mut execution,
            &source,
            QueryOptions::EMPTY,
            |_| {
                let _read = source.resolve(subject);
                7
            },
        ));
    }
    let source = Source::at(RefusalSite::Never);
    assert_source_refusal(engine.execute_fallible(
        &mut execution,
        &source,
        QueryOptions::EMPTY,
        |_| {
            source.refuse();
            7
        },
    ));
    let source = Source::at(RefusalSite::Never);
    assert_eq!(
        engine
            .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |_| 7)
            .expect("retained workspace answers valid neighbor"),
        (7, false)
    );
}
fn refused_reporting_admission_remains_typed_even_before_sticky_failure() {
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare fixture");
    let source = Source::at(RefusalSite::Reporting);
    assert_source_refusal(engine.query_prepared_fallible_view(
        &source,
        &prepared,
        &[],
        QueryOptions::EMPTY,
    ));
    assert_source_refusal(engine.query_fallible_view(
        &source,
        request("not SPARQL"),
        QueryOptions::EMPTY,
    ));
    assert_source_refusal(engine.query_governed_fallible_view(
        &source,
        request(QUERY),
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    ));
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    assert_source_refusal(engine.query_prepared_governed_fallible_in_operation(
        &source,
        &prepared,
        &[],
        QueryOptions::EMPTY,
        &state,
    ));
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare execution");
    assert_source_refusal(engine.execute_fallible(
        &mut execution,
        &source,
        QueryOptions::EMPTY,
        |_| 7,
    ));
    assert!(source.read_error().is_none());
}
purrdf_testkit::harness_main!(
    typed_owned_and_governed_queries_never_publish_iterator_or_materialization_failures,
    preexisting_source_failure_outranks_parse_and_empty_pattern_success,
    retained_prepared_visits_remain_inside_the_checked_publication_scope,
    refused_reporting_admission_remains_typed_even_before_sticky_failure,
);
