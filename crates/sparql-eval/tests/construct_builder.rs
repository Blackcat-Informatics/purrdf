// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression coverage for typed immutable carrier publication.

use purrdf_core::ir::QuadProbePlan;
use purrdf_core::{
    BlankScope, DatasetView, FallibleDatasetView, GraphMatch, QuadIds, QuadRef, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, SparqlResult, TermId, TermRef, TermValue,
    ViewOperationStatus,
};
use purrdf_sparql_eval::{
    CancellationFlag, FallibleSparqlError, GovernorState, GraphBuildError, LossVocabulary,
    NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const VALUE: &str = "https://example.org/value";

fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("c1", BlankScope::DEFAULT);
    let p = b.intern_iri(VALUE);
    let o = b.intern_literal(RdfLiteral::simple("value"));
    b.push_quad(s, p, o, None);
    b.freeze().expect("data")
}

fn assert_same(left: &RdfDataset, right: &RdfDataset) {
    assert_eq!(
        left.owned_quads().collect::<Vec<_>>(),
        right.owned_quads().collect::<Vec<_>>()
    );
    assert_eq!(
        left.owned_reifiers().collect::<Vec<_>>(),
        right.owned_reifiers().collect::<Vec<_>>()
    );
    assert_eq!(
        left.owned_annotations().collect::<Vec<_>>(),
        right.owned_annotations().collect::<Vec<_>>()
    );
    assert_eq!(
        left.owned_named_graphs().collect::<Vec<_>>(),
        right.owned_named_graphs().collect::<Vec<_>>()
    );
}

#[test]
fn direct_append_matches_native_construct_and_keeps_fresh_blanks() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let plan = engine.prepare_query("CONSTRUCT { _:fresh <https://example.org/value> ?o . ?s <https://example.org/value> ?o } WHERE { ?s <https://example.org/value> ?o }", None).expect("plan");
    let SparqlResult::Graph(expected) = engine
        .query_prepared(&data, &plan, &[], QueryOptions::EMPTY)
        .expect("ordinary")
    else {
        panic!("graph")
    };
    let mut builder = RdfDatasetBuilder::new();
    let stats = engine
        .construct_prepared_into_view(data.as_ref(), &plan, &[], QueryOptions::EMPTY, &mut builder)
        .expect("direct");
    assert_eq!(stats.intermediate_freezes, 0);
    assert_eq!(stats.statements, 2);
    let actual = builder.freeze().expect("publication");
    assert_same(&expected, &actual);
    assert_eq!(
        actual.quad_count(),
        2,
        "the minted blank stays distinct from data c1"
    );
}

#[test]
fn configured_rdf12_projection_loss_is_identical() {
    let engine = NativeSparqlEngine::new().with_loss_vocabulary(LossVocabulary::new(
        "https://example.org/loss",
        "https://example.org/lossCode",
        "https://example.org/lostReifies",
    ));
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("https://example.org/s");
    let p = b.intern_iri(VALUE);
    let o = b.intern_literal(RdfLiteral::simple("claim"));
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri("https://example.org/r");
    b.push_reifier(r, triple);
    b.push_annotation(r, p, o);
    let data = b.freeze().expect("data");
    let plan = engine.prepare_query("CONSTRUCT { ?s ?p ?o } WHERE { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( ?s ?p ?o )>> }", None).expect("plan");
    let SparqlResult::Graph(expected) = engine
        .query_prepared(&data, &plan, &[], QueryOptions::EMPTY)
        .expect("ordinary")
    else {
        panic!("graph")
    };
    let mut output = RdfDatasetBuilder::new();
    engine
        .construct_prepared_into_view(data.as_ref(), &plan, &[], QueryOptions::EMPTY, &mut output)
        .expect("direct");
    let actual = output.freeze().expect("output");
    assert_same(&expected, &actual);
    assert!(
        actual.quad_count() > 1,
        "loss evidence must be present in the graph"
    );
    assert!(
        actual
            .term_id_by_iri("https://example.org/lossCode")
            .is_some()
    );
}

#[test]
fn governor_trip_and_cancellation_leave_existing_builder_untouched() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let plan = engine.prepare_query("CONSTRUCT { ?s <https://example.org/value> ?o } WHERE { ?s <https://example.org/value> ?o }", None).expect("plan");
    for governors in [QueryGovernors::UNBOUNDED.with_max_answers(0), {
        let flag = Arc::new(CancellationFlag::new());
        flag.cancel();
        QueryGovernors::UNBOUNDED.with_stop_signal(flag)
    }] {
        let state = Arc::new(GovernorState::new(&governors));
        let mut target = RdfDatasetBuilder::new();
        let s = target.intern_iri("https://example.org/existing");
        let p = target.intern_iri(VALUE);
        target.push_quad(s, p, s, None);
        assert!(matches!(
            engine.construct_prepared_in_operation_into_view(
                data.as_ref(),
                &plan,
                &[],
                QueryOptions::EMPTY,
                &state,
                &mut target
            ),
            Err(GraphBuildError::BudgetExhausted { .. })
        ));
        assert_eq!(target.freeze().expect("unchanged").quad_count(), 1);
    }
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let mut target = RdfDatasetBuilder::new();
    let receipt = engine
        .construct_prepared_in_operation_into_view(
            data.as_ref(),
            &plan,
            &[],
            QueryOptions::EMPTY,
            &state,
            &mut target,
        )
        .expect("complete");
    assert_eq!(receipt.governors, Some(state.evidence()));
}

#[derive(Clone, Debug)]
struct SourceFailure;
impl std::fmt::Display for SourceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("final source failure")
    }
}
impl std::error::Error for SourceFailure {}

struct FinalFailure {
    data: Arc<RdfDataset>,
    checkpoints: AtomicUsize,
}
impl DatasetView for FinalFailure {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.data.quads()
    }
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        self.data.quad_refs()
    }
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.data.resolve(id)
    }
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        self.data.term_id_by_value(value)
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.data.capabilities()
    }
    fn probe_plan(&self, s: bool, p: bool, o: bool, g: GraphMatch) -> QuadProbePlan {
        RdfDataset::probe_plan(s, p, o, g)
    }
    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.data.quads_for_pattern_with_plan(plan, s, p, o, g)
    }
    fn term_count(&self) -> usize {
        self.data.term_count()
    }
}
impl FallibleDatasetView for FinalFailure {
    type Error = SourceFailure;
    type Evidence = usize;
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        let evidence = self.checkpoints.fetch_add(1, Ordering::SeqCst);
        if evidence == 0 {
            ViewOperationStatus::Ready { evidence }
        } else {
            ViewOperationStatus::Failed {
                error: SourceFailure,
                evidence,
            }
        }
    }
}

#[test]
fn final_operational_failure_outranks_budget_and_publishes_nothing() {
    let engine = NativeSparqlEngine::new();
    let data = FinalFailure {
        data: dataset(),
        checkpoints: AtomicUsize::new(0),
    };
    let plan = engine.prepare_query("CONSTRUCT { ?s <https://example.org/value> ?o } WHERE { ?s <https://example.org/value> ?o }", None).expect("plan");
    let state = Arc::new(GovernorState::new(
        &QueryGovernors::UNBOUNDED.with_max_answers(0),
    ));
    let mut target = RdfDatasetBuilder::new();
    let error = engine
        .construct_prepared_fallible_in_operation_into_view(
            &data,
            &plan,
            &[],
            QueryOptions::EMPTY,
            &state,
            &mut target,
        )
        .expect_err("operational failure");
    assert!(matches!(
        error,
        FallibleSparqlError::Operational {
            error: SourceFailure,
            ..
        }
    ));
    assert_eq!(target.freeze().expect("unpublished").quad_count(), 0);
}

#[test]
fn successive_appends_keep_minted_blanks_fresh_against_destination_only_nodes() {
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let plan = engine.prepare_query("CONSTRUCT { _:fresh <https://example.org/value> ?o . ?s <https://example.org/carried> ?o } WHERE { ?s <https://example.org/value> ?o }", None).expect("plan");
    let mut target = RdfDatasetBuilder::new();
    let original = target.intern_blank("append0_c1", BlankScope::DEFAULT);
    let p = target.intern_iri(VALUE);
    let o = target.intern_literal(RdfLiteral::simple("destination"));
    target.push_quad(original, p, o, None);
    for _ in 0..2 {
        engine
            .construct_prepared_into_view(
                data.as_ref(),
                &plan,
                &[],
                QueryOptions::EMPTY,
                &mut target,
            )
            .expect("append");
    }
    let graph = target.freeze().expect("graph");
    let predicate = graph.term_id_by_iri(VALUE).expect("predicate");
    let subjects: std::collections::BTreeSet<_> = graph
        .quads_for_pattern(None, Some(predicate), None, GraphMatch::Any)
        .map(|q| q.s)
        .collect();
    assert_eq!(
        subjects.len(),
        3,
        "each template mint is fresh against the destination and other appends"
    );
    let carried = graph
        .term_id_by_iri("https://example.org/carried")
        .expect("carried");
    assert_eq!(
        graph
            .quads_for_pattern(None, Some(carried), None, GraphMatch::Any)
            .count(),
        1,
        "data-carried identity is shared across appends"
    );
}

#[test]
fn mutated_construct_is_depth_admitted_before_survey_or_substitution() {
    use purrdf_sparql_algebra::{GraphPattern, Query};
    let engine = NativeSparqlEngine::new();
    let data = dataset();
    let prepared = engine.prepare_query("CONSTRUCT { ?s <https://example.org/value> ?o } WHERE { ?s <https://example.org/value> ?o }",None).expect("plan");
    let mut algebra = prepared.query.clone();
    let Query::Construct { pattern, .. } = &mut algebra else {
        panic!("construct")
    };
    for _ in 0..=purrdf_sparql_algebra::MAX_GRAPH_PATTERN_DEPTH {
        *pattern = GraphPattern::Join {
            left: Box::new(std::mem::replace(
                pattern,
                GraphPattern::Bgp {
                    patterns: Vec::new(),
                },
            )),
            right: Box::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
        };
    }
    // Public algebra remains mutable, so the publication entry must re-admit it.
    let mut prepared =
        purrdf_sparql_eval::PreparedQuery::rewritten(prepared.query.clone(), QueryOptions::EMPTY)
            .expect("owned admitted plan");
    prepared.query = algebra;
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let mut target = RdfDatasetBuilder::new();
    let result = engine.construct_prepared_in_operation_into_view(
        data.as_ref(),
        &prepared,
        &[("s".to_owned(), TermValue::blank("c1"))],
        QueryOptions::EMPTY,
        &state,
        &mut target,
    );
    assert!(matches!(result, Err(GraphBuildError::Query(_))));
    assert_eq!(target.freeze().expect("untouched").quad_count(), 0);
}

#[test]
fn typed_publication_preserves_nested_directional_terms_and_named_statement_metadata() {
    let mut input = RdfDatasetBuilder::new();
    let s = input.intern_blank("data", BlankScope(17));
    let p = input.intern_iri(VALUE);
    let o = input.intern_literal(RdfLiteral {
        direction: Some(purrdf_core::RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("claim", "ar")
    });
    let inner = input.intern_triple(s, p, o);
    let outer = input.intern_triple(s, p, inner);
    let r = input.intern_blank("reifier", BlankScope(17));
    input.push_reifier(r, outer);
    input.push_annotation(r, p, o);
    let input = input.freeze().expect("input");
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query("CONSTRUCT { GRAPH <https://example.org/output> { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( ?s <https://example.org/value> ?inner )>> . ?r <https://example.org/value> ?o } } WHERE { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( ?s <https://example.org/value> ?inner )>> . ?r <https://example.org/value> ?o }", None).expect("plan");
    let SparqlResult::Graph(expected) = engine
        .query_prepared(&input, &prepared, &[], QueryOptions::EMPTY)
        .expect("ordinary")
    else {
        panic!("graph")
    };
    let mut target = RdfDatasetBuilder::new();
    engine
        .construct_prepared_into_view(
            input.as_ref(),
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &mut target,
        )
        .expect("direct");
    let actual = target.freeze().expect("publication");
    assert_same(&expected, &actual);
    assert_eq!(actual.reifiers_with_graph().count(), 1);
    assert_eq!(actual.annotations_with_graph().count(), 1);
    assert!(actual.reifiers_with_graph().all(|(_, _, g)| g.is_some()));
    assert!(
        actual
            .annotations_with_graph()
            .all(|(_, _, _, g)| g.is_some())
    );
    assert!(actual.term_id_by_blank("data", BlankScope(17)).is_some());
    assert!(
        actual
            .term_id_by_literal(
                "claim",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
                Some("ar"),
                Some(purrdf_core::RdfTextDirection::Rtl)
            )
            .is_some()
    );
}
