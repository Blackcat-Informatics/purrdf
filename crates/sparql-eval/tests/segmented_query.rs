// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified operator residency over persistent global IDs and sparse eviction.

mod support;

use support::segmented::{
    CEILING, QUERY, baseline_ceiling, fixture, fixture_with_layout, open, open_with_evidence,
};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use purrdf_core::{
    DatasetView, GraphMatch, SegmentedBytes, SegmentedError, SegmentedEvidence, SegmentedProvider,
    SegmentedReadLimits, SegmentedSession, SegmentedSnapshot, SparqlRequest, SparqlResult,
    TermFactory, TermValue,
};
use purrdf_sparql_eval::{
    CancellationFlag, FallibleSparqlError, FallibleSparqlResult, GovernedEvidence, GovernorState,
    NativeSparqlEngine, NodeCharges, PreparedQuery, QueryGovernors, QueryOptions,
    ResourceDimension,
};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[test]
fn chronological_segmented_stack_matches_eager_raw_and_prepared_queries() {
    use purrdf_core::{
        RdfDatasetBuilder, SegmentedLayer, SegmentedLayerMetadata, SegmentedResidentHead,
        SegmentedStack,
    };
    let (first, resident) = fixture();
    let (second, _) = fixture_with_layout(1024, 8);
    let read_limits = SegmentedReadLimits::new(CEILING, 2, 65_536, 100_000_000, 8);
    let mut stack = SegmentedStack::default();
    stack
        .publish(
            [first, second]
                .into_iter()
                .map(|image| {
                    Arc::new(
                        SegmentedLayer::new(
                            Arc::new(image.provider()),
                            image.receipt().clone(),
                            SegmentedLayerMetadata::default(),
                        )
                        .unwrap(),
                    )
                })
                .collect(),
            read_limits,
        )
        .unwrap();
    let target = resident
        .term_id_by_value(&TermValue::iri("https://example.org/target"))
        .unwrap()
        .unwrap();
    let predicate = resident
        .term_id_by_value(&TermValue::iri("https://example.org/p"))
        .unwrap()
        .unwrap();
    let removed = resident
        .quads()
        .find(|row| row.s == target && row.p == predicate)
        .unwrap();
    let copy_row = |builder: &mut RdfDatasetBuilder, row: purrdf_core::QuadIds| {
        let s = builder
            .intern_value(&resident.as_ref().term_value(row.s))
            .unwrap();
        let p = builder
            .intern_value(&resident.as_ref().term_value(row.p))
            .unwrap();
        let o = builder
            .intern_value(&resident.as_ref().term_value(row.o))
            .unwrap();
        let g = row.g.map(|id| {
            builder
                .intern_value(&resident.as_ref().term_value(id))
                .unwrap()
        });
        builder.push_quad(s, p, o, g).unwrap();
    };
    let mut removals = RdfDatasetBuilder::new();
    copy_row(&mut removals, removed);
    let removals = removals.freeze().unwrap();
    let mut declarations = RdfDatasetBuilder::new();
    let empty = declarations
        .intern_iri("https://example.org/empty")
        .unwrap();
    declarations.declare_named_graph(empty).unwrap();
    let declarations = declarations.freeze().unwrap();
    let add_scoped = |builder: &mut RdfDatasetBuilder| {
        for scope in [7, 8] {
            let blank = TermValue::Blank {
                label: "same".into(),
                scope: purrdf_core::BlankScope(scope),
            };
            let nested = TermValue::Triple {
                s: blank.clone().into(),
                p: TermValue::iri("https://example.org/nested").into(),
                o: TermValue::typed_literal(
                    format!("[_:purrdfesc{scope}_same, 42]"),
                    purrdf_cdt::datatype::CDT_LIST,
                )
                .into(),
            };
            let s = builder.intern_value(&blank).unwrap();
            let p = builder.intern_iri("https://example.org/scoped").unwrap();
            let o = builder.intern_value(&nested).unwrap();
            builder.push_quad(s, p, o, None).unwrap();
        }
    };
    let engine = NativeSparqlEngine::new();
    for reinsert in [false, true] {
        let mut head = RdfDatasetBuilder::new();
        if reinsert {
            copy_row(&mut head, removed);
        }
        add_scoped(&mut head);
        stack.set_head(Some(
            SegmentedResidentHead::new(
                head.freeze().unwrap(),
                SegmentedLayerMetadata::new(
                    Some(Arc::clone(&removals)),
                    Some(Arc::clone(&declarations)),
                )
                .unwrap(),
            )
            .unwrap(),
        ));
        let snapshot = stack.snapshot().unwrap();
        let mut eager = RdfDatasetBuilder::new();
        for row in resident.quads().filter(|row| reinsert || *row != removed) {
            copy_row(&mut eager, row);
        }
        add_scoped(&mut eager);
        let empty = eager.intern_iri("https://example.org/empty").unwrap();
        eager.declare_named_graph(empty).unwrap();
        let eager = eager.freeze().unwrap();
        for text in [
            QUERY,
            "SELECT ?s ?o WHERE { ?s <https://example.org/scoped> ?o } ORDER BY ?s",
            "SELECT ?o WHERE { { <https://example.org/target> <https://example.org/q> ?o } UNION { <https://example.org/target> <https://example.org/q> ?o } } ORDER BY ?o",
            "SELECT ?o ?missing WHERE { <https://example.org/target> <https://example.org/q> ?o OPTIONAL { ?o <https://example.org/absent> ?missing } }",
            "SELECT ?o (1/0 AS ?undefined) WHERE { <https://example.org/target> <https://example.org/q> ?o }",
            "SELECT ?o WHERE { <https://example.org/target> (<https://example.org/p>|<https://example.org/q>) ?o } ORDER BY ?o",
            "SELECT ?o WHERE { <https://example.org/target> <https://example.org/p> ?o OPTIONAL { <https://example.org/target> <https://example.org/q> ?o } } ORDER BY ?o",
            "SELECT ?o WHERE { <https://example.org/target> <https://example.org/q> ?o MINUS { <https://example.org/target> <https://example.org/p> ?o } }",
            "SELECT ?o WHERE { <https://example.org/target> <https://example.org/q> ?o FILTER NOT EXISTS { <https://example.org/target> <https://example.org/p> ?o } }",
            "SELECT (COUNT(*) AS ?n) WHERE { ?s <https://example.org/p> ?o }",
            "SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g",
            "SELECT ?s WHERE { ?s <https://example.org/p> ?o } ORDER BY ?s LIMIT 3",
        ] {
            let prepared = engine.prepare_query(text, None).unwrap();
            let expected = support::solutions(
                engine
                    .query_prepared(&eager, &prepared, &[], QueryOptions::EMPTY)
                    .unwrap(),
            );
            for prepared_entry in [false, true] {
                let view = snapshot.query(read_limits).unwrap();
                let actual = if prepared_entry {
                    engine.query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
                } else {
                    engine.query_fallible_view(
                        &view,
                        SparqlRequest {
                            query: text,
                            base_iri: None,
                            substitutions: &[],
                        },
                        QueryOptions::EMPTY,
                    )
                }
                .unwrap_or_else(|error| match error {
                    FallibleSparqlError::Operational { error, evidence } => panic!(
                        "{text}: {error:?}; requests={}, io={}, evictions={}",
                        evidence.request_count(),
                        evidence.io_bytes(),
                        evidence.evictions()
                    ),
                    _ => panic!("{text}: engine failure while source remained ready"),
                });
                view.check_ready().unwrap();
                drop(view);
                assert_eq!(
                    support::solutions(actual.result),
                    expected,
                    "reinsert={reinsert}, prepared={prepared_entry}: {text}"
                );
                assert_eq!(actual.evidence.identity().layers().len(), 2);
                assert!(actual.evidence.peak_bytes() <= CEILING);
                if text.contains("COUNT(*)") {
                    eprintln!(
                        "COUNT reinsert={reinsert} prepared={prepared_entry}: requests={} io={} evictions={} peak={}",
                        actual.evidence.request_count(),
                        actual.evidence.io_bytes(),
                        actual.evidence.evictions(),
                        actual.evidence.peak_bytes()
                    );
                }
            }
        }
    }
}

#[test]
fn chronological_segmented_provider_failure_preserves_original_final_evidence() {
    use purrdf_core::{SegmentedLayer, SegmentedLayerMetadata, SegmentedStack};
    let (image, _) = fixture();
    let provider = Arc::new(RefusingProvider {
        bytes: image.provider(),
        refuse: AtomicBool::new(false),
        operation: "chronological source read",
    });
    let layer = Arc::new(
        SegmentedLayer::new(
            provider.clone(),
            image.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    );
    let read_limits = SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![layer], read_limits).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query(read_limits).unwrap();
    provider.refuse.store(true, Ordering::Relaxed);
    let error = NativeSparqlEngine::new()
        .query_fallible_view(
            &view,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_err();
    let FallibleSparqlError::Operational { error, evidence } = error else {
        panic!("source error must remain operational");
    };
    assert_eq!(
        error,
        SegmentedError::Provider {
            operation: "chronological source read",
            host_code: Some(5)
        }
    );
    evidence.with_requests(|requests| assert!(!requests.last().unwrap().completed()));
    provider.refuse.store(false, Ordering::Relaxed);
    assert_eq!(view.check_ready(), Err(error));
    drop(view);
    drop(snapshot);
    drop(stack);
    assert_eq!(evidence.identity().layers().len(), 1);
}

#[test]
fn persistent_id_binding_retains_original_admission_and_refuses_atomically() {
    let (image, resident) = fixture();
    let source = open(&image, CEILING);
    let target = TermValue::iri("https://example.org/target");
    let id = source.term_id_by_value(&target).unwrap().unwrap();
    // Resolve the same stored component outside the output allocation window.
    let _ = source.resolve(id).unwrap();
    let engine = NativeSparqlEngine::new();
    let query = "ASK { ?this <https://example.org/p> \"漢字\"@zh }";
    let mut execution = engine
        .prepare_execution(query, None, &["this"], QueryOptions::EMPTY)
        .unwrap();
    let before = source.evidence().live_bytes();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let ((), receipt) = execution.bind_id_fallible(0, &source, id).unwrap();
    let measured = window.close();
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= receipt.peak_bytes());
    assert!(source.evidence().live_bytes() > before);
    execution.unbind_all();
    assert_eq!(source.evidence().live_bytes(), before);
    execution.bind_id_fallible(0, &source, id).unwrap();
    let mut expected = engine
        .prepare_execution(query, None, &["this"], QueryOptions::EMPTY)
        .unwrap();
    expected.bind(0, target).unwrap();
    let expected_answer = engine
        .execute(&mut expected, &*resident, QueryOptions::EMPTY, |answer| {
            matches!(answer, purrdf_sparql_eval::InternedOutcome::Boolean(true))
        })
        .unwrap();
    let (answer, _) = engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |answer| {
            matches!(answer, purrdf_sparql_eval::InternedOutcome::Boolean(true))
        })
        .unwrap();
    assert!(expected_answer);
    assert_eq!(answer, expected_answer);
    let cold = open(&image, CEILING);
    let floor = cold.evidence().peak_bytes();
    drop(cold);
    let small = open(&image, floor);
    let baseline = small.evidence();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refused = execution.bind_id_fallible(0, &small, id);
    let measured = window.close();
    assert!(matches!(
        refused,
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Residency { .. },
            ..
        })
    ));
    drop(refused);
    let released = small.evidence();
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
        baseline.live_bytes() - baseline.unpinned_cache_bytes() - baseline.bookkeeping_bytes()
    );
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= small.evidence().peak_bytes());
    // Failed replacement keeps the old valid binding and its original owner.
    let (answer, _) = engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |answer| {
            matches!(answer, purrdf_sparql_eval::InternedOutcome::Boolean(true))
        })
        .unwrap();
    assert_eq!(answer, expected_answer);
}

#[derive(Clone, Copy)]
enum GovernedEntry<'a> {
    Text(&'a str),
    Prepared,
    Operation(&'a Arc<GovernorState>),
}

impl GovernedEntry<'_> {
    /// Run identical request inputs through the text, per-call prepared, or shared-state boundary.
    #[allow(
        clippy::result_large_err,
        reason = "the fixture preserves the paired receipts returned by each public entry"
    )]
    fn run<'d>(
        self,
        engine: &'d NativeSparqlEngine,
        source: &'d SegmentedSession,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d, 'd>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<SegmentedError, GovernedEvidence<SegmentedEvidence>> {
        match self {
            Self::Text(query) => engine.query_governed_fallible_view(
                source,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions,
                },
                options,
                governors,
            ),
            Self::Prepared => engine.query_prepared_governed_fallible_view(
                source,
                prepared,
                substitutions,
                options,
                governors,
            ),
            Self::Operation(state) => engine.query_prepared_governed_fallible_in_operation(
                source,
                prepared,
                substitutions,
                options,
                state,
            ),
        }
    }
}

#[derive(Debug)]
struct RefusingProvider {
    bytes: SegmentedBytes,
    refuse: AtomicBool,
    operation: &'static str,
}
impl SegmentedProvider for RefusingProvider {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len()
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        if self.refuse.load(Ordering::Relaxed) {
            return Err(SegmentedError::Provider {
                operation: self.operation,
                host_code: Some(5),
            });
        }
        self.bytes.read_at(position, output)
    }
}

/// Dense storage blocks must price only the selected subject range and retain provider refusals.
#[test]
fn dense_pages_price_only_the_exact_subject_range_and_preserve_source_refusal() {
    let (image, resident) = fixture_with_layout(8192, 128);
    let source = open(&image, CEILING);
    let target = source
        .term_id_by_value(&TermValue::iri("https://example.org/target"))
        .unwrap()
        .unwrap();
    let p = source
        .term_id_by_value(&TermValue::iri("https://example.org/p"))
        .unwrap()
        .unwrap();
    let q = source
        .term_id_by_value(&TermValue::iri("https://example.org/q"))
        .unwrap()
        .unwrap();
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(p), None, GraphMatch::Default),
        1
    );
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(q), None, GraphMatch::Default),
        1
    );
    assert!(source.cardinality_estimate(None, Some(p), None, GraphMatch::Default) >= 201);
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let expected = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let answer = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let found = answer.result.solutions().expect("SELECT shape").1;
    let expected = expected.solutions().expect("SELECT shape").1;
    assert_eq!(found, expected);

    let provider = Arc::new(RefusingProvider {
        bytes: image.provider(),
        refuse: AtomicBool::new(false),
        operation: "selective cardinality test",
    });
    let source = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8),
    )
    .unwrap();
    provider.refuse.store(true, Ordering::Relaxed);
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(p), None, GraphMatch::Default),
        0
    );
    assert!(matches!(
        source.read_error(),
        Some(SegmentedError::Provider { .. })
    ));
    assert!(matches!(
        engine.query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY),
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Provider { .. },
            ..
        })
    ));
}

/// A long host-owned cause stays exact while error publication fits the charged reporting ledger.
#[test]
fn long_host_refusal_keeps_its_typed_cause_without_unbounded_report_allocation() {
    let (image, _) = fixture();
    // Host-owned static label is prepared outside the execution allocation window.
    let operation: &'static str = Box::leak("漢字操作 ".repeat(20_000).into_boxed_str());
    for route in 0..3 {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(QUERY, None).unwrap();
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let entry = match route {
            0 => GovernedEntry::Text(QUERY),
            1 => GovernedEntry::Prepared,
            _ => GovernedEntry::Operation(&state),
        };
        let provider = Arc::new(RefusingProvider {
            bytes: image.provider(),
            refuse: AtomicBool::new(false),
            operation,
        });
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let source = SegmentedSession::open(
            provider.clone(),
            image.receipt(),
            SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8),
        )
        .unwrap();
        let opening = source.evidence();
        provider.refuse.store(true, Ordering::Relaxed);
        let outcome = entry.run(
            &engine,
            &source,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        );
        let ledger = source.evidence().peak_bytes();
        let measured = window.close();
        let Err(FallibleSparqlError::Operational {
            error:
                SegmentedError::Provider {
                    operation: found,
                    host_code: Some(5),
                },
            evidence,
        }) = outcome
        else {
            panic!("provider refusal must retain its typed cause")
        };
        assert_eq!(found, operation);
        assert_eq!(
            evidence.view.request_count(),
            source.evidence().request_count()
        );
        assert_eq!(
            evidence.view.request_digest(),
            source.evidence().request_digest()
        );
        assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= ledger);
        assert!(ledger <= CEILING);
        drop(evidence);
        drop(prepared);
        drop(engine);
        drop(state);
        let released = source.evidence();
        assert_eq!(
            released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
            opening.live_bytes() - opening.unpinned_cache_bytes() - opening.bookkeeping_bytes(),
            "typed provider refusal releases all native report, preparation and execution owners"
        );
    }
}

/// Sparse cache eviction preserves the resident join answer while measured allocations fit the ledger.
#[test]
fn selective_join_matches_resident_and_actual_peak_stays_below_shared_ledger() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let expected = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let source = open_with_evidence(&image, CEILING, 2, 16_384);
    let answer = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let (actual_variables, actual_rows) = answer.result.solutions().expect("SELECT shape");
    let SparqlResult::Solutions {
        variables: expected_variables,
        rows: expected_rows,
        ..
    } = &expected
    else {
        panic!("SELECT returns rows")
    };
    assert_eq!(actual_variables, expected_variables);
    assert_eq!(actual_rows, expected_rows);
    assert_eq!(answer.result.solutions().expect("SELECT shape").1.len(), 1);
    assert!(answer.evidence.evictions() > 0);
    let ledger = answer.evidence.peak_bytes();
    let measured = window.close();
    assert!(
        u64::try_from(measured.peak_working_bytes).unwrap() <= ledger,
        "actual {} exceeds shared reservation {ledger}",
        measured.peak_working_bytes
    );
    assert!(ledger <= CEILING);
    assert!(source.read_error().is_none());
}

/// Retained-result consumption keeps its workspace live, and ordinary owned egress still succeeds.
#[test]
fn scoped_and_prepared_engine_egresses_hold_the_reservation_through_consumption() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let source = open(&image, CEILING);
    let before = source.evidence().live_bytes();
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .unwrap();
    engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |outcome| {
            assert!(
                source.evidence().live_bytes() > before,
                "execution owns its working storage through the visitor"
            );
            let purrdf_sparql_eval::InternedOutcome::Solutions(rows) = outcome else {
                panic!("SELECT returns rows")
            };
            assert_eq!(rows.rows().len(), 1);
        })
        .unwrap();
    let request = SparqlRequest {
        query: QUERY,
        base_iri: None,
        substitutions: &[],
    };
    let answer = engine
        .query_fallible_view(&open(&image, CEILING), request, QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(answer.result.solutions().expect("SELECT shape").1.len(), 1);
}

#[test]
/// Real segmented EXPLAIN stays within charged capacity and releases workspace between runs.
fn fallible_explain_measures_the_segmented_store_within_its_certified_reservation() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    engine.prepare_query(QUERY, None).unwrap();
    let expected = engine
        .explain_query(&resident, QUERY, None)
        .unwrap()
        .render();

    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    // Four measuring runs share the session's demand-grown request ledger.
    let source = SegmentedSession::open(
        Arc::new(image.provider()),
        image.receipt(),
        SegmentedReadLimits::new(CEILING, 2, 8192, 20_000_000, 8),
    )
    .unwrap();
    let opening = source.evidence();
    let (explanation, evidence) = engine
        .explain_query_fallible_view(&source, QUERY, None)
        .expect("the selective persistent join is certified");
    let after = source.evidence();
    let measured = window.close();
    support::assert_scan_explanation_matches_resident(&explanation, &expected);
    assert!(
        evidence.evictions() > 0,
        "the measuring run used the sparse storage cache"
    );
    let fuel: u64 = explanation
        .ledger()
        .iter()
        .map(NodeCharges::fuel_total)
        .sum();
    assert_eq!(
        fuel,
        explanation.evidence().consumed_in(ResourceDimension::Fuel)
    );
    assert!(fuel > 0);
    assert!(
        u64::try_from(measured.peak_working_bytes).unwrap() <= evidence.peak_bytes(),
        "actual {} exceeds the certified peak {}",
        measured.peak_working_bytes,
        evidence.peak_bytes()
    );
    assert!(evidence.peak_bytes() <= CEILING);
    assert!(
        after.live_bytes() - after.unpinned_cache_bytes() - after.bookkeeping_bytes()
            > opening.live_bytes() - opening.unpinned_cache_bytes() - opening.bookkeeping_bytes(),
        "returned explanation and native cache retain their original live owners"
    );
    assert!(source.read_error().is_none());

    let held = source.evidence().live_bytes();
    let shared = explanation.clone();
    drop(explanation);
    assert_eq!(source.evidence().live_bytes(), held);
    drop(shared);
    let released_receipt = source.evidence();
    let released = released_receipt.live_bytes() - released_receipt.bookkeeping_bytes();
    assert!(
        released < held - released_receipt.bookkeeping_bytes(),
        "the final explanation owner releases its buffer grant"
    );

    // Repeat on the same healthy storage session. A retained execution guard
    // would accumulate a live charge even though each measuring run has ended.
    for _ in 0..3 {
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let (again, receipt) = engine
            .explain_query_fallible_view(&source, QUERY, None)
            .unwrap();
        let live_receipt = source.evidence();
        let live = live_receipt.live_bytes() - live_receipt.bookkeeping_bytes();
        let measured = window.close();
        support::assert_scan_explanation_matches_resident(&again, &expected);
        assert!(
            live > released,
            "the repeated explanation owns its native buffers"
        );
        assert_eq!(receipt.request_count(), source.evidence().request_count());
        assert_eq!(receipt.request_digest(), source.evidence().request_digest());
        drop(again);
        let released_receipt = source.evidence();
        assert_eq!(
            released_receipt.live_bytes() - released_receipt.bookkeeping_bytes(),
            released,
            "completed execution and last explanation release their grants"
        );
        assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= receipt.peak_bytes());
        assert!(receipt.peak_bytes() <= CEILING);
    }
    drop(evidence);
    drop(engine);
    let released = source.evidence();
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
        opening.live_bytes() - opening.unpinned_cache_bytes() - opening.bookkeeping_bytes(),
        "all repeated explanation, native cache and execution owners return the original account"
    );
}

#[test]
/// A real segmented cardinality-read failure preserves its typed cause over cancellation.
fn fallible_explain_preserves_a_segmented_provider_fault_over_a_fired_stop() {
    let (image, _) = fixture();
    let provider = Arc::new(RefusingProvider {
        bytes: image.provider(),
        refuse: AtomicBool::new(false),
        operation: "explain cardinality read",
    });
    let source = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8),
    )
    .unwrap();
    let engine = NativeSparqlEngine::new();
    engine.prepare_query(QUERY, None).unwrap();
    let before = source.evidence();
    provider.refuse.store(true, Ordering::Relaxed);
    let stop = Arc::new(CancellationFlag::new());
    stop.cancel();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let error = engine
        .explain_query_with_stop_signal_fallible_view(
            &source,
            QUERY,
            None,
            QueryOptions::EMPTY,
            stop,
        )
        .expect_err("storage refusal outranks the measuring run's stop signal");
    let after = source.evidence();
    let measured = window.close();
    assert_eq!(
        error.operational_error(),
        Some(&SegmentedError::Provider {
            operation: "explain cardinality read",
            host_code: Some(5),
        })
    );
    assert!(error.diagnostic().is_none());
    assert!(error.partial_answers().is_none());
    assert!(after.request_count() > before.request_count());
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= error.evidence().peak_bytes());
    assert!(error.evidence().peak_bytes() <= CEILING);
    drop(error);
    drop(engine);
    let released = source.evidence();
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
        before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes(),
        "original provider fault retains its report only until the last report and native cache owner die"
    );
}
#[test]
fn bounded_select_algebra_matches_resident_bags_across_public_entries() {
    const CASES: &[(&str, &str, usize)] = &[
        (
            "dateTimeStamp admitted calendar casts",
            "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> SELECT ?stamp ?date WHERE { VALUES ?value { \"2026-10-04T12:00:00Z\" \"2026-10-04T12:00:00\" } BIND(xsd:dateTimeStamp(?value) AS ?stamp) BIND(xsd:date(?stamp) AS ?date) }",
            2,
        ),
        (
            "zero-column bag",
            "SELECT * WHERE { VALUES () { () () } }",
            2,
        ),
        (
            "wide schema join",
            "SELECT ?a ?b ?c ?d ?e ?f ?g ?h ?i ?j WHERE { VALUES (?a ?b ?c ?d ?e) { (1 2 3 4 5) } VALUES (?f ?g ?h ?i ?j) { (6 7 8 9 10) } }",
            1,
        ),
        (
            "wide union padding",
            "SELECT ?a ?b ?c ?d ?e ?f ?g ?h ?i ?j WHERE { { VALUES (?a ?b ?c ?d ?e ?f ?g ?h ?i ?j) { (1 2 3 4 5 6 7 8 9 10) } } UNION { VALUES ?a { 2 } } } ORDER BY ?a",
            2,
        ),
        (
            "duplicate union",
            "SELECT ?x WHERE { { VALUES ?x { 1 } } UNION { VALUES ?x { 1 } } }",
            2,
        ),
        (
            "all undef bag",
            "SELECT ?x WHERE { VALUES ?x { UNDEF UNDEF } }",
            2,
        ),
        (
            "unbound optional",
            "SELECT ?s ?x WHERE { VALUES ?s { <https://example.org/target> } OPTIONAL { ?s <https://example.org/missing> ?x } }",
            1,
        ),
        (
            "correlated optional bind",
            "SELECT ?s ?x WHERE { VALUES ?s { <https://example.org/target> } OPTIONAL { BIND(STR(?s) AS ?x) } }",
            1,
        ),
        (
            "computed bind",
            "SELECT ?x WHERE { VALUES ?s { <https://example.org/target> } BIND(CONCAT(STR(?s), '/computed') AS ?x) }",
            1,
        ),
        (
            "filter",
            "SELECT ?s WHERE { ?s <https://example.org/p> ?o FILTER(?s = <https://example.org/target>) }",
            1,
        ),
        (
            "ordered slice",
            "SELECT ?s WHERE { ?s <https://example.org/p> ?o } ORDER BY ?s OFFSET 1 LIMIT 3",
            3,
        ),
        (
            "distinct",
            "SELECT DISTINCT ?x WHERE { VALUES ?x { 1 1 2 } } ORDER BY ?x",
            2,
        ),
        (
            "reduced upper bag",
            "SELECT REDUCED ?x WHERE { VALUES ?x { 1 2 } } ORDER BY ?x",
            2,
        ),
        (
            "group",
            "SELECT ?s (COUNT(?o) AS ?n) WHERE { VALUES ?s { <https://example.org/target> } ?s <https://example.org/p> ?o } GROUP BY ?s",
            1,
        ),
        (
            "empty global aggregate",
            "SELECT (COUNT(*) AS ?n) WHERE { FILTER(false) }",
            1,
        ),
        (
            "subquery",
            "SELECT ?s WHERE { { SELECT ?s WHERE { ?s <https://example.org/p> ?o } ORDER BY ?s LIMIT 2 } } ORDER BY ?s",
            2,
        ),
        (
            "path alternative preserves bag",
            "SELECT ?o WHERE { <https://example.org/target> (<https://example.org/p>|<https://example.org/p>) ?o }",
            2,
        ),
        (
            "path reflexive reach",
            "SELECT ?o WHERE { <https://example.org/target> <https://example.org/p>* ?o } ORDER BY ?o",
            2,
        ),
        (
            "inverse path",
            "SELECT ?s WHERE { ?s ^<https://example.org/p> <https://example.org/target> }",
            1,
        ),
        (
            "absent named graph",
            "SELECT ?s WHERE { GRAPH <https://example.org/missing> { ?s ?p ?o } }",
            0,
        ),
        (
            "explicit empty default",
            "SELECT ?s FROM <https://example.org/missing> WHERE { ?s ?p ?o }",
            0,
        ),
    ];
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    for &(name, query, expected_count) in CASES {
        let prepared = engine
            .prepare_query(query, None)
            .unwrap_or_else(|error| panic!("{name}: parse: {error:?}"));
        let expected = engine
            .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
            .unwrap_or_else(|error| panic!("{name}: resident: {error:?}"));
        if name == "zero-column bag" {
            let (variables, rows) = expected.solutions().expect("SELECT");
            assert_eq!(variables, &[] as &[String]);
            assert!(rows.iter().all(Vec::is_empty));
        }
        if name == "wide schema join" || name == "wide union padding" {
            let (variables, rows) = expected.solutions().expect("SELECT");
            assert_eq!(variables.len(), 10);
            for (column, value) in rows[0].iter().enumerate() {
                let Some(TermValue::Literal { lexical_form, .. }) = value else {
                    panic!("wide first row must bind every column");
                };
                assert_eq!(lexical_form, &(column + 1).to_string());
            }
            if name == "wide union padding" {
                assert!(rows[1][1..].iter().all(Option::is_none));
            }
        }
        if name == "empty global aggregate" || name == "group" {
            let (variables, rows) = expected.solutions().expect("aggregate SELECT");
            let column = variables
                .iter()
                .position(|variable| variable == "n")
                .expect("count column");
            let Some(TermValue::Literal { lexical_form, .. }) = &rows[0][column] else {
                panic!("COUNT must yield a literal");
            };
            assert_eq!(lexical_form, if name == "group" { "1" } else { "0" });
        }
        let expected_rows = support::sorted_rows(&expected, |value| format!("{value:?}"));
        assert_eq!(
            expected_rows.len(),
            expected_count,
            "{name}: independent bag cardinality"
        );
        for route in 0..3 {
            let source = open_with_evidence(&image, CEILING, 2, 16_384);
            let result = match route {
                0 => {
                    engine
                        .query_fallible_view(
                            &source,
                            SparqlRequest {
                                query,
                                base_iri: None,
                                substitutions: &[],
                            },
                            QueryOptions::EMPTY,
                        )
                        .unwrap_or_else(|error| panic!("{name}: direct: {error:?}"))
                        .result
                }
                1 => {
                    engine
                        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                        .unwrap_or_else(|error| panic!("{name}: prepared: {error:?}"))
                        .result
                }
                _ => {
                    engine
                        .query_prepared_governed_fallible_view(
                            &source,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &QueryGovernors::METERED,
                        )
                        .unwrap_or_else(|error| panic!("{name}: governed: {error:?}"))
                        .result
                }
            };
            assert_eq!(
                support::sorted_rows(&result, |value| format!("{value:?}")),
                expected_rows,
                "{name}: route {route}: exact bag parity"
            );
            assert!(source.read_error().is_none(), "{name}: route {route}");
            assert!(
                source.evidence().peak_bytes() <= CEILING,
                "{name}: route {route}"
            );
        }
    }
}

#[test]
fn duplicate_computed_payload_releases_temporaries_and_retains_extracted_output() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    // Prepared outside the execution allocation window. The variable-dependent
    // empty substring prevents constant folding the complete CONCAT. Every one
    // of the fixture's 201 p rows computes the same 48 KiB output; DISTINCT does
    // not authorize cumulative charging of discarded duplicate payloads.
    let payload = "x".repeat(48 * 1024);
    let query = format!(
        "SELECT DISTINCT ?value WHERE {{ ?subject <https://example.org/p> ?object . \
         BIND(CONCAT(SUBSTR(STR(?object), 1, 0), \"{payload}\") AS ?value) }}"
    );
    let prepared = engine.prepare_query(&query, None).expect("computed query");
    let expected = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .expect("resident computed result");
    let (_, expected_rows) = expected.solutions().expect("SELECT");
    assert_eq!(expected_rows.len(), 1);
    assert_eq!(
        expected_rows[0][0],
        Some(TermValue::simple_literal(payload.as_str()))
    );

    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    // The 201 distinct objects force decoded-term page traffic while the output
    // remains identical. Trace capacity is independently ample; the same 8 MB
    // physical ceiling still governs every source and engine owner.
    let source = open_with_evidence(&image, CEILING, 2, 65_536);
    let answer = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .expect("live temporary ownership must fit independently of row count");
    assert_eq!(answer.result.solutions().expect("SELECT").1, expected_rows);
    let output = answer.result;
    drop(answer.evidence);
    let retained = source.evidence().live_bytes();
    let shared = output.clone();
    assert_eq!(source.evidence().live_bytes(), retained, "shallow clone");
    drop(output);
    assert_eq!(
        source.evidence().live_bytes(),
        retained,
        "last owner remains"
    );
    let extracted = shared.into_solutions().expect("SELECT extraction");
    assert_eq!(extracted.as_parts().1, expected_rows);
    assert_eq!(
        source.evidence().live_bytes(),
        retained,
        "extraction keeps lease"
    );
    drop(extracted);
    assert!(
        source.evidence().live_bytes() < retained,
        "final output releases admission"
    );
    let peak = source.evidence().peak_bytes();
    let measured = window.close();
    assert!(peak <= CEILING);
    assert!(
        u64::try_from(measured.peak_working_bytes).expect("measured peak fits") <= peak,
        "computed output actual allocation {measured:?}, source peak {peak}"
    );
    assert!(source.read_error().is_none());
}

#[test]
fn bounded_ask_construct_describe_match_resident_across_public_entries() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    for (query, expected_size) in [
        (
            "ASK { <https://example.org/target> <https://example.org/p> ?o }",
            1,
        ),
        (
            "ASK { <https://example.org/target> <https://example.org/missing> ?o }",
            0,
        ),
        (
            "CONSTRUCT { <https://example.org/target> <https://example.org/p> ?o } WHERE { <https://example.org/target> <https://example.org/p> ?o }",
            1,
        ),
        ("DESCRIBE <https://example.org/target>", 2),
    ] {
        let prepared = engine.prepare_query(query, None).expect("graph-form query");
        let expected = engine
            .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
            .expect("resident graph-form result");
        for route in 0..3 {
            let source = open_with_evidence(&image, CEILING, 2, 16_384);
            let found = match route {
                0 => {
                    engine
                        .query_fallible_view(
                            &source,
                            SparqlRequest {
                                query,
                                base_iri: None,
                                substitutions: &[],
                            },
                            QueryOptions::EMPTY,
                        )
                        .expect("direct bounded form")
                        .result
                }
                1 => {
                    engine
                        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                        .expect("prepared bounded form")
                        .result
                }
                _ => {
                    engine
                        .query_prepared_governed_fallible_view(
                            &source,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &QueryGovernors::METERED,
                        )
                        .expect("governed bounded form")
                        .result
                }
            };
            match &expected {
                SparqlResult::Boolean(answer) => {
                    assert_eq!(usize::from(*answer), expected_size);
                    assert_eq!(found.boolean(), Some(*answer), "route {route}");
                }
                SparqlResult::Graph(expected_graph) => {
                    assert_eq!(expected_graph.quad_count(), expected_size);
                    let found_graph = found.graph().expect("retained graph");
                    let render = |graph: &purrdf_core::RdfDataset| {
                        let mut rows: Vec<_> = graph
                            .owned_quads()
                            .map(|quad| format!("{quad:?}"))
                            .collect();
                        rows.sort_unstable();
                        rows
                    };
                    assert_eq!(render(found_graph), render(expected_graph), "route {route}");
                }
                SparqlResult::Solutions { .. } => panic!("expected graph or boolean"),
            }
            assert!(source.read_error().is_none());
            assert!(source.evidence().peak_bytes() <= CEILING);
        }
    }
}

fn rdf12_graph_fixture() -> (purrdf_core::SegmentedImage, Arc<purrdf_core::RdfDataset>) {
    use purrdf_core::{
        QuadIds, RdfDatasetBuilder, SegmentedBuildLimits, SegmentedBuilder, TermBox,
    };
    let limits = SegmentedBuildLimits::new(4096, 500_000, 4096, 512, 4)
        .expect("limits")
        .with_first_term_index((1_u64 << 53) + 1)
        .expect("wide ids");
    let mut persistent = SegmentedBuilder::new(limits).unwrap();
    let mut resident = RdfDatasetBuilder::new();
    let values = [
        TermValue::iri("https://example.org/target"),
        TermValue::iri("https://example.org/p"),
        TermValue::simple_literal("root"),
        TermValue::iri("https://example.org/g"),
        TermValue::lang_literal("named", "en"),
        TermValue::iri("https://example.org/empty"),
        TermValue::iri("https://example.org/claim"),
        TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/target")),
            p: TermBox::new(TermValue::iri("https://example.org/q")),
            o: TermBox::new(TermValue::simple_literal("v")),
        },
        TermValue::iri("https://example.org/flag"),
        TermValue::simple_literal("yes"),
    ];
    let mut global = Vec::new();
    persistent
        .intern_batch(&values, |_, id| global.push(id))
        .expect("terms");
    let native: Vec<_> = values
        .iter()
        .map(|value| {
            purrdf_core::term_fixture::intern_value(&mut resident, value).expect("authored fixture")
        })
        .collect();
    for (s, p, o, g) in [(0, 1, 2, None), (0, 1, 4, Some(3))] {
        persistent
            .push_quad(QuadIds {
                s: global[s],
                p: global[p],
                o: global[o],
                g: g.map(|graph| global[graph]),
            })
            .expect("ordinary row");
        resident
            .push_quad(
                native[s],
                native[p],
                native[o],
                g.map(|graph| native[graph]),
            )
            .unwrap();
    }
    persistent
        .push_reifier(global[6], global[7], None)
        .expect("reifier stream");
    resident
        .push_reifier_in_graph(native[6], native[7], None)
        .unwrap();
    persistent
        .push_annotation(QuadIds {
            s: global[6],
            p: global[8],
            o: global[9],
            g: None,
        })
        .expect("annotation stream");
    resident
        .push_annotation_in_graph(native[6], native[8], native[9], None)
        .unwrap();
    persistent
        .declare_named_graph(global[5])
        .expect("empty graph");
    resident.declare_named_graph(native[5]).unwrap();
    (
        persistent.seal().expect("persisted fixture"),
        resident.freeze().expect("resident fixture"),
    )
}

#[test]
fn bounded_virtual_streams_and_empty_graph_declarations_match_resident() {
    let (image, resident) = rdf12_graph_fixture();
    let reifies = purrdf_iri::vocab::rdf::REIFIES;
    let triple_query = format!(
        "SELECT ?claim WHERE {{ ?claim <{reifies}> <<( <https://example.org/target> <https://example.org/q> \"v\" )>> }}"
    );
    let cases = [
        ("SELECT ?s ?p ?o WHERE { ?s ?p ?o }", 3),
        (
            "SELECT ?claim ?flag WHERE { ?claim <https://example.org/flag> ?flag }",
            1,
        ),
        (triple_query.as_str(), 1),
        ("SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g", 2),
        ("SELECT * WHERE { GRAPH <https://example.org/empty> {} }", 1),
        (
            "SELECT * WHERE { GRAPH <https://example.org/missing> {} }",
            0,
        ),
        (
            "SELECT ?s ?o WHERE { GRAPH <https://example.org/g> { ?s <https://example.org/p> ?o } }",
            1,
        ),
        (
            "SELECT ?s ?o FROM <https://example.org/g> WHERE { ?s <https://example.org/p> ?o }",
            1,
        ),
        (
            "SELECT ?s FROM NAMED <https://example.org/g> WHERE { ?s ?p ?o }",
            0,
        ),
        (
            "SELECT ?g FROM NAMED <https://example.org/empty> WHERE { GRAPH ?g {} }",
            1,
        ),
    ];
    let engine = NativeSparqlEngine::new();
    for (query, count) in cases {
        let prepared = engine
            .prepare_query(query, None)
            .expect("RDF 1.2 graph query");
        let expected = engine
            .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
            .expect("resident graph scope");
        let expected = support::sorted_rows(&expected, |value| format!("{value:?}"));
        assert_eq!(
            expected.len(),
            count,
            "independent graph/stream cardinality: {query}"
        );
        for route in 0..3 {
            let source = open_with_evidence(&image, CEILING, 2, 16_384);
            let found = match route {
                0 => {
                    engine
                        .query_fallible_view(
                            &source,
                            SparqlRequest {
                                query,
                                base_iri: None,
                                substitutions: &[],
                            },
                            QueryOptions::EMPTY,
                        )
                        .expect("direct graph scope")
                        .result
                }
                1 => {
                    engine
                        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                        .expect("prepared graph scope")
                        .result
                }
                _ => {
                    engine
                        .query_prepared_governed_fallible_view(
                            &source,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &QueryGovernors::METERED,
                        )
                        .expect("governed graph scope")
                        .result
                }
            };
            assert_eq!(
                support::sorted_rows(&found, |value| format!("{value:?}")),
                expected,
                "route {route}: {query}"
            );
            assert!(source.read_error().is_none());
            assert!(source.evidence().peak_bytes() <= CEILING);
        }
    }
}

/// LIMIT over real persisted reifiers must stop the source traversal, not merely
/// discard already translated rows in the consumer callback.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn persisted_reifier_limit_bounds_touched_terms_and_full_count_matches() {
    use purrdf_core::{
        PackBuilder, RdfDatasetBuilder, SegmentedBuildLimits, SegmentedFileProvider,
        SegmentedImage, SegmentedLayer, SegmentedLayerMetadata, SegmentedStack, TermBox,
    };
    for claims in [2000, 200_000] {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = TermValue::iri("https://example.org/claim-predicate");
        let annotation = builder
            .intern_value(&TermValue::iri("https://example.org/annotation"))
            .unwrap();
        for index in 0..claims {
            let reifier = builder
                .intern_value(&TermValue::iri(format!(
                    "https://example.org/claim/{index}"
                )))
                .unwrap();
            let object = TermValue::simple_literal(format!("value {}", index % 256));
            let triple = builder
                .intern_value(&TermValue::Triple {
                    s: TermBox::new(TermValue::iri(format!(
                        "https://example.org/subject/{index}"
                    ))),
                    p: TermBox::new(predicate.clone()),
                    o: TermBox::new(object.clone()),
                })
                .unwrap();
            let object = builder.intern_value(&object).unwrap();
            builder
                .push_reifier_in_graph(reifier, triple, None)
                .unwrap();
            builder
                .push_annotation_in_graph(reifier, annotation, object, None)
                .unwrap();
        }
        let resident = builder.freeze().unwrap();
        let packed = PackBuilder::build_view_bytes(resident.as_ref()).unwrap();
        for (block_bytes, records_per_block) in [(4096, 32), (4096, 64), (4096, 128), (8192, 128)] {
            let built = SegmentedImage::from_pack_v1(
                &packed,
                SegmentedBuildLimits::new(
                    u32::MAX - 1,
                    u32::MAX,
                    u32::MAX,
                    block_bytes,
                    records_per_block,
                )
                .unwrap(),
            );
            let image = built.unwrap();
            eprintln!(
                "claims={claims} block_bytes={block_bytes} records_per_block={records_per_block} terms={} pack_bytes={} image_bytes={}",
                resident.term_count(),
                packed.len(),
                image.bytes().len()
            );
            let mut kinds = [0_u64; 256];
            let authenticated_bytes = image.receipt().block_count() * u64::from(block_bytes);
            for block in image.bytes()[..usize::try_from(authenticated_bytes).unwrap()]
                .chunks_exact(usize::try_from(block_bytes).unwrap())
                .skip(1)
            {
                kinds[usize::from(block[0])] += u64::from(block_bytes);
            }
            eprintln!(
                "claims={claims} block_bytes={block_bytes} records_per_block={records_per_block} block_kind_bytes={:?} authentication_bytes={}",
                &kinds[..6],
                image.receipt().byte_len() - authenticated_bytes
            );
            let directory = purrdf_testkit::temp_dir!("persisted-reifier-limit-").unwrap();
            let path = directory.path().join("claims.seg");
            std::fs::write(&path, image.bytes()).unwrap();
            let provider = SegmentedFileProvider::from_file(
                std::fs::File::open(&path).unwrap(),
                image.receipt().snapshot(),
            )
            .unwrap();
            let read_limits =
                SegmentedReadLimits::new(256 * 1024 * 1024, 32, u32::MAX, u64::MAX, 32);
            let mut stack = SegmentedStack::default();
            stack
                .publish(
                    vec![Arc::new(
                        SegmentedLayer::new(
                            Arc::new(provider),
                            image.receipt().clone(),
                            SegmentedLayerMetadata::default(),
                        )
                        .unwrap(),
                    )],
                    read_limits,
                )
                .unwrap();
            drop(image);
            let snapshot = stack.snapshot().unwrap();
            let engine = NativeSparqlEngine::new();
            let text = format!(
                "SELECT ?r WHERE {{ ?r <{}> ?t }} LIMIT 10",
                purrdf_iri::vocab::rdf::REIFIES
            );
            let source = snapshot.query(read_limits).unwrap();
            let limited = engine
                .query_fallible_view(
                    &source,
                    SparqlRequest {
                        query: &text,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                )
                .unwrap();
            assert_eq!(support::solutions(limited.result).1.len(), 10);
            assert!(
                source.touched_terms() <= 64,
                "claims={claims}: touched {} terms",
                source.touched_terms()
            );
            eprintln!(
                "claims={claims} limit_touched={} requests={} io_bytes={} peak_bytes={}",
                source.touched_terms(),
                limited.evidence.request_count(),
                limited.evidence.io_bytes(),
                limited.evidence.peak_bytes()
            );
            source.check_ready().unwrap();
            drop(source);
            // Both side tables must use the subject-keyed logical path. Check
            // existing and absent keys against independent counts and the
            // resident evaluator, including the final claim in the full image.
            for (claim, rows) in [(claims / 2, 2), (claims - 1, 2), (claims, 0)] {
                let text = format!(
                    "SELECT (COUNT(*) AS ?n) WHERE {{ <https://example.org/claim/{claim}> ?p ?o }}"
                );
                let prepared = engine.prepare_query(&text, None).unwrap();
                let expected = support::solutions(
                    engine
                        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
                        .unwrap(),
                );
                assert_eq!(
                    expected.1,
                    vec![vec![Some(TermValue::typed_literal(
                        rows.to_string(),
                        purrdf_xsd::XsdDatatype::Integer.iri()
                    ))]]
                );
                let source = snapshot
                    .query(SegmentedReadLimits::new(
                        256 * 1024 * 1024,
                        4096,
                        u32::MAX,
                        u64::MAX,
                        32,
                    ))
                    .unwrap();
                let counted = engine
                    .query_fallible_view(
                        &source,
                        SparqlRequest {
                            query: &text,
                            base_iri: None,
                            substitutions: &[],
                        },
                        QueryOptions::EMPTY,
                    )
                    .unwrap();
                assert_eq!(support::solutions(counted.result), expected);
                assert!(source.touched_terms() <= 64);
                assert!(
                    counted.evidence.io_bytes() < 1024 * 1024,
                    "claims={claims} key={claim}: {} bytes",
                    counted.evidence.io_bytes()
                );
                source.check_ready().unwrap();
            }
            let text = "SELECT (COUNT(*) AS ?n) WHERE { ?s ?p ?o }";
            let prepared = engine.prepare_query(text, None).unwrap();
            let expected = support::solutions(
                engine
                    .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
                    .unwrap(),
            );
            assert_eq!(
                expected.1,
                vec![vec![Some(TermValue::typed_literal(
                    (2 * claims).to_string(),
                    purrdf_xsd::XsdDatatype::Integer.iri()
                ))]]
            );
            let source = snapshot.query(read_limits).unwrap();
            let counted = engine
                .query_fallible_view(
                    &source,
                    SparqlRequest {
                        query: text,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                )
                .unwrap();
            assert_eq!(support::solutions(counted.result), expected);
            source.check_ready().unwrap();
            eprintln!(
                "claims={claims} count_touched={} requests={} io_bytes={} peak_bytes={}",
                source.touched_terms(),
                counted.evidence.request_count(),
                counted.evidence.io_bytes(),
                counted.evidence.peak_bytes()
            );
        }
        drop(packed);
    }
}

/// ORDER BY has the resident bag; physical refusal remains independently typed.
#[test]
fn ordered_query_matches_resident_and_tiny_capacity_refuses() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    let ordered = format!("{QUERY} ORDER BY ?label");
    for query in [QUERY, ordered.as_str()] {
        let prepared = engine.prepare_query(query, None).unwrap();
        let expected = engine
            .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
            .unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let source = open(&image, CEILING);
        let answer = engine
            .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
            .unwrap();
        let measured = window.close();
        assert_eq!(
            answer.result.solutions().unwrap(),
            expected.solutions().unwrap()
        );
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= answer.evidence.peak_bytes()
        );
        assert!(answer.evidence.peak_bytes() <= CEILING);
        assert!(source.read_error().is_none());
        let small = open(&image, baseline_ceiling(&image));
        let before = small.evidence();
        let refused =
            engine.query_prepared_fallible_view(&small, &prepared, &[], QueryOptions::EMPTY);
        assert!(matches!(
            refused,
            Err(FallibleSparqlError::Operational {
                error: SegmentedError::Residency { .. },
                ..
            })
        ));
        assert_eq!(small.evidence().request_count(), before.request_count());
        assert_eq!(
            small.evidence().live_bytes()
                - small.evidence().unpinned_cache_bytes()
                - small.evidence().bookkeeping_bytes(),
            before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes()
        );
    }
}

/// Original governor controls are admitted before either text or prepared work.
#[test]
fn raw_evaluator_cannot_bypass_a_bounded_source_ownership_capability() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let source = open(&image, CEILING);
    let before = source.evidence().request_count();
    let mut ctx = purrdf_sparql_eval::EvalCtx::new(&source);
    let error = purrdf_sparql_eval::evaluate_query(prepared.query(), &mut ctx).unwrap_err();
    assert!(matches!(
        error,
        purrdf_sparql_eval::EvalError::WorkspaceUnpriced(_)
    ));
    assert_eq!(source.evidence().request_count(), before);
}

/// Original governor controls are admitted before either text or prepared work.
#[test]
fn governed_reporting_refuses_independently_tight_capacity() {
    let (image, _) = fixture();
    let opening = baseline_ceiling(&image);
    let ordered = format!("{QUERY} ORDER BY ?label");
    for query in [QUERY, ordered.as_str()] {
        for headroom in [0, 1] {
            for route in 0..3 {
                let engine = NativeSparqlEngine::new();
                let prepared = engine.prepare_query(query, None).unwrap();
                let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
                let entry = match route {
                    0 => GovernedEntry::Text(query),
                    1 => GovernedEntry::Prepared,
                    _ => GovernedEntry::Operation(&state),
                };
                let ceiling = opening + headroom;
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let source = open(&image, ceiling);
                let before = source.evidence();
                let outcome = entry.run(
                    &engine,
                    &source,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                );
                let measured = window.close();
                assert!(matches!(
                    outcome,
                    Err(FallibleSparqlError::Operational {
                        error: SegmentedError::Residency { .. },
                        ..
                    })
                ));
                assert!(source.read_error().is_none());
                assert_eq!(source.evidence().request_count(), before.request_count());
                drop(outcome);
                drop(engine);
                assert_eq!(
                    source.evidence().live_bytes()
                        - source.evidence().unpinned_cache_bytes()
                        - source.evidence().bookkeeping_bytes(),
                    before.live_bytes()
                        - before.unpinned_cache_bytes()
                        - before.bookkeeping_bytes(),
                    "failed invocation releases original controls and preparation arrays"
                );
                assert!(
                    u64::try_from(measured.peak_working_bytes).unwrap()
                        <= source.evidence().peak_bytes()
                );
                assert!(source.evidence().peak_bytes() <= ceiling);
            }
        }
    }
}

/// Construction uses the admitted caller prefix and preserves destination rows.
#[test]
fn construct_mint_prefix_completes_and_tight_refusal_preserves_destination() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let query = "CONSTRUCT { _:fresh <http://example.org/p> ?o } WHERE { <https://example.org/target> <https://example.org/p> ?o }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let prefix = "x".repeat(100_000);
    let options = QueryOptions::EMPTY.with_bnode_mint_prefix(Some(&prefix));
    for tight in [false, true] {
        let ceiling = if tight {
            baseline_ceiling(&image)
        } else {
            CEILING
        };
        let source = open(&image, ceiling);
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let mut destination = purrdf_core::RdfDatasetBuilder::new();
        let blank = destination
            .intern_blank("existing", purrdf_core::BlankScope::DEFAULT)
            .unwrap();
        let predicate = destination.intern_iri("http://example.org/p").unwrap();
        let object = destination.intern_iri("http://example.org/o").unwrap();
        destination
            .push_quad(blank, predicate, object, None)
            .unwrap();
        let before = source.evidence();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = engine.construct_prepared_fallible_in_operation_into_view(
            &source,
            &prepared,
            &[],
            options,
            &state,
            &mut destination,
        );
        let measured = window.close();
        if tight {
            assert!(matches!(
                result,
                Err(FallibleSparqlError::Operational {
                    error: SegmentedError::Residency { .. },
                    ..
                })
            ));
            assert_eq!(source.evidence().request_count(), before.request_count());
            assert_eq!(destination.freeze().unwrap().quad_count(), 1);
        } else {
            result.unwrap();
            let graph = destination.freeze().unwrap();
            assert_eq!(graph.quad_count(), 2);
            assert!(graph.owned_quads().any(|quad| matches!(quad.subject, purrdf_core::RdfTerm::BlankNode(label) if label.starts_with(&prefix))));
            assert!(source.read_error().is_none());
        }
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes()
        );
        assert!(source.evidence().peak_bytes() <= ceiling);
    }
}

/// Spilled caller parameter metadata and a long prefix use the original owners.
#[test]
fn request_metadata_and_prefix_match_resident_and_refuse_tight_capacity() {
    let (image, resident) = fixture();
    let long_prefix = "x".repeat(100_000);
    let substitutions: Vec<_> = (0..128)
        .map(|i| {
            (
                format!("parameter{i}"),
                TermValue::iri("http://example.org/o"),
            )
        })
        .collect();
    for (values, options) in [
        (substitutions.as_slice(), QueryOptions::EMPTY),
        (
            &[][..],
            QueryOptions::EMPTY.with_bnode_mint_prefix(Some(&long_prefix)),
        ),
    ] {
        let expected_engine = NativeSparqlEngine::new();
        let expected = expected_engine
            .query_with_options_view(
                resident.as_ref(),
                SparqlRequest {
                    query: QUERY,
                    base_iri: None,
                    substitutions: values,
                },
                options,
            )
            .unwrap();
        for route in 0..3 {
            let engine = NativeSparqlEngine::new();
            let prepared = engine.prepare_query(QUERY, None).unwrap();
            let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
            let entry = match route {
                0 => GovernedEntry::Text(QUERY),
                1 => GovernedEntry::Prepared,
                _ => GovernedEntry::Operation(&state),
            };
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let source = open_with_evidence(&image, CEILING, 2, 16_384);
            let answer = entry
                .run(
                    &engine,
                    &source,
                    &prepared,
                    values,
                    options,
                    &QueryGovernors::METERED,
                )
                .unwrap();
            let measured = window.close();
            assert_eq!(
                answer.result.solutions().unwrap(),
                expected.solutions().unwrap()
            );
            assert!(
                u64::try_from(measured.peak_working_bytes).unwrap()
                    <= answer.evidence.view.peak_bytes()
            );
            assert!(answer.evidence.view.peak_bytes() <= CEILING);
            assert!(source.read_error().is_none());
            drop(answer);
            drop(engine);
            let tight_engine = NativeSparqlEngine::new();
            let small = open(&image, baseline_ceiling(&image));
            let before = small.evidence();
            let outcome = tight_engine.query_fallible_view(
                &small,
                SparqlRequest {
                    query: QUERY,
                    base_iri: None,
                    substitutions: values,
                },
                options,
            );
            assert!(matches!(
                outcome,
                Err(FallibleSparqlError::Operational {
                    error: SegmentedError::Residency { .. },
                    ..
                })
            ));
            drop(outcome);
            drop(tight_engine);
            assert_eq!(small.evidence().request_count(), before.request_count());
            assert_eq!(
                small.evidence().live_bytes()
                    - small.evidence().unpinned_cache_bytes()
                    - small.evidence().bookkeeping_bytes(),
                before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes()
            );
        }
    }
    // A separately large caller-owned input must be physically refused without
    // degrading supported substitution semantics to an unsupported-workspace error.
    let large: Vec<_> = (0..20_000)
        .map(|i| {
            (
                format!("parameter{i}"),
                TermValue::iri("http://example.org/o"),
            )
        })
        .collect();
    let source = open(&image, baseline_ceiling(&image));
    let engine = NativeSparqlEngine::new();
    let before = source.evidence();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = engine.query_fallible_view(
        &source,
        SparqlRequest {
            query: QUERY,
            base_iri: None,
            substitutions: &large,
        },
        QueryOptions::EMPTY,
    );
    let measured = window.close();
    assert!(matches!(
        result,
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Residency { .. },
            ..
        })
    ));
    drop(result);
    drop(engine);
    assert_eq!(source.evidence().request_count(), before.request_count());
    assert_eq!(
        source.evidence().live_bytes()
            - source.evidence().unpinned_cache_bytes()
            - source.evidence().bookkeeping_bytes(),
        before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes()
    );
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
}

/// All query algebra is admitted by physical capacity rather than shape refusal.
#[test]
fn explain_tight_capacity_releases_native_preparation_and_reporting() {
    let (image, _) = fixture();
    let opening = baseline_ceiling(&image);
    let ordered = format!("{QUERY} ORDER BY ?label");
    for query in [QUERY, ordered.as_str()] {
        for headroom in [0, 1] {
            let engine = NativeSparqlEngine::new();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let source = open(&image, opening + headroom);
            let before = source.evidence();
            let error = engine
                .explain_query_fallible_view(&source, query, None)
                .unwrap_err();
            let measured = window.close();
            assert!(matches!(
                error,
                FallibleSparqlError::Operational {
                    error: SegmentedError::Residency { .. },
                    ..
                }
            ));
            assert!(error.partial_answers().is_none());
            assert_eq!(source.evidence().request_count(), before.request_count());
            assert!(
                u64::try_from(measured.peak_working_bytes).unwrap()
                    <= error.evidence().peak_bytes()
            );
            assert!(error.evidence().peak_bytes() <= opening + headroom);
            drop(error);
            drop(engine);
            assert_eq!(
                source.evidence().live_bytes()
                    - source.evidence().unpinned_cache_bytes()
                    - source.evidence().bookkeeping_bytes(),
                before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes()
            );
        }
    }
}

/// Configured EXPLAIN has resident evidence, owned output and bounded allocation.
#[test]
fn configured_explain_matches_resident_and_retains_last_owner() {
    let (image, resident) = fixture();
    let prefix = "x".repeat(100_000);
    let ordered = format!("{QUERY} ORDER BY ?label");
    for query in [QUERY, ordered.as_str()] {
        for options in [
            QueryOptions::EMPTY.with_bnode_mint_prefix(Some(&prefix)),
            QueryOptions::EMPTY.with_call_depth(1),
        ] {
            for signalled in [false, true] {
                let expected_engine = NativeSparqlEngine::new();
                let expected = expected_engine
                    .explain_query_with_options(&resident, query, None, options)
                    .unwrap()
                    .render();
                let engine = NativeSparqlEngine::new();
                let source = open_with_evidence(&image, CEILING, 2, 16_384);
                let stop = Arc::new(CancellationFlag::new());
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let (explanation, receipt) = if signalled {
                    engine.explain_query_with_stop_signal_fallible_view(
                        &source, query, None, options, stop,
                    )
                } else {
                    engine.explain_query_with_options_fallible_view(&source, query, None, options)
                }
                .unwrap();
                let measured = window.close();
                support::assert_scan_explanation_matches_resident(&explanation, &expected);
                assert!(
                    u64::try_from(measured.peak_working_bytes).unwrap() <= receipt.peak_bytes()
                );
                assert!(receipt.peak_bytes() <= CEILING);
                assert!(source.read_error().is_none());
                drop(engine);
                let live = source.evidence().live_bytes();
                let shared = explanation.clone();
                assert_eq!(source.evidence().live_bytes(), live);
                drop(explanation);
                assert_eq!(
                    source.evidence().live_bytes(),
                    live,
                    "shared explanation owns original buffers"
                );
                support::assert_scan_explanation_matches_resident(&shared, &expected);
                drop(shared);
                assert!(
                    source.evidence().live_bytes() < live,
                    "last explanation owner releases original admission"
                );
                drop(receipt);
            }
        }
    }
}

/// Cold text/prepared/operation routes share answers and governor charges, while
/// their original controls and output leases retain their actual owner lifetimes.
#[test]
fn prepared_governed_cold_sessions_match_text_receipts_and_release_reporting() {
    let (image, _) = fixture();
    let scoped_source = open(&image, CEILING);
    let before = scoped_source.evidence();
    let before_owners =
        before.live_bytes() - before.unpinned_cache_bytes() - before.bookkeeping_bytes();
    let ((), scoped_evidence) = {
        let engine = NativeSparqlEngine::new();
        let mut execution = engine
            .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
            .unwrap();
        engine
            .execute_fallible(
                &mut execution,
                &scoped_source,
                QueryOptions::EMPTY,
                |outcome| {
                    let active = scoped_source.evidence();
                    assert!(
                        active.live_bytes()
                            - active.unpinned_cache_bytes()
                            - active.bookkeeping_bytes()
                            > before_owners
                    );
                    let purrdf_sparql_eval::InternedOutcome::Solutions(rows) = outcome else {
                        panic!("SELECT")
                    };
                    assert_eq!(rows.rows().len(), 1);
                },
            )
            .unwrap()
    };
    let released = scoped_source.evidence();
    assert_eq!(
        scoped_evidence.request_count(),
        released.request_count(),
        "scoped publication retains the exact final read prefix"
    );
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
        before_owners,
        "scoped execution and its distinct native cache owners release every grant"
    );
    let resident_engine = NativeSparqlEngine::new();
    let prepared = resident_engine.prepare_query(QUERY, None).unwrap();
    for governors in [
        QueryGovernors::METERED,
        QueryGovernors::METERED.with_max_answers(0),
    ] {
        let mut expected = None;
        for route in 0..3 {
            let engine = NativeSparqlEngine::new();
            let state = Arc::new(GovernorState::new(&governors));
            let entry = match route {
                0 => GovernedEntry::Text(QUERY),
                1 => GovernedEntry::Prepared,
                _ => GovernedEntry::Operation(&state),
            };
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let source = open(&image, CEILING);
            let outcome = entry.run(
                &engine,
                &source,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &governors,
            );
            let measured = window.close();
            let (rows, evidence) = match &outcome {
                Ok(answer) => {
                    let rows = answer.result.solutions().expect("SELECT").1;
                    assert_eq!(rows.len(), 1);
                    (rows.to_vec(), (*answer.evidence).clone())
                }
                Err(FallibleSparqlError::BudgetExhausted {
                    partial, evidence, ..
                }) => {
                    let rows = partial
                        .result()
                        .unwrap()
                        .result()
                        .solutions()
                        .expect("SELECT")
                        .1;
                    assert_eq!(rows, &[] as &[Vec<Option<TermValue>>]);
                    (rows.to_vec(), evidence.clone())
                }
                error => panic!("healthy query completes or exhausts answers: {error:?}"),
            };
            assert!(
                u64::try_from(measured.peak_working_bytes).unwrap() <= evidence.view.peak_bytes()
            );
            assert!(evidence.view.peak_bytes() <= CEILING);
            assert!(source.read_error().is_none());
            if let Some((expected_rows, expected_evidence)) = &expected {
                assert_eq!(&rows, expected_rows);
                let expected_evidence: &GovernedEvidence<SegmentedEvidence> = expected_evidence;
                assert_eq!(evidence.governors, expected_evidence.governors);
                assert_eq!(
                    evidence.view.request_count(),
                    expected_evidence.view.request_count()
                );
            } else {
                expected = Some((rows, evidence));
            }
            // Remove only the cache's plan owners, leaving published owners alive.
            drop(engine);
            let held = source.evidence().live_bytes();
            let shared = outcome.clone();
            assert_eq!(
                source.evidence().live_bytes(),
                held,
                "publication clone shares owners"
            );
            drop(outcome);
            assert_eq!(
                source.evidence().live_bytes(),
                held,
                "the final publication clone remains live"
            );
            drop(shared);
            assert!(
                source.evidence().live_bytes() < held,
                "last publication owner releases its output and reporting controls"
            );
        }
    }
}

/// A borrowed graph can leave the scoped visitor only with its original lease.
#[test]
fn interned_graph_retention_keeps_original_account_until_last_extraction() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let query = "CONSTRUCT { <https://example.org/target> <https://example.org/p> ?o } WHERE { <https://example.org/target> <https://example.org/p> ?o }";
    let mut execution = engine
        .prepare_execution(query, None, &[], QueryOptions::EMPTY)
        .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let source = open(&image, CEILING);
    let opening = source.evidence();
    let opening_owners =
        opening.live_bytes() - opening.unpinned_cache_bytes() - opening.bookkeeping_bytes();
    let (graph, evidence) = engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |outcome| {
            let purrdf_sparql_eval::InternedOutcome::Graph(graph) = outcome else {
                panic!("CONSTRUCT returns graph")
            };
            graph
                .retain()
                .map(|retained| (retained, graph.dataset_handle()))
        })
        .unwrap();
    let (graph, direct_handle) = graph.unwrap();
    let measured = window.close();
    assert_eq!(graph.quad_count(), 1);
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= evidence.peak_bytes());
    assert!(evidence.peak_bytes() <= CEILING);
    drop(execution);
    drop(engine);
    let retained = source.evidence().live_bytes();
    assert!(
        retained - source.evidence().unpinned_cache_bytes() - source.evidence().bookkeeping_bytes()
            > opening_owners,
        "graph owns its original grant after the distinct execution and engine owners are destroyed"
    );
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let shared = graph.clone();
    let extracted_handle = shared.dataset_handle();
    let clones = window.close();
    assert_eq!(
        clones.peak_working_bytes, 0,
        "graph clone shares payload and admission"
    );
    drop(graph);
    assert_eq!(source.evidence().live_bytes(), retained);
    let result = shared.into_result();
    assert_eq!(result.graph().unwrap().quad_count(), 1);
    assert_eq!(
        source.evidence().live_bytes(),
        retained,
        "common result extraction keeps the original grant"
    );
    drop(result);
    assert!(
        source.evidence().live_bytes()
            - source.evidence().unpinned_cache_bytes()
            - source.evidence().bookkeeping_bytes()
            > opening_owners,
        "dataset handles retain the graph's original physical account after wrapper extraction"
    );
    assert_eq!(direct_handle.quad_count(), 1);
    assert!(std::ptr::eq(
        direct_handle.as_ref(),
        extracted_handle.as_ref()
    ));
    drop(direct_handle);
    drop(extracted_handle);
    let released = source.evidence();
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes() - released.bookkeeping_bytes(),
        opening_owners,
        "last graph owner releases every graph grant back to the original account"
    );
    assert!(source.read_error().is_none());
}

#[test]
fn authenticated_annotation_ranges_reduce_cold_reads_and_keep_exact_filter() {
    use purrdf_core::{PackBuilder, RdfDatasetBuilder, SegmentedBuildLimits, SegmentedImage};
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("urn:range:value").unwrap();
    let relation = builder.intern_iri("urn:range:claim").unwrap();
    let object = builder.intern_iri("urn:range:object").unwrap();
    for index in 0..1024 {
        let claim = builder
            .intern_iri(&format!("urn:range:{index:05}"))
            .unwrap();
        let target = builder.intern_triple(claim, relation, object).unwrap();
        builder.push_reifier(claim, target).unwrap();
        let lexical = if index == 100 {
            "ill-typed".to_owned()
        } else {
            index.to_string()
        };
        let value = builder
            .intern_value(&TermValue::typed_literal(
                lexical,
                purrdf_xsd::datatype::XSD_INTEGER,
            ))
            .unwrap();
        builder.push_annotation(claim, predicate, value).unwrap();
    }
    let resident = builder.freeze().unwrap();
    let image = SegmentedImage::from_pack_v1(
        &PackBuilder::build_bytes(&resident).unwrap(),
        SegmentedBuildLimits::new(8192, 2_000_000, 8192, 1024, 16).unwrap(),
    )
    .unwrap();
    let engine = NativeSparqlEngine::new();
    let run = |value: &str| {
        let query = format!(
            "SELECT ?claim ?value WHERE {{ ?claim <urn:range:value> ?value FILTER({value} >= 512 && {value} < 522) }} ORDER BY ?claim"
        );
        let source = SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(64_000_000, 64, 200_000, 1_000_000_000, 8),
        )
        .unwrap();
        let before = source.evidence();
        assert!(source.snapshot_identity().unwrap().is_some());
        assert_eq!(
            source.evidence().request_count(),
            before.request_count(),
            "snapshot identity is a certified descriptor lookup"
        );
        let result = engine
            .query_fallible_view(
                &source,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .unwrap();
        (support::solutions(result.result), source.evidence())
    };
    let (baseline, baseline_reads) = run("(?value + 0)");
    let (pruned, pruned_reads) = run("?value");
    assert_eq!(pruned, baseline);
    assert_eq!(pruned.1.len(), 10);
    assert!(
        pruned_reads.io_bytes() < baseline_reads.io_bytes(),
        "authenticated ranges must save actual cold I/O: {} versus {}",
        pruned_reads.io_bytes(),
        baseline_reads.io_bytes()
    );
}
