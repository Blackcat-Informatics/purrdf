// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Candidate work depths reach native reads; every original rank/evidence survives.

mod common;
#[path = "support/registry.rs"]
mod registry;

use purrdf_core::{FastMap, TermValue, datatype::XSD_INTEGER};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDepths, CandidateExecutionResult, CandidatePlan,
    CandidatePlanError, DuplicatePolicy, IndexGeneration, Iri, OrderFidelity, PfAttestation,
    PlanId, ProducerStatus, ProtocolError, RankFidelity, ReadSchedule, ReadStratum, RequestTerm,
    ServiceLevel, Statistics, StreamContract, StreamEnding, Term, UnionError, block_on,
    compile_candidates, execute, execute_within, plan_candidates, union,
};
use purrdf_sparql_eval::{
    BindingPattern, Completeness, DepthPlacement, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, TermKind, TermPattern, Volatility,
};
use registry::{lexical_term, ranked};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn iri(local: &str) -> Iri {
    Iri::parse(&format!("http://example.org/{local}")).unwrap()
}
struct NoDepthStatistics;
impl Statistics for NoDepthStatistics {
    fn source(&self) -> &'static str {
        "caller-metadata-only"
    }
    fn revision(&self) -> &'static str {
        "r1"
    }
    fn cardinality(&self, _: &Iri) -> Option<u64> {
        panic!("candidate work depths must never consult cardinality");
    }
    fn selectivity_ppm(&self, _: &Iri, _: &RequestTerm) -> Option<u64> {
        panic!("candidate work depths must never consult estimated selectivity");
    }
}

fn mocks() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    for (stratum, values) in [
        ("lexical", vec!["a", "b", "c", "d"]),
        ("vector", vec!["b", "a", "e"]),
    ] {
        registry.register_ranked(
            format!("http://example.org/pf/{stratum}"),
            Arc::new(registry::MockProducer {
                arity: PfArity::new(1, 1),
                mode: PfArity::new(1, 1).all_free_mode(),
                rows: 20,
                emitted: values
                    .into_iter()
                    .map(|value| {
                        vec![
                            TermValue::iri(format!("http://example.org/{value}")),
                            TermValue::simple_literal("echo"),
                        ]
                    })
                    .collect(),
            }),
            ranked(
                &format!("http://example.org/{stratum}"),
                vec![TermPattern::of_kind(TermKind::Literal)],
                false,
            ),
        );
    }
    registry
}

#[test]
fn independent_depths_compile_and_run_identical_prefixes_under_both_schedules() {
    let registry = mocks();
    let depths = CandidateDepths::new([(iri("lexical"), 3), (iri("vector"), 1)]).unwrap();
    let plan = plan_candidates(&[lexical_term()], &depths, &registry, &NoDepthStatistics).unwrap();
    let decoded = CandidatePlan::from_canonical_bytes(&plan.canonical_bytes()).unwrap();
    assert_eq!(plan, decoded);
    assert_eq!(plan.canonical_bytes(), decoded.canonical_bytes());
    let compiled = compile_candidates(
        &plan,
        &AdmissionEnvironment {
            registry: &registry,
            statistics: &NoDepthStatistics,
            fusion_profile: None,
        },
    )
    .unwrap();
    for unit in compiled.units() {
        assert_eq!(Some(unit.depth()), depths.get(&unit.stratum));
        assert!(
            unit.sparql()
                .contains(&format!("LIMIT {}", unit.depth() + 1))
        );
    }
    let mut observed = None;
    for schedule in [ReadSchedule::Materialised, ReadSchedule::OnDemand] {
        let executed = block_on(execute_within(
            &compiled,
            &registry,
            common::empty_dataset(),
            schedule,
        ))
        .unwrap();
        let result = block_on(union(executed, &depths)).unwrap();
        assert!(result.completed_prefix);
        assert_eq!(result.candidates.len(), 3);
        assert_eq!(
            result.candidates[&Term::new("<http://example.org/b>")].ranks,
            BTreeMap::from([(iri("lexical"), vec![2]), (iri("vector"), vec![1])])
        );
        assert!(
            !result
                .candidates
                .contains_key(&Term::new("<http://example.org/e>"))
        );
        for stratum in ["lexical", "vector"] {
            let evidence = &result.producers[&iri(stratum)];
            let depth = depths.get(&iri(stratum)).unwrap();
            assert_eq!(evidence.requested_depth, depth);
            assert_eq!(evidence.rows_pulled, u64::from(depth));
            assert_eq!(evidence.rows_materialised, Some(u64::from(depth) + 1));
            assert_eq!(
                evidence.status,
                Some(ProducerStatus::DepthReached {
                    rank: u64::from(depth)
                })
            );
            assert!(evidence.receipt_verified);
        }
        if let Some(previous) = &observed {
            assert_eq!(previous, &result);
        }
        observed = Some(result);
    }
}

#[test]
fn zero_is_an_explicit_empty_prefix_and_a_different_question() {
    let registry = mocks();
    let depths = CandidateDepths::new([(iri("lexical"), 0), (iri("vector"), 0)]).unwrap();
    let plan = plan_candidates(&[lexical_term()], &depths, &registry, &NoDepthStatistics).unwrap();
    let compiled = compile_candidates(
        &plan,
        &AdmissionEnvironment {
            registry: &registry,
            statistics: &NoDepthStatistics,
            fusion_profile: None,
        },
    )
    .unwrap();
    assert!(
        compiled
            .units()
            .iter()
            .all(|unit| unit.sparql().contains("LIMIT 1"))
    );
    let result = block_on(union(
        block_on(execute(&compiled, &registry, common::empty_dataset())).unwrap(),
        &depths,
    ))
    .unwrap();
    assert!(result.candidates.is_empty());
    assert!(result.completed_prefix);
    assert!(
        result
            .producers
            .values()
            .all(|producer| producer.rows_materialised == Some(1))
    );
    let changed = CandidateDepths::new([(iri("lexical"), 1), (iri("vector"), 0)]).unwrap();
    assert_ne!(
        plan.id(),
        plan_candidates(&[lexical_term()], &changed, &registry, &NoDepthStatistics)
            .unwrap()
            .id()
    );
}

struct ArgumentProducer {
    mode: BindingPattern,
    asked: Arc<AtomicU64>,
}
impl PropertyFunction for ArgumentProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn arity(&self) -> PfArity {
        PfArity::new(1, 2)
    }
    fn modes(&self) -> &[BindingPattern] {
        std::slice::from_ref(&self.mode)
    }
    fn rows_per_invocation(&self, _: BindingPattern) -> u64 {
        2
    }
    fn open(&self, args: &PfArgs<'_>, _: Option<u64>) -> Result<Box<dyn PfCursor>, EvalError> {
        let args: Vec<_> = args.flattened().collect();
        let Some(TermValue::Literal { lexical_form, .. }) = args[2] else {
            panic!("actual compiled depth argument must be bound");
        };
        let depth: u64 = lexical_form.parse().unwrap();
        self.asked.store(depth, Ordering::SeqCst);
        assert_eq!(
            depth, 2,
            "the actual producer cap, not the deeper caller prefix"
        );
        Ok(Box::new(registry::RowCursor {
            rows: vec![
                vec![
                    TermValue::iri("http://example.org/a"),
                    args[1].unwrap().clone(),
                    args[2].unwrap().clone(),
                ],
                vec![
                    TermValue::iri("http://example.org/b"),
                    args[1].unwrap().clone(),
                    args[2].unwrap().clone(),
                ],
            ]
            .into_iter(),
        }))
    }
}

#[test]
fn a_requested_prefix_above_the_producer_cap_is_reported_unfulfilled_not_exhausted() {
    let asked = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::new();
    let mut declaration = ranked(
        "http://example.org/capped",
        vec![TermPattern::of_kind(TermKind::Literal)],
        false,
    );
    declaration.depth_placement = Some(DepthPlacement {
        position: 2,
        datatype: XSD_INTEGER.to_owned(),
    });
    registry.register_ranked(
        "http://example.org/pf/capped",
        Arc::new(ArgumentProducer {
            mode: PfArity::new(1, 2).all_free_mode(),
            asked: asked.clone(),
        }),
        declaration,
    );
    let depths = CandidateDepths::new([(iri("capped"), 7)]).unwrap();
    let plan = plan_candidates(&[lexical_term()], &depths, &registry, &NoDepthStatistics).unwrap();
    assert_eq!(plan.declared_rows()[&iri("capped")], 2);
    let compiled = compile_candidates(
        &plan,
        &AdmissionEnvironment {
            registry: &registry,
            statistics: &NoDepthStatistics,
            fusion_profile: None,
        },
    )
    .unwrap();
    assert_eq!(compiled.units()[0].depth(), 7);
    assert!(compiled.units()[0].sparql().contains("LIMIT 8"));
    for schedule in [ReadSchedule::Materialised, ReadSchedule::OnDemand] {
        let result = block_on(union(
            block_on(execute_within(
                &compiled,
                &registry,
                common::empty_dataset(),
                schedule,
            ))
            .unwrap(),
            &depths,
        ))
        .unwrap();
        assert_eq!(asked.load(Ordering::SeqCst), 2);
        assert_eq!(
            result.producers[&iri("capped")].status,
            Some(ProducerStatus::RowBoundReached { rank: 2 })
        );
        assert!(!result.completed_prefix);
        assert_eq!(result.candidates.len(), 2);
    }
}

fn assembled(
    rows: Vec<(u64, Term, purrdf_retrieval::RowBlock)>,
    ending: StreamEnding,
    duplicates: DuplicatePolicy,
) -> (CandidateExecutionResult<'static>, CandidateDepths) {
    let stratum = iri("scripted");
    let depths = CandidateDepths::new([(stratum.clone(), 3)]).unwrap();
    let contract = StreamContract::new(
        duplicates,
        RankFidelity::EXACT,
        purrdf_retrieval::CandidateDomains::Unrestricted,
        purrdf_retrieval::ExclusionBasis::Unavailable,
    );
    let plan_id = PlanId::from_canonical(b"caller-scripted-candidate-prefix");
    (
        CandidateExecutionResult {
            streams: vec![ReadStratum {
                stratum: stratum.clone(),
                plan_id,
                requested_depth: 3,
                contract: contract.clone(),
                attestation: PfAttestation::UNDECLARED,
                stream: purrdf_retrieval::RankedStreamImpl::new(rows, ending),
            }],
            statuses: FastMap::default(),
            contracts: std::iter::once((stratum, contract)).collect(),
            plan_id,
            depths: depths.clone(),
            unserved_terms: Vec::new(),
            producer_bindings: Vec::new(),
            producer_decisions: Vec::new(),
        },
        depths,
    )
}

#[test]
fn every_duplicate_rank_is_kept_and_a_unique_promise_is_refused() {
    let rows = vec![
        (1, Term::new("a"), purrdf_retrieval::RowBlock::Undeclared),
        (2, Term::new("a"), purrdf_retrieval::RowBlock::Undeclared),
        (3, Term::new("b"), purrdf_retrieval::RowBlock::Undeclared),
    ];
    let (execution, depths) = assembled(
        rows.clone(),
        StreamEnding::Exhausted,
        DuplicatePolicy::Allowed,
    );
    let result = block_on(union(execution, &depths)).unwrap();
    assert!(result.completed_prefix);
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(
        result.candidates[&Term::new("a")].ranks[&iri("scripted")],
        [1, 2]
    );
    let (execution, depths) = assembled(rows, StreamEnding::Exhausted, DuplicatePolicy::Unique);
    let failure = block_on(union(execution, &depths)).unwrap_err();
    assert!(
        matches!(failure.cause, UnionError::Protocol(ProtocolError::DuplicateItem { ref item, .. }) if item == "a")
    );
    assert!(!failure.partial.completed_prefix);
    assert_eq!(
        failure.partial.candidates[&Term::new("a")].ranks[&iri("scripted")],
        [1]
    );
}

#[test]
fn malformed_rank_receipt_and_depths_never_certify_a_candidate_prefix() {
    let (execution, depths) = assembled(
        vec![(2, Term::new("a"), purrdf_retrieval::RowBlock::Undeclared)],
        StreamEnding::Exhausted,
        DuplicatePolicy::Unique,
    );
    assert!(matches!(
        block_on(union(execution, &depths)).unwrap_err().cause,
        UnionError::Protocol(ProtocolError::NonContiguousRanks { gap: 1 })
    ));
    let (execution, depths) = assembled(
        vec![(1, Term::new("a"), purrdf_retrieval::RowBlock::Undeclared)],
        StreamEnding::DepthReached { rank: 3 },
        DuplicatePolicy::Unique,
    );
    let error = block_on(union(execution, &depths)).unwrap_err();
    assert!(matches!(
        error.cause,
        UnionError::Protocol(ProtocolError::ForgedReceipt {
            declared: 3,
            actual: 1
        })
    ));
    assert_eq!(
        error.partial.producers[&iri("scripted")].rows_emitted,
        Some(1)
    );
    let (execution, _) = assembled(Vec::new(), StreamEnding::Exhausted, DuplicatePolicy::Unique);
    let other = CandidateDepths::new([(iri("scripted"), 2)]).unwrap();
    assert!(matches!(
        block_on(union(execution, &other)).unwrap_err().cause,
        UnionError::DepthMismatch { .. }
    ));
}

#[test]
fn failed_and_lossy_short_producers_remain_distinct_from_completed_prefixes() {
    let (mut execution, depths) = assembled(
        vec![(1, Term::new("a"), purrdf_retrieval::RowBlock::Undeclared)],
        StreamEnding::Exhausted,
        DuplicatePolicy::Unique,
    );
    let contract = StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from("caller approximate search"),
            },
            order: OrderFidelity::Faithful,
        },
        purrdf_retrieval::CandidateDomains::Unrestricted,
        purrdf_retrieval::ExclusionBasis::Unavailable,
    );
    execution.streams[0].contract = contract.clone();
    execution.contracts.insert(iri("scripted"), contract);
    execution.streams[0].attestation = PfAttestation {
        generation: IndexGeneration::declared("g7"),
        service: ServiceLevel::Incomplete {
            reason: "missing shard".to_owned(),
        },
    };
    // Caller streams settle no native witness; this stays an explicit absence.
    let result = block_on(union(execution, &depths)).unwrap();
    assert!(result.completed_prefix);
    assert!(
        result.producers[&iri("scripted")]
            .contract
            .as_ref()
            .unwrap()
            .fidelity
            .may_omit()
    );
    assert_eq!(
        result.producers[&iri("scripted")]
            .announced
            .as_ref()
            .unwrap()
            .service,
        ServiceLevel::Incomplete {
            reason: "missing shard".to_owned()
        }
    );
    assert!(result.producers[&iri("scripted")].settled.is_none());
    let (mut execution, depths) =
        assembled(Vec::new(), StreamEnding::Exhausted, DuplicatePolicy::Unique);
    execution.streams.clear();
    execution.statuses.insert(
        iri("scripted"),
        ProducerStatus::ExecutionFailed {
            reason: "host denied".to_owned(),
        },
    );
    let result = block_on(union(execution, &depths)).unwrap();
    assert!(!result.completed_prefix);
    assert_eq!(result.producers[&iri("scripted")].rows_materialised, None);
    assert!(matches!(
        result.producers[&iri("scripted")].status,
        Some(ProducerStatus::ExecutionFailed { .. })
    ));
}

#[test]
fn caller_depth_coverage_is_exact_and_never_clamped() {
    assert!(matches!(
        CandidateDepths::new([(iri("s"), 1), (iri("s"), 2)]),
        Err(CandidatePlanError::DuplicateDepth { .. })
    ));
    assert!(matches!(
        CandidateDepths::new([(iri("s"), u32::MAX)]),
        Err(CandidatePlanError::DepthWithoutProbe { .. })
    ));
    let registry = mocks();
    assert!(matches!(
        plan_candidates(
            &[lexical_term()],
            &CandidateDepths::new([(iri("lexical"), 3)]).unwrap(),
            &registry,
            &NoDepthStatistics
        ),
        Err(CandidatePlanError::MissingDepth { .. })
    ));
    assert!(matches!(
        plan_candidates(
            &[lexical_term()],
            &CandidateDepths::new([(iri("lexical"), 3), (iri("vector"), 1), (iri("absent"), 2)])
                .unwrap(),
            &registry,
            &NoDepthStatistics
        ),
        Err(CandidatePlanError::UnexpectedDepth { .. })
    ));
}

#[derive(Clone, Copy)]
enum NativeFault {
    FailsAfterOne,
    MovesAfterOne,
    BeatsDeclaration,
}

struct FaultingProducer {
    mode: BindingPattern,
    fault: NativeFault,
}
struct FaultingCursor {
    fault: NativeFault,
    needle: TermValue,
    emitted: u64,
}
impl PropertyFunction for FaultingProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }
    fn modes(&self) -> &[BindingPattern] {
        std::slice::from_ref(&self.mode)
    }
    fn rows_per_invocation(&self, _: BindingPattern) -> u64 {
        if matches!(self.fault, NativeFault::BeatsDeclaration) {
            1
        } else {
            20
        }
    }
    fn open(&self, args: &PfArgs<'_>, _: Option<u64>) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(FaultingCursor {
            fault: self.fault,
            emitted: 0,
            needle: args.flattened().nth(1).flatten().unwrap().clone(),
        }))
    }
}
impl PfCursor for FaultingCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted == 1 && matches!(self.fault, NativeFault::FailsAfterOne) {
            return Err(EvalError::function(
                "native candidate test producer refused".to_owned(),
            ));
        }
        if self.emitted == 4 {
            return Ok(None);
        }
        self.emitted += 1;
        Ok(Some(vec![
            TermValue::iri(format!("http://example.org/fault/{}", self.emitted)),
            self.needle.clone(),
        ]))
    }
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::declared(
            if self.emitted >= 1 && matches!(self.fault, NativeFault::MovesAfterOne) {
                "after"
            } else {
                "before"
            },
        )
    }
}

fn fault_registry(fault: NativeFault) -> PropertyFunctionRegistry {
    let mut registry = mocks();
    registry.register_ranked(
        "http://example.org/pf/fault",
        Arc::new(FaultingProducer {
            mode: PfArity::new(1, 1).all_free_mode(),
            fault,
        }),
        ranked(
            "http://example.org/fault",
            vec![TermPattern::of_kind(TermKind::Literal)],
            false,
        ),
    );
    registry
}

#[test]
fn an_actual_partial_native_failure_keeps_its_rank_and_healthy_sibling_evidence() {
    let registry = fault_registry(NativeFault::FailsAfterOne);
    let depths =
        CandidateDepths::new([(iri("fault"), 3), (iri("lexical"), 3), (iri("vector"), 1)]).unwrap();
    let planned =
        plan_candidates(&[lexical_term()], &depths, &registry, &NoDepthStatistics).unwrap();
    let compiled = compile_candidates(
        &planned,
        &AdmissionEnvironment {
            registry: &registry,
            statistics: &NoDepthStatistics,
            fusion_profile: None,
        },
    )
    .unwrap();
    let execution = block_on(execute_within(
        &compiled,
        &registry,
        common::empty_dataset(),
        ReadSchedule::OnDemand,
    ))
    .unwrap();
    let result = block_on(union(execution, &depths)).unwrap();
    assert!(!result.completed_prefix);
    let failed = &result.producers[&iri("fault")];
    assert_eq!(failed.rows_materialised, Some(1));
    assert_eq!(failed.rows_emitted, Some(1));
    assert!(!failed.receipt_verified && !failed.completed_prefix);
    assert!(matches!(
        failed.read_error,
        Some(ProtocolError::ReadFailed { rows_before: 1, .. })
    ));
    assert!(matches!(
        failed.status,
        Some(ProducerStatus::ExecutionFailed { .. })
    ));
    assert_eq!(
        result.candidates[&Term::new("<http://example.org/fault/1>")].ranks[&iri("fault")],
        [1]
    );
    for sibling in [iri("lexical"), iri("vector")] {
        assert!(result.producers[&sibling].completed_prefix);
    }
}

#[test]
fn a_moving_native_attestation_or_a_breached_row_declaration_refuses_the_whole_union() {
    for fault in [NativeFault::MovesAfterOne, NativeFault::BeatsDeclaration] {
        let registry = fault_registry(fault);
        let depths =
            CandidateDepths::new([(iri("fault"), 3), (iri("lexical"), 3), (iri("vector"), 1)])
                .unwrap();
        let planned =
            plan_candidates(&[lexical_term()], &depths, &registry, &NoDepthStatistics).unwrap();
        let compiled = compile_candidates(
            &planned,
            &AdmissionEnvironment {
                registry: &registry,
                statistics: &NoDepthStatistics,
                fusion_profile: None,
            },
        )
        .unwrap();
        let execution = block_on(execute_within(
            &compiled,
            &registry,
            common::empty_dataset(),
            ReadSchedule::OnDemand,
        ))
        .unwrap();
        let failure = block_on(union(execution, &depths)).unwrap_err();
        assert!(!failure.partial.completed_prefix);
        assert!(
            failure
                .partial
                .producers
                .values()
                .all(|producer| producer.rows_emitted.is_some())
        );
        match fault {
            NativeFault::MovesAfterOne => assert!(matches!(
                failure.cause,
                UnionError::Protocol(ProtocolError::AttestationMoved { .. })
            )),
            NativeFault::BeatsDeclaration => assert!(matches!(
                failure.cause,
                UnionError::Protocol(ProtocolError::ReadFailed { rows_before: 1, .. })
            )),
            NativeFault::FailsAfterOne => unreachable!("the other test covers isolated refusal"),
        }
    }
}

#[test]
fn unserved_terms_and_producer_decisions_move_unchanged_into_the_candidate_answer() {
    let registry = mocks();
    let depths = CandidateDepths::new([(iri("lexical"), 5), (iri("vector"), 5)]).unwrap();
    let terms = [
        lexical_term(),
        RequestTerm::EntitySeed {
            entity: Term::new("<http://example.org/seed>"),
        },
    ];
    let planned = plan_candidates(&terms, &depths, &registry, &NoDepthStatistics).unwrap();
    let expected_unserved = planned.unserved_evidence();
    assert_ne!(expected_unserved, [] as [purrdf_retrieval::UnservedTerm; 0]);
    let compiled = compile_candidates(
        &planned,
        &AdmissionEnvironment {
            registry: &registry,
            statistics: &NoDepthStatistics,
            fusion_profile: None,
        },
    )
    .unwrap();
    let answer = block_on(union(
        block_on(execute(&compiled, &registry, common::empty_dataset())).unwrap(),
        &depths,
    ))
    .unwrap();
    assert_eq!(answer.unserved_terms, expected_unserved);
    assert_eq!(answer.producer_bindings, planned.producer_bindings());
    assert_eq!(answer.producer_decisions, planned.producer_decisions());
    assert!(
        answer.completed_prefix,
        "each short producer actually exhausted"
    );
}
