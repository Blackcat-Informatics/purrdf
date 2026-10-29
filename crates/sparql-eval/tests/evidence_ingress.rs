// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original engine claim ingress and alternative-witness coalescing on native/WASM.

mod support;

use purrdf_core::evidence::{
    EvidenceRecord, Frame, Information, InformationMap, ReasonDescription, ReasonId,
    SearchCompleteness,
};
use purrdf_core::{GraphMatch, QuadIds, RdfDatasetBuilder, TermId};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_sparql_eval::evidence_exec::{EvidenceScope, EvidenceSource};
use purrdf_sparql_eval::{
    EvalError, InternedOutcome, InternedRequest, NativeSparqlEngine, QueryOptions,
};

struct Claims {
    negative: TermId,
    record: EvidenceRecord,
    fail: bool,
    completeness: SearchCompleteness,
    sum_nonnegative: bool,
    closure_declared: bool,
}

fn verdict_stops_the_actual_answer_producer_after_filters_and_exact_polarities() {
    use purrdf_core::evidence::{
        AnswerRequest, AnswerTier, StandpointKey, WitnessQuestion, WitnessVerdict,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountedClaims<I> {
        opposed: I,
        poison: I,
        record: EvidenceRecord,
        calls: AtomicUsize,
        completeness: SearchCompleteness,
    }
    impl<I: purrdf_core::ViewTermId> EvidenceSource<I> for CountedClaims<I> {
        fn claim(&self, quad: &QuadIds<I>) -> Result<Option<&EvidenceRecord>, EvalError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if quad.p == self.poison {
                return Err(EvalError::WorkspaceUnpriced("unvisited producer suffix"));
            }
            Ok((quad.p == self.opposed).then_some(&self.record))
        }
        fn scope(&self, _: GraphMatch<I>) -> Result<EvidenceScope, EvalError> {
            Ok(EvidenceScope {
                completeness: self.completeness,
                closure_declared: true,
                missing_reason: None,
            })
        }
        fn reason(&self, _: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
            Ok(None)
        }
    }
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/item").unwrap();
    let value = builder.intern_iri("https://example.org/value").unwrap();
    let positive = builder.intern_iri("https://example.org/positive").unwrap();
    let opposed = builder.intern_iri("https://example.org/opposed").unwrap();
    let poison = builder.intern_iri("https://example.org/poison").unwrap();
    for predicate in [positive, opposed, poison] {
        builder.push_quad(subject, predicate, value, None).unwrap();
    }
    let dataset = builder.freeze().unwrap();
    fn check<D: purrdf_core::FallibleDatasetView + Sync>(dataset: &D) {
        let mut source = CountedClaims {
            opposed: dataset
                .term_id_by_value(&purrdf_core::TermValue::iri("https://example.org/opposed"))
                .unwrap()
                .unwrap(),
            poison: dataset
                .term_id_by_value(&purrdf_core::TermValue::iri("https://example.org/poison"))
                .unwrap()
                .unwrap(),
            record: EvidenceRecord::from_parts(
                &InformationMap::uniform(Information::Opposed),
                SearchCompleteness::Complete,
                &Frame::actual(),
                &[],
                &[],
                &mut Memory::new(&mut Resident),
            )
            .unwrap(),
            calls: AtomicUsize::new(0),
            completeness: SearchCompleteness::Complete,
        };
        let engine = NativeSparqlEngine::new();
        let positive_arm = "{ ?s <https://example.org/positive> ?o }";
        let negative_arm = "{ ?s <https://example.org/opposed> ?o }";
        let poison_arm = "{ ?s <https://example.org/poison> ?o }";
        for form in [
            "SELECT ?s",
            "ASK",
            "CONSTRUCT { ?s <https://example.org/result> ?o }",
            "DESCRIBE ?s",
        ] {
            for (body, question, calls, information) in [
                (
                    format!("{positive_arm} UNION {poison_arm}"),
                    WitnessQuestion::Support,
                    1,
                    Information::Supported,
                ),
                (
                    format!("{positive_arm} UNION {negative_arm} UNION {poison_arm}"),
                    WitnessQuestion::Contradiction,
                    2,
                    Information::Both,
                ),
                (
                    format!(
                        "{{ ?s <https://example.org/positive> ?o FILTER(false) }} UNION {positive_arm} UNION {poison_arm}"
                    ),
                    WitnessQuestion::Support,
                    2,
                    Information::Supported,
                ),
            ] {
                source.calls.store(0, Ordering::Relaxed);
                let query = format!("{form} WHERE {{ {body} }}");
                engine
                    .query_evidence_status_fallible_view(
                        dataset,
                        request(&query),
                        QueryOptions::default(),
                        &source,
                        AnswerRequest {
                            tier: AnswerTier::Verdict,
                            question: Some(question),
                            standpoint: StandpointKey::Unscoped,
                            input_completeness: SearchCompleteness::Complete,
                            closure_declared: true,
                        },
                        |_, status| {
                            assert_eq!(status.verdict, Some(WitnessVerdict::Yes));
                            assert_eq!(status.information, information);
                            assert_eq!(status.completeness, SearchCompleteness::NotEvaluated);
                            assert_eq!(status.reasons, []);
                        },
                    )
                    .expect("settled verdict leaves the hard-failing suffix unopened");
                assert_eq!(source.calls.load(Ordering::Relaxed), calls, "{query}");
            }
            source.calls.store(0, Ordering::Relaxed);
            let query = format!("{form} WHERE {{ {positive_arm} UNION {negative_arm} }}");
            engine
                .query_evidence_status_fallible_view(
                    dataset,
                    request(&query),
                    QueryOptions::default(),
                    &source,
                    AnswerRequest {
                        tier: AnswerTier::Summary,
                        question: Some(WitnessQuestion::Support),
                        standpoint: StandpointKey::Unscoped,
                        input_completeness: SearchCompleteness::Complete,
                        closure_declared: true,
                    },
                    |_, status| {
                        assert_eq!(status.information, Information::Both);
                        assert_eq!(status.completeness, SearchCompleteness::Complete);
                        assert_eq!(status.established, 2);
                    },
                )
                .unwrap();
            assert_eq!(source.calls.load(Ordering::Relaxed), 2);
        }
        source.completeness = SearchCompleteness::Undetermined;
        source.calls.store(0, Ordering::Relaxed);
        engine
            .query_evidence_status_fallible_view(
                dataset,
                request("SELECT ?s WHERE { ?s <https://example.org/poison> ?o }"),
                QueryOptions::default(),
                &source,
                AnswerRequest {
                    tier: AnswerTier::Verdict,
                    question: Some(WitnessQuestion::ClosedAbsence),
                    standpoint: StandpointKey::Unscoped,
                    input_completeness: SearchCompleteness::Complete,
                    closure_declared: true,
                },
                |_, status| {
                    assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
                    assert_eq!(status.established, 0);
                },
            )
            .unwrap();
        assert_eq!(
            source.calls.load(Ordering::Relaxed),
            0,
            "declared incompleteness settles absence before opening the producer"
        );
    }
    check(&dataset);
    let paged = purrdf_core::PagedDataset::from_provider(std::sync::Arc::new(
        purrdf_core::InMemoryPageProvider::with_byte_lengths(
            vec![(dataset, 64)],
            purrdf_core::PageGeneration(69),
        ),
    ))
    .unwrap();
    let view = paged.query_view(purrdf_core::PagedQueryLimits::UNBOUNDED);
    check(&view);
}

fn actual_claim_ingress_inherits_sharper_standpoints_without_scoping_unspecified_claims() {
    use purrdf_core::evidence::{FrameComponent, StandpointKey, StandpointOrder};
    use purrdf_hash::hex::Digest32;

    struct ScopedClaims<'a> {
        record: &'a EvidenceRecord,
        order: Option<&'a StandpointOrder<Digest32>>,
    }
    impl EvidenceSource<TermId> for ScopedClaims<'_> {
        fn standpoint_order(&self) -> Option<&StandpointOrder<Digest32>> {
            self.order
        }
        fn claim(&self, _: &QuadIds<TermId>) -> Result<Option<&EvidenceRecord>, EvalError> {
            Ok(Some(self.record))
        }
        fn scope(&self, _: GraphMatch<TermId>) -> Result<EvidenceScope, EvalError> {
            Ok(EvidenceScope {
                completeness: SearchCompleteness::Complete,
                closure_declared: true,
                missing_reason: None,
            })
        }
        fn reason(&self, _: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
            Ok(None)
        }
    }
    let broad = Digest32::new([1; 32]);
    let sharp = Digest32::new([2; 32]);
    let sharper = Digest32::new([3; 32]);
    let unrelated = Digest32::new([4; 32]);
    let unscoped = Digest32::new([0; 32]);
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let order = StandpointOrder::new(
        &[unscoped, broad, sharp, sharper, unrelated],
        &[(sharp, broad), (sharper, sharp)],
        Some(unscoped),
        &mut memory,
    )
    .expect("original caller sharpening order");
    let information = InformationMap::from_entries(
        Information::Neither,
        &[
            (StandpointKey::Declared(broad), Information::Supported),
            (StandpointKey::Declared(sharp), Information::Opposed),
            (StandpointKey::Unscoped, Information::Opposed),
        ],
        &mut memory,
    )
    .expect("original scoped and unscoped declarations");
    let frame = Frame::from_components(&[FrameComponent::Imagined(unrelated)], &mut memory)
        .expect("original scenario restriction");
    let record = EvidenceRecord::from_parts(
        &information,
        SearchCompleteness::Complete,
        &frame,
        &[],
        &[],
        &mut memory,
    )
    .expect("original independent facets");
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/item").unwrap();
    let p = builder.intern_iri("https://example.org/p").unwrap();
    let o = builder.intern_iri("https://example.org/value").unwrap();
    builder.push_quad(s, p, o, None).unwrap();
    let dataset = builder.freeze().expect("original claim");
    let engine = NativeSparqlEngine::new();
    let query = "SELECT ?s WHERE { ?s <https://example.org/p> ?o }";
    engine
        .query_interned_evidence_view(
            &dataset,
            request(query),
            QueryOptions::default(),
            &ScopedClaims {
                record: &record,
                order: Some(&order),
            },
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("native scoped SELECT");
                };
                assert_eq!(rows.len(), 1);
                rows.rows()[0]
                    .evidence()
                    .with_record(|held| {
                        assert_eq!(
                            held.information().get(StandpointKey::Declared(broad)),
                            Information::Supported
                        );
                        assert_eq!(
                            held.information().get(StandpointKey::Declared(sharp)),
                            Information::Both
                        );
                        assert_eq!(
                            held.information().get(StandpointKey::Declared(sharper)),
                            Information::Both
                        );
                        assert_eq!(
                            held.information().get(StandpointKey::Declared(unrelated)),
                            Information::Neither
                        );
                        assert_eq!(
                            held.information().get(StandpointKey::Unscoped),
                            Information::Opposed
                        );
                        assert_eq!(held.information().fallback(), Information::Neither);
                        assert_eq!(held.frame(), &frame);
                    })
                    .expect("original dictionary inherits through the declared home");
            },
        )
        .expect("actual claim ingress expands only sharper contexts");
    let missing: Result<(), _> = engine.query_interned_evidence_view(
        &dataset,
        request(query),
        QueryOptions::default(),
        &ScopedClaims {
            record: &record,
            order: None,
        },
        |_| panic!("missing order cannot publish a partial claim"),
    );
    assert!(
        missing
            .expect_err("scoped claims require explicit order")
            .message
            .contains("original sharpening order")
    );
    let incomplete_order = StandpointOrder::new(&[sharp], &[], None, &mut memory)
        .expect("valid but different declared universe");
    let undeclared: Result<(), _> = engine.query_interned_evidence_view(
        &dataset,
        request(query),
        QueryOptions::default(),
        &ScopedClaims {
            record: &record,
            order: Some(&incomplete_order),
        },
        |_| panic!("undeclared witness cannot publish"),
    );
    assert!(
        undeclared
            .expect_err("original declaration error is hard")
            .message
            .contains("undeclared standpoint")
    );
}

impl EvidenceSource<TermId> for Claims {
    fn claim(&self, quad: &QuadIds<TermId>) -> Result<Option<&EvidenceRecord>, EvalError> {
        if self.fail {
            return Err(EvalError::WorkspaceUnpriced(
                "original claim source refusal",
            ));
        }
        Ok((quad.o == self.negative).then_some(&self.record))
    }

    fn scope(&self, _: GraphMatch<TermId>) -> Result<EvidenceScope, EvalError> {
        Ok(EvidenceScope {
            completeness: self.completeness,
            closure_declared: self.closure_declared,
            missing_reason: None,
        })
    }

    fn reason(&self, _: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
        Ok(None)
    }

    fn sum_nonnegative(
        &self,
        _: &purrdf_sparql_algebra::AggregateExpression,
        _: GraphMatch<TermId>,
    ) -> Result<bool, EvalError> {
        Ok(self.sum_nonnegative)
    }
}

fn request(query: &str) -> InternedRequest<'_> {
    InternedRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

struct ServiceClaims {
    gaps: Gaps,
    opposed: EvidenceRecord,
    unknown: EvidenceRecord,
    scope: SearchCompleteness,
    closure_declared: bool,
    fail: bool,
}

impl EvidenceSource<TermId> for ServiceClaims {
    fn claim(&self, _: &QuadIds<TermId>) -> Result<Option<&EvidenceRecord>, EvalError> {
        Ok(None)
    }

    fn scope(&self, graph: GraphMatch<TermId>) -> Result<EvidenceScope, EvalError> {
        self.gaps.scope(graph)
    }

    fn reason(&self, id: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
        self.gaps.reason(id)
    }

    fn service_scope(&self, endpoint: &str) -> Result<EvidenceScope, EvalError> {
        assert_eq!(endpoint, "https://example.org/service");
        Ok(EvidenceScope {
            completeness: self.scope,
            closure_declared: self.closure_declared,
            missing_reason: (self.scope != SearchCompleteness::Complete || !self.closure_declared)
                .then_some(self.gaps.reason),
        })
    }

    fn service_row(
        &self,
        endpoint: &str,
        position: usize,
        variables: &[purrdf_sparql_algebra::Variable],
        binding: &[Option<purrdf_core::TermValue>],
    ) -> Result<Option<purrdf_sparql_eval::evidence_exec::ServiceRowEvidence<'_>>, EvalError> {
        use purrdf_sparql_eval::evidence_exec::ServiceRowEvidence;
        assert_eq!(endpoint, "https://example.org/service");
        assert_eq!(variables[0].as_str(), "s");
        assert_eq!(binding.len(), 1);
        if self.fail {
            return Err(EvalError::WorkspaceUnpriced("original SERVICE declaration"));
        }
        Ok(match position {
            0 => None,
            1 => Some(ServiceRowEvidence {
                record: &self.opposed,
                undecided: false,
            }),
            2 => Some(ServiceRowEvidence {
                record: &self.unknown,
                undecided: true,
            }),
            _ => panic!("original response has three rows"),
        })
    }
}

struct ServiceRows {
    variable: purrdf_sparql_algebra::Variable,
    count: usize,
    refusal: bool,
}

impl purrdf_sparql_eval::remote::ServiceResolver for ServiceRows {
    fn resolve(
        &self,
        _: purrdf_sparql_eval::remote::ServiceRequest<'_>,
    ) -> Result<purrdf_sparql_eval::remote::ResolvedBindings, purrdf_sparql_eval::remote::RemoteError>
    {
        use purrdf_core::TermValue;
        Ok(purrdf_sparql_eval::remote::ResolvedBindings {
            variables: vec![self.variable.clone()],
            rows: (0..self.count)
                .map(|position| {
                    vec![Some(TermValue::iri(if position < 2 {
                        "https://example.org/known"
                    } else {
                        "https://example.org/undecided"
                    }))]
                })
                .collect(),
            cell_limit_exceeded_at: None,
        })
    }

    fn resolve_admitted(
        &self,
        _request: purrdf_sparql_eval::remote::ServiceRequest<'_>,
        workspace: purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<
        purrdf_sparql_eval::remote::AdmittedResolvedBindings,
        purrdf_sparql_eval::remote::ServiceResolutionError,
    > {
        if self.refusal {
            return Err(
                purrdf_sparql_eval::remote::ServiceResolutionError::Operational(
                    EvalError::WorkspaceUnpriced("original SERVICE producer"),
                ),
            );
        }
        // Exercise the original native response owner; the test's three fixed
        // terms are admitted through that same frame before publication.
        purrdf_sparql_eval::remote::AdmittedResolvedBindings::try_build(&workspace, |memory| {
            let variables = memory.collect([self.variable.clone()])?;
            let mut rows = Vec::new();
            for position in 0..self.count {
                let iri = memory.string(if position < 2 {
                    "https://example.org/known"
                } else {
                    "https://example.org/undecided"
                })?;
                let cells = memory.collect([Some(purrdf_core::TermValue::Iri(iri))])?;
                memory.push(&mut rows, cells)?;
            }
            Ok(purrdf_sparql_eval::remote::ResolvedBindings {
                variables,
                rows,
                cell_limit_exceeded_at: None,
            })
        })
    }
}

fn service_rows_keep_signed_and_undecided_declarations_through_original_operators() {
    let (dataset, gaps) = gap_dataset();
    let record = |information, completeness, reasons: &[ReasonId]| {
        EvidenceRecord::from_parts(
            &InformationMap::uniform(information),
            completeness,
            &Frame::actual(),
            reasons,
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap()
    };
    let mut source = ServiceClaims {
        opposed: record(Information::Opposed, SearchCompleteness::Complete, &[]),
        unknown: record(
            Information::Neither,
            SearchCompleteness::Undetermined,
            &[gaps.reason],
        ),
        gaps,
        scope: SearchCompleteness::Complete,
        closure_declared: true,
        fail: false,
    };
    let remote = ServiceRows {
        variable: purrdf_sparql_algebra::Variable::new("s"),
        count: 3,
        refusal: false,
    };
    let engine = NativeSparqlEngine::new();
    for query in [
        "SELECT DISTINCT ?s WHERE { SERVICE <https://example.org/service> { ?s ?p ?o } FILTER(BOUND(?s)) }",
        "SELECT DISTINCT ?s WHERE { { SELECT ?s WHERE { SERVICE <https://example.org/service> { ?s ?p ?o } } } BIND(?s AS ?copy) FILTER(BOUND(?copy)) }",
    ] {
        engine
            .query_interned_evidence_view(
                &dataset,
                request(query),
                QueryOptions::default().with_remote(Some(&remote)),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("native SELECT");
                    };
                    assert_eq!(rows.len(), 2);
                    assert_eq!(rows.established_rows().count(), 1);
                    assert_eq!(rows.undecided_rows().count(), 1);
                    let known = rows.established_rows().next().unwrap();
                    known
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(record.information().fallback(), Information::Both);
                            assert_eq!(record.completeness(), SearchCompleteness::Complete);
                        })
                        .unwrap();
                    let missing = rows.undecided_rows().next().unwrap();
                    missing
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(record.information().fallback(), Information::Neither);
                            assert_eq!(record.reasons(), &[source.gaps.reason]);
                        })
                        .unwrap();
                },
            )
            .unwrap();
    }
    engine.query_interned_evidence_view(&dataset,
        request("SELECT DISTINCT ?one WHERE { SERVICE <https://example.org/service> { [] <https://example.org/p> [] } BIND(true AS ?one) }"),
        QueryOptions::default().with_remote(Some(&remote)), &source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("native SELECT"); };
            assert_eq!(rows.len(), 1);
            assert_eq!(rows.established_rows().count(), 1);
            rows.rows()[0].evidence().with_record(|record| {
                assert_eq!(record.information().fallback(), Information::Both);
            }).unwrap();
        }).unwrap();
    source.fail = true;
    let query = "SELECT ?s WHERE { SERVICE SILENT <https://example.org/service> { ?s ?p ?o } }";
    assert!(
        engine
            .query_interned_evidence_view(
                &dataset,
                request(query),
                QueryOptions::default().with_remote(Some(&remote)),
                &source,
                |_| {}
            )
            .is_err(),
        "SILENT cannot swallow an original declaration failure"
    );
    source.fail = false;
    let remote = ServiceRows {
        variable: purrdf_sparql_algebra::Variable::new("s"),
        count: 0,
        refusal: true,
    };
    assert!(
        engine
            .query_interned_evidence_view(
                &dataset,
                request(query),
                QueryOptions::default().with_remote(Some(&remote)),
                &source,
                |_| {}
            )
            .is_err(),
        "SILENT cannot turn a physical failure into semantic unknown"
    );
}

fn empty_service_keeps_search_reason_without_inventing_an_answer_row() {
    use purrdf_core::evidence::{
        AnswerRequest, AnswerTier, StandpointKey, WitnessQuestion, WitnessVerdict,
    };
    let (dataset, gaps) = gap_dataset();
    let mut source = ServiceClaims {
        opposed: EvidenceRecord::default(),
        unknown: EvidenceRecord::default(),
        gaps,
        scope: SearchCompleteness::Undetermined,
        closure_declared: true,
        fail: false,
    };
    let remote = ServiceRows {
        variable: purrdf_sparql_algebra::Variable::new("s"),
        count: 0,
        refusal: false,
    };
    for (completeness, closed, expected) in [
        (
            SearchCompleteness::Undetermined,
            true,
            WitnessVerdict::Unknown,
        ),
        (SearchCompleteness::Complete, false, WitnessVerdict::Unknown),
        (SearchCompleteness::Complete, true, WitnessVerdict::Yes),
    ] {
        source.scope = completeness;
        source.closure_declared = closed;
        NativeSparqlEngine::new()
            .query_evidence_status_view(
                &dataset,
                request("SELECT ?s WHERE { SERVICE <https://example.org/service> { ?s ?p ?o } }"),
                QueryOptions::default().with_remote(Some(&remote)),
                &source,
                AnswerRequest {
                    tier: AnswerTier::Summary,
                    question: Some(WitnessQuestion::ClosedAbsence),
                    standpoint: StandpointKey::Unscoped,
                    input_completeness: SearchCompleteness::Complete,
                    closure_declared: true,
                },
                |outcome, status| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("native SELECT");
                    };
                    assert_eq!(rows.len(), 0);
                    assert_eq!(status.verdict, Some(expected));
                    assert_eq!(status.completeness, completeness);
                    assert_eq!((status.established, status.undecided), (0, 0));
                    if completeness != SearchCompleteness::Complete || !closed {
                        assert_eq!(status.reasons.len(), 1);
                        assert_eq!(status.reasons[0].reason, source.gaps.reason);
                        assert_eq!(
                            (status.reasons[0].established, status.reasons[0].undecided),
                            (0, 0)
                        );
                    } else {
                        assert_eq!(status.reasons, []);
                    }
                },
            )
            .unwrap();
    }
    source.scope = SearchCompleteness::Complete;
    source.closure_declared = false;
    NativeSparqlEngine::new().query_interned_evidence_view(&dataset,
        request("SELECT ?x WHERE { BIND(1 AS ?x) FILTER NOT EXISTS { SERVICE <https://example.org/service> { ?s ?p ?o } } }"),
        QueryOptions::default().with_remote(Some(&remote)), &source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("native SELECT"); };
            assert_eq!(rows.len(), 1);
            assert_eq!(rows.established_rows().count(), 0);
            assert_eq!(rows.undecided_rows().count(), 1);
            rows.rows()[0].evidence().with_record(|record| {
                assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                assert_eq!(record.reasons(), &[source.gaps.reason]);
            }).unwrap();
        }).unwrap();
}

fn sparql_function_suspensions_retain_original_select_ask_and_nested_body_evidence() {
    use purrdf_sparql_eval::extension_env::ExtensionEnv;
    use purrdf_sparql_eval::user_fn::{
        TypeConstraint, UserFnBody, UserFunction, UserFunctionRegistry,
    };
    use std::sync::Arc;

    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/subject").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let negative = builder.intern_iri("https://example.org/negative").unwrap();
    builder
        .push_quad(subject, predicate, negative, None)
        .unwrap();
    let dataset = builder.freeze().unwrap();
    let source = Claims {
        negative,
        record: EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Opposed),
            SearchCompleteness::Complete,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap(),
        fail: false,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    let mut functions = UserFunctionRegistry::default();
    for (iri, kind, body) in [
        (
            "https://example.org/select",
            UserFnBody::Select,
            "SELECT ?o WHERE { ?s <https://example.org/p> ?o }",
        ),
        (
            "https://example.org/ask",
            UserFnBody::Ask,
            "ASK { ?s <https://example.org/p> ?o }",
        ),
        (
            "https://example.org/nested",
            UserFnBody::Select,
            "SELECT (<https://example.org/select>() AS ?o) WHERE {}",
        ),
    ] {
        functions.insert(
            iri,
            UserFunction {
                params: Vec::new(),
                required: 0,
                body: Arc::from(body),
                kind,
                return_constraint: TypeConstraint::default(),
            },
        );
    }
    let engine = NativeSparqlEngine::new();
    let functions = engine
        .bind_functions(functions, ExtensionEnv::empty())
        .unwrap();
    for iri in [
        "https://example.org/select",
        "https://example.org/ask",
        "https://example.org/nested",
    ] {
        for expression in [format!("<{iri}>()"), format!("IF(true, <{iri}>(), 0)")] {
            let query = format!(
                "SELECT ?value WHERE {{ BIND({expression} AS ?value) FILTER(BOUND(?value)) }}"
            );
            engine
                .query_interned_evidence_view(
                    &dataset,
                    request(&query),
                    QueryOptions::default().with_functions(&functions),
                    &source,
                    |outcome| {
                        let InternedOutcome::Solutions(rows) = outcome else {
                            panic!("native SELECT");
                        };
                        assert_eq!(rows.len(), 1);
                        assert!(rows.rows()[0][0].is_some());
                        rows.rows()[0]
                            .evidence()
                            .with_record(|record| {
                                assert_eq!(record.information().fallback(), Information::Opposed);
                                assert_eq!(record.completeness(), SearchCompleteness::Complete);
                            })
                            .unwrap();
                    },
                )
                .unwrap();
        }
    }
    let failed = Claims {
        fail: true,
        ..source
    };
    let result: Result<(), _> = engine.query_interned_evidence_view(
        &dataset,
        request("SELECT (<https://example.org/nested>() AS ?value) WHERE {}"),
        QueryOptions::default().with_functions(&functions),
        &failed,
        |_| panic!("an original function-body source refusal cannot publish"),
    );
    assert!(result.is_err());
    engine
        .query_interned_evidence_view(
            &dataset,
            request("SELECT (IF(false, <https://example.org/nested>(), 1) AS ?value) WHERE {}"),
            QueryOptions::default().with_functions(&functions),
            &failed,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("native SELECT");
                };
                assert_eq!(rows.len(), 1);
                assert!(rows.rows()[0].evidence().is_default());
            },
        )
        .unwrap();
}

fn distinct_coalesces_both_polarities_without_changing_binding_identity() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/subject").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let positive = builder.intern_iri("https://example.org/positive").unwrap();
    let negative = builder.intern_iri("https://example.org/negative").unwrap();
    builder
        .push_quad(subject, predicate, positive, None)
        .unwrap();
    builder
        .push_quad(subject, predicate, negative, None)
        .unwrap();
    let dataset = builder.freeze().expect("frozen native view");
    let source = Claims {
        negative,
        record: EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Opposed),
            SearchCompleteness::Complete,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .expect("opposed input declaration"),
        fail: false,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    let engine = NativeSparqlEngine::new();
    let query = "SELECT DISTINCT ?s WHERE { ?s <https://example.org/p> ?o }";
    let evidence = engine
        .query_interned_evidence_view(
            &dataset,
            request(query),
            QueryOptions::default(),
            &source,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("SELECT result");
                };
                assert_eq!(rows.len(), 1);
                assert_eq!(rows.variables().len(), 1);
                let row = &rows.rows()[0];
                row.evidence()
                    .with_record(|record| {
                        assert_eq!(record.information().fallback(), Information::Both);
                        assert_eq!(record.completeness(), SearchCompleteness::Complete);
                        assert!(record.frame().is_unrestricted_actual());
                    })
                    .expect("native evidence inspection");
                row.evidence().clone()
            },
        )
        .expect("original engine execution");
    drop(engine);
    drop(dataset);
    evidence
        .with_record(|record| {
            assert_eq!(record.information().fallback(), Information::Both);
        })
        .expect("evidence owner survives execution");
}

fn source_refusal_never_publishes_partial_or_undetermined_rows() {
    let mut builder = RdfDatasetBuilder::new();
    let term = builder.intern_iri("https://example.org/item").unwrap();
    builder.push_quad(term, term, term, None).unwrap();
    let dataset = builder.freeze().expect("frozen native view");
    let source = Claims {
        negative: term,
        record: EvidenceRecord::default(),
        fail: true,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    let mut visited = false;
    let result = NativeSparqlEngine::new().query_interned_evidence_view(
        &dataset,
        request("SELECT * WHERE { ?s ?p ?o }"),
        QueryOptions::default(),
        &source,
        |_| {
            visited = true;
        },
    );
    assert!(result.is_err());
    assert!(
        !visited,
        "a failed source cannot publish evidence as a semantic gap"
    );
}

fn unmatched_optional_preserves_information_and_declared_missing_scope() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/subject").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let object = builder.intern_iri("https://example.org/object").unwrap();
    builder.push_quad(subject, predicate, object, None).unwrap();
    let dataset = builder.freeze().expect("frozen native view");
    let engine = NativeSparqlEngine::new();
    for completeness in [
        SearchCompleteness::Complete,
        SearchCompleteness::Undetermined,
    ] {
        let source = Claims {
            negative: predicate,
            record: EvidenceRecord::default(),
            fail: false,
            completeness,
            sum_nonnegative: false,
            closure_declared: true,
        };
        engine.query_interned_evidence_view(
            &dataset, request("SELECT ?s ?missing WHERE { ?s <https://example.org/p> ?o OPTIONAL { ?s <https://example.org/missing> ?missing } }"),
            QueryOptions::default(), &source, |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else { panic!("SELECT result"); };
                assert_eq!(rows.len(), 1);
                let row = &rows.rows()[0];
                assert!(row[rows.column("missing").expect("optional column")].is_none());
                row.evidence().with_record(|record| {
                    assert_eq!(record.information().fallback(), Information::Supported);
                    assert_eq!(record.completeness(), completeness);
                }).expect("original row evidence");
            },
        ).expect("OPTIONAL native execution");
    }
}

fn group_bounds_follow_whole_scope_and_declared_sum_domain() {
    use purrdf_core::evidence::{AggregateBound, aggregate_column_identity};
    let mut builder = RdfDatasetBuilder::new();
    let unused = builder.intern_iri("https://example.org/unused").unwrap();
    let dataset = builder.freeze().expect("frozen native view");
    let engine = NativeSparqlEngine::new();
    for (completeness, nonnegative, values) in [
        (SearchCompleteness::Complete, false, "-2 5"),
        (SearchCompleteness::Undetermined, false, "-2 5"),
        (SearchCompleteness::Undetermined, true, "2 5"),
    ] {
        let source = Claims {
            negative: unused,
            record: EvidenceRecord::default(),
            fail: false,
            completeness,
            sum_nonnegative: nonnegative,
            closure_declared: true,
        };
        let query = format!(
            "SELECT (COUNT(*) AS ?count) (SUM(?n) AS ?sum) (MIN(?n) AS ?min) (MAX(?n) AS ?max) (AVG(?n) AS ?avg) WHERE {{ VALUES ?n {{ {values} }} }}"
        );
        engine
            .query_interned_evidence_view(
                &dataset,
                request(&query),
                QueryOptions::default(),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("SELECT result");
                    };
                    assert_eq!(rows.len(), 1);
                    rows.rows()[0]
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(record.completeness(), completeness);
                            assert_eq!(record.aggregate_bounds().len(), 5);
                            for (column, partial_bound) in [
                                ("count", AggregateBound::Lower),
                                (
                                    "sum",
                                    if nonnegative {
                                        AggregateBound::Lower
                                    } else {
                                        AggregateBound::Undetermined
                                    },
                                ),
                                ("min", AggregateBound::Upper),
                                ("max", AggregateBound::Lower),
                                ("avg", AggregateBound::Undetermined),
                            ] {
                                let expected = if completeness == SearchCompleteness::Complete {
                                    AggregateBound::Exact
                                } else {
                                    partial_bound
                                };
                                let id = aggregate_column_identity(column);
                                assert_eq!(
                                    record
                                        .aggregate_bounds()
                                        .iter()
                                        .find(|(key, _)| *key == id)
                                        .map(|(_, bound)| *bound),
                                    Some(expected),
                                    "{column}"
                                );
                            }
                        })
                        .expect("native per-column evidence");
                },
            )
            .expect("native GROUP execution");
    }
}

fn nested_projection_retains_only_visible_aggregate_aliases() {
    use purrdf_core::evidence::{AggregateBound, aggregate_column_identity};
    let mut builder = RdfDatasetBuilder::new();
    let unused = builder.intern_iri("https://example.org/unused").unwrap();
    let dataset = builder.freeze().expect("frozen native view");
    let source = Claims {
        negative: unused,
        record: EvidenceRecord::default(),
        fail: false,
        completeness: SearchCompleteness::Undetermined,
        sum_nonnegative: false,
        closure_declared: true,
    };
    NativeSparqlEngine::new().query_interned_evidence_view(
        &dataset,
        request("SELECT ?visible WHERE { { SELECT (COUNT(*) AS ?count) (MIN(?n) AS ?hidden) WHERE { VALUES ?n { 2 5 } } } BIND(?count AS ?visible) }"),
        QueryOptions::default(),
        &source,
        |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("SELECT result"); };
            assert_eq!(rows.len(), 1);
            assert_eq!(rows.variables().len(), 1);
            rows.rows()[0].evidence().with_record(|record| {
                assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                assert_eq!(record.aggregate_bounds(), &[(aggregate_column_identity("visible"), AggregateBound::Lower)]);
            }).expect("visible column certificate");
        },
    ).expect("nested aggregate execution");
}

struct Gaps {
    subject: TermId,
    reason: ReasonId,
    cause: purrdf_core::TermValue,
    concern: purrdf_core::TermValue,
    fail: bool,
}

impl EvidenceSource<TermId> for Gaps {
    fn claim(&self, _: &QuadIds<TermId>) -> Result<Option<&EvidenceRecord>, EvalError> {
        Ok(None)
    }

    fn scope(&self, _: GraphMatch<TermId>) -> Result<EvidenceScope, EvalError> {
        Ok(EvidenceScope {
            completeness: SearchCompleteness::Complete,
            closure_declared: true,
            missing_reason: None,
        })
    }

    fn reason(&self, id: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
        Ok((id == self.reason).then_some(ReasonDescription {
            cause: &self.cause,
            concern: &self.concern,
            constraint: None,
            vault: None,
            watermark: None,
        }))
    }

    fn lossy_producer_reason(
        &self,
        producer: &str,
        evidence: &str,
    ) -> Result<Option<ReasonId>, EvalError> {
        Ok(
            (producer == "https://example.org/search" && evidence == "original missing shard")
                .then_some(self.reason),
        )
    }

    fn expression_gap(
        &self,
        _: &purrdf_sparql_algebra::Expression,
        _: &[purrdf_sparql_algebra::Variable],
        row: &[Option<purrdf_sparql_eval::SolutionTerm<TermId>>],
        _: GraphMatch<TermId>,
    ) -> Result<Option<ReasonId>, EvalError> {
        if self.fail {
            return Err(EvalError::WorkspaceUnpriced(
                "original expression declaration refusal",
            ));
        }
        Ok(row
            .contains(&Some(purrdf_sparql_eval::SolutionTerm::Existing(
                self.subject,
            )))
            .then_some(self.reason))
    }
}

fn gap_dataset() -> (std::sync::Arc<purrdf_core::RdfDataset>, Gaps) {
    use purrdf_core::{TermBox, TermValue};
    let mut builder = RdfDatasetBuilder::new();
    let missing = builder.intern_iri("https://example.org/missing").unwrap();
    let known = builder.intern_iri("https://example.org/known").unwrap();
    let kind = builder.intern_iri("https://example.org/kind").unwrap();
    let value = builder.intern_iri("https://example.org/value").unwrap();
    let two = builder.intern_iri("https://example.org/two").unwrap();
    builder.push_quad(missing, kind, kind, None).unwrap();
    builder.push_quad(known, kind, kind, None).unwrap();
    builder.push_quad(known, value, two, None).unwrap();
    let dataset = builder.freeze().expect("original native input");
    let cause = TermValue::iri("https://example.org/missing-value");
    let concern = TermValue::Triple {
        s: TermBox::new(TermValue::iri("https://example.org/constraint")),
        p: TermBox::new(TermValue::iri("https://example.org/value")),
        o: TermBox::new(TermValue::iri("https://example.org/required")),
    };
    let reason = ReasonDescription {
        cause: &cause,
        concern: &concern,
        constraint: None,
        vault: None,
        watermark: None,
    }
    .identity(&mut Memory::new(&mut Resident))
    .expect("canonical original reason");
    let source = Gaps {
        subject: missing,
        reason,
        cause,
        concern,
        fail: false,
    };
    (dataset, source)
}

fn registered_lossy_search_keeps_original_candidates_and_caller_reason() {
    use purrdf_core::TermValue;
    use purrdf_core::evidence::{
        AnswerRequest, AnswerTier, StandpointKey, WitnessQuestion, WitnessVerdict,
    };
    use purrdf_sparql_eval::{
        AcceptedTerm, CandidateDomains, Completeness, DuplicatePolicy, ExclusionBasis,
        ExtensionEnv, MemoryRelation, OrderFidelity, PropertyFunctionRegistry, RankArithmetic,
        RankFidelity, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
    };
    use std::sync::Arc;
    let (dataset, source) = gap_dataset();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        "https://example.org/search",
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri("https://example.org/document"),
                    TermValue::simple_literal("needle"),
                ]],
            )
            .unwrap(),
        ),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri("https://example.org/stratum").unwrap(),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern {
                    kind: TermKind::Literal,
                    datatype: None,
                    language: None,
                    predicate: None,
                },
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: None,
            rank_source: purrdf_sparql_eval::RankSource::Ordinal,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            fidelity: RankFidelity {
                completeness: Completeness::Lossy {
                    evidence: Arc::from("original missing shard"),
                },
                order: OrderFidelity::Faithful,
            },
            arithmetic: RankArithmetic::FloatFree,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
            mandatory: true,
        },
    );
    let env = ExtensionEnv::over_relations(registry).unwrap();
    let query = "SELECT ?doc WHERE { ?doc <https://example.org/search> \"needle\" }";
    let engine = NativeSparqlEngine::new();
    engine
        .query_interned_evidence_view(
            &dataset,
            request(query),
            QueryOptions::default().with_env(&env),
            &source,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("native SELECT");
                };
                assert_eq!(rows.len(), 1);
                assert_eq!(
                    rows.established_rows().count(),
                    1,
                    "a named candidate remains a witness"
                );
                rows.rows()[0]
                    .evidence()
                    .with_record(|record| {
                        assert_eq!(record.information().fallback(), Information::Supported);
                        assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                        assert_eq!(record.reasons(), &[source.reason]);
                    })
                    .unwrap();
            },
        )
        .unwrap();
    let empty = "SELECT ?doc WHERE { ?doc <https://example.org/search> \"absent\" }";
    engine.query_evidence_status_view(&dataset,
        request("SELECT ?doc WHERE { { ?doc <https://example.org/search> \"absent\" } UNION { BIND(<https://example.org/known> AS ?doc) } }"),
        QueryOptions::default().with_env(&env), &source,
        AnswerRequest { tier: AnswerTier::Verdict, question: Some(WitnessQuestion::ClosedAbsence), standpoint: StandpointKey::Unscoped, input_completeness: SearchCompleteness::Complete, closure_declared: true },
        |outcome, status| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("native SELECT"); };
            assert_eq!(rows.len(), 0, "empty lossy invocation stops before the supported UNION suffix");
            assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
            assert_eq!((status.established, status.undecided), (0, 0));
        }).expect("a zero-row invocation publishes its actual scope to the verdict consumer");
    engine
        .query_evidence_status_view(
            &dataset,
            request(empty),
            QueryOptions::default().with_env(&env),
            &source,
            AnswerRequest {
                tier: AnswerTier::Summary,
                question: Some(WitnessQuestion::Support),
                standpoint: StandpointKey::Unscoped,
                input_completeness: SearchCompleteness::Complete,
                closure_declared: true,
            },
            |outcome, status| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("native SELECT")
                };
                assert_eq!(rows.len(), 0);
                assert_eq!(status.information, Information::Neither);
                assert_eq!(status.completeness, SearchCompleteness::Undetermined);
                assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
                assert_eq!((status.established, status.undecided), (0, 0));
                assert_eq!(status.reasons.len(), 1);
                assert_eq!(status.reasons[0].reason, source.reason);
                assert_eq!(
                    (status.reasons[0].established, status.reasons[0].undecided),
                    (0, 0)
                );
            },
        )
        .unwrap();
    // The producer ran but every emitted row failed the bound needle. Its
    // registered missing shard is still an original search gap, not closed absence.
    for expression in [
        "EXISTS { ?doc <https://example.org/search> \"absent\" }",
        "NOT EXISTS { ?doc <https://example.org/search> \"absent\" }",
        "EXISTS { { ?doc <https://example.org/search> \"absent\" } UNION { FILTER(false) } }",
        "NOT EXISTS { { ?doc <https://example.org/search> \"absent\" } UNION { FILTER(false) } }",
    ] {
        let query = format!("SELECT ?outer WHERE {{ VALUES ?outer {{ 1 }} FILTER({expression}) }}");
        engine
            .query_interned_evidence_view(
                &dataset,
                request(&query),
                QueryOptions::default().with_env(&env),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("native SELECT");
                    };
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows.undecided_rows().count(), 1);
                    rows.rows()[0]
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                            assert_eq!(record.reasons(), &[source.reason]);
                            assert_eq!(record.information().fallback(), Information::Neither);
                        })
                        .unwrap();
                },
            )
            .unwrap();
    }
}

fn declared_filter_gap_retains_original_row_and_reason_without_promoting_completeness() {
    let (dataset, mut source) = gap_dataset();
    let missing = source.subject;
    let reason = source.reason;
    let query = "SELECT ?s WHERE { ?s <https://example.org/kind> ?kind OPTIONAL { ?s <https://example.org/value> ?value } FILTER(?value = <https://example.org/required>) }";
    let engine = NativeSparqlEngine::new();
    for (query, established) in [
        (query, false),
        (
            "SELECT DISTINCT ?s WHERE { { ?s <https://example.org/kind> ?kind OPTIONAL { ?s <https://example.org/value> ?value } FILTER(?value = <https://example.org/required>) } UNION { VALUES ?s { <https://example.org/missing> } } }",
            true,
        ),
    ] {
        engine
            .query_interned_evidence_view(
                &dataset,
                request(query),
                QueryOptions::default(),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("SELECT result");
                    };
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows.established_rows().count(), usize::from(established));
                    assert_eq!(rows.undecided_rows().count(), usize::from(!established));
                    assert_eq!(
                        rows.rows()[0][0],
                        Some(purrdf_sparql_eval::SolutionTerm::Existing(missing))
                    );
                    rows.rows()[0]
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(
                                record.completeness(),
                                if established {
                                    SearchCompleteness::Complete
                                } else {
                                    SearchCompleteness::Undetermined
                                }
                            );
                            assert_eq!(record.reasons(), &[reason]);
                        })
                        .expect("original retained evidence");
                    rows.rows()[0]
                        .evidence()
                        .with_reason(reason, |description| {
                            assert_eq!(description.cause, &source.cause);
                            assert_eq!(description.concern, &source.concern);
                        })
                        .expect("original canonical reason owner");
                },
            )
            .expect("native declared gap execution");
    }
    source.fail = true;
    let mut visited = false;
    assert!(
        engine
            .query_interned_evidence_view(
                &dataset,
                request(query),
                QueryOptions::default(),
                &source,
                |_| {
                    visited = true;
                }
            )
            .is_err()
    );
    assert!(
        !visited,
        "operational failure cannot publish an unknown row"
    );
}

fn all_query_forms_publish_original_unknown_status_at_requested_count_tiers() {
    use purrdf_core::evidence::{
        AnswerRequest, AnswerTier, Information, StandpointKey, WitnessQuestion, WitnessVerdict,
    };
    let (dataset, source) = gap_dataset();
    let engine = NativeSparqlEngine::new();
    let body = "WHERE { ?s <https://example.org/kind> ?kind OPTIONAL { ?s <https://example.org/value> ?value } FILTER(?value = <https://example.org/required>) }";
    for form in [
        "SELECT ?s",
        "ASK",
        "CONSTRUCT { ?s <https://example.org/result> <https://example.org/established> }",
        "DESCRIBE ?s",
    ] {
        let query = format!("{form} {body}");
        for tier in [AnswerTier::Verdict, AnswerTier::Summary, AnswerTier::Rows] {
            let answer = AnswerRequest {
                tier,
                question: Some(WitnessQuestion::Support),
                standpoint: StandpointKey::Unscoped,
                input_completeness: SearchCompleteness::Complete,
                closure_declared: true,
            };
            engine
                .query_evidence_status_view(
                    &dataset,
                    request(&query),
                    QueryOptions::default(),
                    &source,
                    answer,
                    |outcome, status| {
                        assert_eq!(status.information, Information::Neither);
                        assert_eq!(status.completeness, SearchCompleteness::Undetermined);
                        assert_eq!((status.established, status.undecided), (0, 1));
                        assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
                        if tier == AnswerTier::Verdict {
                            assert_eq!(status.reasons, []);
                        } else {
                            assert_eq!(status.reasons.len(), 1);
                            assert_eq!(status.reasons[0].reason, source.reason);
                            assert_eq!(
                                (status.reasons[0].established, status.reasons[0].undecided),
                                (0, 1)
                            );
                        }
                        match outcome {
                            InternedOutcome::Solutions(rows) => {
                                assert_eq!(rows.undecided_rows().count(), 1);
                            }
                            InternedOutcome::Boolean(value) => {
                                assert!(!value, "unknown is no established ASK witness");
                            }
                            InternedOutcome::Graph(graph) => assert_eq!(
                                graph.dataset_handle().quad_count(),
                                0,
                                "unknown template or description cannot publish established triples"
                            ),
                        }
                    },
                )
                .expect("native original outcome and evidence status");
        }
    }
}

fn minus_absence_requires_closure_but_disjoint_domains_are_decidable() {
    let mut builder = RdfDatasetBuilder::new();
    let unused = builder.intern_iri("https://example.org/unused").unwrap();
    let dataset = builder.freeze().expect("original empty input");
    let engine = NativeSparqlEngine::new();
    for (completeness, closure_declared) in [
        (SearchCompleteness::Complete, true),
        (SearchCompleteness::Complete, false),
        (SearchCompleteness::Undetermined, true),
    ] {
        let source = Claims {
            negative: unused,
            record: EvidenceRecord::default(),
            fail: false,
            completeness,
            sum_nonnegative: false,
            closure_declared,
        };
        for (variable, shared_domain) in [("s", true), ("other", false)] {
            let query = format!(
                "SELECT ?s WHERE {{ VALUES ?s {{ <https://example.org/item> }} MINUS {{ ?{variable} <https://example.org/missing> ?value }} }}"
            );
            let unknown = shared_domain
                && (completeness != SearchCompleteness::Complete || !closure_declared);
            engine
                .query_interned_evidence_view(
                    &dataset,
                    request(&query),
                    QueryOptions::default(),
                    &source,
                    |outcome| {
                        let InternedOutcome::Solutions(rows) = outcome else {
                            panic!("SELECT result");
                        };
                        assert_eq!(rows.len(), 1);
                        assert_eq!(rows.undecided_rows().count(), usize::from(unknown));
                        assert_eq!(rows.established_rows().count(), usize::from(!unknown));
                        rows.rows()[0]
                            .evidence()
                            .with_record(|record| {
                                assert_eq!(
                                    record.completeness(),
                                    if unknown {
                                        SearchCompleteness::Undetermined
                                    } else {
                                        SearchCompleteness::Complete
                                    }
                                );
                            })
                            .expect("original absence evidence");
                    },
                )
                .expect("native MINUS scope execution");
        }
    }
    let (dataset, source) = gap_dataset();
    engine.query_interned_evidence_view(&dataset,
        request("SELECT ?s WHERE { VALUES ?s { <https://example.org/missing> } MINUS { ?s <https://example.org/kind> ?kind OPTIONAL { ?s <https://example.org/value> ?value } FILTER(?value = <https://example.org/required>) } }"),
        QueryOptions::default(), &source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("SELECT result"); };
            assert_eq!(rows.len(), 1);
            assert_eq!(rows.undecided_rows().count(), 1);
            rows.rows()[0].evidence().with_record(|record| assert_eq!(record.reasons(), &[source.reason]))
                .expect("original undecided right-witness reason");
        }).expect("undecided RHS is not a removing witness");
}

fn minus_negates_all_original_witnesses_under_declared_closure() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/item").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let negative = builder.intern_iri("https://example.org/negative").unwrap();
    builder
        .push_quad(subject, predicate, negative, None)
        .unwrap();
    let dataset = builder.freeze().expect("original matched claim");
    let engine = NativeSparqlEngine::new();
    let query = "SELECT ?s WHERE { VALUES ?s { <https://example.org/item> } MINUS { ?s <https://example.org/p> ?o } }";
    for information in [
        Information::Supported,
        Information::Opposed,
        Information::Both,
        Information::Neither,
    ] {
        for (completeness, closure_declared) in [
            (SearchCompleteness::Complete, true),
            (SearchCompleteness::Complete, false),
            (SearchCompleteness::Undetermined, true),
        ] {
            let source = Claims {
                negative,
                record: EvidenceRecord::from_parts(
                    &InformationMap::uniform(information),
                    SearchCompleteness::Complete,
                    &Frame::default(),
                    &[],
                    &[],
                    &mut Memory::new(&mut Resident),
                )
                .expect("original polarity declaration"),
                fail: false,
                completeness,
                sum_nonnegative: false,
                closure_declared,
            };
            let closed = completeness == SearchCompleteness::Complete && closure_declared;
            engine
                .query_interned_evidence_view(
                    &dataset,
                    request(query),
                    QueryOptions::default(),
                    &source,
                    |outcome| {
                        let InternedOutcome::Solutions(rows) = outcome else {
                            panic!("original MINUS result");
                        };
                        if closed && information == Information::Supported {
                            assert!(rows.is_empty(), "supported closed witness excludes the row");
                            return;
                        }
                        assert_eq!(rows.len(), 1);
                        assert_eq!(rows.undecided_rows().count(), usize::from(!closed));
                        rows.rows()[0]
                            .evidence()
                            .with_record(|record| {
                                assert_eq!(
                                    record.information().fallback(),
                                    if closed {
                                        information.negate()
                                    } else {
                                        information
                                    },
                                    "negation requires complete search and declared closure"
                                );
                                assert_eq!(
                                    record.completeness(),
                                    if closed {
                                        SearchCompleteness::Complete
                                    } else {
                                        SearchCompleteness::Undetermined
                                    }
                                );
                            })
                            .expect("original negated witness record");
                    },
                )
                .expect("native MINUS evidence execution");
        }
    }
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/item").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let positive = builder.intern_iri("https://example.org/positive").unwrap();
    let negative = builder.intern_iri("https://example.org/negative").unwrap();
    builder
        .push_quad(subject, predicate, positive, None)
        .unwrap();
    builder
        .push_quad(subject, predicate, negative, None)
        .unwrap();
    let dataset = builder.freeze().expect("opposing original witness bag");
    let mut source = Claims {
        negative,
        record: EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Opposed),
            SearchCompleteness::Complete,
            &Frame::default(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .expect("opposed declaration"),
        fail: false,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    engine
        .query_interned_evidence_view(
            &dataset,
            request(query),
            QueryOptions::default(),
            &source,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("original mixed-witness MINUS result");
                };
                assert_eq!(rows.established_rows().count(), 1);
                rows.rows()[0]
                    .evidence()
                    .with_record(|record| {
                        assert_eq!(record.information().fallback(), Information::Both);
                    })
                    .expect("all witnesses survive information-order coalescing");
            },
        )
        .expect("mixed witnesses are not reduced to the first boolean match");
    source.fail = true;
    let refusal: Result<(), _> = engine.query_interned_evidence_view(
        &dataset,
        request(query),
        QueryOptions::default(),
        &source,
        |_| panic!("source refusal cannot publish an unknown answer"),
    );
    let refusal = refusal.expect_err("original source failure remains a hard error");
    assert_eq!(refusal.code, "native-sparql-workspace-unpriced");
    assert!(refusal.message.contains("original claim source refusal"));
}

fn exists_transports_original_witnesses_through_vm_memos_and_deferred_sites() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/item").unwrap();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let negative = builder.intern_iri("https://example.org/negative").unwrap();
    builder
        .push_quad(subject, predicate, negative, None)
        .unwrap();
    let dataset = builder.freeze().expect("original existential claim");
    let engine = NativeSparqlEngine::new();
    for information in [
        Information::Supported,
        Information::Opposed,
        Information::Both,
        Information::Neither,
    ] {
        for (completeness, closure_declared) in [
            (SearchCompleteness::Complete, true),
            (SearchCompleteness::Complete, false),
            (SearchCompleteness::Undetermined, true),
        ] {
            let source = Claims {
                negative,
                record: EvidenceRecord::from_parts(
                    &InformationMap::uniform(information),
                    SearchCompleteness::Complete,
                    &Frame::default(),
                    &[],
                    &[],
                    &mut Memory::new(&mut Resident),
                )
                .expect("original FOUR witness"),
                fail: false,
                completeness,
                sum_nonnegative: false,
                closure_declared,
            };
            for (body, negated) in [
                ("FILTER EXISTS { ?s <https://example.org/p> ?o }", false),
                ("FILTER NOT EXISTS { ?s <https://example.org/p> ?o }", true),
                (
                    "BIND(EXISTS { ?s <https://example.org/p> ?o } AS ?found)",
                    false,
                ),
                (
                    "FILTER EXISTS { FILTER EXISTS { ?s <https://example.org/p> ?o } }",
                    false,
                ),
                (
                    "FILTER EXISTS { ?s <https://example.org/p> ?o BIND(BNODE() AS ?fresh) }",
                    false,
                ),
            ] {
                let query = format!(
                    "SELECT ?s WHERE {{ VALUES ?s {{ <https://example.org/item> <https://example.org/item> }} {body} }}"
                );
                let closed = completeness == SearchCompleteness::Complete && closure_declared;
                let excluded = negated && closed && information == Information::Supported;
                let undecided = negated && !closed;
                engine
                    .query_interned_evidence_view(
                        &dataset,
                        request(&query),
                        QueryOptions::default(),
                        &source,
                        |outcome| {
                            let InternedOutcome::Solutions(rows) = outcome else {
                                panic!("original existential SELECT");
                            };
                            assert_eq!(rows.len(), if excluded { 0 } else { 2 }, "{body}");
                            assert_eq!(
                                rows.undecided_rows().count(),
                                if undecided { 2 } else { 0 },
                                "{body}"
                            );
                            for row in rows.rows() {
                                row.evidence()
                                    .with_record(|record| {
                                        assert_eq!(
                                            record.information().fallback(),
                                            if negated && closed {
                                                information.negate()
                                            } else {
                                                information
                                            },
                                            "{body}"
                                        );
                                        assert_eq!(
                                            record.completeness(),
                                            if undecided {
                                                SearchCompleteness::Undetermined
                                            } else {
                                                SearchCompleteness::Complete
                                            },
                                            "{body}"
                                        );
                                    })
                                    .expect("original existential dictionary owner");
                            }
                        },
                    )
                    .expect("native existential VM and cache route");
            }
        }
    }
    let mut mixed = RdfDatasetBuilder::new();
    let subject = mixed.intern_iri("https://example.org/item").unwrap();
    let predicate = mixed.intern_iri("https://example.org/p").unwrap();
    let positive = mixed.intern_iri("https://example.org/positive").unwrap();
    let mixed_negative = mixed.intern_iri("https://example.org/negative").unwrap();
    mixed.push_quad(subject, predicate, positive, None).unwrap();
    mixed
        .push_quad(subject, predicate, mixed_negative, None)
        .unwrap();
    let mixed = mixed
        .freeze()
        .expect("original conflicting existential witnesses");
    let source = Claims {
        negative: mixed_negative,
        record: EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Opposed),
            SearchCompleteness::Complete,
            &Frame::default(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .expect("original negative existential witness"),
        fail: false,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    for body in [
        "FILTER EXISTS { ?s <https://example.org/p> ?o }",
        "FILTER NOT EXISTS { ?s <https://example.org/p> ?o }",
        "BIND(EXISTS { ?s <https://example.org/p> ?o } AS ?found)",
        "FILTER EXISTS { FILTER EXISTS { ?s <https://example.org/p> ?o } }",
    ] {
        let query =
            format!("SELECT ?s WHERE {{ VALUES ?s {{ <https://example.org/item> }} {body} }}");
        engine
            .query_interned_evidence_view(
                &mixed,
                request(&query),
                QueryOptions::default(),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("original mixed-witness existential result");
                    };
                    assert_eq!(rows.established_rows().count(), 1, "{body}");
                    rows.rows()[0]
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(
                                record.information().fallback(),
                                Information::Both,
                                "{body}"
                            );
                        })
                        .expect("every original witness participates in existential information");
                },
            )
            .expect("conflicting existential witnesses are not first-match booleans");
    }
    let source = Claims {
        negative,
        record: EvidenceRecord::default(),
        fail: true,
        completeness: SearchCompleteness::Complete,
        sum_nonnegative: false,
        closure_declared: true,
    };
    engine.query_interned_evidence_view(
        &dataset,
        request("SELECT ?s WHERE { VALUES ?s { <https://example.org/item> } FILTER IF(true, true, EXISTS { ?s <https://example.org/p> ?o }) }"),
        QueryOptions::default(), &source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("original conditional result"); };
            assert_eq!(rows.len(), 1, "the skipped native branch must not read its failing source");
        },
    ).expect("native short circuit remains unchanged");
    let refused: Result<(), _> = engine.query_interned_evidence_view(
        &dataset,
        request("SELECT ?s WHERE { VALUES ?s { <https://example.org/item> } FILTER NOT EXISTS { ?s <https://example.org/p> ?o } }"),
        QueryOptions::default(), &source, |_| panic!("hard source refusal cannot publish a semantic unknown"),
    );
    let refused = refused.expect_err("existential source failure remains hard");
    assert_eq!(refused.code, "native-sparql-workspace-unpriced");
    assert!(refused.message.contains("original claim source refusal"));
}

fn actual_unknown_answer_exposes_queryable_original_rdf_ledger() {
    use purrdf_core::evidence::{
        FrameVocabulary, LedgerDataset, LedgerNodeKey, LedgerNodes, LedgerVocabulary,
        ReasonVocabulary, RowVocabulary,
    };
    use purrdf_core::{DatasetView, SparqlRequest, TermValue};
    use purrdf_sparql_eval::evidence_exec::{AcceptedLedgerRead, LedgerRowTerms};
    #[derive(Debug)]
    struct Nodes([(LedgerNodeKey, TermValue); 2]);
    impl LedgerNodes for Nodes {
        fn node(&self, key: LedgerNodeKey) -> Option<&TermValue> {
            self.0
                .iter()
                .find(|entry| entry.0 == key)
                .map(|entry| &entry.1)
        }
    }
    let (dataset, source) = gap_dataset();
    let nodes = Nodes([
        (
            LedgerNodeKey::Reason(source.reason),
            TermValue::iri("https://example.org/canonical-reason"),
        ),
        (
            LedgerNodeKey::Frame(Frame::actual().identity()),
            TermValue::iri("https://example.org/actual-frame"),
        ),
    ]);
    let row_v: [TermValue; 22] =
        core::array::from_fn(|index| TermValue::iri(format!("https://example.org/row-v/{index}")));
    let extra: [TermValue; 14] = core::array::from_fn(|index| {
        TermValue::iri(format!("https://example.org/extra-v/{index}"))
    });
    let vocabulary = LedgerVocabulary {
        row: RowVocabulary {
            completeness: &row_v[0],
            information: &row_v[1],
            information_context: &row_v[2],
            standpoint: &row_v[3],
            frame: &row_v[4],
            reason: &row_v[5],
            accepted_reason: &row_v[6],
            aggregate: &row_v[7],
            column: &row_v[8],
            bound: &row_v[9],
            unscoped: &row_v[10],
            information_values: [&row_v[11], &row_v[12], &row_v[13], &row_v[14]],
            completeness_values: [&row_v[15], &row_v[16], &row_v[17]],
            bound_values: [&row_v[18], &row_v[19], &row_v[20], &row_v[21]],
        },
        reason: ReasonVocabulary {
            cause: &extra[0],
            constraint: &extra[1],
            vault: &extra[2],
            watermark: &extra[3],
        },
        frame: FrameVocabulary {
            mode: &extra[4],
            alternative: &extra[5],
            components: [&extra[6], &extra[7], &extra[8], &extra[9], &extra[10]],
            actual: &extra[11],
            conjunction: &extra[12],
            disjunction: &extra[13],
        },
    };
    let row_node = TermValue::iri("https://example.org/original-row");
    let solution = TermValue::Triple {
        s: purrdf_core::TermBox::new(TermValue::iri("https://example.org/missing")),
        p: purrdf_core::TermBox::new(TermValue::iri("https://example.org/kind")),
        o: purrdf_core::TermBox::new(TermValue::iri("https://example.org/kind")),
    };
    let engine = NativeSparqlEngine::new();
    use purrdf_core::evidence::{
        AnswerRequest, AnswerTier, StandpointKey, WitnessQuestion, WitnessVerdict,
    };
    for form in [
        "SELECT ?s",
        "ASK",
        "CONSTRUCT { ?s <https://example.org/result> <https://example.org/established> }",
        "DESCRIBE ?s",
    ] {
        let query = format!(
            "{form} WHERE {{ ?s <https://example.org/kind> ?kind OPTIONAL {{ ?s <https://example.org/value> ?value }} FILTER(?value = <https://example.org/required>) }}"
        );
        engine.query_evidence_answer_view(&dataset, request(&query),
        QueryOptions::default(), &source, AnswerRequest {
            tier: AnswerTier::Ledger,
            question: Some(WitnessQuestion::Support),
            standpoint: StandpointKey::Unscoped,
            input_completeness: SearchCompleteness::Complete,
            closure_declared: true,
        }, |outcome, rows, status| {
            assert_eq!((status.established, status.undecided), (0, 1));
            assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
            match outcome {
                InternedOutcome::Solutions(output) => assert_eq!(output.len(), 1),
                InternedOutcome::Boolean(value) => assert!(!value),
                InternedOutcome::Graph(graph) => assert_eq!(graph.dataset_handle().quad_count(), 0),
            }
            assert_eq!(rows.undecided_rows().count(), 1);
            assert!(rows.with_evidence_ledger(&[], &nodes, &vocabulary, |_| ()).is_err(), "missing original row declaration must refuse");
            rows.with_evidence_ledger(&[LedgerRowTerms { node: &row_node, solution: &solution }],
                &nodes, &vocabulary, |ledger| {
                    let view = LedgerDataset::new(ledger, Resident).expect("borrowed actual ledger source");
                    assert!(view.quads().all(|quad| quad.g.is_none()), "ledger annotations use the answer graph");
                    let query = "SELECT ?reason ?cause WHERE { ?row <https://example.org/row-v/5> ?reason . ?reason <https://example.org/extra-v/0> ?cause }";
                    let result = engine.query_fallible_view(&view, SparqlRequest { query, base_iri: None, substitutions: &[] }, QueryOptions::default()).expect("same evaluator queries actual answer ledger");
                    let (_, result) = result.result.solutions().expect("ledger SELECT result");
                    assert_eq!(result.len(), 1);
                    assert_eq!(result[0][1], Some(source.cause.clone()));
                    assert_eq!(view.read_error(), None);
                }).expect("original evidence dictionary ledger publication");
            rows.with_accepted_rows(&[], |merged| assert_eq!(merged.count(), 0))
                .expect("no implicit override");
            assert!(rows.with_accepted_rows(&[source.reason, source.reason], |_| ())
                .is_err(), "repeated acceptance must refuse before publication");
            rows.with_accepted_rows(&[source.reason], |mut merged| {
                let accepted = merged.next().expect("original accepted row").expect("valid original owner");
                assert!(std::ptr::eq(std::ptr::from_ref(accepted.row), std::ptr::from_ref(&rows.rows()[0])));
                assert!(accepted.is_accepted());
                assert_eq!(accepted.accepted_reasons().collect::<Vec<_>>(), [source.reason]);
                assert_eq!(accepted.evidence.completeness(), SearchCompleteness::Undetermined);
                assert_eq!(accepted.evidence.reasons(), &[source.reason]);
                let original = purrdf_hash::hex::Digest32::new([7; 32]);
                assert_ne!(accepted.identity(original), original);
                assert!(merged.next().is_none());
            }).expect("native accepted merge");
            let terms = [LedgerRowTerms { node: &row_node, solution: &solution }];
            for accepted in [&[][..], &[source.reason][..]] {
                rows.with_accepted_evidence_ledger(&AcceptedLedgerRead {
                    accepted, terms: &terms, nodes: &nodes, vocabulary: &vocabulary,
                }, |ledger| {
                    let view = LedgerDataset::new(ledger, Resident).expect("original merged ledger");
                    if accepted.is_empty() {
                        assert_eq!(view.quads().count(), 0);
                        return;
                    }
                    let query = "SELECT ?reason ?state WHERE { ?row <https://example.org/row-v/6> ?reason . ?row <https://example.org/row-v/0> ?state }";
                    let result = engine.query_fallible_view(&view, SparqlRequest { query, base_iri: None, substitutions: &[] }, QueryOptions::default()).expect("query visible acceptance annotation");
                    let (_, result) = result.result.solutions().expect("merged ledger SELECT");
                    assert_eq!(result.len(), 1);
                    assert_eq!(result[0][0], Some(nodes.0[0].1.clone()));
                    assert_eq!(result[0][1], Some(row_v[16].clone()), "acceptance cannot promote semantic completeness");
                }).expect("native merged ledger publication");
            }
        }).expect("actual query retains original ledger owners");
    }

    // Exercise dictionary sharing and safe projection through the same engine,
    // using actual undecided query rows rather than a synthetic ledger source.
    let ordinals = (0..2_279)
        .map(|ordinal| ordinal.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let query = format!(
        "SELECT ?s ?ordinal WHERE {{ ?s <https://example.org/kind> ?kind OPTIONAL {{ ?s <https://example.org/value> ?value }} FILTER(?value = <https://example.org/required>) VALUES ?ordinal {{ {ordinals} }} }} ORDER BY ?ordinal"
    );
    let row_nodes: Vec<_> = (0..2_279)
        .map(|ordinal| TermValue::iri(format!("https://example.org/answer/{ordinal}")))
        .collect();
    let solutions: Vec<_> = (0..2_279)
        .map(|ordinal| TermValue::Triple {
            s: purrdf_core::TermBox::new(TermValue::iri("https://example.org/missing")),
            p: purrdf_core::TermBox::new(TermValue::iri("https://example.org/ordinal")),
            o: purrdf_core::TermBox::new(TermValue::integer(ordinal)),
        })
        .collect();
    let terms: Vec<_> = row_nodes
        .iter()
        .zip(&solutions)
        .map(|(node, solution)| LedgerRowTerms { node, solution })
        .collect();
    engine.query_interned_evidence_view(&dataset, request(&query), QueryOptions::default(), &source, |outcome| {
        let InternedOutcome::Solutions(rows) = outcome else { panic!("actual SELECT rows"); };
        assert_eq!(rows.len(), 2_279);
        assert_eq!(rows.undecided_rows().count(), 2_279);
        let ordinal_column = rows.column("ordinal").expect("original ordinal binding");
        for (row, solution) in rows.rows().iter().zip(&solutions) {
            let TermValue::Triple { o, .. } = solution else { panic!("genuine solution triple term"); };
            assert_eq!(rows.try_value_of(row[ordinal_column].expect("original ordinal")).expect("original retained value"), **o);
        }
        rows.with_evidence_ledger(&terms, &nodes, &vocabulary, |ledger| {
            let view = LedgerDataset::new(ledger, Resident).expect("original population ledger");
            let result = engine.query_fallible_view(&view, SparqlRequest {
                query: "SELECT ?reason (COUNT(?row) AS ?count) WHERE { ?row <https://example.org/row-v/5> ?reason } GROUP BY ?reason",
                base_iri: None, substitutions: &[],
            }, QueryOptions::default()).expect("production GROUP BY over original ledger");
            let (_, grouped) = result.result.solutions().expect("grouped reason rows");
            assert_eq!(grouped.len(), 1, "all original rows share one reason node");
            assert_eq!(grouped[0][0], Some(nodes.0[0].1.clone()));
            let Some(TermValue::Literal { lexical_form, .. }) = &grouped[0][1] else { panic!("exact COUNT"); };
            assert_eq!(lexical_form, "2279");

            let projected = engine.query_fallible_view(&view, SparqlRequest {
                query: "CONSTRUCT { ?row <https://example.org/row-v/0> ?state . ?reason <https://example.org/extra-v/0> ?cause } WHERE { ?row <https://example.org/row-v/0> ?state . ?row <https://example.org/row-v/5> ?reason . ?reason <https://example.org/extra-v/0> ?cause }",
                base_iri: None, substitutions: &[],
            }, QueryOptions::default()).expect("production privacy projection");
            let graph = projected.result.graph().expect("CONSTRUCT graph");
            assert_eq!(graph.quad_count(), 2_280, "original incomplete rows plus one retained cause");
            let retained = engine.query_fallible_view(graph, SparqlRequest {
                query: "SELECT ?row ?state ?cause WHERE { ?row <https://example.org/row-v/0> ?state . <https://example.org/canonical-reason> <https://example.org/extra-v/0> ?cause }",
                base_iri: None, substitutions: &[],
            }, QueryOptions::default()).expect("query independently retained incompleteness and cause");
            let (_, retained) = retained.result.solutions().expect("projected evidence rows");
            assert_eq!(retained.len(), 2_279);
            assert!(retained.iter().all(|row| row[1] == Some(row_v[16].clone()) && row[2] == Some(source.cause.clone())));
            let withheld = engine.query_fallible_view(graph, SparqlRequest {
                query: "SELECT ?row WHERE { ?row <https://example.org/row-v/5> ?reason }",
                base_iri: None, substitutions: &[],
            }, QueryOptions::default()).expect("query withheld reason links");
            assert_eq!(withheld.result.solutions().expect("withheld-link rows").1.len(), 0);
            assert_eq!(view.read_error(), None);
        }).expect("original population publication");
    }).expect("actual undecided population query");
}

fn acceptance_merges_original_order_without_replaying_cells_or_losing_scores() {
    let (dataset, source) = gap_dataset();
    let engine = NativeSparqlEngine::new();
    engine.query_interned_evidence_view(&dataset,
        request("SELECT ?s ?score WHERE { { ?s <https://example.org/kind> ?kind OPTIONAL { ?s <https://example.org/value> ?value } FILTER(?value = <https://example.org/required>) BIND(2 AS ?score) } UNION { VALUES (?s ?score) { (<https://example.org/a> 1) (<https://example.org/z> 3) } } } ORDER BY ?score"),
        QueryOptions::default(), &source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("SELECT result"); };
            assert_eq!(rows.len(), 3);
            for (accepted, expected) in [(&[][..], &[0, 2][..]), (&[source.reason][..], &[0, 1, 2][..])] {
                rows.with_accepted_rows(accepted, |merged| {
                    let mut positions = Vec::new();
                    for item in merged {
                        let item = item.expect("original merge law");
                        let position = usize::try_from(item.position).expect("original ordinal");
                        assert!(std::ptr::eq(std::ptr::from_ref(item.row), std::ptr::from_ref(&rows.rows()[position])));
                        assert_eq!(item.is_accepted(), position == 1);
                        let score = item.row[rows.column("score").expect("original score column")].expect("retained score");
                        let purrdf_core::TermValue::Literal { lexical_form, .. } = rows.try_value_of(score).expect("original native score") else { panic!("integer score"); };
                        assert_eq!(lexical_form, (position + 1).to_string());
                        positions.push(position);
                    }
                    assert_eq!(positions, expected);
                }).expect("ordered native acceptance");
            }
        }).expect("original ordered query");
}

fn native_paths_keep_derivation_polarity_and_complete_found_witnesses() {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri("https://example.org/a").unwrap();
    let b = builder.intern_iri("https://example.org/b").unwrap();
    let c = builder.intern_iri("https://example.org/c").unwrap();
    let z = builder.intern_iri("https://example.org/z").unwrap();
    let end = builder.intern_iri("https://example.org/end").unwrap();
    let p = builder.intern_iri("https://example.org/p").unwrap();
    let q = builder.intern_iri("https://example.org/q").unwrap();
    for (s, predicate, o) in [
        (a, p, b),
        (a, p, c),
        (b, p, z),
        (c, p, z),
        (z, p, end),
        (end, p, a),
        (b, q, end),
        (c, q, end),
    ] {
        builder.push_quad(s, predicate, o, None).unwrap();
    }
    let dataset = builder.freeze().expect("native cyclic path dataset");
    let source = Claims {
        negative: b,
        record: EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Opposed),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .expect("original incomplete opposed claim"),
        fail: false,
        completeness: SearchCompleteness::Undetermined,
        sum_nonnegative: false,
        closure_declared: false,
    };
    let engine = NativeSparqlEngine::new();
    engine
        .query_interned_evidence_view(
            &dataset,
            request("SELECT ?s WHERE { ?s <https://example.org/p> <https://example.org/b> }"),
            QueryOptions::default(),
            &source,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("original claim SELECT");
                };
                assert_eq!(rows.len(), 1);
                rows.rows()[0]
                    .evidence()
                    .with_record(|record| {
                        assert_eq!(
                            record.information().fallback(),
                            Information::Opposed,
                            "the original edge declaration must reach a direct claim match"
                        );
                        assert_eq!(record.completeness(), SearchCompleteness::Complete);
                    })
                    .expect("original matched claim evidence");
            },
        )
        .expect("original claim source control");
    for (path, expected) in [
        (
            "<https://example.org/p>/<https://example.org/q>",
            &[Information::Opposed, Information::Supported][..],
        ),
        ("<https://example.org/p>+", &[Information::Both][..]),
        ("<https://example.org/p>{3,3}", &[Information::Both][..]),
        ("<https://example.org/p>{67,67}", &[Information::Both][..]),
    ] {
        let query = format!(
            "SELECT ?s WHERE {{ BIND(<https://example.org/a> AS ?s) ?s {path} <https://example.org/end> }}"
        );
        // The same cyclic/duplicate derivations without a declaration must
        // retain their original bag cardinality through the ordinary path.
        engine
            .query_interned_view(
                &dataset,
                request(&query),
                QueryOptions::default(),
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("ordinary path SELECT");
                    };
                    assert_eq!(rows.len(), expected.len(), "ordinary {path}");
                    for row in rows.rows() {
                        assert!(row.evidence().is_default());
                    }
                },
            )
            .expect("ordinary path family");
        engine
            .query_interned_evidence_view(
                &dataset,
                request(&query),
                QueryOptions::default(),
                &source,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("path SELECT");
                    };
                    let mut observed = Vec::new();
                    for row in rows.rows() {
                        row.evidence()
                            .with_record(|record| {
                                assert_eq!(
                                    record.completeness(),
                                    SearchCompleteness::Complete,
                                    "missing edges elsewhere cannot weaken a found path"
                                );
                                assert!(record.frame().is_unrestricted_actual());
                                observed.push(record.information().fallback());
                            })
                            .expect("actual path evidence owner");
                    }
                    observed.sort();
                    let mut expected = expected.to_vec();
                    expected.sort();
                    assert_eq!(observed, expected, "{path}");
                },
            )
            .expect("native path family");
    }
    let owned_source = Claims {
        negative: b,
        record: EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap(),
        fail: false,
        completeness: SearchCompleteness::Undetermined,
        sum_nonnegative: false,
        closure_declared: false,
    };
    for path in [
        "<https://example.org/p>/<https://example.org/q>",
        "<https://example.org/p>+",
        "<https://example.org/p>{3,3}",
    ] {
        engine.query_interned_evidence_view(&dataset, request(&format!(
            "SELECT ?s WHERE {{ BIND(<https://example.org/a> AS ?s) ?s {path} <https://example.org/end> }}"
        )), QueryOptions::default(), &owned_source, |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("owned default path"); };
            assert!(!rows.is_empty());
            assert!(rows.rows().iter().any(|row| !row.evidence().is_virtual()));
            for row in rows.rows() {
                assert!(row.evidence().is_default());
                row.evidence().with_record(|record| assert_eq!(record, &EvidenceRecord::default())).unwrap();
            }
        }).expect("original complete default owner survives path derivations");
    }
    let mut refusing = source;
    refusing.fail = true;
    let mut published = false;
    let failed = engine.query_interned_evidence_view(
        &dataset,
        request("SELECT ?s WHERE { ?s <https://example.org/p>+ <https://example.org/end> }"),
        QueryOptions::default(),
        &refusing,
        |_| {
            published = true;
        },
    );
    assert!(failed.is_err());
    assert!(!published, "path source refusal must precede publication");
}

fn native_property_rows_retain_original_declarations_through_normal_ingestion() {
    use purrdf_core::TermValue;
    use purrdf_sparql_eval::{
        AdmittedPfRow, BindingPattern, ExtensionEnv, MemoryRelation, PfArgs, PfArity, PfCursor,
        PropertyFunction, PropertyFunctionRegistry, QueryEvidence, Volatility, WorkspaceCapability,
    };
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    enum SearchVisibility {
        Opening,
        AfterPull,
        OpeningOnly,
    }
    #[derive(Clone, Copy)]
    enum Membership {
        Established,
        Undecided,
        OwnedComplete,
    }
    struct DeclaredRelation {
        inner: MemoryRelation,
        record: EvidenceRecord,
        rank: Option<core::num::NonZeroU64>,
        membership: Membership,
        alternative: Option<EvidenceRecord>,
        search: Option<EvidenceRecord>,
        fail_search: bool,
        visibility: SearchVisibility,
        unclosed_search: bool,
        query: Arc<std::sync::Mutex<Option<QueryEvidence>>>,
    }
    struct DeclaredCursor {
        row: Option<AdmittedPfRow>,
        alternative: Option<AdmittedPfRow>,
        search: Option<purrdf_sparql_eval::RowEvidence>,
        fail_search: bool,
        visibility: SearchVisibility,
    }
    impl PfCursor for DeclaredCursor {
        fn carries_row_evidence(&self) -> bool {
            true
        }
        fn search_evidence(&self) -> Result<Option<&purrdf_sparql_eval::RowEvidence>, EvalError> {
            if self.fail_search
                && (!matches!(self.visibility, SearchVisibility::OpeningOnly) || self.row.is_some())
            {
                return Err(EvalError::AllocationFailed {
                    construct: "original native search declaration",
                });
            }
            Ok(
                if matches!(self.visibility, SearchVisibility::AfterPull) && self.row.is_some() {
                    None
                } else {
                    self.search.as_ref()
                },
            )
        }
        fn next(&mut self) -> Result<Option<Vec<TermValue>>, EvalError> {
            Err(EvalError::WorkspaceUnpriced(
                "original evidence requires its envelope",
            ))
        }
        fn next_admitted(&mut self) -> Result<Option<AdmittedPfRow>, EvalError> {
            let row = self.row.take().or_else(|| self.alternative.take());
            if matches!(self.visibility, SearchVisibility::OpeningOnly) {
                self.search = None;
            }
            Ok(row)
        }
    }
    impl PropertyFunction for DeclaredRelation {
        fn volatility(&self) -> Volatility {
            self.inner.volatility()
        }
        fn arity(&self) -> PfArity {
            self.inner.arity()
        }
        fn modes(&self) -> &[BindingPattern] {
            self.inner.modes()
        }
        fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
            self.inner.rows_per_invocation(mode)
        }
        fn open<'cursor>(
            &self,
            args: &PfArgs<'_>,
            ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor + 'cursor>, EvalError>
        where
            Self: 'cursor,
        {
            self.open_admitted(args, ceiling, WorkspaceCapability::resident())
                .and_then(purrdf_sparql_eval::WorkspaceCursor::into_resident)
        }
        fn open_admitted<'cursor>(
            &self,
            args: &PfArgs<'_>,
            _ceiling: Option<u64>,
            workspace: WorkspaceCapability,
        ) -> Result<purrdf_sparql_eval::WorkspaceCursor<'cursor>, EvalError>
        where
            Self: 'cursor,
        {
            let query = {
                let mut retained = self
                    .query
                    .lock()
                    .map_err(|_| EvalError::internal("poisoned fixture query owner"))?;
                if retained.is_none() {
                    *retained = Some(QueryEvidence::new(&workspace)?);
                }
                retained.as_ref().expect("admitted query owner").clone()
            };
            let evidence = match self.membership {
                Membership::Undecided => query.retain_undecided(&self.record)?,
                Membership::Established => query.retain(&self.record)?,
                Membership::OwnedComplete => query.with_completeness(
                    &query.retain(&self.record)?,
                    SearchCompleteness::Complete,
                    None,
                )?,
            };
            let evidence = self.rank.map_or_else(
                || evidence.clone(),
                |rank| evidence.clone().with_original_rank(rank),
            );
            let make_row = |evidence| {
                AdmittedPfRow::from_terms(
                    [
                        workspace.iri("https://example.org/document")?,
                        workspace.clone_term(args.get(1).expect("bound original needle"))?,
                    ]
                    .into_iter(),
                    &workspace,
                )
                .map(|row| row.with_evidence(evidence))
            };
            let row = make_row(evidence)?;
            let alternative = self
                .alternative
                .as_ref()
                .map(|record| make_row(query.retain(record)?))
                .transpose()?;
            purrdf_sparql_eval::WorkspaceCursor::new(
                DeclaredCursor {
                    row: (!self.inner.rows().is_empty()).then_some(row),
                    alternative,
                    search: self
                        .search
                        .as_ref()
                        .map(|record| {
                            query
                                .retain(record)
                                .map(|search| search.with_unclosed_search(self.unclosed_search))
                        })
                        .transpose()?,
                    fail_search: self.fail_search,
                    visibility: self.visibility,
                },
                &workspace,
                "declared producer cursor",
            )
        }
    }
    // One admitted owner per actual query, shared by its repeated invocations.
    // Clear it at each entry below before a new query account is installed.
    let native_owner = Arc::new(std::sync::Mutex::new(None));
    let record = EvidenceRecord::from_parts(
        &InformationMap::uniform(Information::Opposed),
        SearchCompleteness::Complete,
        &Frame::actual(),
        &[],
        &[(
            purrdf_core::evidence::aggregate_column_identity("hidden"),
            purrdf_core::evidence::AggregateBound::Lower,
        )],
        &mut Memory::new(&mut Resident),
    )
    .unwrap();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "https://example.org/declared",
        Arc::new(DeclaredRelation {
            inner: MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri("https://example.org/document"),
                    TermValue::simple_literal("needle"),
                ]],
            )
            .unwrap(),
            record,
            rank: None,
            membership: Membership::Established,
            alternative: None,
            search: None,
            fail_search: false,
            visibility: SearchVisibility::Opening,
            unclosed_search: false,
            query: Arc::clone(&native_owner),
        }),
    );
    let env = ExtensionEnv::over_relations(registry).unwrap();
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    NativeSparqlEngine::new().query_interned_view(
        &dataset,
        request("SELECT DISTINCT ?doc ?kept WHERE { ?doc <https://example.org/declared> \"needle\" BIND(true AS ?kept) FILTER(?kept) }"),
        QueryOptions::default().with_env(&env),
        |outcome| {
            let InternedOutcome::Solutions(rows) = outcome else { panic!("native SELECT"); };
            assert_eq!(rows.len(), 1);
            rows.rows()[0].evidence().with_record(|record| {
                assert_eq!(record.information().fallback(), Information::Opposed);
                assert_eq!(record.completeness(), SearchCompleteness::Complete);
                assert_eq!(record.aggregate_bounds(), []);
            }).unwrap();
        },
    ).expect("original producer envelope reaches normal row operators");

    // Native producers carry declarations even when the ordinary entry has no
    // external EvidenceSource. A later established witness must improve the
    // same binding before DISTINCT publishes it, including through grouping.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "https://example.org/declared",
        Arc::new(DeclaredRelation {
            inner: MemoryRelation::new(
                1,
                1,
                vec![
                    vec![
                        TermValue::iri("https://example.org/document"),
                        TermValue::simple_literal("needle"),
                    ],
                    vec![
                        TermValue::iri("https://example.org/document"),
                        TermValue::simple_literal("needle"),
                    ],
                ],
            )
            .unwrap(),
            record: EvidenceRecord::from_parts(
                &InformationMap::uniform(Information::Opposed),
                SearchCompleteness::Undetermined,
                &Frame::actual(),
                &[],
                &[],
                &mut Memory::new(&mut Resident),
            )
            .unwrap(),
            rank: None,
            membership: Membership::Undecided,
            alternative: Some(EvidenceRecord::default()),
            search: None,
            fail_search: false,
            visibility: SearchVisibility::Opening,
            unclosed_search: false,
            query: Arc::clone(&native_owner),
        }),
    );
    let env = ExtensionEnv::over_relations(registry).unwrap();
    for query in [
        "SELECT DISTINCT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" }",
        "SELECT (COUNT(?doc) AS ?count) WHERE { { SELECT DISTINCT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" } } }",
        "SELECT ?value WHERE { BIND(EXISTS { ?doc <https://example.org/declared> \"needle\" } AS ?value) }",
        "SELECT ?value WHERE { BIND(IF(true, EXISTS { ?doc <https://example.org/declared> \"needle\" }, false) AS ?value) }",
        "SELECT ?value WHERE { BIND(NOT EXISTS { ?doc <https://example.org/declared> \"needle\" } AS ?value) }",
        "SELECT ?doc WHERE { VALUES ?doc { <https://example.org/document> } FILTER EXISTS { ?doc <https://example.org/declared> \"needle\" } }",
        "SELECT DISTINCT ?doc WHERE { VALUES ?doc { <https://example.org/document> <https://example.org/document> } FILTER EXISTS { ?doc <https://example.org/declared> \"needle\" } }",
        "SELECT ?doc WHERE { VALUES ?doc { <https://example.org/document> } FILTER EXISTS { VALUES ?seed { 1 } FILTER EXISTS { ?doc <https://example.org/declared> \"needle\" } } }",
        "SELECT ?doc WHERE { VALUES ?doc { <https://example.org/document> } MINUS { ?doc <https://example.org/declared> \"needle\" } }",
    ] {
        let check = |outcome: InternedOutcome<'_, '_, _>| {
            let InternedOutcome::Solutions(rows) = outcome else {
                panic!("native alternatives SELECT");
            };
            assert_eq!(rows.len(), 1);
            assert!(!rows.rows()[0].evidence().is_undecided());
            rows.rows()[0]
                .evidence()
                .with_record(|record| {
                    assert_eq!(record.information().fallback(), Information::Both);
                    assert_eq!(record.completeness(), SearchCompleteness::Complete);
                    if query.contains("COUNT") {
                        assert_eq!(
                            record.aggregate_bounds(),
                            &[(
                                purrdf_core::evidence::aggregate_column_identity("count"),
                                purrdf_core::evidence::AggregateBound::Exact,
                            )]
                        );
                    }
                })
                .unwrap();
            rows.existential_evidence()
                .unwrap()
                .with_record(|record| {
                    assert_eq!(record.information().fallback(), Information::Both);
                    assert_eq!(record.completeness(), SearchCompleteness::Complete);
                })
                .unwrap();
        };
        for exists_memo in [false, true] {
            native_owner.lock().unwrap().take();
            NativeSparqlEngine::new()
                .with_eval_options(purrdf_sparql_eval::EvalOptions {
                    exists_memo,
                    ..purrdf_sparql_eval::EvalOptions::default()
                })
                .query_interned_view(
                    &dataset,
                    request(query),
                    QueryOptions::default().with_env(&env),
                    check,
                )
                .expect("native alternative declarations reach ordinary DISTINCT and grouping");
        }
        native_owner.lock().unwrap().take();
        let state = Arc::new(purrdf_sparql_eval::GovernorState::new(
            &purrdf_sparql_eval::QueryGovernors::METERED,
        ));
        let result = NativeSparqlEngine::new()
            .query_governed_interned_in_operation(
                &dataset,
                request(query),
                QueryOptions::default().with_env(&env),
                &state,
                check,
            )
            .expect("admitted native alternatives use the same operator homes");
        assert!(matches!(
            result,
            purrdf_sparql_eval::InternedGoverned::Complete { .. }
        ));
    }

    let engine = NativeSparqlEngine::new();
    let mut functions = purrdf_sparql_eval::user_fn::UserFunctionRegistry::default();
    for (iri, kind, body) in [
        (
            "https://example.org/native-select",
            purrdf_sparql_eval::user_fn::UserFnBody::Select,
            "SELECT DISTINCT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" }",
        ),
        (
            "https://example.org/native-ask",
            purrdf_sparql_eval::user_fn::UserFnBody::Ask,
            "ASK { ?doc <https://example.org/declared> \"needle\" }",
        ),
        (
            "https://example.org/native-nested",
            purrdf_sparql_eval::user_fn::UserFnBody::Select,
            "SELECT (<https://example.org/native-select>() AS ?doc) WHERE {}",
        ),
        (
            "https://example.org/native-empty",
            purrdf_sparql_eval::user_fn::UserFnBody::Ask,
            "ASK { VALUES ?doc {} }",
        ),
    ] {
        functions.insert(
            iri,
            purrdf_sparql_eval::user_fn::UserFunction {
                params: Vec::new(),
                required: 0,
                body: Arc::from(body),
                kind,
                return_constraint: purrdf_sparql_eval::user_fn::TypeConstraint::default(),
            },
        );
    }
    let functions = engine.bind_functions(functions, &env).unwrap();
    for iri in [
        "https://example.org/native-select",
        "https://example.org/native-ask",
        "https://example.org/native-nested",
    ] {
        for expression in [format!("<{iri}>()"), format!("IF(true, <{iri}>(), 0)")] {
            native_owner.lock().unwrap().take();
            let query = format!(
                "SELECT DISTINCT ?value WHERE {{ BIND({expression} AS ?value) FILTER(BOUND(?value)) }}"
            );
            engine.query_interned_view(&dataset, request(&query), QueryOptions::default().with_env(&env).with_functions(&functions), |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else { panic!("native function declaration"); };
                assert_eq!(rows.len(), 1);
                assert!(!rows.rows()[0].evidence().is_undecided());
                rows.rows()[0].evidence().with_record(|record| {
                    assert_eq!(record.information().fallback(), Information::Both);
                    assert_eq!(record.completeness(), SearchCompleteness::Complete);
                }).unwrap();
            }).expect("native producer declarations survive SELECT/ASK/nested function suspension without an external source");
        }
    }

    for expression in [
        "EXISTS { VALUES ?doc {} }",
        "<https://example.org/native-empty>()",
        "IF(true, <https://example.org/native-empty>(), true)",
    ] {
        native_owner.lock().unwrap().take();
        let query = format!("SELECT ?value WHERE {{ BIND({expression} AS ?value) }}");
        engine
            .query_interned_view(
                &dataset,
                request(&query),
                QueryOptions::default()
                    .with_env(&env)
                    .with_functions(&functions),
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("empty native witness");
                    };
                    assert_eq!(rows.len(), 1);
                    rows.rows()[0]
                        .evidence()
                        .with_record(|record| {
                            assert_eq!(record.information().fallback(), Information::Opposed);
                            assert_eq!(record.completeness(), SearchCompleteness::Complete);
                        })
                        .unwrap();
                },
            )
            .expect("empty native ASK and direct EXISTS share the absence witness law");
    }

    // Exercise the actual native producer even when it emits no row. Complete
    // closed absence is a false EXISTS witness and a true NOT EXISTS witness;
    // neither an open producer nor an open external scope grants that polarity.
    let mut scope_terms = RdfDatasetBuilder::new();
    let unused_claim = scope_terms
        .intern_iri("https://example.org/unused-claim")
        .unwrap();
    for (completeness, unclosed_search) in [
        (SearchCompleteness::Complete, false),
        (SearchCompleteness::Complete, true),
        (SearchCompleteness::Undetermined, false),
    ] {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            "https://example.org/declared",
            Arc::new(DeclaredRelation {
                inner: MemoryRelation::new(1, 1, Vec::new()).unwrap(),
                record: EvidenceRecord::default(),
                rank: None,
                membership: Membership::Established,
                alternative: None,
                search: Some(
                    EvidenceRecord::from_parts(
                        &InformationMap::default(),
                        completeness,
                        &Frame::actual(),
                        &[],
                        &[],
                        &mut Memory::new(&mut Resident),
                    )
                    .unwrap(),
                ),
                fail_search: false,
                visibility: SearchVisibility::Opening,
                unclosed_search,
                query: Arc::clone(&native_owner),
            }),
        );
        let env = ExtensionEnv::over_relations(registry).unwrap();
        for scope in [
            None,
            Some((SearchCompleteness::Complete, true)),
            Some((SearchCompleteness::Complete, false)),
            Some((SearchCompleteness::Undetermined, true)),
        ] {
            let (scope_completeness, closure_declared) =
                scope.unwrap_or((SearchCompleteness::Complete, true));
            let source = Claims {
                negative: unused_claim,
                record: EvidenceRecord::default(),
                fail: false,
                completeness: scope_completeness,
                sum_nonnegative: false,
                closure_declared,
            };
            let closed = completeness == SearchCompleteness::Complete
                && !unclosed_search
                && scope_completeness == SearchCompleteness::Complete
                && closure_declared;
            for (body, drops_closed) in [
                (
                    "FILTER EXISTS { ?doc <https://example.org/declared> \"needle\" }",
                    true,
                ),
                (
                    "FILTER NOT EXISTS { ?doc <https://example.org/declared> \"needle\" }",
                    false,
                ),
                (
                    "MINUS { VALUES ?s { 1 2 } FILTER NOT EXISTS { ?doc <https://example.org/declared> \"needle\" } }",
                    true,
                ),
            ] {
                let query = format!("SELECT ?s WHERE {{ VALUES ?s {{ 1 2 }} {body} }}");
                let check = |outcome: InternedOutcome<'_, '_, _>| {
                    let InternedOutcome::Solutions(rows) = outcome else {
                        panic!("nested native absence");
                    };
                    assert_eq!(
                        rows.len(),
                        if drops_closed && closed { 0 } else { 2 },
                        "{query}; closed={closed}"
                    );
                    for row in rows.rows() {
                        assert_eq!(row.evidence().is_undecided(), !closed);
                        if closed {
                            row.evidence()
                                .with_record(|record| {
                                    assert_eq!(
                                        record.information().fallback(),
                                        Information::Supported
                                    );
                                })
                                .unwrap();
                        }
                    }
                };
                native_owner.lock().unwrap().take();
                let options = QueryOptions::default().with_env(&env);
                if scope.is_some() {
                    engine
                        .query_interned_evidence_view(
                            &dataset,
                            request(&query),
                            options,
                            &source,
                            check,
                        )
                        .unwrap();
                } else {
                    engine
                        .query_interned_view(&dataset, request(&query), options, check)
                        .unwrap();
                }
                assert!(
                    native_owner.lock().unwrap().is_some(),
                    "the declared producer actually ran"
                );
            }
        }
    }

    // Cross the production worker threshold with original native search attached.
    let mut wide_bind = String::from("SELECT ?doc ?value WHERE { VALUES ?seed { ");
    for ordinal in 0..1025 {
        use std::fmt::Write as _;
        write!(wide_bind, "{ordinal} ").unwrap();
    }
    wide_bind.push_str("} ?doc <https://example.org/declared> \"needle\" BIND(1 AS ?value) }");
    for (incomplete, unclosed_search) in [(true, false), (false, true)] {
        let completeness = if incomplete {
            SearchCompleteness::Undetermined
        } else {
            SearchCompleteness::Complete
        };
        let source_bound = [(
            purrdf_core::evidence::aggregate_column_identity("source"),
            purrdf_core::evidence::AggregateBound::Exact,
        )];
        for emitted in [false, true] {
            for clear_search in [false, true] {
                let search_record = EvidenceRecord::from_parts(
                    &InformationMap::default(),
                    completeness,
                    &Frame::actual(),
                    &[],
                    if incomplete { &[] } else { &source_bound },
                    &mut Memory::new(&mut Resident),
                )
                .unwrap();
                let mut registry = PropertyFunctionRegistry::new();
                let relation = Arc::new(DeclaredRelation {
                    inner: MemoryRelation::new(
                        1,
                        1,
                        if emitted {
                            vec![vec![
                                TermValue::iri("https://example.org/document"),
                                TermValue::simple_literal("needle"),
                            ]]
                        } else {
                            Vec::new()
                        },
                    )
                    .unwrap(),
                    record: EvidenceRecord::default(),
                    rank: None,
                    membership: Membership::Established,
                    alternative: None,
                    search: Some(search_record),
                    fail_search: false,
                    visibility: if clear_search {
                        SearchVisibility::OpeningOnly
                    } else if emitted {
                        SearchVisibility::AfterPull
                    } else {
                        SearchVisibility::Opening
                    },
                    unclosed_search,
                    query: Arc::clone(&native_owner),
                });
                let registered = Arc::clone(&relation);
                registry.register("https://example.org/declared", registered);
                let original_record = relation
                    .search
                    .as_ref()
                    .expect("original native search declaration");
                let env = ExtensionEnv::over_relations(registry).unwrap();
                native_owner.lock().unwrap().take();
                let options = QueryOptions::default().with_env(&env);
                let prepared = engine
                .prepare_query_with_options(
                    "SELECT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" } LIMIT 1",
                    None,
                    options,
                )
                .unwrap();
                let mut cursor = engine.open_call_cursor(&prepared, options).unwrap();
                cursor
                    .search_evidence()
                    .with_record(|record| {
                        assert_eq!(
                            record.completeness(),
                            if emitted && !clear_search {
                                SearchCompleteness::Complete
                            } else {
                                completeness
                            }
                        );
                    })
                    .unwrap();
                let row = cursor.next_admitted_row(&dataset).unwrap();
                assert_eq!(row.is_some(), emitted);
                if let Some(row) = row {
                    row.evidence
                        .with_record(|record| {
                            assert_eq!(record.completeness(), SearchCompleteness::Complete);
                        })
                        .unwrap();
                }
                // The emitted invocation stops at its original LIMIT before another
                // producer pull. Settling cannot fabricate closure for that prefix.
                assert!(cursor.next_admitted_row(&dataset).unwrap().is_none());
                let search = cursor.settled_search_evidence().unwrap();
                assert_eq!(search.permits_closed_absence(), !unclosed_search);
                let owner = native_owner.lock().unwrap().as_ref().unwrap().clone();
                owner
                    .combine(
                        &search,
                        &owner.retain(original_record).unwrap(),
                        purrdf_core::evidence::EvidenceCombination::Conjunction,
                    )
                    .expect("on-demand empty search retains the actual native owner");
                for (query, expected_rows) in [
                    (
                        "SELECT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" }",
                        usize::from(emitted),
                    ),
                    (
                        "SELECT ?doc ?value WHERE { ?doc <https://example.org/declared> \"needle\" BIND(1 AS ?value) }",
                        usize::from(emitted),
                    ),
                    (wide_bind.as_str(), if emitted { 1025 } else { 0 }),
                ] {
                    native_owner.lock().unwrap().take();
                    engine.query_interned_view(
                &dataset,
                request(query),
                options,
                |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else { panic!("native row/search parity"); };
                    assert_eq!(rows.len(), expected_rows);
                    for row in rows.rows() {
                        row.evidence().with_record(|record| {
                            assert_eq!(record.completeness(), SearchCompleteness::Complete);
                        }).unwrap();
                    }
                    rows.search_evidence().with_record(|record| {
                        assert_eq!(record.completeness(), completeness);
                    }).unwrap();
                    assert_eq!(rows.search_evidence().permits_closed_absence(), !unclosed_search);
                    let owner = native_owner.lock().unwrap().as_ref().unwrap().clone();
                    owner.combine(rows.search_evidence(), &owner.retain(original_record).unwrap(),
                        purrdf_core::evidence::EvidenceCombination::Conjunction)
                        .expect("native search owner survives materialized BIND");
                },
            ).expect("materialized rows retain original complete witnesses separately from whole search");
                }
                let mut functions = purrdf_sparql_eval::user_fn::UserFunctionRegistry::default();
                functions.insert(
                    "https://example.org/native-unknown-ask",
                    purrdf_sparql_eval::user_fn::UserFunction {
                        params: Vec::new(),
                        required: 0,
                        body: Arc::from("ASK { ?doc <https://example.org/declared> \"needle\" }"),
                        kind: purrdf_sparql_eval::user_fn::UserFnBody::Ask,
                        return_constraint: purrdf_sparql_eval::user_fn::TypeConstraint::default(),
                    },
                );
                let functions = engine.bind_functions(functions, &env).unwrap();
                for query in [
                    "SELECT (COUNT(?doc) AS ?count) WHERE { ?doc <https://example.org/declared> \"needle\" }",
                    "SELECT ?value WHERE { BIND(EXISTS { ?doc <https://example.org/declared> \"needle\" } AS ?value) }",
                    "SELECT ?value WHERE { BIND(IF(true, EXISTS { ?doc <https://example.org/declared> \"needle\" }, false) AS ?value) }",
                    "SELECT ?value WHERE { BIND(<https://example.org/native-unknown-ask>() AS ?value) }",
                    "SELECT ?value WHERE { BIND(IF(true, <https://example.org/native-unknown-ask>(), false) AS ?value) }",
                ] {
                    let check = |outcome: InternedOutcome<'_, '_, _>| {
                        let InternedOutcome::Solutions(rows) = outcome else {
                            panic!("original native whole search");
                        };
                        assert_eq!(rows.len(), 1);
                        let count = query.contains("COUNT");
                        rows.rows()[0]
                            .evidence()
                            .with_record(|record| {
                                assert_eq!(
                                    record.completeness(),
                                    if count || !emitted {
                                        completeness
                                    } else {
                                        SearchCompleteness::Complete
                                    }
                                );
                                assert_eq!(
                                    record.information().fallback(),
                                    if count || emitted {
                                        Information::Supported
                                    } else {
                                        Information::Neither
                                    }
                                );
                                if count {
                                    assert_eq!(
                                        record.aggregate_bounds(),
                                        &[(
                                            purrdf_core::evidence::aggregate_column_identity(
                                                "count"
                                            ),
                                            purrdf_core::evidence::AggregateBound::Lower,
                                        )]
                                    );
                                }
                            })
                            .unwrap();
                        let value = rows.try_value_of(rows.rows()[0][0].unwrap()).unwrap();
                        let TermValue::Literal { lexical_form, .. } = value else {
                            panic!("actual COUNT/EXISTS/ASK value");
                        };
                        assert_eq!(
                            lexical_form,
                            if count {
                                if emitted { "1" } else { "0" }
                            } else if emitted {
                                "true"
                            } else {
                                "false"
                            }
                        );
                        let owner = native_owner.lock().unwrap().as_ref().unwrap().clone();
                        let original = owner.retain(original_record).unwrap();
                        owner.combine(rows.rows()[0].evidence(), &original,
                            purrdf_core::evidence::EvidenceCombination::Conjunction)
                            .expect("original empty/emitted search owner retained through production callers");
                    };
                    let options = QueryOptions::default()
                        .with_env(&env)
                        .with_functions(&functions);
                    native_owner.lock().unwrap().take();
                    engine
                    .query_interned_view(&dataset, request(query), options, check)
                    .expect(
                        "native whole search needs no external EvidenceInput, on native and WASM",
                    );
                    native_owner.lock().unwrap().take();
                    let state = Arc::new(purrdf_sparql_eval::GovernorState::new(
                        &purrdf_sparql_eval::QueryGovernors::METERED,
                    ));
                    let result = engine
                        .query_governed_interned_in_operation(
                            &dataset,
                            request(query),
                            options,
                            &state,
                            check,
                        )
                        .expect(
                            "admitted empty and emitted native search retains its original owner",
                        );
                    assert!(matches!(
                        result,
                        purrdf_sparql_eval::InternedGoverned::Complete { .. }
                    ));
                }
            }
        }
    }

    for clear_search in [false, true] {
        let search_record = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap();
        native_owner.lock().unwrap().take();
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            "https://example.org/declared",
            Arc::new(DeclaredRelation {
                inner: MemoryRelation::new(
                    1,
                    1,
                    if clear_search {
                        vec![vec![
                            TermValue::iri("https://example.org/document"),
                            TermValue::simple_literal("needle"),
                        ]]
                    } else {
                        Vec::new()
                    },
                )
                .unwrap(),
                record: EvidenceRecord::default(),
                rank: None,
                membership: Membership::Established,
                alternative: None,
                search: Some(search_record),
                fail_search: true,
                visibility: if clear_search {
                    SearchVisibility::OpeningOnly
                } else {
                    SearchVisibility::Opening
                },
                unclosed_search: false,
                query: Arc::clone(&native_owner),
            }),
        );
        let env = ExtensionEnv::over_relations(registry).unwrap();
        let mut published = false;
        let workspace = WorkspaceCapability::resident();
        let options = QueryOptions::default().with_env(&env);
        let prepared = engine
            .prepare_query_with_options(
                "SELECT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" }",
                None,
                options,
            )
            .unwrap();
        let Err(error) = engine.open_call_cursor_admitted(&prepared, options, &workspace) else {
            panic!("original opening search failure is typed on demand");
        };
        assert!(matches!(
            error,
            purrdf_sparql_eval::EvaluationFailure::Evaluation(EvalError::AllocationFailed {
                construct: "original native search declaration"
            })
        ));
        native_owner.lock().unwrap().take();
        let mut execution = engine.prepare_execution_admitted(
        "SELECT (COUNT(?doc) AS ?count) WHERE { ?doc <https://example.org/declared> \"needle\" }",
        None, &[], options, &workspace,
    ).unwrap();
        let error = engine
            .execute_witnessed_admitted(&mut execution, &dataset, options, None, &workspace, |_| {
                published = true;
            })
            .expect_err("native whole-search operational failure must precede publication");
        assert!(!published);
        assert!(matches!(
            error,
            purrdf_sparql_eval::EvaluationFailure::Evaluation(EvalError::AllocationFailed {
                construct: "original native search declaration"
            })
        ));
    }

    let (dataset, source) = gap_dataset();
    for (ordinal, owned) in [
        (Some(1), false),
        (Some(2), false),
        (Some(1), true),
        (Some(2), true),
        (None, true),
    ] {
        native_owner.lock().unwrap().take();
        let rank = ordinal.map(|ordinal| core::num::NonZeroU64::new(ordinal).unwrap());
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            "https://example.org/declared",
            Arc::new(DeclaredRelation {
                inner: MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        TermValue::iri("https://example.org/document"),
                        TermValue::simple_literal("needle"),
                    ]],
                )
                .unwrap(),
                record: if owned {
                    EvidenceRecord::from_parts(
                        &InformationMap::default(),
                        SearchCompleteness::Undetermined,
                        &Frame::actual(),
                        &[],
                        &[],
                        &mut Memory::new(&mut Resident),
                    )
                    .unwrap()
                } else {
                    EvidenceRecord::default()
                },
                rank,
                membership: if owned {
                    Membership::OwnedComplete
                } else {
                    Membership::Established
                },
                alternative: None,
                search: None,
                fail_search: false,
                visibility: SearchVisibility::Opening,
                unclosed_search: false,
                query: Arc::clone(&native_owner),
            }),
        );
        let env = ExtensionEnv::over_relations(registry).unwrap();
        // The raw production cursor must refuse even semantically ordinary
        // evidence when its original owner or rank still needs transport.
        let engine = NativeSparqlEngine::new();
        let options = QueryOptions::default().with_env(&env);
        let prepared = engine
            .prepare_query_with_options(
                "SELECT ?doc WHERE { ?doc <https://example.org/declared> \"needle\" }",
                None,
                options,
            )
            .unwrap();
        let mut raw = engine.open_call_cursor(&prepared, options).unwrap();
        assert!(matches!(
            raw.next_row(&dataset),
            Err(EvalError::WorkspaceUnpriced(
                "a raw property read cannot discard original row evidence"
            ))
        ));
        drop(raw);
        native_owner.lock().unwrap().take();
        NativeSparqlEngine::new().query_interned_view(
            &dataset,
            request("SELECT DISTINCT ?doc ?kept WHERE { ?doc <https://example.org/declared> \"needle\" BIND(true AS ?kept) ?other <https://example.org/value> ?value FILTER(?kept) OPTIONAL { ?doc <https://example.org/missing> ?absent FILTER(true) } } ORDER BY ?kept"),
            QueryOptions::default().with_env(&env),
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else { panic!("ordinary API ranked SELECT"); };
                assert_eq!(rows.len(), 1);
                assert!(rows.rows()[0].evidence().is_default());
                assert_eq!(rows.rows()[0].evidence().original_rank(), rank);
                assert!(!rows.rows()[0].evidence().is_virtual());
                if owned {
                    let query = native_owner.lock().unwrap().as_ref().unwrap().clone();
                    let transported = query.with_completeness(rows.rows()[0].evidence(),
                        SearchCompleteness::Complete, None).unwrap();
                    assert!(transported.is_default());
                    assert!(!transported.is_virtual());
                    assert_eq!(transported.original_rank(), rank);
                }
                let workspace = WorkspaceCapability::resident();
                let raw = AdmittedPfRow::new(Vec::new(), workspace.charge(0).unwrap())
                    .with_evidence(rows.rows()[0].evidence().clone());
                assert!(matches!(raw.try_into_resident(), Err(EvalError::WorkspaceUnpriced(
                    "raw property-function cells cannot transport original row evidence"
                ))));
            },
        ).expect("physical rank attachment survives ordinary BGP, BIND and projection");
        native_owner.lock().unwrap().take();
        NativeSparqlEngine::new().query_interned_evidence_view(
            &dataset,
            request("SELECT DISTINCT ?doc ?kept WHERE { ?doc <https://example.org/declared> \"needle\" BIND(true AS ?kept) FILTER(?kept) OPTIONAL { ?doc <https://example.org/missing> ?absent FILTER(true) } } ORDER BY ?kept"),
            QueryOptions::default().with_env(&env),
            &source,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else { panic!("original supported ranked row"); };
                assert_eq!(rows.len(), 1);
                let clone = rows.rows()[0].clone();
                assert!(clone.evidence().is_default());
                assert_eq!(clone.evidence().original_rank(), rank);
                rows.with_accepted_rows(&[], |merged| {
                    let merged: Vec<_> = merged.map(|row| row.expect("original accepted merge")).collect();
                    assert_eq!(merged.len(), 1);
                    assert!(std::ptr::eq(merged[0].row, &raw const rows.rows()[0]));
                    assert!(!merged[0].is_accepted());
                    assert_eq!(merged[0].row.evidence().original_rank(), rank);
                }).expect("original supported rank survives operators and acceptance");
            },
        ).expect("default-record rank uses the actual producer and operators");
        if owned {
            native_owner.lock().unwrap().take();
            let claims = Claims {
                negative: dataset
                    .term_id_by_value(&TermValue::iri("https://example.org/two"))
                    .unwrap(),
                record: EvidenceRecord::from_parts(
                    &InformationMap::uniform(Information::Opposed),
                    SearchCompleteness::Complete,
                    &Frame::actual(),
                    &[],
                    &[],
                    &mut Memory::new(&mut Resident),
                )
                .unwrap(),
                fail: false,
                completeness: SearchCompleteness::Complete,
                sum_nonnegative: false,
                closure_declared: true,
            };
            NativeSparqlEngine::new().query_interned_evidence_view(&dataset,
                request("SELECT ?doc WHERE { ?other <https://example.org/value> ?value . ?doc <https://example.org/declared> \"needle\" }"),
                QueryOptions::default().with_env(&env), &claims, |outcome| {
                    let InternedOutcome::Solutions(rows) = outcome else { panic!("native declaration over admitted claims"); };
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows.rows()[0].evidence().original_rank(), rank);
                    rows.rows()[0].evidence().with_record(|record| {
                        assert_eq!(record.information().fallback(), Information::Opposed);
                    }).unwrap();
                    rows.with_accepted_rows(&[], |merged| {
                        let merged: Vec<_> = merged.map(|row| row.unwrap()).collect();
                        assert_eq!(merged.len(), 1);
                        assert_eq!(merged[0].row.evidence().original_rank(), rank);
                    }).unwrap();
                }).expect("native ownership binds to already admitted original claims");
        }
    }
}

struct UniformClaims(EvidenceRecord);

impl<I: purrdf_core::ViewTermId> EvidenceSource<I> for UniformClaims {
    fn claim(&self, _: &QuadIds<I>) -> Result<Option<&EvidenceRecord>, EvalError> {
        Ok(Some(&self.0))
    }

    fn scope(&self, _: GraphMatch<I>) -> Result<EvidenceScope, EvalError> {
        Ok(EvidenceScope {
            completeness: self.0.completeness(),
            closure_declared: true,
            missing_reason: None,
        })
    }

    fn reason(&self, _: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError> {
        Ok(None)
    }
}

fn uniform_claims() -> UniformClaims {
    UniformClaims(
        EvidenceRecord::from_parts(
            &InformationMap::uniform(Information::Supported),
            SearchCompleteness::Complete,
            &Frame::actual(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap(),
    )
}

fn healthy_paged_evidence_source() -> purrdf_core::PagedDataset {
    use purrdf_core::{InMemoryPageProvider, PagedDataset};
    PagedDataset::from_provider(std::sync::Arc::new(InMemoryPageProvider::new(vec![
        support::local_dataset([("a", "p", "b")]),
        support::local_dataset([("b", "q", "c")]),
    ])))
    .unwrap()
}

fn fallible_evidence_doors_keep_original_rows_and_all_query_form_status() {
    use purrdf_core::PagedQueryLimits;
    use purrdf_core::evidence::{AnswerRequest, AnswerTier, StandpointKey};
    let paged = healthy_paged_evidence_source();
    let engine = NativeSparqlEngine::new();
    let claims = uniform_claims();
    for form in ["SELECT ?s", "ASK", "CONSTRUCT { ?s ?p ?o }", "DESCRIBE ?s"] {
        let query = format!("{form} WHERE {{ ?s ?p ?o }}");
        for tier in [AnswerTier::Verdict, AnswerTier::Summary, AnswerTier::Rows] {
            let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
            let answer = AnswerRequest {
                tier,
                question: None,
                standpoint: StandpointKey::Unscoped,
                input_completeness: SearchCompleteness::Complete,
                closure_declared: true,
            };
            let (value, evidence) = engine
                .query_evidence_answer_fallible_view(
                    &view,
                    request(&query),
                    QueryOptions::EMPTY,
                    &claims,
                    answer,
                    |outcome, rows, status| {
                        match outcome {
                            InternedOutcome::Solutions(rows) => assert_eq!(rows.len(), 2),
                            InternedOutcome::Boolean(value) => assert!(value),
                            InternedOutcome::Graph(graph) => {
                                assert!(graph.dataset_handle().quad_count() >= 2);
                            }
                        }
                        (
                            rows.len(),
                            status.information,
                            status.established,
                            status.undecided,
                        )
                    },
                )
                .unwrap();
            assert_eq!(value, (2, Information::Supported, 2, 0));
            assert_eq!(evidence.consumed_pages, 2);
            let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
            let (status, _) = engine
                .query_evidence_status_fallible_view(
                    &view,
                    request(&query),
                    QueryOptions::EMPTY,
                    &claims,
                    answer,
                    |_, status| (status.information, status.established, status.undecided),
                )
                .unwrap();
            assert_eq!(status, (Information::Supported, 2, 0));
        }
    }
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let (count, _) = engine
        .query_interned_evidence_fallible_view(
            &view,
            request("SELECT ?s WHERE { ?s ?p ?o }"),
            QueryOptions::EMPTY,
            &claims,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("SELECT rows");
                };
                for row in rows.rows() {
                    row.evidence()
                        .with_record(|record| {
                            assert_eq!(record.information().fallback(), Information::Supported);
                        })
                        .unwrap();
                }
                rows.len()
            },
        )
        .unwrap();
    assert_eq!(count, 2);
}

fn fallible_evidence_refuses_mid_query_reads_and_prepared_execution_recovers() {
    use purrdf_core::evidence::{AnswerRequest, AnswerTier, StandpointKey};
    use purrdf_core::{PageId, PagedQueryError, PagedQueryEvidence, PagedQueryLimits};
    use purrdf_sparql_eval::FallibleSparqlError;
    use support::fallible::{ScriptedFault, two_page_faulting_dataset};
    fn refused(error: FallibleSparqlError<PagedQueryError, PagedQueryEvidence>) {
        assert!(error.diagnostic().is_none());
        assert!(error.partial_answers().is_none());
        let FallibleSparqlError::Operational { error, evidence } = error else {
            panic!("the read refusal must remain operational");
        };
        assert_eq!(
            error,
            PagedQueryError::Provider {
                page: PageId(1),
                message: "object read failed".to_owned(),
            }
        );
        assert_eq!(evidence.requested_pages, vec![PageId(0), PageId(1)]);
        assert_eq!(evidence.consumed_pages, 1);
    }
    let paged = two_page_faulting_dataset(ScriptedFault::Provider);
    let engine = NativeSparqlEngine::new();
    let claims = uniform_claims();
    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let answer = AnswerRequest {
        tier: AnswerTier::Rows,
        question: None,
        standpoint: StandpointKey::Unscoped,
        input_completeness: SearchCompleteness::Complete,
        closure_declared: true,
    };
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    refused(
        engine
            .query_evidence_answer_fallible_view(
                &view,
                request(query),
                QueryOptions::EMPTY,
                &claims,
                answer,
                |_, _, _| 7,
            )
            .unwrap_err(),
    );
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    refused(
        engine
            .query_evidence_status_fallible_view(
                &view,
                request(query),
                QueryOptions::EMPTY,
                &claims,
                answer,
                |_, _| 7,
            )
            .unwrap_err(),
    );
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    refused(
        engine
            .query_interned_evidence_fallible_view(
                &view,
                request(query),
                QueryOptions::EMPTY,
                &claims,
                |_| 7,
            )
            .unwrap_err(),
    );
    let mut execution = engine
        .prepare_execution(query, None, &[], QueryOptions::EMPTY)
        .unwrap();
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    refused(
        engine
            .execute_witnessed_evidence_fallible(
                &mut execution,
                &view,
                QueryOptions::EMPTY,
                &claims,
                |_| 7,
            )
            .unwrap_err(),
    );
    let healthy = healthy_paged_evidence_source();
    let view = healthy.query_view(PagedQueryLimits::UNBOUNDED);
    let ((count, _witness), evidence) = engine
        .execute_witnessed_evidence_fallible(
            &mut execution,
            &view,
            QueryOptions::EMPTY,
            &claims,
            |outcome| match outcome {
                InternedOutcome::Solutions(rows) => rows.len(),
                _ => panic!("prepared SELECT"),
            },
        )
        .unwrap();
    assert_eq!(count, 2);
    assert_eq!(evidence.consumed_pages, 2);
}

purrdf_testkit::harness_main!(
    fallible_evidence_doors_keep_original_rows_and_all_query_form_status,
    fallible_evidence_refuses_mid_query_reads_and_prepared_execution_recovers,
    verdict_stops_the_actual_answer_producer_after_filters_and_exact_polarities,
    actual_claim_ingress_inherits_sharper_standpoints_without_scoping_unspecified_claims,
    service_rows_keep_signed_and_undecided_declarations_through_original_operators,
    empty_service_keeps_search_reason_without_inventing_an_answer_row,
    sparql_function_suspensions_retain_original_select_ask_and_nested_body_evidence,
    distinct_coalesces_both_polarities_without_changing_binding_identity,
    source_refusal_never_publishes_partial_or_undetermined_rows,
    unmatched_optional_preserves_information_and_declared_missing_scope,
    group_bounds_follow_whole_scope_and_declared_sum_domain,
    nested_projection_retains_only_visible_aggregate_aliases,
    registered_lossy_search_keeps_original_candidates_and_caller_reason,
    declared_filter_gap_retains_original_row_and_reason_without_promoting_completeness,
    all_query_forms_publish_original_unknown_status_at_requested_count_tiers,
    minus_absence_requires_closure_but_disjoint_domains_are_decidable,
    minus_negates_all_original_witnesses_under_declared_closure,
    exists_transports_original_witnesses_through_vm_memos_and_deferred_sites,
    actual_unknown_answer_exposes_queryable_original_rdf_ledger,
    acceptance_merges_original_order_without_replaying_cells_or_losing_scores,
    native_paths_keep_derivation_polarity_and_complete_found_witnesses,
    native_property_rows_retain_original_declarations_through_normal_ingestion,
);
